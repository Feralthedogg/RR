#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path getcwd);
use File::Path qw(make_path);
use File::Spec;
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(encode_json);
use Time::HiRes qw(time);

my $report;
my $budget_file;
my $filter = $ENV{RR_PERF_GATE_FILTER} // q{};
my $repo;

GetOptions(
    'report=s'      => \$report,
    'budget-file=s' => \$budget_file,
    'filter=s'      => \$filter,
    'repo=s'        => \$repo,
) or die "usage: $0 [--repo PATH] [--report PATH] [--budget-file PATH] [--filter SUBSTR]\n";

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
$budget_file //= File::Spec->catfile($root, 'policy', 'perf_budget.txt');
$report //= File::Spec->catfile($root, '.artifacts', 'ci', 'perf-governance.json');

sub read_budget {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my %budget;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;
        my ($name, $seconds) = split /\|/, $line, 2;
        $budget{$name} = 0 + $seconds;
    }
    close $fh;
    return \%budget;
}

sub run_command {
    my ($name, $cmd, $results, $budget) = @_;
    return if $filter ne q{} && index($name, $filter) < 0;

    print "== perf check: $name ==\n";
    my $started = time();
    my $cwd = getcwd();
    chdir $root or die "failed to chdir to $root: $!";
    my $status = system(@{$cmd});
    chdir $cwd or die "failed to restore cwd to $cwd: $!";
    my $elapsed = time() - $started;
    my $entry = {
        name            => $name,
        elapsed_seconds => sprintf('%.3f', $elapsed) + 0,
        status          => $status >> 8,
        budget_seconds  => $budget->{$name},
        command         => $cmd,
    };
    push @{$results}, $entry;

    die "perf check '$name' failed with status " . ($status >> 8) . "\n" if $status != 0;
    if (exists $budget->{$name} && $elapsed > $budget->{$name}) {
        die "perf check '$name' exceeded budget: ${elapsed}s > $budget->{$name}s\n";
    }
}

my $budget = read_budget($budget_file);
my @results;

run_command('perf_regression_gate', ['cargo', 'test', '--test', 'perf_regression_gate', '--quiet'], \@results, $budget);
run_command('benchmark_vectorization', ['cargo', 'test', '--test', 'benchmark_vectorization', '--quiet'], \@results, $budget);
run_command('commercial_determinism', ['cargo', 'test', '--test', 'commercial_determinism', '--quiet'], \@results, $budget);
run_command('example_perf_smoke', ['cargo', 'test', '--test', 'example_perf_smoke', '--', '--ignored', '--nocapture'], \@results, $budget);

my (undef, $report_dir, undef) = File::Spec->splitpath($report);
make_path($report_dir) if defined $report_dir && $report_dir ne q{};
open my $fh, '>', $report or die "failed to write $report: $!";
print {$fh} encode_json(
    {
        repo_root => $root,
        git_sha   => do {
            my $sha = `git -C "$root" rev-parse HEAD 2>/dev/null`;
            chomp $sha;
            $sha;
        },
        generated_at_epoch => time(),
        results   => \@results,
    }
);
close $fh;

print "perf governance passed\n";
exit 0;
