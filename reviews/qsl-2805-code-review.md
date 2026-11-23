---
id: SR-61008
title: "code review of PR #3588 (QSL-2805): un-ignore the compile request ceiling test, re-own four CLI tests"
type: SpecReview
analysis: code-review
date: 2026-11-23
scope: "agent-ix/filament-spec-language; PR #3588 git diff origin/main...HEAD at 6b090c68 (tests/it/check_imports.rs, tests/it/request_surface_s1.rs, tests/it/verb_limits.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-869
  type: references
---
# SR-61008: code review of PR #3588 (QSL-2805)

## Summary

Ticket: QSL-2805. Code review with the rust-review lane folded in, scoped to the three test files the PR changes (24 insertions, 14 deletions, no production code, no reformat of unrelated lines). The PR un-ignores `verb_limits::a_compile_request_over_its_ceiling_names_the_option` and rewrites it against FR-869-AC-1, and re-points the ignore reasons of four still-failing tests to QSL-2976 and QSL-2977.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-869-AC-1 | spec/functional/FR-869-raise-every-limit-of-parse-check-and-compile-from-the-command-line.md | examined |
| TC-2461 | spec/test-cases/TC-2461-compile-entry-takes-caller-limits.md | examined |
| FR-026-AC-7 | spec/functional/FR-026-run-standalone-native-workflow.md | context_only |
| FR-287-AC-4 | spec/functional/FR-287-reach-fsl-through-the-driver-cli.md | context_only |
| FR-1047 | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md | context_only |
| FR-866-AC-3 | spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md | examined |
| FR-662-AC-1 | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md | examined |
| FR-662-AC-2 | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md | examined |
| check_argv | fsl-command/src/command.rs | examined |
| compile_outcome_inner | fsl-command/src/command.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The ignore reason on check_without_markdown_reads_native_source is copied from the two --markdown tests and ends "instead of reporting per-fence items"; this test's first call has no --markdown and expects the native invalid_syntax refusal, not per-fence items. The argv cause (--format not parsed) is right; the stated expected behaviour is wrong for this test. | tests/it/request_surface_s1.rs:1216 |
| FND-002 | low | Duplicate owning tickets. The re-pointed tests were owned by QSL-2879 (verb_limits, check_imports) and QSL-2876 (the three Markdown tests), both still open in Backlog. QSL-2976 and QSL-2977 now own the same tests with no duplicate relation, and QSL-2879 still lists the verb_limits test this PR fixes. Two open tickets per test is tracker churn: mark QSL-2876 a duplicate of QSL-2977, and trim QSL-2879 (or relate it to QSL-2976). | tests/it/check_imports.rs:17, tests/it/request_surface_s1.rs:1144, tests/it/request_surface_s1.rs:1183, tests/it/request_surface_s1.rs:1216 |

## Verdict

**Mergeable; two low findings, neither blocks.** The verb_limits rewrite is right per the spec. FR-869-AC-1 and TC-2461 step 1 say an over-ceiling compile request settles `incomplete` with `resource_exhausted`, exit 22, naming `request_bytes`, its maximum and `--request-bytes`, through `compile_outcome_with_limits_and_overrides`. FR-869 "Decision: compile's intake ceilings become options" makes the old exit-20 refusal expectation stale. FR-1047 covers missing and unread request members, not request size, so it is not the clause this test should reach. The new assertions are exact: exit 22, code, status, limit, maximum 1048576 and the option string. They are stronger than the old `contains` checks. `compile_file` was already in process (`command::compile`), so the switch from the harness to the library entry does not drop a CLI surface. The test now calls the entry FR-869 names. The raised-ceiling half of FR-869-AC-1 is asserted by `fsl-command/tests/it/compile_limits.rs::compile_request_byte_limit_is_caller_configurable`, and the dependent and S1 halves by `tests/it/compile_command.rs::a_library_source_over_the_default_dependent_byte_ceiling_can_be_raised`, so the test's narrow scope leaves no AC gap. The four remaining product gaps are real. `check_argv` matches only `check <source>`, `check <authority> <name> <source>` and `check --extensions ...`, so `--markdown` and `--format` fall to the invalid-request arm. The compile-path missing_import message lacks `--model` and `models` (QSL-2976 records the measured message). These are specified features (FR-662, FSpec FR-563, FR-866-AC-3), owned by open tickets, with no AC narrowed. No other `#[ignore]` remains in the three files. Evidence: run by name on titan at 6b090c68 (`cargo test -j 3 --test it verb_limits::a_compile_request_over_its_ceiling_names_the_option -- --exact --test-threads=3`, debuginfo 0), the test passed: 1 passed, 0 failed. The target directory was deleted afterwards.

Outside the diff, noted only: `tests/it/compile_command.rs::compile_uses_fixed_byte_ceilings` (FR-027-AC-3) still asserts that the plain `command::compile` entry names no option. FR-287-AC-4 says `quire compile`'s over-limit outcome names the option. That is the driver's surface, not this PR's.
