#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use File::Spec;
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);

my $workflow;
my $required_file;
my $repo;

GetOptions(
    'workflow=s'      => \$workflow,
    'required-file=s' => \$required_file,
    'repo=s'          => \$repo,
) or die "usage: $0 [--repo PATH] [--workflow PATH] [--required-file PATH]\n";

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
$workflow //= File::Spec->catfile($root, '.github', 'workflows', 'ci.yml');
$required_file //= File::Spec->catfile($root, 'policy', 'required_ci_checks.txt');

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
    return [grep { defined && length && $_ !~ /^\s*#/ } @lines];
}

sub extract_simple_names {
    my ($text) = @_;
    my %names;
    while ($text =~ /^\s{4}name:\s+(.+?)\s*$/mg) {
        my $name = $1;
        $names{$name} = 1;
    }
    return \%names;
}

sub expected_matrix_variants {
    my ($text) = @_;
    my %names;

    if ($text =~ /name:\s+Rust Matrix \/ \$\{\{\s*matrix\.channel\s*\}\}[\s\S]*?channel:\s*\n((?:\s*-\s*[^\n]+\n)+)/m) {
        my $block = $1;
        while ($block =~ /^\s*-\s*([^\n]+)\s*$/mg) {
            $names{"Rust Matrix / $1"} = 1;
        }
    }

    if ($text =~ /name:\s+R Runtime Matrix \/ \$\{\{\s*matrix\.r-version\s*\}\}[\s\S]*?r-version:\s*\n((?:\s*-\s*[^\n]+\n)+)/m) {
        my $block = $1;
        while ($block =~ /^\s*-\s*([^\n]+)\s*$/mg) {
            $names{"R Runtime Matrix / $1"} = 1;
        }
    }

    if ($text =~ /name:\s+Platform Build \/ \$\{\{\s*matrix\.label\s*\}\}[\s\S]*?include:\s*\n((?:\s*-\s*[^\n]+\n(?:\s{12,}[^\n]+\n)*)+)/m) {
        my $block = $1;
        while ($block =~ /^\s{12}label:\s*([^\n]+)\s*$/mg) {
            $names{"Platform Build / $1"} = 1;
        }
    }

    return \%names;
}

my $workflow_text = slurp_text($workflow);
my $required = slurp_lines($required_file);
my %available = (%{extract_simple_names($workflow_text)}, %{expected_matrix_variants($workflow_text)});

my @missing;
for my $required_name (@{$required}) {
    push @missing, $required_name if !$available{$required_name};
}

if (@missing) {
    print STDERR "required CI checks missing from workflow contract:\n";
    print STDERR "  - $_\n" for @missing;
    exit 1;
}

print "required CI checks contract passed\n";
exit 0;
