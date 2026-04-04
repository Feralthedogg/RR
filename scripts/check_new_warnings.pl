#!/usr/bin/env perl
use strict;
use warnings;

use Getopt::Long qw(GetOptions);

my $baseline_file;
my $log_file;

GetOptions(
    'baseline=s' => \$baseline_file,
    'log=s'      => \$log_file,
) or die "usage: $0 --baseline PATH --log PATH\n";

die "missing --baseline\n" if !defined $baseline_file;
die "missing --log\n" if !defined $log_file;

open my $baseline_fh, '<', $baseline_file or die "failed to read $baseline_file: $!";
my @baseline;
while (my $line = <$baseline_fh>) {
    chomp $line;
    next if $line =~ /^\s*#/;
    next if $line =~ /^\s*$/;
    my ($code, $path_re) = split /\|/, $line, 2;
    push @baseline, [$code, qr/$path_re/];
}
close $baseline_fh;

open my $log_fh, '<', $log_file or die "failed to read $log_file: $!";
my @unexpected;
while (my $line = <$log_fh>) {
    chomp $line;
    next if $line !~ /^warn\[([^\]]+)\]\s+([^:]+):/;
    my ($code, $path) = ($1, $2);
    my $matched = 0;
    for my $entry (@baseline) {
        if ($code eq $entry->[0] && $path =~ $entry->[1]) {
            $matched = 1;
            last;
        }
    }
    push @unexpected, "$code @ $path" if !$matched;
}
close $log_fh;

if (@unexpected) {
    print STDERR "new warnings detected outside baseline:\n";
    print STDERR "  - $_\n" for @unexpected;
    exit 1;
}

print "warning baseline check passed\n";
exit 0;
