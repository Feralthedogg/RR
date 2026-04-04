#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use FindBin qw($Bin);
use Getopt::Long qw(GetOptions);
use JSON::PP qw(decode_json);
use File::Spec;
use HTTP::Tiny;

use lib File::Spec->catdir($Bin, 'lib');
use RR::Subsystems qw(load_subsystems_policy match_subsystems normalize_path);

my $changed_files_file;
my $reviews_json_file;
my $base;
my $print_plan = 0;
my $repo;

GetOptions(
    'changed-files-file=s' => \$changed_files_file,
    'reviews-json-file=s'  => \$reviews_json_file,
    'base=s'               => \$base,
    'repo=s'               => \$repo,
    'print-plan'           => \$print_plan,
) or die "usage: $0 [--repo PATH] [--changed-files-file PATH] [--reviews-json-file PATH] [--base SHA] [--print-plan]\n";

my $root = abs_path($repo // File::Spec->catdir($Bin, '..'));
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
    return [grep { defined && length } map { normalize_path($_) } @lines];
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

sub normalize_owner {
    my ($owner) = @_;
    $owner //= q{};
    $owner =~ s/^@//;
    return lc $owner;
}

sub fetch_reviews_from_api {
    my ($event) = @_;
    my $pull_request = $event->{pull_request} || {};
    my $base_url = $pull_request->{url} // q{};
    return [] if $base_url eq q{};

    my $token = $ENV{GITHUB_TOKEN} // q{};
    if ($token eq q{}) {
        die "missing GITHUB_TOKEN for subsystem review check\n";
    }

    my $url = "$base_url/reviews?per_page=100";
    my $http = HTTP::Tiny->new(
        default_headers => {
            Authorization         => "Bearer $token",
            Accept                => 'application/vnd.github+json',
            'X-GitHub-Api-Version' => '2022-11-28',
        },
        verify_SSL => 1,
    );
    my $response = $http->get($url);
    if (!$response->{success}) {
        die "failed to fetch PR reviews: $response->{status} $response->{reason}\n";
    }
    my $decoded = decode_json($response->{content});
    return ref($decoded) eq 'ARRAY' ? $decoded : [];
}

sub effective_review_states {
    my ($reviews) = @_;
    my %states;
    for my $review (@{$reviews // []}) {
        my $login = normalize_owner($review->{user}{login});
        next if $login eq q{};
        my $state = uc($review->{state} // q{});
        next if $state eq q{};
        $states{$login} = $state;
    }
    return \%states;
}

my $event = read_event();
my $base_sha = defined $base ? $base : read_base_from_event($event);
my $changed_files = $changed_files_file
    ? slurp_lines($changed_files_file)
    : read_changed_files_from_git($base_sha);
my $match = match_subsystems($policy, $changed_files);
die "unmapped files for subsystem review check: " . join(', ', @{$match->{unmapped}}) . "\n"
    if @{$match->{unmapped}};

if (!@{$match->{touched}}) {
    print "no subsystem-mapped files in scope\n";
    exit 0;
}

my $reviews;
if ($reviews_json_file) {
    my $decoded = decode_json(slurp_text($reviews_json_file));
    $reviews = ref($decoded) eq 'ARRAY' ? $decoded : [];
} elsif (($ENV{GITHUB_EVENT_NAME} // q{}) eq 'pull_request') {
    $reviews = fetch_reviews_from_api($event);
} else {
    print "skip: subsystem review checks run on pull requests or with --reviews-json-file\n";
    exit 0;
}

my $states = effective_review_states($reviews);
my %allowed_states = map { uc($_) => 1 } @{$policy->{meta}{required_review_states} // ['APPROVED']};
my $min_approvals = ($policy->{meta}{min_owner_approvals} // '1') + 0;
my @errors;

for my $entry (@{$match->{touched}}) {
    my @owners = map { normalize_owner($_) } @{$entry->{owners} // []};
    my @approved = grep {
        my $state = $states->{$_} // q{};
        $allowed_states{$state};
    } @owners;

    if ($print_plan) {
        print "subsystem=$entry->{name}\n";
        print "owners=" . join(',', @owners) . "\n";
        print "approved=" . join(',', @approved) . "\n";
        print "paths=" . join(',', @{$entry->{paths}}) . "\n";
        print "\n";
    }

    if (@approved < $min_approvals) {
        push @errors,
            "subsystem '$entry->{name}' requires $min_approvals owner approval(s); owners="
            . join(',', @owners)
            . " approved="
            . join(',', @approved);
    }
}

if (@errors) {
    print STDERR "subsystem review check failed:\n";
    print STDERR "  - $_\n" for @errors;
    exit 1;
}

print "subsystem review check passed\n";
exit 0;
