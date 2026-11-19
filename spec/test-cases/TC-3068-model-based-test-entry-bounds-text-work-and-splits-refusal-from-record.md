---
id: TC-3068
title: "The model-based test entry bounds Text work and splits request refusals from settled records"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-3068: The model-based test entry bounds Text work and splits request refusals from settled records

## Description

Verify `run_model_based_test`'s contract: Text count and value work charged to the run's accounting meter before allocation, request refusals as `Err`, every FSpec FR-190 settlement as a record, the `RootAdmission` seam, and the plan coordinator's limit mapping. Scope: FR-752-AC-10, FR-752-AC-11.

Catches a generator that allocates a caller-sized Text table before charging it, panics or settles `unsupported` on a large Text root, returns oracle disagreement as an error, admits refusal cases through anything other than `admit_arguments`, or ignores `AnalyzeLimits.simulation`.

## Test Procedure

1. Run the value clause `Huge` (`t: Text[0, 1099511627776; nfc]`, body `size(t) >= 0`) with seed 7, `max_cases` 30 and `accounting.work_units` 1000000; repeat over `unicode-scalars`; repeat over `Text[0, 4; nfc]`.
2. Call the entry with `max_cases` 0, `max_trace_length` 0, an unknown clause identity, and over an empty domain.
3. Over FR-752-AC-7's model, call the entry with a test-double `RootAdmission` that accepts `by` 7, then with `NativeAdmission`.
4. Run FR-752-AC-7's model as a `model_based_test` plan step with `AnalyzeLimits.simulation` `explore.states` 1, then with the published defaults.

## Expected Results

- Step 1: `Ok`, `incomplete`, cause `limit-reached` `work_units`/`accounting.work_units`/1000000, `cases.generated` 0, no panic, not `unsupported`, twice; then `tested` with `cases.generated` 30.
- Step 2: `Err(ModelBasedTestRefusal)` naming each member, no case run; the empty domain `invalid_model_binding`/`malformed-declaration`.
- Step 3: `Ok` settled `failed` `oracle-disagreement`; then `Ok` settled `tested`, the `by` 0 and 7 cases refused by `admit_arguments`.
- Step 4: `jam` `uncovered` `undetermined` naming `explore.states`; `cases` and `result` equal to the default-limit step.
