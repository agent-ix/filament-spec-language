---
id: TC-1581
title: "The analyze outcome document carries verdict, certificate, counterexample, replay and accounting"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-1581: The analyze outcome document carries verdict, certificate, counterexample, replay and accounting

## Description

Verify FR-621's item members and document category. Scope: FR-621-AC-1 to FR-621-AC-7, FR-621-AC-19 to FR-621-AC-22, and FR-126-AC-26's accounting of admitted initial states.

## Test Procedure

1. Analyze `Bounded` over the Counter fixture.
2. Analyze `always holds(c.value <= 2)`.
3. Analyze `Bounded` with `temporal.limits.max_states` 2.
4. Check every category and label of steps 1 to 3 against FSpec FR-352's registry; hand the writer an unregistered kind.
5. Compare the library document with the driver's machine-mode document for step 2.
6. Render step 2 in text mode.
7. Run the temporal clause `Reaches` through an FR-641 clause-run request over the `finite` samples 0, 1, where it is pending at the trace end; analyze a `finite-replay` item naming a temporal clause; and read the value spelling of step 2's counterexample.
8. In `public_analyze_proves_fair_each_service_and_refutes_unfair_service`, verify FR-621-AC-19 through AC-22 by deriving the `native-analyze/1` lock-fairness request from `examples/lock/requests/fairness/one-served.json` as AC-19 specifies; use `examples/lock/data/lock-free.json` and its ordered claims `ServedWithoutFairness`, `ServedIfAcquireIsStronglyFair` and `ServedIfEachAcquireIsStronglyFair`. Assert each output item's literal `request_index` and `claim_name` against its request position; result-only assertions cannot detect swapping the first two refuted items.
9. In the same public analyze request, set the sole fairness claim's `temporal.limits.max_states` to 1. Verify the incomplete result reports one explored state and no counterexample; the initial-state limit must not discard the state already admitted to exploration.

## Expected Results

- Step 1: `success`, `proved`, basis `exhaustive`, `certified`, `certificate.accepted` true, `exploration.states` 4, `exploration.transitions` 3, `aggregate.status` `complete`, exit 0.
- Step 2: `refuted`, counterexample ending at `value = 3`, `replay.reproduced` true, exit 10.
- Step 3: `incomplete` `LimitReached{max_states}`, `exploration.states` 2, no counterexample, `aggregate` incomplete naming index 0, exit 22.
- Step 4: every kind registered; the unregistered kind gives `internal-failure`, exit 30.
- Step 5: byte-equal documents.
- Step 6: first line `0 Bounded refuted`, exit 10.
- Step 7: the clause run reports kind `pending`, category `pending`, basis and `axes.settlement` `unsettled`, exit 0; the `finite-replay` item settles `inapplicable-kind`, family `temporal-trace`, exit 20; `value` spelled `{"type": "integer", "value": "3"}`.
- Step 8 (AC-19, AC-20, AC-21 and AC-22): item 0 has `request_index` `0`, `claim_name` `ServedWithoutFairness`, `refuted`, and a nonempty `counterexample.loop`; item 1 has `request_index` `1`, `claim_name` `ServedIfAcquireIsStronglyFair`, `refuted`, and a nonempty `counterexample.loop`; item 2 has `request_index` `2`, `claim_name` `ServedIfEachAcquireIsStronglyFair`, `proved`, and `certificate.accepted` `true`; the document contains exactly three items and exits 10.
- Step 9 (FR-126-AC-26): the limited item is `incomplete`, reports `exploration.states` `1`, has `record.cause.kind` `limit-reached`, `record.cause.limit` `max_states`, `record.cause.setting` `model_check.max_states` and `record.cause.value` `1`, has no `counterexample`, and the document exits 22.
- Assertions compare the outcome document's typed members and the exit code, never message text.
