#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use File::Copy qw(copy);
use File::Path qw(make_path);
use File::Spec;
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(encode_json decode_json);

use lib File::Spec->catdir($Bin, 'lib');
use RR::Subsystems qw(load_subsystems_policy match_subsystems normalize_path);

my $bundle_dir;
my $label = 'ci-failure';
my $base;
my $repo;
my $repro_cmd;
my $changed_files_file;
my @logs;

GetOptions(
    'bundle-dir=s'        => \$bundle_dir,
    'label=s'             => \$label,
    'base=s'              => \$base,
    'repo=s'              => \$repo,
    'repro-cmd=s'         => \$repro_cmd,
    'changed-files-file=s' => \$changed_files_file,
    'log=s@'              => \@logs,
) or die "usage: $0 --bundle-dir PATH [--repo PATH] [--label NAME] [--base SHA] [--log PATH ...] [--repro-cmd CMD] [--changed-files-file PATH]\n";

die "missing --bundle-dir\n" if !defined $bundle_dir || $bundle_dir eq q{};

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
my $policy = load_subsystems_policy(File::Spec->catfile($root, 'policy', 'subsystems.toml'));
my $bundle_root = abs_path($bundle_dir) // File::Spec->rel2abs($bundle_dir, $root);

sub slurp_text {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    local $/;
    my $text = <$fh>;
    close $fh;
    return $text // q{};
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

sub read_changed_files_from_git {
    my ($base_sha) = @_;
    my $cmd = defined $base_sha && length $base_sha
        ? qq{git -C "$root" diff --name-only "$base_sha...HEAD" --}
        : qq{{ git -C "$root" diff --name-only HEAD --; git -C "$root" diff --cached --name-only --; git -C "$root" ls-files --others --exclude-standard; }};
    my @lines = `$cmd`;
    chomp @lines;
    return [grep { defined && length } map { normalize_path($_) } @lines];
}

sub slurp_lines {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @lines = map { chomp; $_ } <$fh>;
    close $fh;
    return [grep { defined && length } map { normalize_path($_) } @lines];
}

sub capture_optional {
    my (@cmd) = @_;
    my $out = qx{@cmd 2>/dev/null};
    return q{} if $? != 0;
    $out =~ s/\s+\z//;
    return $out;
}

sub priority_for_subsystems {
    my ($names) = @_;
    my %critical = map { $_ => 1 } @{$policy->{meta}{triage_critical_subsystems} // []};
    for my $name (@{$names}) {
        return 'high' if $critical{$name};
    }
    return 'medium';
}

sub default_repro_cmd {
    my ($bundle_label) = @_;
    my %map = (
        'tier0'             => 'bash scripts/ci_contributing_audit.sh && bash scripts/test_tier.sh tier0',
        'ci-contract'       => 'perl scripts/check_required_ci_jobs.pl',
        'semantic-audit'    => 'bash scripts/ci_contributing_semantic_audit.sh',
        'subsystem-matrix'  => 'perl scripts/ci_subsystem_matrix.pl --mode diff',
        'process-gates'     => 'perl scripts/check_commit_metadata.pl && perl scripts/check_commit_series.pl --verify-buildability && perl scripts/check_subsystem_reviews.pl && python3 scripts/check_pr_evidence.py',
        'perf-governance'   => 'perl scripts/perf_governance.pl',
        'perf-delta'        => 'perl scripts/perf_delta_gate.pl',
        'nightly-stress'    => 'FUZZ_SECONDS=5 ./scripts/fuzz_smoke.sh && perl scripts/perf_governance.pl',
    );
    return $map{$bundle_label} // 'cargo check && cargo test -q';
}

my $event = read_event();
my $base_sha = defined $base ? $base : read_base_from_event($event);
my $changed_files = $changed_files_file
    ? slurp_lines($changed_files_file)
    : read_changed_files_from_git($base_sha);
my $match = match_subsystems($policy, $changed_files);
my @subsystems = @{$match->{touched}};
my @subsystem_names = map { $_->{name} } @subsystems;

make_path($bundle_root);
make_path(File::Spec->catdir($bundle_root, 'logs'));

open my $changed_fh, '>', File::Spec->catfile($bundle_root, 'changed_files.txt')
    or die "failed to create changed_files.txt: $!";
print {$changed_fh} "$_\n" for @{$changed_files};
close $changed_fh;

open my $versions_fh, '>', File::Spec->catfile($bundle_root, 'versions.txt')
    or die "failed to create versions.txt: $!";
for my $entry (
    ['rustc',  [qw(rustc --version)]],
    ['cargo',  [qw(cargo --version)]],
    ['perl',   [qw(perl -v)]],
    ['python3',[qw(python3 --version)]],
    ['Rscript',[qw(Rscript --version)]],
) {
    my ($label_name, $cmd) = @{$entry};
    print {$versions_fh} "## $label_name\n";
    print {$versions_fh} capture_optional(@{$cmd}) . "\n\n";
}
close $versions_fh;

my @copied_logs;
for my $log (@logs) {
    next if !defined $log || $log eq q{} || !-f $log;
    my ($volume, $dirs, $file) = File::Spec->splitpath($log);
    my $dest = File::Spec->catfile($bundle_root, 'logs', $file);
    copy($log, $dest) or die "failed to copy $log to $dest: $!";
    push @copied_logs, File::Spec->catfile('logs', $file);
}

my $metadata = {
    label            => $label,
    repo_root        => $root,
    sha              => capture_optional('git', '-C', $root, 'rev-parse', 'HEAD'),
    base_sha         => $base_sha,
    ref              => $ENV{GITHUB_REF} // q{},
    workflow         => $ENV{GITHUB_WORKFLOW} // q{},
    event_name       => $ENV{GITHUB_EVENT_NAME} // q{},
    actor            => $ENV{GITHUB_ACTOR} // q{},
    subsystems       => \@subsystem_names,
    priority         => priority_for_subsystems(\@subsystem_names),
    copied_logs      => \@copied_logs,
    changed_file_count => scalar(@{$changed_files}),
    repro_command    => ($repro_cmd // default_repro_cmd($label)),
};

open my $meta_fh, '>', File::Spec->catfile($bundle_root, 'metadata.json')
    or die "failed to create metadata.json: $!";
print {$meta_fh} encode_json($metadata);
close $meta_fh;

open my $repro_fh, '>', File::Spec->catfile($bundle_root, 'repro.sh')
    or die "failed to create repro.sh: $!";
print {$repro_fh} "#!/usr/bin/env bash\nset -euo pipefail\ncd \"$root\"\n";
print {$repro_fh} ($repro_cmd // default_repro_cmd($label)) . "\n";
close $repro_fh;

open my $summary_fh, '>', File::Spec->catfile($bundle_root, 'triage-summary.md')
    or die "failed to create triage-summary.md: $!";
print {$summary_fh} "# Failure Bundle\n\n";
print {$summary_fh} "- Label: `$label`\n";
print {$summary_fh} "- Priority: `$metadata->{priority}`\n";
print {$summary_fh} "- SHA: `$metadata->{sha}`\n";
print {$summary_fh} "- Base: `" . ($base_sha || 'n/a') . "`\n";
print {$summary_fh} "- Subsystems: " . (@subsystem_names ? join(', ', map { "`$_`" } @subsystem_names) : '`none`') . "\n";
print {$summary_fh} "- Repro: `" . $metadata->{repro_command} . "`\n";
print {$summary_fh} "\n## Owners\n\n";
if (@subsystems) {
    for my $entry (@subsystems) {
        print {$summary_fh} "- `$entry->{name}`: " . join(', ', @{$entry->{owners}}) . "\n";
    }
} else {
    print {$summary_fh} "- no mapped subsystems\n";
}
print {$summary_fh} "\n## Logs\n\n";
if (@copied_logs) {
    print {$summary_fh} "- `$_`\n" for @copied_logs;
} else {
    print {$summary_fh} "- no logs captured\n";
}
close $summary_fh;

print "failure bundle written to $bundle_root\n";
exit 0;
