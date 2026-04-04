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
my $base;
my $print_plan = 0;

GetOptions(
    'changed-files-file=s' => \$changed_files_file,
    'base=s'               => \$base,
    'print-plan'           => \$print_plan,
) or die "usage: $0 [--changed-files-file PATH] [--base SHA] [--print-plan]\n";

my $root = abs_path(File::Spec->catdir($Bin, '..'));
my $policy = load_subsystems_policy(File::Spec->catfile($root, 'policy', 'subsystems.toml'));

sub slurp_lines {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @lines = map { chomp; $_ } <$fh>;
    close $fh;
    return [grep { defined && length } @lines];
}

sub read_base_from_event {
    return q{} if ($ENV{GITHUB_EVENT_NAME} // q{}) eq q{};
    return q{} if ($ENV{GITHUB_EVENT_PATH} // q{}) eq q{};
    open my $fh, '<', $ENV{GITHUB_EVENT_PATH} or die "failed to read $ENV{GITHUB_EVENT_PATH}: $!";
    local $/;
    my $event = decode_json(<$fh>);
    close $fh;
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

my $changed_files = $changed_files_file
    ? slurp_lines($changed_files_file)
    : read_changed_files_from_git(defined $base ? $base : read_base_from_event());

my $match = match_subsystems($policy, $changed_files);
if (@{$match->{unmapped}}) {
    print STDERR "unmapped changed files:\n";
    print STDERR "  - $_\n" for @{$match->{unmapped}};
    exit 1;
}

if ($print_plan) {
    for my $entry (@{$match->{touched}}) {
        print "subsystem=$entry->{name}\n";
        print "owners=" . join(',', @{$entry->{owners}}) . "\n";
        print "checks=" . join(',', @{$entry->{checks}}) . "\n";
        print "paths=" . join(',', @{$entry->{paths}}) . "\n";
        print "\n";
    }
}

if (!@{$match->{touched}}) {
    print "no subsystem-mapped files in scope\n";
    exit 0;
}

print "touched subsystems:\n";
for my $entry (@{$match->{touched}}) {
    print "  - $entry->{name}: owners=" . join(',', @{$entry->{owners}}) . " checks=" . join(',', @{$entry->{checks}}) . "\n";
}

exit 0;
