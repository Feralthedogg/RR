#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use File::Path qw(remove_tree);
use File::Spec;
use File::Temp qw(tempdir);
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json);

use lib File::Spec->catdir($Bin, 'lib');
use RR::Subsystems qw(load_subsystems_policy match_subsystems normalize_path);

my $base;
my $commit_list_file;
my $repo;
my $verify_buildability = 0;
my $skip_buildability = 0;
my $build_command;
my $print_plan = 0;

GetOptions(
    'base=s'              => \$base,
    'commit-list-file=s'  => \$commit_list_file,
    'repo=s'              => \$repo,
    'verify-buildability' => \$verify_buildability,
    'skip-buildability'   => \$skip_buildability,
    'build-command=s'     => \$build_command,
    'print-plan'          => \$print_plan,
) or die "usage: $0 [--repo PATH] [--base SHA] [--commit-list-file PATH] [--verify-buildability] [--skip-buildability] [--build-command CMD] [--print-plan]\n";

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
my $policy = load_subsystems_policy(File::Spec->catfile($root, 'policy', 'subsystems.toml'));

sub slurp_text {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    local $/;
    my $text = <$fh>;
    close $fh;
    return $text // q{};
}

sub slurp_lines {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @lines = map { chomp; $_ } <$fh>;
    close $fh;
    return [grep { defined && length } @lines];
}

sub read_event {
    return {} if ($ENV{GITHUB_EVENT_NAME} // q{}) eq q{};
    return {} if ($ENV{GITHUB_EVENT_PATH} // q{}) eq q{};
    return {} if !-f ($ENV{GITHUB_EVENT_PATH} // q{});
    return decode_json(slurp_text($ENV{GITHUB_EVENT_PATH}));
}

sub read_base_from_event {
    my ($event) = @_;
    if (($ENV{GITHUB_EVENT_NAME} // q{}) eq 'pull_request') {
        return $event->{pull_request}{base}{sha} // q{};
    }
    if (($ENV{GITHUB_EVENT_NAME} // q{}) eq 'push') {
        my $before = $event->{before} // q{};
        return $before =~ /^0+$/ ? q{} : $before;
    }
    return q{};
}

sub read_commits_from_git {
    my ($base_sha) = @_;
    my @commits;
    if (defined $base_sha && length $base_sha) {
        @commits = `git -C "$root" rev-list --reverse "$base_sha..HEAD"`;
    } else {
        @commits = `git -C "$root" rev-parse HEAD`;
    }
    chomp @commits;
    return [grep { defined && length } @commits];
}

sub run_git_capture {
    my (@cmd) = @_;
    my $output = qx{@cmd};
    die "command failed: @cmd\n" if $? != 0;
    return $output;
}

sub read_commit_subject_body {
    my ($commit) = @_;
    my $text = run_git_capture('git', '-C', $root, 'show', '-s', '--format=%s%n%b', $commit);
    my ($subject, @rest) = split /\n/, $text;
    my $body = join "\n", @rest;
    return ($subject // q{}, $body);
}

sub read_commit_files {
    my ($commit) = @_;
    my $text = run_git_capture('git', '-C', $root, 'diff-tree', '--root', '--no-commit-id', '--name-only', '-r', $commit, '--');
    my @files = grep { defined && length } map { chomp; normalize_path($_) } split /\n/, $text;
    return \@files;
}

sub parse_trailers {
    my ($body) = @_;
    my %trailers;
    for my $line (split /\n/, $body) {
        next if $line !~ /^([A-Za-z][A-Za-z-]+):\s*(.+)$/;
        push @{$trailers{$1}}, $2;
    }
    return \%trailers;
}

sub placeholder {
    my ($text) = @_;
    my $value = lc($text // q{});
    $value =~ s/^\s+|\s+$//g;
    return 1 if $value eq q{};
    return 1 if $value =~ /^(?:n\/a|na|none|not applicable|- none|- n\/a)$/;
    return 0;
}

sub any_matches {
    my ($paths, $prefixes) = @_;
    for my $path (@{$paths // []}) {
        for my $prefix (@{$prefixes // []}) {
            my $norm = normalize_path($prefix);
            return 1 if $path eq $norm || index($path, "$norm/") == 0 || index($path, $norm) == 0;
        }
    }
    return 0;
}

sub touches_dependency_file {
    my ($paths, $names) = @_;
    for my $path (@{$paths // []}) {
        for my $name (@{$names // []}) {
            return 1 if $path eq $name || $path =~ m{/\Q$name\E$};
        }
    }
    return 0;
}

sub check_required_trailers {
    my ($errors, $commit, $trailers, $names) = @_;
    for my $name (@{$names // []}) {
        if (!$trailers->{$name} || !@{$trailers->{$name}}) {
            push @{$errors}, "$commit missing trailer '$name:'";
            next;
        }
        if (grep { placeholder($_) } @{$trailers->{$name}}) {
            push @{$errors}, "$commit trailer '$name:' must not be placeholder text";
        }
    }
}

sub normalize_owner_text {
    my ($value) = @_;
    $value //= q{};
    $value =~ s/^@//;
    return lc $value;
}

sub verify_buildability_for_commits {
    my ($commits) = @_;
    return if !$verify_buildability || $skip_buildability;

    my $limit = ($policy->{meta}{max_buildability_commits} // '6') + 0;
    if (@{$commits} > $limit) {
        die "commit series buildability check refused: " . scalar(@{$commits}) . " commits exceeds limit $limit; split the series or rerun with fewer commits\n";
    }

    my $cmd = $build_command // ($policy->{meta}{build_command} // 'cargo check -q');
    my $tmp = tempdir('rr-commit-series.XXXXXX', TMPDIR => 1, CLEANUP => 0);
    my $target_dir = File::Spec->catdir($root, 'target', 'commit_series_check');

    system('git', '-C', $root, 'worktree', 'add', '--detach', $tmp, $commits->[0]) == 0
        or die "failed to create temporary worktree for commit series check\n";

    my $cleanup = sub {
        system('git', '-C', $root, 'worktree', 'remove', '--force', $tmp);
        remove_tree($tmp, { error => \my $err });
    };

    for my $commit (@{$commits}) {
        system('git', '-C', $tmp, 'checkout', '--detach', $commit) == 0
            or die "failed to checkout $commit in temporary worktree\n";
        my $shell = qq{cd "$tmp" && CARGO_TARGET_DIR="$target_dir" $cmd};
        my $status = system('/bin/sh', '-lc', $shell);
        if ($status != 0) {
            $cleanup->();
            die "commit $commit failed buildability command '$cmd'\n";
        }
    }

    $cleanup->();
}

my $event = read_event();
my $base_sha = defined $base ? $base : read_base_from_event($event);
my $commits = $commit_list_file ? slurp_lines($commit_list_file) : read_commits_from_git($base_sha);

if (!@{$commits}) {
    print "no commits in series scope\n";
    exit 0;
}

my @errors;
for my $commit (@{$commits}) {
    my ($subject, $body) = read_commit_subject_body($commit);
    my $files = read_commit_files($commit);
    my $match = match_subsystems($policy, $files);
    if (@{$match->{unmapped}}) {
        push @errors, "$commit has unmapped paths: " . join(', ', @{$match->{unmapped}});
        next;
    }

    my @touched = map { $_->{name} } @{$match->{touched}};
    my %allowed = map { $_ => 1 } (@{$policy->{meta}{allowed_commit_prefixes} // []}, @touched);
    if ($subject !~ /^([a-z0-9][a-z0-9_-]*):\s+\S/) {
        push @errors, "$commit subject must look like 'subsystem: summary' -> $subject";
    } elsif (!$allowed{$1}) {
        push @errors, "$commit subject prefix '$1' is not allowed for touched subsystems (" . join(', ', @touched) . ")";
    }

    my $trailers = parse_trailers($body);
    check_required_trailers(\@errors, $commit, $trailers, $policy->{meta}{required_commit_trailers});
    if (exists $trailers->{Subsystem}) {
        my $subsystem_text = lc join "\n", @{$trailers->{Subsystem}};
        for my $name (@touched) {
            push @errors, "$commit trailer 'Subsystem:' must mention touched subsystem '$name'"
                if index($subsystem_text, lc($name)) < 0;
        }
    }

    check_required_trailers(\@errors, $commit, $trailers, $policy->{meta}{perf_commit_trailers})
        if any_matches($files, $policy->{meta}{perf_sensitive_prefixes});
    check_required_trailers(\@errors, $commit, $trailers, $policy->{meta}{dependency_commit_trailers})
        if touches_dependency_file($files, $policy->{meta}{dependency_files});
    check_required_trailers(\@errors, $commit, $trailers, $policy->{meta}{exception_commit_trailers})
        if any_matches($files, $policy->{meta}{exception_sensitive_prefixes});

    if ($print_plan) {
        print "commit=$commit\n";
        print "subject=$subject\n";
        print "subsystems=" . join(',', @touched) . "\n";
        print "files=" . join(',', @{$files}) . "\n";
        print "\n";
    }
}

if (@errors) {
    print STDERR "commit series check failed:\n";
    print STDERR "  - $_\n" for @errors;
    exit 1;
}

verify_buildability_for_commits($commits);

print "commit series check passed\n";
exit 0;
