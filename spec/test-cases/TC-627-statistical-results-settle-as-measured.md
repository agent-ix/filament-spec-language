---
id: TC-627
title: "Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-192
    type: verifies
---
# TC-627: Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof

## Description

Verify the outcome-to-record map, the verdict content, the pipeline gate, proof accounting, EN-4's manifest and its refusals.

Scope: FR-192-AC-1 to FR-192-AC-4, FR-192-AC-6, FR-192-AC-9.

## Test Procedure

Fixtures: synthetic `StatisticalOutcome` values for each row; FR-189-AC-1's and FR-190-AC-1's results; a request with `P95` and the `2 ms` claim.

1. Map each outcome row, including a `ConfidenceBeyondPrecision` refusal, two `Stopped` outcomes by an expansion naming `EvaluationMeter`, one whose exhausted counter is `work_units` under a meter budget whose `work_units` is 4096 and one whose exhausted counter is `value_occurrences` under a budget whose `value_occurrences` is 7, a `Stopped` by an expansion naming `MaxCandidates` under `max_candidates` 2, and two expansion stops naming no limit, one an internal failure and one not.
2. Settle FR-189-AC-1's Okamoto and SPRT results and FR-190-AC-1's result; inspect each verdict's `bound`.
3. Gate the two-item request, then the request without the rejected item; read the proof summary.
4. Read EN-4's manifest; run EN-4's gate (FR-195) on the checked claims: an every-scheduler claim, `NoFault` without confidence and an `expected accumulate` claim.
5. Settle a quantile claim's two-test outcomes whose least favourable test has `quantile` `Value(2)` and `Censored`, and a quantile claim whose least favourable test has no estimate, the `P95`, mean-of-a-fraction and `LongRun` results, and the `>= 3` quantile claim of FR-192-AC-6 with estimates `2/5` and `9/20`.
6. Settle a fixed-N preflight stop above `max_samples` and `max_draws`, an SPRT stop at `max_draws`, and FR-189-AC-8's undefined first sample; submit accepted, rejected and undecided measured verdicts with the correct bound, then remove or alter each bound.

Tag the tests `#[trace("TC-627", "FR-192-AC-n")]`.

## Expected Results

- Step 1: each row as FR-192's table states, with no truth and no basis; the `ConfidenceBeyondPrecision` refusal settles `unsupported`, `confidence-beyond-precision`; the meter stop naming `work_units` under a `work_units` budget of 4096 settles `incomplete`, `limit-reached` with limit `work_units`, value 4096 and setting the `work_units` counter's FR-255 accounting setting name, the meter stop naming `value_occurrences` under a budget of 7 settles with limit `value_occurrences`, value 7 and that counter's setting name, and the candidate stop under `max_candidates` 2 settles with limit `max_candidates`, value 2 and setting `expansion.max_candidates`; the internal unnamed stop settles `failed`, `runtime_invariant`/`established-invariant-broken`; the non-internal unnamed stop settles `failed` carrying the stop's own `ExpansionStop.cause` (FSL FR-120) as its one diagnostic.
- Step 2: the values of FR-192-AC-2; accepted, rejected and undecided verdicts each carry the exact comparison and threshold from their checked claim.
- Step 3: the gate fails as on one violation, then passes; 0 proved.
- Step 4: the manifest equals the one FR-280-AC-4 fixes; `EveryScheduler`, `MissingConfidence`, `ExactOnlyForm`, each `unsupported` with no sample drawn.
- Step 5: the verdict `quantile` `Value(2)` and `Censored`; absent for the no-estimate quantile claim; absent for the other three; `estimate` `9/20` for the `>= 3` claim.
- Step 6: preflight stops settle incomplete with `limit-reached{limit, value, setting, required}` carrying the correct setting, configured value and required N, and have zero samples/draws; SPRT stop reports reached sample/draw counts; FR-189-AC-8's first undefined sample reports `draws = 1`, `samples = 0`, and its test result records `UndefinedEvaluation`; correct bound objects are admitted and missing, malformed or claim-mismatched bounds are rejected.
