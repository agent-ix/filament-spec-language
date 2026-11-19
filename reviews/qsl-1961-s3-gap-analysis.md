---
id: SR-2026111414542500000008
title: "Gap analysis of QSL-1961 S3 protocol step view and refinement witness (PR #2721)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-1961; PR #2721 head 32f4ab2fb468a3675dab618845fe4dba9b72d36e; FR-578 (AC-1 to AC-6, Behavior); TC-1422; TC-1423; fsl-analyze/tests/it/protocol_step_view.rs; fsl-replay/src/refinement/{step_view.rs,protocol_witness.rs}"
review_set: subset
---
# Gap analysis: QSL-1961 S3

## Summary

Ticket: QSL-1961. Planless, scoped to the PR diff. Plan completion: not
assessed. `quoin matrix --json` (quoin 0.29.4) on the PR head shows
FR-578-AC-1 to AC-6 each `tagged` by one test in
`fsl-analyze/tests/it/protocol_step_view.rs` (no run evidence, as the gate has
not run).

- AC-2 to AC-6: each test asserts what its criterion states.
- Code without an owning requirement: none; `protocol_step_view`,
  `RefinementWitness` and the two new `AbstractProtocolSystem` methods are
  owned by FR-578.
- No stubs or coverage inflation found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-578-AC-1 states every step carries its control's node and, exactly for a `send` or `receive`, its channel's node and record; the test checks channel exactness on every step but checks `record.is_some()` only on four chosen steps and never checks that a non-communicating step has no record or that every step with a control has a node, so a view that attached records to other steps or dropped a node would pass | fsl-analyze/tests/it/protocol_step_view.rs:46-71 |
| FND-002 | medium | FR-578 Behavior requires the view to refuse a state cut by the memory or live-instance bound, a reached state with an undetermined finding or terminal class, and a step naming no unique node, but no AC states these refusals and no test exercises `StepViewRefusal::Cut`, `Undetermined` or `UnnamedNode`; the code-review FND-001 defect in the undetermined check is the kind this leaves undetected | spec/functional/FR-578-serve-a-linked-protocol-s-step-view-and-refinement-witness-record.md:81-86; fsl-replay/src/refinement/step_view.rs:261-272 |

## Verdict

Every new AC is tagged and five of six are fully backed. AC-1's record
exactness is only partly asserted, and the "never truncates" refusals other
than the state limit have neither an AC nor a test.
