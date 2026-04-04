#!/usr/bin/env perl
use strict;
use warnings;

use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json encode_json);

my $base_report;
my $head_report;
my $threshold_file;
my $output_json;

GetOptions(
    'base=s'        => \$base_report,
    'head=s'        => \$head_report,
    'threshold=s'   => \$threshold_file,
    'output-json=s' => \$output_json,
) or die "usage: $0 --base PATH --head PATH --threshold PATH [--output-json PATH]\n";

die "missing --base\n" if !defined $base_report;
die "missing --head\n" if !defined $head_report;
die "missing --threshold\n" if !defined $threshold_file;

sub slurp_text {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    local $/;
    my $text = <$fh>;
    close $fh;
    return $text // q{};
}

sub load_thresholds {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my %out;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;
        my ($name, $ratio, $abs) = split /\|/, $line, 3;
        $out{$name} = {
            max_ratio       => 0 + $ratio,
            max_abs_seconds => 0 + $abs,
        };
    }
    close $fh;
    return \%out;
}

sub results_by_name {
    my ($report) = @_;
    my %map;
    for my $entry (@{$report->{results} // []}) {
        $map{$entry->{name}} = $entry;
    }
    return \%map;
}

my $base = decode_json(slurp_text($base_report));
my $head = decode_json(slurp_text($head_report));
my $thresholds = load_thresholds($threshold_file);
my $base_map = results_by_name($base);
my $head_map = results_by_name($head);
my @failures;
my @comparisons;

for my $name (sort keys %{$thresholds}) {
    my $base_entry = $base_map->{$name} or next;
    my $head_entry = $head_map->{$name} or next;
    my $base_sec = 0 + ($base_entry->{elapsed_seconds} // 0);
    my $head_sec = 0 + ($head_entry->{elapsed_seconds} // 0);
    my $ratio = $base_sec > 0 ? ($head_sec / $base_sec) : 0;
    my $abs_delta = $head_sec - $base_sec;

    push @comparisons, {
        name        => $name,
        base        => $base_sec,
        head        => $head_sec,
        ratio       => $ratio,
        abs_delta   => $abs_delta,
        max_ratio   => $thresholds->{$name}{max_ratio},
        max_abs     => $thresholds->{$name}{max_abs_seconds},
    };

    if ($head_sec > $base_sec
        && $ratio > $thresholds->{$name}{max_ratio}
        && $abs_delta > $thresholds->{$name}{max_abs_seconds})
    {
        push @failures,
            "$name regressed: base=${base_sec}s head=${head_sec}s ratio="
            . sprintf('%.3f', $ratio)
            . " abs_delta="
            . sprintf('%.3f', $abs_delta)
            . "s";
    }
}

if (defined $output_json) {
    open my $fh, '>', $output_json or die "failed to write $output_json: $!";
    print {$fh} encode_json(
        {
            base_sha     => $base->{git_sha},
            head_sha     => $head->{git_sha},
            comparisons  => \@comparisons,
            failures     => \@failures,
        }
    );
    close $fh;
}

if (@failures) {
    print STDERR "perf delta gate failed:\n";
    print STDERR "  - $_\n" for @failures;
    exit 1;
}

print "perf delta gate passed\n";
exit 0;
