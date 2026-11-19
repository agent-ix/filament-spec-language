---
id: SR-2026111613152603
title: "Criterion strength review of the QSL-2311 settlement member"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; PR #2752 branch qsl-2311-settlement at 1ba65452c98d984b69ac158aad5352877c69b1ec: FR-643-AC-7, FR-644-AC-7, TC-2321, TC-2340"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: reviews
---

## Summary

Ticket: QSL-2311. Jev (typesafe.ai System One) is not installed on this host (`jev not found`), so the Jev-calibrated judgement was skipped and each criterion was judged by hand over the spec text.

FR-643-AC-7 and FR-644-AC-7 are strong. Each names the full `Settlement` value and the report's category and basis. Each states the absence of `finite_window` and of the ledger, which is exactly the leak the ruling removes. Each also has a control: records 0 and 1 carry no `settlement`, and `B(1)` and `B(2)` are `violation` while the clause is `Pending`. An implementation that reports the synthesized open-scope run fails both. TC-2321 and TC-2340 restate the criteria without drift.

What remains is coverage. The new closed record defines members and triggers that no criterion exercises.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No criterion exercises `trigger` `limit`, the `through` member, `position` `null`, or a timed or finite-window `horizon`. FR-644 has no criterion for a summary entry of `{"step": "<n>", "trigger": "finish"}`, because FR-644-AC-7's `E` settles before the end. An implementation that omits `through`, emits `basis` for `limit`, or writes the wrong `n` passes every AC. Fix: extend FR-643-AC-5 (`Recent` `limit`: `{step: "3", trigger: "limit"}`) and FR-643-AC-10 or FR-644-AC-6 (`Ack` via `through` 250: `basis`, `horizon` and `through` "250"). Add to FR-644-AC-5 `Reaches`'s summary `settlement` `{"step": "4", "trigger": "finish"}`. | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:319-324; spec/functional/FR-644-stream-observations-through-the-monitor-command.md:161-163 |

## Scope and evidence

- FR-643-AC-7 (spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md): E over 0, 1, 2 settles at step 3 with report exactly B(3) (success, closed-scope, no finite_window, no ledger) and Settlement {step 3, decided, decisive-witness, horizon 2, position 2}; over 0, 1, 1 decisive-counterexample, position 0; finish over 0, 1 gives {step 2, finish}. Strong. Role: examined.
- FR-644-AC-7 (spec/functional/FR-644-stream-observations-through-the-monitor-command.md): record 2 settled true with disposition equal to the native-run/1 run and the full settlement; records 0 and 1 without settlement; summary with the same settlement and no ledger. Strong. Role: examined.
- TC-2321 Step 2, TC-2340 (spec/test-cases/): match their ACs. Role: examined.
- FR-643-AC-5, FR-643-AC-10, FR-644-AC-5, FR-644-AC-6: read to place the missing coverage. Role: context_only.

## Verdict

PASS WITH ONE MEDIUM.

## Dispositions

Round 1 is checked at head 3e7a52e3952840908642ee5d9f1bf49bc17b27e0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-643-AC-5 now expects Recent's `{step: "3", trigger: "limit"}` and no `basis`, `horizon` or `position`. FR-643-AC-10 expects Ack's `{step: "2", trigger: "decided", basis: "closed-scope", horizon: "250", position: "0", through: "250"}`: two counted calls, basis per FR-266/FR-283, and `"250"` a valid FR-642 string. FR-644-AC-5 expects Reaches' `{"step": "4", "trigger": "finish"}`. TC-1667, TC-2324 and TC-1668 match. `position` `null` and the fixed-sample and finite-window horizons remain unexercised; they are not blocking. |
