---
id: SR-2700004
title: "gap analysis of PR 2725 analyze wire against FR-620 and FR-621"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 2725 head da05353e1420d36513426526843edfe21497949c; FR-620-AC-1..10, FR-621-AC-1..8; fsl-analyze/tests/it, fsl-conformance/tests/it/analyze_row.rs, fsl-replay/tests/it/run_axes.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: references
---

## Summary

Ticket: QSL-2121. This is a planless audit using `quire matrix --scope .`. Every FR-620 and FR-621 AC is tagged. FR-620-AC-8 is tagged only by a test ignored for QSL-2131, which is owned elsewhere. The stubs were un-ignored without their assertions being weakened. Two tagged tests do not verify what their AC states.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The TC-2185 universes case cannot fail. Its subject `{"universes": [...]}` omits the required `initial_states` and `proof_bounds` (`SubjectSelection` has no serde defaults), so the request is refused as malformed JSON before `check_universes` runs. Exit 20 is reached whether or not unprovisioned entries are refused. Fix: write the full subject (`initial_states` [start], `proof_bounds` []) and assert that the diagnostic names `subject.universes[0]`. Also assert that the third default item is deadlock-freedom and that `bogus` is named. | fsl-analyze/tests/it/analyze_request_wire.rs:305 |
| FND-002 | medium | FR-621-AC-8 requires `model` to be the Counter model's identity text, but the test checks only the member names, so `model: null` passes. Fix: assert `identity["model"] == "test/counter"`, and assert `node_id` and `ordinal` values. | fsl-analyze/tests/it/analyze_request_wire.rs:264-271 |
| FND-003 | medium | FR-621-AC-7's first half (a pending `finite-replay` item with `disposition.temporal` `pending` and `axes.settlement` `unsettled`) is untested. An item with `trace` settles `unsupported` (`unrouted`), yet the test carries the full AC-7 tag. Fix: implement the finite-replay route, or split the tag and link an owning ticket for the pending half. | fsl-analyze/src/analyze/document.rs:447 |

## Verdict

FAIL under the gap-analysis rule because FND-001 is high. The other ACs trace to real assertions: FR-620-AC-1 to AC-7 and AC-9, FR-621-AC-1 to AC-6, the conformance wire cells (holds, refuted, incomplete, timeout, cancelled) and run_axes. The `assert_eq!(run.exit, 22)` change in TC-1581 matches FR-621-AC-3. FR-620-AC-8 is deferred to QSL-2131.

## Coverage

18 ACs examined, 18 tagged, 1 tagged only by an ignored test (FR-620-AC-8, QSL-2131). Reverse gap: `Registry::restrict_to_timing` is owned by FR-243. Semantic review was skipped, beyond the targeted checks above.

Plan completion: not assessed
