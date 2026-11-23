---
id: SR-61009
title: "gap analysis of PR #3588 (QSL-2805): CLI contract tests"
type: SpecReview
analysis: gap-analysis
date: 2026-11-23
scope: "agent-ix/filament-spec-language; PR #3588 git diff origin/main...HEAD at 6b090c68 (tests/it/check_imports.rs, tests/it/request_surface_s1.rs, tests/it/verb_limits.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-869
  type: references
---
# SR-61009: gap analysis of PR #3588 (QSL-2805)

## Summary

Ticket: QSL-2805. This is a planless gap analysis scoped to the criteria the diff's tests are tagged with. `quire matrix --scope . --format json` was run on the worktree at 6b090c68. Plan completion: not assessed. Semantic review was not run as a separate pass. The diff changes only tests, so there is no reverse-gap surface.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-869-AC-1 | spec/functional/FR-869-raise-every-limit-of-parse-check-and-compile-from-the-command-line.md | examined |
| FR-866-AC-3 | spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md | examined |
| FR-662-AC-1 | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md | examined |
| FR-662-AC-2 | spec/functional/FR-662-check-every-native-fence-of-a-markdown-specification.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-869-AC-1: tagged by four binders. With this PR, the un-ignored `tests/it/verb_limits.rs::a_compile_request_over_its_ceiling_names_the_option` adds exit 22, status, code and option-name assertions. `fsl-command/tests/it/compile_limits.rs` covers the raised ceiling, and `tests/it/compile_command.rs` covers the dependent and S1 halves. All parts of the AC are asserted by running tests.
- FR-866-AC-3: the compile half is asserted only by the ignored root test (open QSL-2976). The check half is asserted by `fsl-command/tests/it/check_imports.rs::check_reports_a_missing_model_selection_at_admission`. This is a known, ticketed product gap, not introduced here.
- FR-662-AC-1 and FR-662-AC-2: the library half is asserted in `fsl-replay/tests/it/integer_division_check.rs`. The argv-surface tests stay ignored under open QSL-2977, also a known, ticketed product gap.
- Stubs and coverage inflation: none. The un-ignored test asserts exact values.
- Plan completion: not assessed

## Verdict

**PASS** for the diff's scope. The diff adds no criterion gap. The only partially backed criteria are the ignored tests, and each cites an open ticket that names the missing behaviour.
