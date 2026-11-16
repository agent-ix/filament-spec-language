---
id: FR-941
title: "Decide a finite-replay item over a supplied protocol trace"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-433
    type: depends_on
---
# FR-941: Decide a finite-replay item over a supplied protocol trace

## Description

A `finite-replay` item (FR-636) SHALL decide its protocol clause over the one
finite trace the caller supplies, in two phases: it first reads the whole trace
document against FSpec FR-433's trace spelling, and only then replays it step
by step through the protocol system (FR-217). A **bare protocol claim**, a
clause that names the protocol and states no temporal formula, asserts that
the trace is a behaviour of the protocol, so it is `proved` exactly when every
step replays.

### Decision: a bare claim is proved when every step replays

A bare protocol claim's content is conformance: each step is a protocol
transition enabled at its pre-state, with the recorded post-state and send
record. A finite trace is a recorded prefix of a run, so it need not end in a
terminal state, and nothing beyond its last step is claimed. When every step
replays, every obligation of the claim that a finite prefix can settle holds,
so the item is `proved`. The proof is about that trace only: the record names
the trace and the number of steps replayed, and it is never read as a proof
over every behaviour, which is `global-conformance`'s item (FR-896).

### Decision: the wire shape is read before replay

FSpec FR-433 makes a `send` step without its `send_record`, and any other step
with one, a wire-shape defect that refuses before replay, like any missing or
extra member. A supplied trace is read by the same reader, so the same rule
applies to it: the defect is found when the document is read, before step 0
replays, and not when replay reaches that step. Reading first means the
answer never depends on how far replay got: a trace that is both malformed at
step 3 and stale at step 0 reports its shape defect, the same answer the
counterexample reader gives for the same document. In analyze the defect
settles the item, since the trace is item input, so it is `declined`
`invalid-input` and not a whole-request refusal.

## Inputs

- A `finite-replay` item naming a protocol clause, with its `trace`
  `{authority, identity}` (FR-636).
- The analyzed package, its subject, initial states and bindings.

## Outputs

The item's record: `proved` with the trace identity and replayed step count,
`refuted` with the trace as counterexample, or `declined` `invalid-input` with a
diagnostic naming the defect.

## Behavior

- **Read.** The item SHALL read the whole supplied document against FSpec
  FR-433's trace spelling before any step replays. A missing, extra or
  mistyped member, a `send` step without a `send_record` and a non-`send` step
  with one SHALL each settle the item `declined` `invalid-input`, diagnostic
  `invalid_runtime_input`/`missing-member`, `unknown-member` or
  `wrong-value-kind`, with `details.position` the zero-based step index when
  the defect is inside a step and `details.path` the JSON pointer of the
  defect inside the trace document (such as `/prefix/0/send_record`). The
  first defect in document order is reported.
- **Replay.** A trace that reads SHALL then be replayed step by step as FR-217
  replays a counterexample. A step that is stale or not enabled at its
  pre-state, or whose recorded post-state or send record differs from the
  replayed one, SHALL settle the item `declined` `invalid-input`, diagnostic
  `invalid_runtime_input`/`invalid-value`, `details.position` that step.
- **Bare claim.** When every step replays and the clause states no temporal
  formula, the item SHALL settle `proved` over the trace. The trace need not
  end in a terminal state.
- **Claim with a formula.** When every step replays and the clause states a
  temporal formula, the item SHALL evaluate the formula over the replayed
  finite behaviour with FSpec FR-431's step counting: `proved` when it holds,
  `refuted` with the trace as counterexample when it fails, and FR-636's
  existing settlements otherwise.
- **Undefined step.** A protocol expression undefined at a replayed step
  SHALL settle the item `refuted`, cause `undefined-evaluation`, at that step
  (FSpec FR-433 step 6).

## Acceptance Criteria

The order fixture is FR-636's.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-941-AC-1 | A bare `finite-replay` item over the supplied trace `Placed`, `Got`, `Ship` settles `proved` naming the trace and 3 replayed steps; over the prefix `Placed`, `Got`, which ends before `Seller`'s choice, it also settles `proved`. | Test (TC-2621) |
| FR-941-AC-2 | The trace of AC-1 with `send_record` removed from step 0 settles `declined` `invalid-input`, `missing-member`, `details.path` `/prefix/0/send_record`; with a `send_record` added to the `receive` step it settles `unknown-member` at that step; a trace whose step 0 is stale and whose step 2 lacks a required member reports the step-2 shape defect, not the stale step. | Test (TC-2621) |
| FR-941-AC-3 | A trace beginning with `Got` settles `declined` `invalid-input`, `invalid-value`, `details.position` 0. | Test (TC-2621) |

## Dependencies

- FR-636 owns the `finite-replay` item kind; FR-217 owns protocol replay.
- FSpec FR-433 owns the trace spelling and its wire-shape rule.
