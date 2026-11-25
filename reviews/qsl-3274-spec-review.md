---
id: SR-1000001
title: "spec-review of QSL-3274 item 1: no-call refusal reconciled with FR-1047 missing members (filament-spec-language PR 3922)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@ef6a1da11d5e1a47801343506866f3a2cbd142d1; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md; spec/test-cases/TC-3170-request-refusals-teach-the-request-shape.md; spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1047
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
---

# Spec review of QSL-3274 item 1 (PR 3922)

## Summary

Ticket: QSL-3274 item 1. Diff: git diff origin/main...HEAD at head ef6a1da11d5e1a47801343506866f3a2cbd142d1. origin/main has since moved by #3921 (fsl-eval/src/simulation/protocol.rs only), which does not touch these files. Spec-only change; no cargo run.

Examined, clean: FR-1047 "Missing members" exception clause and new "A selected source with no run member" section (first and third bullets); FR-1047-AC-7 and AC-10; FR-100 refusal table split; FR-100 edition bullet; new FR-100 no-call bullet; FR-100-AC-2, AC-3, AC-25; TC-3170 step 7; TC-450 step 5. Context: FR-1039-AC-13, FR-100-AC-22/23, FR-031 extraction checks, FR-640 clocks rule, FR-312-AC-2.

Test compatibility under the amended text:

- `fsl-command/tests/it/request_diagnostics.rs::every_request_example_is_read_back_and_each_member_is_required`: the run example is `models`, `program`, `call`. Without `call` it selects exactly one source and lacks only the group, so FR-1047 now gives `details` `{"missing":[[call,clause,items]]}`, which is what the test asserts. The test does not assert `message`. Removing `models` or `program` keeps the missing-members refusal. Passes.
- `tests/it/spine_run.rs::tc_450_step_5_no_call_refuses_before_dependent_reads`: both requests (program with full models/libraries selections; package with `models` `[]`) select exactly one source, lack nothing else and carry no precedence member (`libraries` beside `program` is not one). So FR-100-AC-25 gives the exact no-call message. Passes.
- `package_member_conflicts_precede_no_call_before_file_reads` (FR-1039-AC-13): matches the second bullet.
- `tests/it/config_version_spine.rs::clause_run_command_refuses_ambiguous_and_missing_selection_members` (FR-312-AC-2, clause removed): the AC-25 message contains `call`. Passes.
- FR-100-AC-2 tests all carry `call`, so the new "In requests that carry `call`" scope breaks none.

Code at the head (`run_shape` returns `RUN_SELECTED` for a selected source with no call, so no group is listed; the program path refuses NoCall only after program intake) still needs QSL-3274's code item. That is expected for a spec-only PR. No test embeds the edited spec files: `standard_pin.rs` only locates files by id prefix, and the names are unchanged. `quire validate` on the four files is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The precedence list in FR-1047's second new bullet is only partly backed by existing requirements. FR-1039-AC-13 gives `libraries` and `refinement_internals` beside `package` precedence over the no-call refusal. `integer_division` and `clocks` beside `package` match current code (`package_run::validate_request` runs before NoCall), but no requirement states that precedence. Precedence for `sources` beside `program` (FR-100-AC-23) and for `extraction-clause-count` (FR-031) is a new ordering decision: neither requirement mentions the no-call case, and no AC or test covers these four (FR-1047-AC-10 covers `libraries` only). The list is also presented as exhaustive, but the program path has other pre-read refusals. For example, `refinement_internals` without `package` (command.rs:4512, refused before NoCall today) is not in the list, so the text now orders the no-call/missing refusal ahead of it, which reverses current behaviour. Fix: either cite or add the backing requirement and an AC row for each listed member, or reduce the list to FR-1039-AC-13's members and state that other member refusals keep their current order. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:228-234 |
| FND-002 | medium | The third new bullet says a request that "selects both ... of `program` and `package`" gets the missing-members message and full list. The FR-100 refusal row it sits beside says a request carrying both "names ... the second member" (unconditional on `call`). For both-plus-no-call the two rows now give different messages. This was already implicit in FR-1047's general rule (and is what `admit` does today), but the new bullet makes it explicit without reconciling the FR-100 row. Fix: qualify the FR-100 row ("when the request lacks no FR-1047 entry"), or drop "both" from the bullet. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:235-237 |
| FND-003 | low | FR-1047-AC-7 gains "the run example without `call` carries FR-1047-AC-10's message", but its traced test (`every_request_example_is_read_back_and_each_member_is_required`) asserts only `details`, so this clause of AC-7 has no assertion. Per the decision not to change test assertions, it repeats AC-10. Fix: drop the clause from AC-7, or record that AC-10's test backs it. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:355 |

## Verdict

The core reconciliation is consistent. A selected source with only the `call`/`clause`/`items` group missing keeps `details.missing` (FR-1047) and carries the FR-100-AC-25 message, refused before any named file, the program included. FR-100, FR-1047, TC-450 and TC-3170 agree with each other on this. Both named tests, and the FR-1039-AC-13 and FR-312-AC-2 tests, pass under the amended text with no assertion change. Behaviour consequence, justified by FR-1047's "before any file the request names is read": a no-call request over a `0-draft`/`7-draft` program now refuses no-call, not `unknown_edition`, with a message that says "a 1-draft program". FR-100's edition bullet and AC-2 are scoped to match. FND-001 and FND-002 are text-precision gaps on edge orderings outside the two tests. They do not affect the decided case.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Reviewed at head 9bac7ba7e8446c47984dbc614a811674e3a21dd2. The `program` row of the new FR-1047 precedence table lists "a model in a format other than `semantic-ir/2.0.0` and `native-model-unit/1` (FR-100)" as a refusal that comes before the no-call refusal with no named file read, and FR-1047-AC-14 tests it in an empty directory (`native-rule-model/1` -> `invalid-request`). Neither the current code nor a stated requirement supports this. The code checks model format only in `prepare_complete_run` (fsl-command/src/command.rs:4988-4994), after it reads the program (`intake.source`) and checks the header (`require_native_header`). Today that request refuses `io-error` at stage `file` for `p.fsl`. FR-100 gates the refusal on the program's edition: the table row is "A `1-draft` function-run request carrying ... a model in a format other than ...", and the bullet is "If the program declares `1-draft`, ... refuse a request carrying a member the refusal table names". The new text already places the no-call refusal before the edition read. It also conflicts with FR-1047-AC-16, which gives AC-10 for the same model at `read_clause_run_request` and over a package. Fix: move the model-format check to the "Every other check comes after" bullet and make AC-14's last case an AC-16 case that refuses as AC-10 states. | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:242 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 1 notes (head 9bac7ba7e8446c47984dbc614a811674e3a21dd2):

- FND-001 is fixed. Each listed member now has a table row tied to a criterion (AC-11 to AC-15), and an explicit "every other check comes after" bullet with AC-16. I checked every row against the code at this head:
  - The size limit runs in `with_request_limits` before `admit`.
  - Decode defects, `requirements` without `verification_plan` and `sources` (`wire::Request::validate`, `runtime_sources::validate_provisions`) are checked in `request()` at all three entries.
  - The clocks override is refused in `request_with_overrides` (`ClocksGivenTwice`).
  - The package members are checked in `package_run::validate_request` before NoCall.
  - On the program path, `refinement_internals` without `package` (4512), `read_integer_division`, `validate_extension_entries`, the file ceiling (`intake_with_bytes`, FILES=64) and the `formal_revision` bind (`wire::SourceFile::bind`, before the file read) all run before NoCall.
  - `extraction-clause-count` is checked in `read_clause_run_request_bytes` before the missing-clause check, and not in `prepare_run`.
  - `read_clause_run_request` performs none of the program-row checks before its missing-clause refusal.
  - The one exception is model format (FND-004).
  - The package rows and AC-10 change `read_clause_run_request`, which today refuses every package request naming `program`. That follows from FR-1039 and AC-10 and is for the code item.
  - The `export_evidence` case is justified by FR-1047 Definitions: run-reader entries read the run reading.
- FND-002 is fixed. The FR-100 table rows are split, and FR-1039 is qualified. "Both" names `program` only when nothing FR-1047 lists is missing, which matches `admit` and `validate_request`.
- FND-003 is fixed. The clause was dropped from FR-1047-AC-7.
- FR-1047-AC-11 to AC-16 are each concrete and testable in process. AC-15 and the extraction cases of AC-16 need `quire-extraction`, which is fsl-command's default build.
- FR-961 note: FR-961 says to prepare the request before building the registry, but `run_configured` builds the registry first. That gap predates this PR, which does not touch it, and the new FR-1047 text explicitly neither states nor changes that order. No test in this PR's scope registers a conflicting provider. It is a separate ticket (FR-961 versus `run_configured`), not a finding here.
