package RR::Subsystems;

use strict;
use warnings;

use Exporter qw(import);

our @EXPORT_OK = qw(
  load_subsystems_policy
  match_subsystems
  normalize_path
);

sub normalize_path {
    my ($path) = @_;
    $path =~ s{\\}{/}g;
    $path =~ s{^\./}{};
    $path =~ s{/+$}{};
    return $path;
}

sub _assign_value {
    my ($target, $key, $value) = @_;
    $target->{$key} = $value;
}

sub _parse_array_items {
    my ($text) = @_;
    my @items = ($text =~ /"([^"]+)"/g);
    return \@items;
}

sub load_subsystems_policy {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";

    my %policy = (
        meta       => {},
        subsystems => [],
    );
    my $section = q{};
    my $current;
    my $pending_key;
    my @pending_items;

    while (my $line = <$fh>) {
        chomp $line;
        $line =~ s/\r$//;
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;

        if (defined $pending_key) {
            if ($line =~ /^\s*\]\s*$/) {
                _assign_value($section eq 'meta' ? $policy{meta} : $current, $pending_key, [@pending_items]);
                undef $pending_key;
                @pending_items = ();
                next;
            }
            push @pending_items, @{_parse_array_items($line)};
            next;
        }

        if ($line =~ /^\[meta\]\s*$/) {
            $section = 'meta';
            $current = undef;
            next;
        }
        if ($line =~ /^\[\[subsystems\]\]\s*$/) {
            $section = 'subsystems';
            $current = {};
            push @{$policy{subsystems}}, $current;
            next;
        }

        if ($line =~ /^(\w+)\s*=\s*"(.*)"\s*$/) {
            _assign_value($section eq 'meta' ? $policy{meta} : $current, $1, $2);
            next;
        }
        if ($line =~ /^(\w+)\s*=\s*\[\s*\]\s*$/) {
            _assign_value($section eq 'meta' ? $policy{meta} : $current, $1, []);
            next;
        }
        if ($line =~ /^(\w+)\s*=\s*\[(.*)\]\s*$/) {
            _assign_value($section eq 'meta' ? $policy{meta} : $current, $1, _parse_array_items($2));
            next;
        }
        if ($line =~ /^(\w+)\s*=\s*\[\s*$/) {
            $pending_key = $1;
            @pending_items = ();
            next;
        }

        die "unsupported policy line in $path: $line";
    }

    close $fh;
    return \%policy;
}

sub _path_matches {
    my ($path, $prefix) = @_;
    $path = normalize_path($path);
    $prefix = normalize_path($prefix);
    return 1 if $path eq $prefix;
    return 1 if $prefix ne q{} && index($path, "$prefix/") == 0;
    return 1 if $prefix ne q{} && index($path, $prefix) == 0;
    return 0;
}

sub match_subsystems {
    my ($policy, $paths_ref) = @_;
    my @paths = map { normalize_path($_) } @{$paths_ref // []};
    my %touched;
    my @unmapped;

    PATH:
    for my $path (@paths) {
        for my $ignored (@{$policy->{meta}{ignored_paths} // []}) {
            next PATH if _path_matches($path, $ignored);
        }

        my @matches;
        for my $subsystem (@{$policy->{subsystems}}) {
            for my $prefix (@{$subsystem->{paths} // []}) {
                if (_path_matches($path, $prefix)) {
                    push @matches, $subsystem;
                    last;
                }
            }
        }

        if (!@matches) {
            push @unmapped, $path;
            next;
        }

        for my $subsystem (@matches) {
            my $entry = ($touched{$subsystem->{name}} //= {
                name   => $subsystem->{name},
                owners => [@{$subsystem->{owners} // []}],
                checks => [@{$subsystem->{checks} // []}],
                paths  => [],
            });
            push @{$entry->{paths}}, $path;
        }
    }

    my @ordered = map { $touched{$_} } sort keys %touched;
    return {
        touched  => \@ordered,
        unmapped => \@unmapped,
    };
}

1;
