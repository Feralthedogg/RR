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
my $mode = 'diff';
my $print_plan = 0;

GetOptions(
    'changed-files-file=s' => \$changed_files_file,
    'base=s'               => \$base,
    'mode=s'               => \$mode,
    'print-plan'           => \$print_plan,
) or die "usage: $0 [--changed-files-file PATH] [--base SHA] [--mode diff|all] [--print-plan]\n";

my $root = abs_path(File::Spec->catdir($Bin, '..'));
my $policy = load_subsystems_policy(File::Spec->catfile($root, 'policy', 'subsystems.toml'));

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

sub slurp_lines {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @lines = map { chomp; $_ } <$fh>;
    close $fh;
    return [grep { defined && length } @lines];
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

my $event = read_event();
my $changed_files;
if ($mode eq 'all') {
    my @lines = `git -C "$root" ls-files -- src tests docs scripts policy .github CONTRIBUTING.md`;
    chomp @lines;
    $changed_files = [map { normalize_path($_) } grep { defined && length } @lines];
} elsif ($changed_files_file) {
    $changed_files = slurp_lines($changed_files_file);
} else {
    $changed_files = read_changed_files_from_git(defined $base ? $base : read_base_from_event($event));
}

my $match = match_subsystems($policy, $changed_files);
die "unmapped files in subsystem matrix: " . join(', ', @{$match->{unmapped}}) . "\n" if @{$match->{unmapped}};

my %command_map = (
    docs_generated    => ['python3', 'scripts/render_contributing_docs.py', '--check'],
    ci_contract       => ['perl', 'scripts/check_required_ci_jobs.pl'],
    process_smoke     => ['cargo', 'test', '-q', '--test', 'contributing_audit_smoke', '--test', 'process_gate_smoke'],
    tier0             => ['bash', 'scripts/test_tier.sh', 'tier0'],
    tier1             => ['bash', 'scripts/test_tier.sh', 'tier1'],
    semantic_audit    => ['env', 'RR_SEMANTIC_AUDIT_SCOPE=diff', 'bash', 'scripts/ci_contributing_semantic_audit.sh'],
    perf_governance   => ['perl', 'scripts/perf_governance.pl'],
    cache_matrix      => ['cargo', 'test', '-q', '--test', 'cache_equivalence_matrix', '--test', 'incremental_strict_verify'],
    optimizer_matrix  => ['cargo', 'test', '-q', '--test', 'sccp_overflow_regression', '--test', 'opt_level_equivalence', '--test', 'numeric_property_differential'],
    runtime_matrix    => ['cargo', 'test', '-q', '--test', 'runtime_semantics_regression', '--test', 'hermetic_determinism'],
    parallel_matrix   => ['cargo', 'test', '-q', '--test', 'fallback_correctness_matrix', '--test', 'parallel_optional_fallback_semantics', '--test', 'native_optional_fallback', '--test', 'poly_vopt_fallback'],
    determinism_matrix => ['cargo', 'test', '-q', '--test', 'commercial_determinism', '--test', 'compiler_parallel_equivalence'],
    optimizer_legality => ['bash', 'scripts/optimizer_suite.sh', 'legality'],
);

my @checks;
my %seen;
for my $entry (@{$match->{touched}}) {
    for my $check (@{$entry->{checks}}) {
        next if $seen{$check}++;
        push @checks, $check;
    }
}

if ($print_plan) {
    for my $entry (@{$match->{touched}}) {
        print "subsystem=$entry->{name} checks=" . join(',', @{$entry->{checks}}) . "\n";
    }
    print "plan=" . join(',', @checks) . "\n";
    exit 0;
}

if (!@checks) {
    print "no subsystem checks selected\n";
    exit 0;
}

for my $check (@checks) {
    my $cmd = $command_map{$check} or die "no command mapping for check '$check'\n";
    print "== subsystem check: $check ==\n";
    my $status = system(@{$cmd});
    if ($status != 0) {
        die "subsystem check '$check' failed with status $status\n";
    }
}

print "subsystem matrix passed\n";
exit 0;
