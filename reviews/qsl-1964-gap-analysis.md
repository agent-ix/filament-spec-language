---
id: SR-2724002
title: "gap analysis of PR 2724 clock selection against FR-640 and FR-326"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 2724 head bef78a9c8cba9c37d5e63de69f94d452128cee20; FR-640-AC-1..9, FR-326-AC-4, FR-326-AC-7; tests/it/clock_selection.rs, tests/it/clock_derivation.rs, tests/it/run_wire.rs, fsl-replay/src/request/package/tests.rs, fsl-semantics tests"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-326
    type: references
---

## Summary

Ticket: QSL-1964. This is a planless audit using `quire matrix --scope .`. All 11 ACs are tagged. Five run_wire.rs stubs stay ignored for QSL-2254 and QSL-2136. Each of them uses the shared fixture, which includes the timed `Ack` clause with a `"kind": "timed"` member. The timed member has no contract variant (QSL-2254), and a timed clause over a trace subject is refused at check (QSL-2136), so the ignores are genuine for these fixtures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-640-AC-8's claim, that rebuilt `clocks` recompile to byte-identical bytes, has no running test. TC-2182 in package/tests.rs covers only the conversion of rows to members, and the end-to-end stub is ignored only because of the timed clause in its fixture. Fix: add a running test over the clock_selection.rs units: compile, rebuild `clocks` from the rows, recompile, and compare the bytes. | fsl-replay/src/request/package/tests.rs:19 |
| FND-002 | medium | FR-640-AC-1 calls for one unit with three clauses under three profiles. The test splits them into three single-profile units, and the module doc says FR-326 forces this, which is stale now that FR-326-AC-7 is implemented. A mixed-profile unit with `clocks` is untested. Fix: use one unit with aliases e/f/w (the run_wire fixture without `Ack`) and update the doc. | tests/it/clock_selection.rs:5-7,133-176 |
| FND-003 | low | In TC-2183, the program clause also uses `plant-ms`, so the missing case does not show that the library clause is the one refused, which FR-640-AC-9 requires. Fix: assert that `details.declaration` is `Late`. | tests/it/clock_derivation.rs:128-135 |

## Verdict

CONDITIONAL: there are only medium and low findings. TC-2180 and TC-2181 un-ignore real assertions.

## Coverage

11 ACs examined and 11 tagged. FR-640-AC-1..6 and AC-8 are also tagged by ignored run_wire stubs. Reverse gap: none in the diff. Semantic review was skipped, beyond the targeted checks above.

Plan completion: not assessed
