---
id: SR-61006
title: "code review of PR #3256 (QSL-2725): refuse extracted bodies declaring more than one clause"
type: SpecReview
analysis: code-review
date: 2026-11-21
scope: "agent-ix/filament-spec-language; PR #3256 git diff origin/main...HEAD (fsl-replay/src/spine/clause.rs, fsl-replay/src/spine/clause/trace_subject.rs, fsl-replay/src/spine/native_result.rs, fsl-replay/src/spine/clause/tests.rs, fsl-command/tests/it/native_workflow_family_cases.rs, tests/it/request_surface_s1.rs, spec/functional/FR-031-run-extracted-native-source.md, spec/test-cases/TC-109-extracted-command.md)"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-031
  type: references
---
# SR-61006: code review of PR #3256 (QSL-2725)

## Summary

Ticket: QSL-2725. This is a code review with the rust-review lane folded in, scoped to the PR diff. The review pass ran at head dc74b683. It was rebuilt from the review comment posted on Linear QSL-2725, because the reviewer that ran it could not commit this file.

The reviewer ran the root test `the_run_command_writes_a_strictly_readable_markdown_extraction_record` at dc74b683. It failed with exit 101: left 20, right 0 at request_surface_s1.rs:565, disposition compile/extraction-clause-count, fields {actual: "3"}.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-031-AC-9 | spec/functional/FR-031-run-extracted-native-source.md | examined |
| FR-031 | spec/functional/FR-031-run-extracted-native-source.md | examined |
| FR-031-AC-5 | spec/functional/FR-031-run-extracted-native-source.md | examined |
| FR-031-AC-10 | spec/functional/FR-031-run-extracted-native-source.md | examined |
| FR-031-AC-1 | spec/functional/FR-031-run-extracted-native-source.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The root test for FR-031-AC-5/AC-10 extracts the 3-clause FR-108 spine unit. The new count refuses it (actual 3, exit 20), but the test asserts exit 0. AC-1's wording names the same unit as the fence body, which AC-9 now refuses. | tests/it/request_surface_s1.rs:543-566, examples/config-version/spine_unit.rs:28-43 |
| FND-002 | medium | The count check on the batch route (compile_program_parts) and on the trace-subject route has no test. Deleting either check leaves every test green, so two of the three claimed routes are unverified. | fsl-replay/src/spine/clause.rs:2433-2438, fsl-replay/src/spine/clause/trace_subject.rs:83-90 |
| FND-003 | low | Each extracted body is parsed twice: once for the count, then again by the front end. The count could come from the front end's parsed unit, which removes the second parse, limit and cancel pass. | fsl-replay/src/spine/clause.rs:311-342 |

## Verdict

**Changes requested at dc74b683.** FND-001 breaks an existing acceptance test. The refusal itself is correct on the single-run route: stage compile, category refusal, exit 20, no truth.

## Disposition pass 1 scope and evidence

Disposition reviewer (a replacement, not the original reviewer) at head `e3c70d886fdb86704ef453329999e0cca248415d`. The fix commit is e3c70d88 on top of 19286a85. Evidence is the diff and the git objects only. No cargo build or test was run by this reviewer. The coder reports focused runs: fsl-replay lib 6 passed, fsl-command it 2 passed, root it 2 passed.

- FND-001: the root test now extracts the spine unit's header and ParentOrder only (`take_while` stops at `invariant NoCycle`), which is one clause. Its assertions are unchanged. `examples/config-version/spine_unit.rs` is not in the diff. FR-031-AC-1 and TC-109 step 1 now say the fence holds FR-108's healthy-parent clause as a one-clause unit. Every Markdown `ix:native` fence in the Rust tests was checked. The only other multi-clause extracted body is `prepared_command.rs:673` (FR-312-AC-10). It refuses at stage `request` (`invalid-request`, observation_limits) before compile, so the count never applies to it. The other fences hold one-clause bodies (`body()`, `unit_and_packages()`), function-only units (integer_division_check), or opaque or unselected bodies.
- FND-002: three new library tests, one per route: `run_clause`, `run_clause_batch` (`CompileRefusal`) and `run_trace_subject_clause`. Each asserts `ExtractionClauseCount { actual: 2 }`, and the run_clause test also checks stage and truth. Each test fails if its route's check is deleted.
- FND-003: `extracted_clause_count(unit, parsed)` reads `parsed.unit().forms()` from the front end's own parse inside each of the three compile closures. The separate pre-parse is gone.
- Ordering on all three routes: the count runs right after `front_end::parse` and before `select`, check, emit, admission and evaluation. `compile_program_context` and `clause_matches_compilation` force `language: None`, so an extracted source cannot skip the check through a precompiled context. All three routes return the same `ClauseDisposition::ExtractionClauseCount`. Its stage (compile), category (refusal, exit 20) and wire rendering (native_result.rs: code `extraction-clause-count`, `fields` {"actual": "n"}, no details) are shared, so a count refusal has the same code, stage and exit on every route. Program sources are never counted (`unit.language?`).
- Ruling: the FR-031 description, AC-9 and TC-109 step 8 state `fields` {"actual": "2"}, written as system behaviour.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | Some new test code is not rustfmt-clean: there is a double blank line before the first new test, a 101-column match arm, and no blank line before the FR-105 section comment. Fix: rustfmt the new block only. | fsl-replay/src/spine/clause/tests.rs:3657, fsl-replay/src/spine/clause/tests.rs:3689, fsl-replay/src/spine/clause/tests.rs:3736 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The root test extracts a one-clause body with its assertions unchanged. The fixture is untouched. AC-1 and TC-109 step 1 are aligned with AC-9. No other extracted multi-clause body reaches compile. |
| FND-002 | fixed | One new test per route (run_clause, run_clause_batch, run_trace_subject_clause) asserts ExtractionClauseCount { actual: 2 }. |
| FND-003 | fixed | The count reads the front end's parsed unit inside each compile closure, so each body is parsed once. |

## Current verdict after disposition pass 1

**PASS.** FND-001, FND-002 and FND-003 are fixed. FND-004 is a low formatting nit that does not block the merge.
