#!/usr/bin/env perl
use strict;
use warnings;

use Cwd qw(abs_path);
use Encode qw(decode FB_CROAK);
use File::Spec;
use File::Temp qw(tempfile);
use FindBin qw($RealBin);

my $ROOT = abs_path(File::Spec->catdir($RealBin, '..'));

my $SCAN_ONLY = 0;
my $ALL_FILES = 0;
my $SKIP_FUZZ = 0;
my $REQUIRE_FUZZ = 0;
my $SKIP_PASS_VERIFY = 0;
my $SKIP_SEMANTIC_SMOKE = 0;
my $BASE_REF = q{};
my @FILES;
my %RAW_ARGS = map { $_ => 1 } @ARGV;

sub usage {
    print <<'EOF';
Usage: perl scripts/contributing_audit.pl [options]

Options:
  --scan-only       Skip cargo/fuzz commands and run static audit only.
  --all             Scan all repo files covered by CONTRIBUTING.md.
  --base <ref>      Scan files changed from the given git base ref.
  --files <paths>   Scan the explicit file list that follows.
  --skip-pass-verify
                    Skip pass-verify smoke even when pass-sensitive files are in scope.
  --skip-semantic-smoke
                    Skip conditional semantic smoke suites for cache/determinism/numeric/fallback.
  --skip-fuzz       Skip fuzz smoke even if cargo-fuzz is installed.
  --require-fuzz    Fail if fuzz smoke cannot be executed.
  --help            Show this help.

Default behavior:
  - scan changed files in the current worktree
  - run cargo check / clippy / test
  - run RR_VERIFY_EACH_PASS smoke when pass-sensitive compiler files are in scope
  - run scope-driven semantic smoke suites for cache/determinism/numeric/fallback regressions
  - run fuzz smoke when cargo-fuzz is available
EOF
}

while (@ARGV) {
    my $arg = shift @ARGV;
    if ($arg eq '--scan-only') {
        $SCAN_ONLY = 1;
    } elsif ($arg eq '--all') {
        $ALL_FILES = 1;
    } elsif ($arg eq '--base') {
        @ARGV or die "missing value for --base\n";
        $BASE_REF = shift @ARGV;
    } elsif ($arg eq '--files') {
        while (@ARGV && $ARGV[0] !~ /^--/) {
            push @FILES, shift @ARGV;
        }
    } elsif ($arg eq '--skip-pass-verify') {
        $SKIP_PASS_VERIFY = 1;
    } elsif ($arg eq '--skip-semantic-smoke') {
        $SKIP_SEMANTIC_SMOKE = 1;
    } elsif ($arg eq '--skip-fuzz') {
        $SKIP_FUZZ = 1;
    } elsif ($arg eq '--require-fuzz') {
        $REQUIRE_FUZZ = 1;
    } elsif ($arg eq '--help' || $arg eq '-h') {
        usage();
        exit 0;
    } else {
        print STDERR "unknown option: $arg\n";
        usage();
        exit 2;
    }
}

$ALL_FILES = 1 if $RAW_ARGS{'--all'};

if ($ALL_FILES && @FILES) {
    print STDERR "--all and --files cannot be used together\n";
    exit 2;
}

sub normalize_path {
    my ($path) = @_;
    $path =~ s{\\}{/}g;
    $path =~ s{^\./}{};
    $path =~ s{/+$}{};
    return $path;
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

sub capture_lines {
    my (@cmd) = @_;
    open my $fh, '-|', @cmd or die "failed to run @cmd: $!";
    my @lines = <$fh>;
    close $fh or die "command failed: @cmd\n";
    chomp @lines;
    return grep { defined && length } @lines;
}

sub capture_text {
    my (@cmd) = @_;
    open my $fh, '-|', @cmd or die "failed to run @cmd: $!";
    local $/;
    my $text = <$fh> // q{};
    close $fh or die "command failed: @cmd\n";
    return $text;
}

sub in_git_worktree {
    my $output = `git -C "$ROOT" rev-parse --is-inside-work-tree 2>/dev/null`;
    return $? == 0;
}

sub unique_sorted {
    my (%seen, @out);
    for my $item (@_) {
        next if $seen{$item}++;
        push @out, $item;
    }
    return sort @out;
}

sub base_ref_usable {
    my ($ref) = @_;
    return 0 if !defined $ref || !length $ref;
    return 0 if !run_status('git', '-C', $ROOT, 'rev-parse', '--verify', "$ref^{commit}");
    return run_status('git', '-C', $ROOT, 'merge-base', $ref, 'HEAD');
}

sub in_scope_path {
    my ($path) = @_;
    $path = normalize_path($path);
    return $path =~ m{^(?:src/|tests/|docs/|scripts/|fuzz/|native/|policy/|\.github/pull_request_template\.md$|CONTRIBUTING\.md$)};
}

sub collect_all_files {
    if (in_git_worktree()) {
        my @lines = capture_lines(
            'git', '-C', $ROOT, 'ls-files', '--',
            'CONTRIBUTING.md',
            '.github/pull_request_template.md',
            'src', 'tests', 'docs', 'scripts', 'fuzz', 'native', 'policy',
        );
        return unique_sorted(@lines);
    }
    my @lines = capture_lines('rg', '--files', 'src', 'tests', 'docs', 'scripts', 'fuzz', 'native', 'policy');
    return unique_sorted(
        'CONTRIBUTING.md',
        '.github/pull_request_template.md',
        @lines,
    );
}

sub collect_changed_files {
    return () if !in_git_worktree();
    my @paths;
    if (length $BASE_REF && base_ref_usable($BASE_REF)) {
        push @paths, capture_lines('git', '-C', $ROOT, 'diff', '--name-only', "$BASE_REF...", '--');
    }
    push @paths, capture_lines('git', '-C', $ROOT, 'diff', '--name-only', 'HEAD', '--');
    push @paths, capture_lines('git', '-C', $ROOT, 'diff', '--cached', '--name-only', '--');
    push @paths, capture_lines('git', '-C', $ROOT, 'ls-files', '--others', '--exclude-standard');
    my @filtered = grep { in_scope_path($_) } @paths;
    return unique_sorted(@filtered);
}

my @scan_files;
if (@FILES) {
    @scan_files = @FILES;
} elsif ($ALL_FILES) {
    if (in_git_worktree()) {
        my @lines = capture_lines(
            'git', '-C', $ROOT, 'ls-files', '--',
            'CONTRIBUTING.md',
            '.github/pull_request_template.md',
            'src', 'tests', 'docs', 'scripts', 'fuzz', 'native', 'policy',
        );
        @scan_files = unique_sorted(@lines);
    } else {
        my @lines = capture_lines('rg', '--files', 'src', 'tests', 'docs', 'scripts', 'fuzz', 'native', 'policy');
        @scan_files = unique_sorted('CONTRIBUTING.md', '.github/pull_request_template.md', @lines);
    }
} else {
    @scan_files = collect_changed_files();
}

my ($file_list_fh, $file_list_path) = tempfile('rr-contributing-audit-files.XXXXXX', TMPDIR => 1, UNLINK => 1);
for my $file (@scan_files) {
    print {$file_list_fh} "$file\n";
}
close $file_list_fh;

my ($ast_fh, $ast_path) = tempfile('rr-contributing-audit-ast.XXXXXX', TMPDIR => 1, UNLINK => 1);
my $ast_text = capture_text(
    'cargo', 'run', '--quiet',
    '--manifest-path', File::Spec->catfile($ROOT, 'scripts', 'contributing_ast_audit', 'Cargo.toml'),
    '--', $ROOT, $file_list_path,
);
print {$ast_fh} $ast_text;
close $ast_fh;

sub scope_matches {
    my ($regex) = @_;
    for my $path (@scan_files) {
        return 1 if normalize_path($path) =~ $regex;
    }
    return 0;
}

sub scope_needs_pass_verify {
    return scope_matches(qr{^(?:src/hir/|src/mir/|src/legacy/ir/|src/codegen/mir_emit\.rs$|src/compiler/pipeline\.rs$|src/compiler/incremental\.rs$|tests/pass_verify_examples\.rs$)});
}

sub scope_touches_core_compiler {
    return scope_matches(qr{^(?:src/compiler/|src/hir/|src/mir/|src/codegen/|src/runtime/|src/main\.rs$|src/main_.*\.rs$)});
}

sub scope_needs_cache_semantics {
    return scope_matches(qr{^(?:src/compiler/incremental\.rs$|src/compiler/pipeline\.rs$|src/main_compile\.rs$|tests/incremental_.*|tests/cli_incremental_default\.rs$)});
}

sub scope_needs_numeric_semantics {
    return scope_matches(qr{^(?:src/hir/|src/mir/|src/codegen/|src/runtime/|src/compiler/|tests/sccp_overflow_regression\.rs$|tests/rr_logic_equivalence_matrix\.rs$|tests/opt_level_equivalence\.rs$)});
}

sub scope_needs_fallback_semantics {
    return scope_matches(qr{^(?:src/main\.rs$|src/main_.*\.rs$|src/runtime/|src/compiler/pipeline\.rs$|src/compiler/scheduler\.rs$|src/mir/semantics/|src/mir/opt/poly/|tests/hybrid_fallback\.rs$|tests/native_optional_fallback\.rs$|tests/parallel_optional_fallback_semantics\.rs$|tests/poly_vopt_fallback\.rs$)});
}

sub scope_needs_determinism_semantics {
    return scope_touches_core_compiler();
}

print "== RR Contributing Audit ==\n";
if (!@scan_files) {
    print "scope: no changed files detected\n";
} else {
    print "scope: " . scalar(@scan_files) . " file(s)\n";
}

my $panic_re = qr/\bpanic!\s*\(/;
my $unwrap_re = qr/\.\s*unwrap(?:_err)?\s*\(/;
my $expect_re = qr/\.\s*expect\s*\(\s*(?:"|r#*"|format!\s*\(|&format!\s*\()/;
my $unsafe_re = qr/\bunsafe\b/;
my $dbg_re = qr/\bdbg!\s*\(/;
my $heading_re = qr/^(#{2,3})\s+(.+?)\s*$/;
my $structured_todo_re = qr/^TODO\([^)]+\):\s+\S/;
my $structured_fixme_re = qr/^FIXME:\s+\S/;
my $structured_note_re = qr/^NOTE:\s+\S/;

my @interesting_doc_prefixes = (
    'src/codegen/mir_emit.rs',
    'src/mir/opt.rs',
    'src/mir/opt/',
    'src/runtime/',
    'src/compiler/pipeline.rs',
    'src/compiler/incremental.rs',
    'src/main.rs',
);
my @interesting_test_prefixes = (
    'src/hir/',
    'src/mir/',
    'src/legacy/ir/',
    'src/codegen/mir_emit.rs',
    'src/mir/opt.rs',
    'src/mir/opt/',
    'src/compiler/pipeline.rs',
    'src/compiler/incremental.rs',
);
my @pass_sensitive_prefixes = (
    'src/hir/',
    'src/mir/',
    'src/legacy/ir/',
    'src/codegen/mir_emit.rs',
    'src/compiler/pipeline.rs',
    'src/compiler/incremental.rs',
);
my @cache_sensitive_prefixes = (
    'src/compiler/incremental.rs',
    'src/compiler/pipeline.rs',
);
my @core_compiler_prefixes = (
    'src/compiler/',
    'src/hir/',
    'src/mir/',
    'src/codegen/',
    'src/runtime/',
);
my @safe_alt_markers = (
    'safe alternative',
    'safe alternatives',
    'safe rust',
    'cannot be expressed safely',
    'cannot express safely',
    'cannot do this safely',
    'cannot use safe',
    'ffi',
    'raw pointer',
    'aliasing',
    'layout',
    'provenance',
);
my @required_contributing_sections = (
    'Scope',
    'Core Principles',
    'Rule Levels',
    'Rules',
    'Exception Process',
    'PR Checklist',
);
my @required_contributing_rule_topics = (
    'Deterministic Output and Traversal',
    'Error Model (User Error vs Compiler Fault)',
    'Testing and Validation Requirements',
    'Unsafe Code Policy',
    'Commenting Rules',
);
my @required_testing_phrases = (
    '`cargo check`',
    '`cargo clippy --all-targets -- -D warnings`',
    'at least one minimal targeted test',
);
my @required_unsafe_phrases = (
    '// SAFETY:',
    'safe alternatives',
);
my @required_pr_checklist_phrases = (
    'Behavior is deterministic',
    'Relevant tests and checks were executed',
    'Docs updated if CLI/runtime/error semantics changed.',
);

sub display_path {
    my ($path) = @_;
    my $resolved = abs_path($path) // $path;
    my $norm_root = normalize_path($ROOT);
    my $norm_path = normalize_path($resolved);
    return substr($norm_path, length($norm_root) + 1)
        if index($norm_path, "$norm_root/") == 0;
    return $norm_path;
}

sub path_matches_prefix {
    my ($path, $prefix) = @_;
    my $normalized = normalize_path($path);
    my $want = normalize_path($prefix);
    return 1 if $normalized eq $want;
    return 1 if index($normalized, "$want/") == 0;
    return 1 if $normalized =~ m{/\Q$want\E$};
    return 1 if $normalized =~ m{/\Q$want\E/};
    return 0;
}

sub audit_key {
    my ($path) = @_;
    my $normalized = normalize_path($path);
    my @parts = grep { length } split m{/}, $normalized;
    return 'CONTRIBUTING.md' if @parts && $parts[-1] eq 'CONTRIBUTING.md';
    my %known = map { $_ => 1 } qw(src tests docs scripts fuzz native policy .github);
    for (my $idx = $#parts - 1; $idx >= 0; --$idx) {
        return join('/', @parts[$idx .. $#parts]) if $known{$parts[$idx]};
    }
    $normalized =~ s{^\./}{};
    return $normalized;
}

sub load_paths {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @paths;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line eq q{};
        my $value = $line;
        my $resolved = File::Spec->file_name_is_absolute($value)
            ? $value
            : File::Spec->catfile($ROOT, $value);
        push @paths, $resolved;
    }
    close $fh;
    return @paths;
}

sub load_ast_findings {
    my ($path) = @_;
    open my $fh, '<', $path or die "failed to read $path: $!";
    my @out;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line eq q{};
        my ($level, $code, $shown, $line_no, $message) = split /\t/, $line, 5;
        push @out, [$level, $code, $shown, 0 + $line_no, $message];
    }
    close $fh;
    return @out;
}

sub is_core_compiler_path {
    my ($path) = @_;
    for my $prefix (@core_compiler_prefixes) {
        return 1 if path_matches_prefix($path, $prefix);
    }
    return 0;
}

sub production_lines {
    my ($lines) = @_;
    my @out;
    for my $line (@{$lines}) {
        last if $line =~ /^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/;
        push @out, $line;
    }
    return \@out;
}

sub is_comment {
    my ($line) = @_;
    return $line =~ /^\s*\/\//;
}

sub has_allow_marker {
    my ($line) = @_;
    return index($line, 'audit: allow') >= 0;
}

sub strip_string_literals {
    my ($line) = @_;
    my $out = q{};
    my $in_string = 0;
    my $escaped = 0;
    for my $ch (split //, $line) {
        if ($in_string) {
            if ($escaped) {
                $escaped = 0;
                next;
            }
            if ($ch eq '\\') {
                $escaped = 1;
                next;
            }
            if ($ch eq '"') {
                $in_string = 0;
                $out .= '"';
            }
            next;
        }
        if ($ch eq '"') {
            $in_string = 1;
            $out .= '"';
            next;
        }
        $out .= $ch;
    }
    return $out;
}

sub has_safety_comment {
    my ($lines, $idx) = @_;
    my $start = $idx - 3;
    $start = 0 if $start < 0;
    for (my $prev = $idx - 1; $prev >= $start; --$prev) {
        my $text = $lines->[$prev];
        $text =~ s/^\s+|\s+$//g;
        next if $text eq q{};
        return 1 if $text =~ m{^// SAFETY:};
        return 0 if $text !~ m{^//};
    }
    return 0;
}

sub has_safe_alt_comment {
    my ($lines, $idx) = @_;
    my $start = $idx - 5;
    $start = 0 if $start < 0;
    for (my $prev = $idx - 1; $prev >= $start; --$prev) {
        my $text = $lines->[$prev];
        $text =~ s/^\s+|\s+$//g;
        next if $text eq q{};
        if ($text =~ m{^//}) {
            my $lower = lc($text);
            for my $marker (@safe_alt_markers) {
                return 1 if index($lower, $marker) >= 0;
            }
            next;
        }
        return 0;
    }
    return 0;
}

sub actionable_comment_error {
    my ($line) = @_;
    return undef if $line !~ /^\s*\/\//;
    (my $comment = $line) =~ s/^\s*\/\/\s*//;
    return ['comment-prefix', 'TODO comments must use // TODO(name/issue): rationale']
        if $comment =~ /^TODO/ && $comment !~ $structured_todo_re;
    return ['comment-prefix', 'FIXME comments must use // FIXME: rationale']
        if $comment =~ /^FIXME/ && $comment !~ $structured_fixme_re;
    return ['comment-prefix', 'NOTE comments must use // NOTE: rationale']
        if $comment =~ /^NOTE/ && $comment !~ $structured_note_re;
    return ['comment-prefix', 'replace // XXX with a structured // FIXME: or // NOTE: comment']
        if $comment =~ /^XXX/;
    return undef;
}

sub collect_headings {
    my ($lines) = @_;
    my @headings;
    for my $idx (0 .. $#{$lines}) {
        my $line = $lines->[$idx];
        if ($line =~ $heading_re) {
            push @headings, [$idx + 1, length($1), $2];
        }
    }
    return \@headings;
}

sub extract_section_body {
    my ($lines, $headings, $wanted) = @_;
    for my $i (0 .. $#{$headings}) {
        my ($line_no, $level, $title) = @{$headings->[$i]};
        next if $title ne $wanted;
        my $end_line = scalar(@{$lines}) + 1;
        for my $j (($i + 1) .. $#{$headings}) {
            my ($next_line, $next_level) = @{$headings->[$j]};
            if ($next_level <= $level) {
                $end_line = $next_line;
                last;
            }
        }
        my @slice = @{$lines}[($line_no) .. ($end_line - 2)];
        my $body = join "\n", @slice;
        $body =~ s/^\s+|\s+$//g;
        return [$line_no, $body];
    }
    return undef;
}

sub add_record {
    my ($bucket, $code, $shown, $line_no, $message) = @_;
    push @{$bucket}, [$code, $shown, $line_no, $message];
}

sub audit_contributing_doc {
    my ($shown, $lines, $errors, $warnings) = @_;
    my $text = join "\n", @{$lines};
    my $headings = collect_headings($lines);
    my %top_sections = map { $_->[2] => $_->[0] } grep { $_->[1] == 2 } @{$headings};
    my @rule_sections;

    for my $title (@required_contributing_sections) {
        add_record($errors, 'contributing-section', $shown, 0, "CONTRIBUTING.md missing required section '## $title'")
            if !exists $top_sections{$title};
    }

    for my $heading (@{$headings}) {
        my ($line_no, $level, $title) = @{$heading};
        next if $level != 3;
        next if $title !~ /^(\d+)\)\s+(.+)$/;
        push @rule_sections, [$line_no, 0 + $1, $2];
    }

    if (!@rule_sections) {
        add_record($errors, 'contributing-rules', $shown, ($top_sections{'Rules'} // 0), "CONTRIBUTING.md must define numbered rule sections under '## Rules'");
    } else {
        my @numbers = map { $_->[1] } @rule_sections;
        my @expected = (1 .. $numbers[-1]);
        if (join(',', @numbers) ne join(',', @expected)) {
            add_record($errors, 'contributing-rule-order', $shown, $rule_sections[0][0], "numbered rule headings must be contiguous starting at 1; found [" . join(', ', @numbers) . "]");
        }

        my %titles = map { $_->[2] => 1 } @rule_sections;
        for my $topic (@required_contributing_rule_topics) {
            add_record($errors, 'contributing-rule-topic', $shown, ($top_sections{'Rules'} // 0), "CONTRIBUTING.md missing rule topic '$topic'")
                if !$titles{$topic};
        }

        for my $rule (@rule_sections) {
            my ($line_no, $number, $title) = @{$rule};
            my $section = extract_section_body($lines, $headings, "$number) $title");
            next if !$section;
            my $body = $section->[1];
            add_record($warnings, 'contributing-rule-strength', $shown, $line_no, "rule section '$title' has no MUST/SHOULD/MAY guidance")
                if $body !~ /\b(?:MUST|SHOULD|MAY)\b/;
        }
    }

    my ($testing_rule) = grep { $_->[2] eq 'Testing and Validation Requirements' } @rule_sections;
    if ($testing_rule) {
        my $section = extract_section_body($lines, $headings, "$testing_rule->[1]) $testing_rule->[2]");
        if ($section) {
            for my $phrase (@required_testing_phrases) {
                add_record($errors, 'contributing-testing', $shown, $section->[0], "testing requirements should mention $phrase")
                    if index($section->[1], $phrase) < 0;
            }
        }
    }

    my ($unsafe_rule) = grep { $_->[2] eq 'Unsafe Code Policy' } @rule_sections;
    if ($unsafe_rule) {
        my $section = extract_section_body($lines, $headings, "$unsafe_rule->[1]) $unsafe_rule->[2]");
        if ($section) {
            for my $phrase (@required_unsafe_phrases) {
                add_record($errors, 'contributing-unsafe', $shown, $section->[0], "unsafe policy should mention $phrase")
                    if index($section->[1], $phrase) < 0;
            }
        }
    }

    my $checklist = extract_section_body($lines, $headings, 'PR Checklist');
    if ($checklist) {
        for my $phrase (@required_pr_checklist_phrases) {
            add_record($errors, 'contributing-pr-checklist', $shown, $checklist->[0], "PR checklist should mention '$phrase'")
                if index($checklist->[1], $phrase) < 0;
        }
    }

    add_record($warnings, 'contributing-doc-link', $shown, 0, 'CONTRIBUTING.md should link to docs/compiler/contributing-audit.md for the concrete verification pass')
        if index($text, 'docs/compiler/contributing-audit.md') < 0;
}

sub read_utf8_text {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "failed to read $path: $!";
    local $/;
    my $raw = <$fh>;
    close $fh;
    my $text;
    eval { $text = decode('UTF-8', $raw, FB_CROAK); 1 } or return undef;
    return $text;
}

my @paths = load_paths($file_list_path);
my @ast_findings = load_ast_findings($ast_path);
my @display_paths = map { display_path($_) } @paths;
my @audit_paths = map { audit_key($_) } @display_paths;

my @errors;
my @warnings;
my @manual;

sub add_manual {
    my ($item) = @_;
    push @manual, $item if !grep { $_ eq $item } @manual;
}

my $docs_touched = scalar(grep { $_ =~ m{^docs/} } @audit_paths) > 0;
my $tests_touched = scalar(grep { $_ =~ m{^tests/} } @audit_paths) > 0;
my $contributing_changed = scalar(grep { $_ eq 'CONTRIBUTING.md' } @audit_paths) > 0;
my $contributing_audit_doc_changed = scalar(grep { path_matches_prefix($_, 'docs/compiler/contributing-audit.md') } @audit_paths) > 0;
my $contributing_policy_changed = scalar(grep { path_matches_prefix($_, 'policy/contributing_rules.toml') } @audit_paths) > 0;
my $interesting_docs_changed = scalar(grep { my $p = $_; scalar grep { path_matches_prefix($p, $_) } @interesting_doc_prefixes } @audit_paths) > 0;
my $interesting_tests_changed = scalar(grep { my $p = $_; scalar grep { path_matches_prefix($p, $_) } @interesting_test_prefixes } @audit_paths) > 0;
my $pass_sensitive_changed = scalar(grep { my $p = $_; scalar grep { path_matches_prefix($p, $_) } @pass_sensitive_prefixes } @audit_paths) > 0;
my $cache_sensitive_changed = scalar(grep { my $p = $_; scalar grep { path_matches_prefix($p, $_) } @cache_sensitive_prefixes } @audit_paths) > 0;
my $contributing_generated_docs_synced = !$contributing_policy_changed
    || run_status('python3', File::Spec->catfile($ROOT, 'scripts', 'render_contributing_docs.py'), '--check');

for my $idx (0 .. $#paths) {
    my $path = $paths[$idx];
    my $shown = $display_paths[$idx];
    my $audit = $audit_paths[$idx];
    next if !-e $path || -d $path;

    my $text = read_utf8_text($path);
    if (!defined $text) {
        add_record(\@warnings, 'binary-skip', $shown, 0, 'skipped non-UTF-8 file during static audit');
        next;
    }

    my @all_lines = split /\n/, $text, -1;
    pop @all_lines if @all_lines && $all_lines[-1] eq q{};
    my $prod_lines = production_lines(\@all_lines);
    my $is_production_src = ($audit =~ m{^src/} && $audit !~ m{^src/legacy/} && $audit !~ m{/tests\.rs$}) ? 1 : 0;
    my $is_core_compiler_src = is_core_compiler_path($audit);
    my $is_rust_file = $shown =~ /\.rs$/ ? 1 : 0;

    audit_contributing_doc($shown, \@all_lines, \@errors, \@warnings) if $audit eq 'CONTRIBUTING.md';

    my $ast_prod_boundary = scalar(@{$prod_lines});
    for my $finding (@ast_findings) {
        my ($level, $code, $ast_shown, $line_no) = @{$finding};
        next if $ast_shown ne $shown;
        next if $line_no > scalar(@all_lines);
        if ($code =~ /^ast-production-(panic|unwrap|dbg)$/) {
            next if !$is_production_src || $line_no > $ast_prod_boundary;
            my %mapped = (
                'ast-production-panic'  => ['production-panic', 'production compiler path contains panic!; use RRException/ICE flow instead'],
                'ast-production-unwrap' => ['production-unwrap', 'production compiler path contains unwrap()/expect()'],
                'ast-production-dbg'    => ['production-dbg', 'production compiler path contains dbg!(); use structured logging or diagnostics instead'],
            );
            add_record(\@errors, $mapped{$code}[0], $shown, $line_no, $mapped{$code}[1]);
            next;
        }
        if ($code eq 'ast-inline-always') {
            add_record(\@errors, 'inline-always', $shown, $line_no, '#[inline(always)] requires benchmark-backed justification');
            next;
        }
        if ($code eq 'ast-for-each-review' && $is_production_src) {
            add_record(\@warnings, 'for-each-review', $shown, $line_no, 'review chained for_each/try_for_each for hidden side effects');
            next;
        }
        if ($code eq 'ast-static-mut' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@errors, 'static-mut', $shown, $line_no, 'core compiler path contains static mut; use explicit state threading or a documented synchronization primitive');
            next;
        }
        if ($code eq 'ast-nondeterministic-rng' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@errors, 'nondeterministic-rng', $shown, $line_no, 'core compiler path touches runtime randomness; plumb a fixed seed or deterministic input instead');
            next;
        }
        if ($code eq 'ast-mutable-global-review' && $is_core_compiler_src) {
            add_record(\@warnings, 'mutable-global-review', $shown, $line_no, 'review mutable global state to confirm it cannot affect compilation results');
            next;
        }
        if ($code eq 'ast-wall-clock-review' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@warnings, 'wall-clock-review', $shown, $line_no, 'review wall-clock access to confirm it cannot affect deterministic compilation results');
            next;
        }
        if ($code eq 'ast-current-dir-review' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@warnings, 'current-dir-review', $shown, $line_no, 'review current_dir usage to confirm cwd-dependent paths are normalized before affecting compilation results');
            next;
        }
        if ($code eq 'ast-temp-dir-review' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@warnings, 'temp-dir-review', $shown, $line_no, 'review temp_dir usage to confirm environment-specific paths stay outside correctness-affecting artifacts');
            next;
        }
        if ($code eq 'ast-thread-spawn-review' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@warnings, 'thread-spawn-review', $shown, $line_no, 'review thread::spawn usage for deterministic scheduling, shutdown, and ownership boundaries');
            next;
        }
        if ($code eq 'ast-process-command-review' && $is_core_compiler_src && $line_no <= $ast_prod_boundary) {
            add_record(\@warnings, 'process-command-review', $shown, $line_no, 'review Command::new usage for hermeticity, cwd normalization, and environment handling');
            next;
        }
        if ($code eq 'ast-hash-order-review' && $is_core_compiler_src) {
            add_record(\@warnings, 'hash-order-review', $shown, $line_no, 'collected hash-backed keys/values into a vector without a nearby sort; confirm deterministic order explicitly');
            next;
        }
    }

    if ($is_production_src) {
        for my $i (0 .. $#{$prod_lines}) {
            my $line_no = $i + 1;
            my $line = $prod_lines->[$i];
            my $stripped = $line;
            $stripped =~ s/^\s+|\s+$//g;
            next if $stripped eq q{} || is_comment($line) || has_allow_marker($line);
            my $code_line = $is_rust_file ? strip_string_literals($line) : $line;
            if (!$is_rust_file && $panic_re && $line =~ $panic_re) {
                add_record(\@errors, 'production-panic', $shown, $line_no, 'production compiler path contains panic!; use RRException/ICE flow instead');
            }
            if (!$is_rust_file && ($line =~ $unwrap_re || $line =~ $expect_re)) {
                add_record(\@errors, 'production-unwrap', $shown, $line_no, 'production compiler path contains unwrap()/expect()');
            }
            if (!$is_rust_file && $line =~ $dbg_re) {
                add_record(\@errors, 'production-dbg', $shown, $line_no, 'production compiler path contains dbg!(); use structured logging or diagnostics instead');
            }
            if ($code_line =~ $unsafe_re) {
                if (!has_safety_comment($prod_lines, $i)) {
                    add_record(\@errors, 'unsafe-missing-safety', $shown, $line_no, 'unsafe usage missing adjacent // SAFETY: rationale');
                } elsif (!has_safe_alt_comment($prod_lines, $i)) {
                    add_record(\@warnings, 'unsafe-safe-alt-review', $shown, $line_no, 'unsafe block should explain why safe alternatives were insufficient');
                }
            }
        }
    }

    for my $i (0 .. $#all_lines) {
        my $line_no = $i + 1;
        my $line = $all_lines[$i];
        my $stripped = $line;
        $stripped =~ s/^\s+|\s+$//g;
        next if $stripped eq q{} || has_allow_marker($line);
        if ($is_rust_file && is_comment($line)) {
            my $comment_error = actionable_comment_error($line);
            add_record(\@errors, $comment_error->[0], $shown, $line_no, $comment_error->[1]) if $comment_error;
            next;
        }
        next if is_comment($line);
    }
}

add_record(\@warnings, 'docs-review', '(scope)', 0, 'runtime/optimizer/codegen surface changed without docs/** updates in the audit scope')
    if $interesting_docs_changed && !$docs_touched;
add_record(\@warnings, 'tests-review', '(scope)', 0, 'compiler implementation files changed without tests/** updates in the audit scope')
    if $interesting_tests_changed && !$tests_touched;
add_record(\@warnings, 'cache-tests-review', '(scope)', 0, 'cache/incremental logic changed without tests/** updates in the audit scope')
    if $cache_sensitive_changed && !$tests_touched;
add_record(\@warnings, 'contributing-doc-sync', '(scope)', 0, 'CONTRIBUTING.md changed without docs/compiler/contributing-audit.md update in the audit scope')
    if $contributing_changed && !$contributing_audit_doc_changed;
add_record(\@warnings, 'contributing-policy-sync', '(scope)', 0, 'policy/contributing_rules.toml changed without refreshing generated contributing docs and PR template in the audit scope')
    if $contributing_policy_changed && !$contributing_generated_docs_synced;

add_manual('confirm externally visible behavior is still deterministic anywhere ordering, hashing, or parallel scheduling could matter');
add_manual('confirm touched semantic areas such as cache behavior, fallback behavior, numeric semantics, and IR invariants still match intent');
add_manual('confirm hot paths did not pick up hidden allocation, formatting, clone cost, or other non-obvious work beyond what automation can prove');
add_manual('confirm unsafe, mutable global state, wall-clock access, cwd access, and temp-path usage cannot change compilation correctness');
add_manual('confirm docs, benchmark evidence, and any exception notes are still accurate rather than placeholder text');

sub dedupe_records {
    my (@records) = @_;
    my %seen;
    my @out;
    for my $record (@records) {
        my $key = join "\x1e", @{$record};
        next if $seen{$key}++;
        push @out, $record;
    }
    return @out;
}

@errors = dedupe_records(@errors);
@warnings = dedupe_records(@warnings);

print "== static rule scan ==\n";
print "files scanned: " . scalar(@paths) . "\n";
for my $record (@errors) {
    my ($code, $shown, $line_no, $message) = @{$record};
    my $location = $line_no == 0 ? $shown : "$shown:$line_no";
    print "error[$code] $location: $message\n";
}
for my $record (@warnings) {
    my ($code, $shown, $line_no, $message) = @{$record};
    my $location = $line_no == 0 ? $shown : "$shown:$line_no";
    print "warn[$code] $location: $message\n";
}
print "no static findings\n" if !@errors && !@warnings;
print "manual follow-up:\n";
print "  - $_\n" for @manual;

exit 1 if @errors;

if ($SCAN_ONLY) {
    print "result: PASS (scan-only)\n";
    exit 0;
}

sub run_cmd {
    my ($label, $env_ref, $cmd_ref) = @_;
    $env_ref ||= {};
    print "== $label ==\n";
    local %ENV = %ENV;
    @ENV{keys %{$env_ref}} = values %{$env_ref} if %{$env_ref};
    my $status = system(@{$cmd_ref});
    exit(($status >> 8) || 1) if $status != 0;
}

sub run_cargo_test_binary {
    my ($test_name) = @_;
    run_cmd("cargo test -q --test $test_name", {}, ['cargo', 'test', '-q', '--test', $test_name]);
}

my @clippy_allow_args = (
    '-A', 'clippy::collapsible_match',
    '-A', 'clippy::needless_range_loop',
    '-A', 'clippy::needless_option_as_deref',
    '-A', 'clippy::too_many_arguments',
    '-A', 'clippy::single_char_add_str',
    '-A', 'clippy::collapsible_if',
    '-A', 'clippy::useless_conversion',
    '-A', 'clippy::option_as_ref_deref',
    '-A', 'clippy::needless_borrow',
    '-A', 'clippy::implicit_saturating_sub',
    '-A', 'clippy::collapsible_else_if',
    '-A', 'clippy::if_same_then_else',
    '-A', 'clippy::op_ref',
    '-A', 'clippy::ptr_arg',
    '-A', 'clippy::unnecessary_sort_by',
    '-A', 'clippy::while_let_loop',
    '-A', 'clippy::question_mark',
);

run_cmd('cargo check', {}, ['cargo', 'check']);
run_cmd(
    'cargo clippy --all-targets -- -D warnings (CI allowlist)',
    {},
    ['cargo', 'clippy', '--all-targets', '--', '-D', 'warnings', @clippy_allow_args],
);
run_cmd('cargo test -q', {}, ['cargo', 'test', '-q']);

if ($SKIP_PASS_VERIFY) {
    print "== pass verify ==\n";
    print "skip: requested by --skip-pass-verify\n";
} elsif (scope_needs_pass_verify()) {
    run_cmd('RR_VERIFY_EACH_PASS=1 cargo test -q --test pass_verify_examples', { RR_VERIFY_EACH_PASS => '1' }, ['cargo', 'test', '-q', '--test', 'pass_verify_examples']);
} else {
    print "== pass verify ==\n";
    print "skip: scope does not touch pass-sensitive compiler files\n";
}

if ($SKIP_SEMANTIC_SMOKE) {
    print "== semantic smoke ==\n";
    print "skip: requested by --skip-semantic-smoke\n";
} else {
    if (scope_needs_cache_semantics()) {
        print "== semantic smoke: cache correctness ==\n";
        run_cargo_test_binary('incremental_phase1');
        run_cargo_test_binary('incremental_phase2');
        run_cargo_test_binary('incremental_phase3');
        run_cargo_test_binary('incremental_auto');
        run_cargo_test_binary('incremental_strict_verify');
        run_cargo_test_binary('cli_incremental_default');
    } else {
        print "== semantic smoke: cache correctness ==\n";
        print "skip: scope does not touch incremental/cache-sensitive files\n";
    }

    if (scope_needs_numeric_semantics()) {
        print "== semantic smoke: numeric semantics ==\n";
        run_cargo_test_binary('sccp_overflow_regression');
        run_cargo_test_binary('rr_logic_equivalence_matrix');
        run_cargo_test_binary('opt_level_equivalence');
    } else {
        print "== semantic smoke: numeric semantics ==\n";
        print "skip: scope does not touch numeric/evaluator-sensitive files\n";
    }

    if (scope_needs_fallback_semantics()) {
        print "== semantic smoke: fallback correctness ==\n";
        run_cargo_test_binary('hybrid_fallback');
        run_cargo_test_binary('parallel_optional_fallback_semantics');
        run_cargo_test_binary('native_optional_fallback');
        run_cargo_test_binary('poly_vopt_fallback');
        run_cargo_test_binary('runtime_semantics_regression');
    } else {
        print "== semantic smoke: fallback correctness ==\n";
        print "skip: scope does not touch fallback/runtime-sensitive files\n";
    }

    if (scope_needs_determinism_semantics()) {
        print "== semantic smoke: determinism ==\n";
        run_cargo_test_binary('commercial_determinism');
        run_cargo_test_binary('compiler_parallel_equivalence');
        my $count = $ENV{RR_RANDOM_DIFFERENTIAL_COUNT} // '8';
        my $seed_a = $ENV{RR_RANDOM_DIFFERENTIAL_SEED} // '0xA11CE5EED55AA11C';
        my $seed_b = $ENV{RR_RANDOM_DIFFERENTIAL_SECOND_SEED} // '0x5EED123456789ABC';
        run_cmd(
            "RR_RANDOM_DIFFERENTIAL_COUNT=$count RR_RANDOM_DIFFERENTIAL_SEED=$seed_a cargo test -q --test random_differential",
            {
                RR_RANDOM_DIFFERENTIAL_COUNT => $count,
                RR_RANDOM_DIFFERENTIAL_SEED  => $seed_a,
            },
            ['cargo', 'test', '-q', '--test', 'random_differential'],
        );
        run_cmd(
            "RR_RANDOM_DIFFERENTIAL_COUNT=$count RR_RANDOM_DIFFERENTIAL_SEED=$seed_b cargo test -q --test random_differential",
            {
                RR_RANDOM_DIFFERENTIAL_COUNT => $count,
                RR_RANDOM_DIFFERENTIAL_SEED  => $seed_b,
            },
            ['cargo', 'test', '-q', '--test', 'random_differential'],
        );
    } else {
        print "== semantic smoke: determinism ==\n";
        print "skip: scope does not touch core compiler files\n";
    }
}

if ($SKIP_FUZZ) {
    print "== fuzz smoke ==\n";
    print "skip: requested by --skip-fuzz\n";
} else {
    my $toolchain = $ENV{RUSTUP_TOOLCHAIN} // 'nightly';
    if (system('cargo', "+$toolchain", 'fuzz', '--help') == 0) {
        my $seconds = $ENV{FUZZ_SECONDS} // '1';
        run_cmd("FUZZ_SECONDS=$seconds ./scripts/fuzz_smoke.sh", { FUZZ_SECONDS => $seconds }, ['./scripts/fuzz_smoke.sh']);
    } elsif ($REQUIRE_FUZZ) {
        print STDERR "== fuzz smoke ==\n";
        print STDERR "fail: cargo-fuzz unavailable for toolchain '$toolchain'\n";
        exit 1;
    } else {
        print "== fuzz smoke ==\n";
        print "skip: cargo-fuzz unavailable for toolchain '$toolchain' (use --require-fuzz to fail)\n";
    }
}

print "result: PASS\n";
exit 0;
