#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json);
use File::Spec;

use lib File::Spec->catdir($Bin, 'lib');
use RR::Subsystems qw(load_subsystems_policy match_subsystems normalize_path);

my $changed_files_file;
my $title_file;
my $body_file;
my $subjects_file;
my $base;

GetOptions(
    'changed-files-file=s' => \$changed_files_file,
    'title-file=s'         => \$title_file,
    'body-file=s'          => \$body_file,
    'subjects-file=s'      => \$subjects_file,
    'base=s'               => \$base,
) or die "usage: $0 [--changed-files-file PATH] [--title-file PATH] [--body-file PATH] [--subjects-file PATH] [--base SHA]\n";

my $root = abs_path(File::Spec->catdir($Bin, '..'));
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
    open my $fh, '<', $ENV{GITHUB_EVENT_PATH} or die "failed to read $ENV{GITHUB_EVENT_PATH}: $!";
    local $/;
    my $event = decode_json(<$fh>);
    close $fh;
    return $event;
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

sub read_changed_files_from_git {
    my ($base_sha) = @_;
    my $cmd = defined $base_sha && length $base_sha
        ? qq{git -C "$root" diff --name-only "$base_sha...HEAD" --}
        : qq{{ git -C "$root" diff --name-only HEAD --; git -C "$root" diff --cached --name-only --; git -C "$root" ls-files --others --exclude-standard; }};
    my @lines = `$cmd`;
    chomp @lines;
    return [grep { defined && length } map { normalize_path($_) } @lines];
}

sub read_commit_subjects {
    my ($base_sha) = @_;
    my $cmd = defined $base_sha && length $base_sha
        ? qq{git -C "$root" log --format=%s "$base_sha..HEAD"}
        : qq{git -C "$root" log -1 --format=%s HEAD};
    my @subjects = `$cmd`;
    chomp @subjects;
    return [grep { defined && length } @subjects];
}

sub parse_sections {
    my ($body) = @_;
    my %sections;
    my $current = q{};
    for my $line (split /\n/, $body) {
        if ($line =~ /^(## .+?)\s*$/) {
            $current = $1;
            $sections{$current} = q{};
            next;
        }
        next if $current eq q{};
        $sections{$current} .= "$line\n";
    }
    return \%sections;
}

sub is_placeholder {
    my ($text) = @_;
    my $normalized = lc($text // q{});
    $normalized =~ s/^\s+|\s+$//g;
    return 1 if $normalized eq q{};
    return 1 if $normalized =~ /^(n\/a|na|none|not applicable|- none|- n\/a)$/;
    return 0;
}

my $event = read_event();
my $pr_title = $title_file ? slurp_text($title_file) : (($event->{pull_request}{title} // q{}));
my $pr_body  = $body_file  ? slurp_text($body_file)  : (($event->{pull_request}{body} // q{}));
my $base_sha = defined $base ? $base : read_base_from_event($event);
my $changed_files = $changed_files_file
    ? slurp_lines($changed_files_file)
    : read_changed_files_from_git($base_sha);
my $subjects = $subjects_file ? slurp_lines($subjects_file) : read_commit_subjects($base_sha);
my $match = match_subsystems($policy, $changed_files);
my @touched = map { $_->{name} } @{$match->{touched}};
my %allowed = map { $_ => 1 } (@{$policy->{meta}{allowed_commit_prefixes} // []}, @touched);

my @errors;
my $subject_re = qr/^([a-z0-9][a-z0-9_-]*):\s+\S/;
for my $subject (@{$subjects}) {
    if ($subject !~ $subject_re) {
        push @errors, "commit subject must look like 'subsystem: summary' -> $subject";
        next;
    }
    my $prefix = $1;
    push @errors, "commit subject prefix '$prefix' is not allowed for touched subsystems" if !$allowed{$prefix};
}

if (($ENV{GITHUB_EVENT_NAME} // q{}) eq 'pull_request' || $title_file || $body_file) {
    chomp $pr_title;
    if ($pr_title !~ $subject_re) {
        push @errors, "PR title must look like 'subsystem: summary'";
    } else {
        my $prefix = $1;
        push @errors, "PR title prefix '$prefix' is not allowed for touched subsystems" if !$allowed{$prefix};
    }

    my $sections = parse_sections($pr_body);
    for my $section (@{$policy->{meta}{required_pr_sections} // []}) {
        if (!exists $sections->{$section}) {
            push @errors, "missing PR body section $section";
            next;
        }
        push @errors, "$section must not be empty or placeholder" if is_placeholder($sections->{$section});
    }

    if (exists $sections->{'## Subsystems'}) {
        my $subsystems_text = lc($sections->{'## Subsystems'});
        for my $name (@touched) {
            push @errors, "## Subsystems must mention touched subsystem '$name'"
                if index($subsystems_text, lc($name)) < 0;
        }
    }
}

if (@errors) {
    print STDERR "commit metadata check failed:\n";
    print STDERR "  - $_\n" for @errors;
    exit 1;
}

print "commit metadata check passed\n";
exit 0;
