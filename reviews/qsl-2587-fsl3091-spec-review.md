---
id: SR-2026111920504600000002
title: "spec-review of QSL-2587 FR-1015 output region as one top-level definition"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3091 at a5e77f1c; spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md (output region bullet lines 74-84, FR-1015-AC-14), spec/test-cases/TC-3020-export-admission-codes-observer-charges-and-cause-spelling.md (scope, step 6); context: FR-1015 charge, limit-reached and stop-order bullets, FR-1015-AC-11, FR-255 export.work_units row and AC-22, FSpec FR-817 counting and AC-11, FSpec FR-814 Text bullets, FSpec FR-121 and FR-125-AC-3, FSL PR #3084 review FND-004 and FND-008"
review_set: subset
---

## Summary

Spec review of FSL PR #3091. Ticket: QSL-2587. The PR defines an *output region* as one top-level definition the text writes, charges one `export.work_units` unit per definition before writing it, and adds FR-1015-AC-14 with TC-3020 step 6 (C + N assembles; C + N - 1 is `incomplete`, exit 22). `quire validate` passes on both files. FR-1015-AC-14 is not on origin/main and no open FSL PR (including #3084) uses it. TC-3020 is changed only here.

## Examined scope

- Charge arithmetic. Classification charges one `export.nodes` and one `export.work_units` unit per distinct closure node, so C (the least `export.nodes` that is not `incomplete`) is also the classification's work charge. The floor charges nothing. The text then charges N. So C + N assembles and C + N - 1 is denied at the Nth (last-written) definition. This matches the stop order bullet, FSpec FR-817's order and the ruling. (examined)
- Outcome shape. The `limit-reached{work_units, C + N - 1, export.work_units}` cause, `records` `[]` and exit 22 match FR-1015's denied-charge bullet, FR-1015-AC-11, FR-255-AC-22 and FSpec FR-817-AC-11. (examined)
- FND-008 of the #3084 review. Charging only closure nodes is already FR-1015's classification rule, and this PR does not change it. (context_only)
- Which definitions count. The enumeration ("each datatype, variable declaration, function or operator ...") is said to equal "each definition FSpec FR-814 precedes with one `fsl:` comment". These two sets differ for definitions with no node, which FR-814's comment cannot name (FND-001). (examined)
- The term "output region". FSpec already uses it for a record's claimed output span (FND-002). (examined)
- The closing sentence of AC-14 (FND-003). (examined)
- Ids and merge order. No collision. The PR depends only on FR-814's `fsl:` comment rule, which is already on FSpec main, so it does not wait on FSpec #757. FSpec #757 merging first is still fine. (examined)

## Verdict

Not mergeable yet. There are no high findings, but two medium findings change what N counts and what the term means, and the #3084 coder implements against both. They should be fixed in this PR. The charge arithmetic, the exit code and the cause shape are consistent with FR-1015, FR-255 and FSpec FR-817.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The enumeration of output regions is equated with "each definition FSpec FR-814 precedes with one `fsl:` comment", but the two sets differ. Generated helpers (`Q_*`, `F_*`, `D_*`), the structural definitions (`State`, `WF`, `Init`, `Step`, `Next`, `Spec`, `TypeOK`, `vars`) and a TLA+ `VARIABLES` statement declaring several variables are datatypes, operators or variable declarations, but they have no node for an `<node id>` comment. The #3084 review (FND-004) found `VARIABLES` and the Lean helpers without a comment. Whether a helper costs a unit is open, and AC-14, which counts N by comment lines, can disagree with the bullet. State which one governs, and how `VARIABLES` counts | spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md:74-84 |
| FND-002 | medium | "Output region" already has a meaning in FSpec: a record's claimed byte span (FR-121 "claim no output region", FR-125-AC-3 "All output regions are in bounds and belong to their record", FR-812/813 "state, action and value regions"). FR-1015 says it charges "FSpec FR-817's counters as that requirement counts them", yet it gives FR-817's "per output region written" a new, narrower meaning in FSL alone, and FSpec #757 does not amend FR-817. Either use a distinct term ("written definition") or move the definition into FSpec FR-817 | spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md:74-84 |
| FND-003 | low | AC-14's last sentence claims too much. Every `fsl:` comment names exactly one node, so an implementation that charges "per node mapped while writing" charges N and cannot fail the AC. "The denied charge is the one for the Nth definition" is not observable, because the outcome carries `records` `[]`, no text and no definition in the cause. Only the bound C + N - 1 is checked | spec/functional/FR-1015-request-an-export-and-write-the-export-outcome.md:201 |
