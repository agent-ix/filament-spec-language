---
id: SR-2026112014571802
title: "gap-analysis of PR 3196 hybrid process providers (QSL-2532, QSL-2534)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3196 head 2496539bba0ddfb387f57a3408fbf69ceaf005cb; FR-750, FR-450, FR-462 process path; tools/fsl-hybrid-test-provider/tests/it/**, fsl-analyze/tests/it/hybrid_requests.rs"
review_set: subset
---

## Summary

Ticket: QSL-2532 (with QSL-2534). A manual AC-to-test-to-code check, scoped to the diff, of the ACs this PR un-ignores or adds. Plan completion was not assessed. No build was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-750 "Dispatch" ("settle it by the same rules") has no process-path test of FR-250's coverage rule, and none of an entry the hybrid reader does not read (a non-hybrid `proved` basis, `tested`); the untested branch is where code-review FND-001 sits | tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:248-294 |
| FND-002 | medium | FR-450-AC-3/AC-9 require the record to retain the complete counterexample; the new assertions check only `disagreement.type`, and the code does not serialize the counterexample | fsl-analyze/tests/it/hybrid_requests.rs:744-750, tools/fsl-hybrid-test-provider/tests/it/process_provider.rs:68-83 |
| FND-003 | low | FSpec FR-462-AC-5 (cancel) is not exercised on the process path, although the host implements it | tools/fsl-hybrid-test-provider/tests/it/host.rs:146-158 |

## Verdict

These ACs are now real and non-stub: FR-750-AC-5, the process halves of AC-7 and AC-11, AC-13 (process locus) and AC-6 (options reach the provider). No `todo!` remains among them. The process `trusted`/`certified` split is tested (tag FR-751-AC-1). FR-1021-AC-3 stays ignored, correctly, because the validation item comes only from plan synthesis (FR-1021-AC-1 is ignored on QSL-1978 too). The gaps above are untested branches, not stubs presented as done.

## Scope and evidence

- FR-750-AC-5 (spec/functional/FR-750-run-a-hybrid-item-through-its-negotiated-hybrid-provider.md): process fixture refuted point trace reproduces → refuted; non-reproducing → inconclusive replay-parity. Test a_process_provider_refutation_settles_by_exact_replay.
- FR-750-AC-6 (same file): options {"step": "1/16"} reach the provider. Test item_options_and_the_deadline_reach_the_process_provider.
- FR-750-AC-7 (same file): process unsupported expression-form settles located unsupported. Test a_process_unsupported_entry_settles_a_located_expression_form.
- FR-750-AC-11 (same file): failed tool-failure entry is NotRead and settles failed. Test a_process_failed_entry_settles_failed_by_fr_290.
- FR-750-AC-13 (same file): emitted-package locus read back. Same test as AC-7.
- FR-751-AC-1 (spec/functional/FR-751-check-hybrid-provider-certificates-in-the-core.md): accepted inductive-enclosure settles certified; rejected stays uncertified (trusted for a plugin). Test a_process_proof_is_trusted_unless_its_certificate_is_accepted.
- FR-450-AC-3, FR-450-AC-9 (spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md): disagreement retains the complete counterexample.
- FR-1021-AC-3 (spec/functional/FR-1021-plan-over-a-hybrid-model.md): validation item Rational objects; ignored (context).

## Recommendations

- FND-001: add process tests: a `bounded-solver` proof with horizon 1 for a 12 s claim settles `insufficient-horizon`; an `exhaustive` basis and a `tested` result are refused `invalid_result`.
- FND-002: assert `record.cause.parity.disagreement.counterexample` equals the provider's counterexample once code-review FND-004 is fixed.
- FND-003: add the cancel test described in code-review FND-005.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, head 9f1341db9d: only_a_solver_basis_proof_settles_proved_and_it_is_trusted covers the short-horizon insufficient-horizon row, exhaustive and bounded-complete bases, tested/measured/declined, and certification claims |
| FND-002 | fixed | Round 1: AC-5 test asserts disagreement.counterexample.trace equals the provider's point_trace("1"); hybrid_requests.rs asserts the counterexample on the not-exact path |
| FND-003 | fixed | Round 1: two cancel tests (late result, hang, and no hello) |
