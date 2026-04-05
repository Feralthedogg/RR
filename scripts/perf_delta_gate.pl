#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use File::Path qw(make_path remove_tree);
use File::Spec;
use File::Temp qw(tempdir);
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json);
use File::Spec ();

my $base;
my $repo;
my $head_report;
my $base_report;
my $compare_report;
my $threshold_file;
my $filter = $ENV{RR_PERF_GATE_FILTER} // q{};

GetOptions(
    'base=s'          => \$base,
    'repo=s'          => \$repo,
    'head-report=s'   => \$head_report,
    'base-report=s'   => \$base_report,
    'compare-report=s'=> \$compare_report,
    'threshold=s'     => \$threshold_file,
    'filter=s'        => \$filter,
) or die "usage: $0 [--repo PATH] [--base SHA] [--head-report PATH] [--base-report PATH] [--compare-report PATH] [--threshold PATH] [--filter SUBSTR]\n";

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
$head_report //= File::Spec->catfile($root, '.artifacts', 'ci', 'perf-head.json');
$base_report //= File::Spec->catfile($root, '.artifacts', 'ci', 'perf-base.json');
$compare_report //= File::Spec->catfile($root, '.artifacts', 'ci', 'perf-compare.json');
$threshold_file //= File::Spec->catfile($root, 'policy', 'perf_delta_thresholds.txt');

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

sub run_status {
    my (@cmd) = @_;
    open my $null, '>', File::Spec->devnull() or die "failed to open devnull: $!";
    open my $stdout_old, '>&', \*STDOUT or die "failed to dup STDOUT: $!";
    open my $stderr_old, '>&', \*STDERR or die "failed to dup STDERR: $!";
    open STDOUT, '>&', $null or die "failed to redirect STDOUT: $!";
    open STDERR, '>&', $null or die "failed to redirect STDERR: $!";
    my $status = system(@cmd);
    open STDOUT, '>&', $stdout_old or die "failed to restore STDOUT: $!";
    open STDERR, '>&', $stderr_old or die "failed to restore STDERR: $!";
    close $stdout_old;
    close $stderr_old;
    close $null;
    return $status == 0;
}

sub base_ref_usable {
    my ($ref) = @_;
    return 0 if !defined $ref || $ref eq q{};
    return 0 if !run_status('git', '-C', $root, 'rev-parse', '--verify', "$ref^{commit}");
    return run_status('git', '-C', $root, 'merge-base', $ref, 'HEAD');
}

my $event = read_event();
my $base_ref = defined $base ? $base : read_base_from_event($event);
if (!base_ref_usable($base_ref) && run_status('git', '-C', $root, 'rev-parse', '--verify', 'HEAD^')) {
    my $fallback = qx{git -C "$root" rev-parse --verify HEAD^};
    chomp $fallback;
    $base_ref = $fallback;
}

if (!base_ref_usable($base_ref)) {
    print "skip: no base ref available for perf delta gate\n";
    exit 0;
}

my (undef, $head_dir, undef) = File::Spec->splitpath($head_report);
my (undef, $base_dir, undef) = File::Spec->splitpath($base_report);
my (undef, $cmp_dir, undef) = File::Spec->splitpath($compare_report);
make_path($head_dir) if defined $head_dir && $head_dir ne q{};
make_path($base_dir) if defined $base_dir && $base_dir ne q{};
make_path($cmp_dir) if defined $cmp_dir && $cmp_dir ne q{};

my @head_cmd = ('perl', File::Spec->catfile($root, 'scripts', 'perf_governance.pl'), '--repo', $root, '--report', $head_report);
push @head_cmd, ('--filter', $filter) if $filter ne q{};
system(@head_cmd) == 0 or die "head perf governance failed\n";

my $tmp = tempdir('rr-perf-base.XXXXXX', TMPDIR => 1, CLEANUP => 0);
system('git', '-C', $root, 'worktree', 'add', '--detach', $tmp, $base_ref) == 0
    or die "failed to create base worktree for perf delta gate\n";

my @base_cmd = ('perl', File::Spec->catfile($root, 'scripts', 'perf_governance.pl'), '--repo', $tmp, '--report', $base_report);
push @base_cmd, ('--filter', $filter) if $filter ne q{};
my $base_status = system(@base_cmd);
system('git', '-C', $root, 'worktree', 'remove', '--force', $tmp);
remove_tree($tmp, { error => \my $err });
die "base perf governance failed\n" if $base_status != 0;

system(
    'perl',
    File::Spec->catfile($root, 'scripts', 'compare_perf_reports.pl'),
    '--base',
    $base_report,
    '--head',
    $head_report,
    '--threshold',
    $threshold_file,
    '--output-json',
    $compare_report,
) == 0 or die "perf comparison failed\n";

print "perf delta gate passed\n";
exit 0;
