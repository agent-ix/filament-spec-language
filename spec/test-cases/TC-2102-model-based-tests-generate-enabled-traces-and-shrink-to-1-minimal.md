---
id: TC-2102
title: "Model-based tests generate enabled traces and shrink to a 1-minimal counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-2102: Model-based tests generate enabled traces and shrink to a 1-minimal counterexample

## Description

Verify generation, the native oracle and the shrinking guarantee. Scope: FR-752-AC-1 to FR-752-AC-3, FR-752-AC-5, FR-752-AC-7 to FR-752-AC-9. Tests live in `fsl-eval/tests/it/model_based_tests.rs`.

## Test Procedure

1. Run model-based testing over `test/counters` with `inc` on `Int[0, 5]` and `value <= 3`.
2. For each seed of a fixed set, record every accepted shrink step and test the retained case for 1-minimality.
3. Run the `verify` step `check t claim Bounded method model_based_test domain M::Counter bound 50;` over a passing clause.
4. Run `Square` (`x: Integer`, body `x * x >= 0`) with seed 7, `max_cases` 20 and `max_value_bits` 8; then the body `x < 200`; then `max_value_bits` 0 and 65, and `max_value_bits` 8 over `value <= 3` with `value: Int[0, 5]`.
5. Over FR-752-AC-7's `inc`/`jam` model, run the postcondition clause `Inc`, the precondition clause of `inc` and the postcondition clause of `jam` with seed 7, `max_cases` 100 and `max_trace_length` 8; then run `jam`'s clause as step `j` of a plan whose step `k` has `depends [j]`.
6. Repeat step 5's `Inc` run with a test-double argument admission that accepts `by` 7, then twice with equal inputs.
7. Run the value clause `Label` (`t: Text[0, 4; nfc]`, body `size(t) <= 4`) with seed 7 and `max_cases` 30, then over `Text[0, 4; unicode-scalars]`, then over `Text[0, 13; unicode-scalars]`; call the generator's `unrank` on FSpec FR-190-AC-18's domains and indices; run `Square` and the `Text[0, 2; unicode-scalars]` clause with seed 7 and read the first seeded value of each (FR-752-AC-9).

## Expected Results

- Step 1: only enabled invocations; boundary cases first; retained counterexample four `inc` from 0, replay-reproduced.
- Step 2: every step strictly smaller, domain-valid, enabled and failing the same clause; no single reduction still fails.
- Step 3: at most 50 cases, the record names the step, `tested` evidence and no verdict.
- Step 4: first cases 0, -1, 1, -255, 255; every `x` in `[-255, 255]`; no refusal partition; one `size_bounded` entry with `values` `"511"`; `tested`, `cases.generated` 20; then the counterexample `x` = 200, replay-reproduced; then `invalid-value` twice and `unknown-member`, each naming `max_value_bits`, no case run.
- Step 5: for `Inc`, cases 0 to 7 the partitions of `value` then `by`; four `domain-refusal` discards with `refused` partitions; the `by` 6 case `precondition-false` with its partition `uncovered`, `bounds` `[]`; `cases.generated` 100 = `passed + failed + discarded`; `tested`. For the precondition clause, no `precondition-false` discard. For `jam`, all 100 cases discarded (`domain-refusal` 4, `precondition-false` 1 at case 6, `invariant-false` 0, `no-check-point` 95), `cases.passed` 0 and `inconclusive` `no-evaluated-case`; in the plan, `k` `blocked` by `j`, exit 22.
- Step 6: `failed` with `tool-failure` `oracle-disagreement`, exit 30, no `tested` record; then byte-equal `cases`, `checks`, `partitions` and `partition_coverage`.
- Step 7: the partition cases in FR-752-AC-9's order, the `upper-refusal` case discarded `domain-refusal`, every seeded value an `nfc` member of length at most 4, `tested`; eight distinct witness cases under `unicode-scalars`; 30 drawn cases for `Text[0, 13]` and never `unsupported`; FSpec FR-190-AC-18's values. `x` = 53 and `t` = U+D51A9 U+87AC9.
