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
`refuted` with the trace as counterexample (a false or undefined required
clause, or an undefined protocol expression, cause `undefined-evaluation`),
`inconclusive`, `unsupported` or `incomplete` for a required clause the
trace does not decide ("Formula claims"), or `declined` `invalid-input` with
a diagnostic naming the defect. A claim with a formula carries the deciding
clause's FSpec FR-352 `disposition.temporal`.

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
- **Bare claim.** When every step replays, no protocol expression is
  undefined at a replayed state, and the clause states no temporal formula,
  the item SHALL settle `proved` over the trace. The trace need not
  end in a terminal state.
- **Claim with a formula.** A protocol clause *states a temporal formula*
  when it carries one or more `requires temporal` requirements (FR-218).
  When every step replays and the clause states a formula, the item SHALL
  evaluate each required temporal clause, in the order of its `requires`
  members, over the replayed finite behaviour, as "Formula claims" below
  states, and settle by the table there.
- **Undefined step.** When a protocol expression evaluates to the kernel's
  undefined value at a replayed state, the item SHALL settle `refuted`,
  cause `undefined-evaluation`, at that state, as "Undefined steps" below
  states, for a bare claim and a claim with a formula alike.

### Decision: a required temporal clause is judged on the replayed behaviour

A protocol clause that requires temporal clauses claims more than
conformance: each run of the protocol must also satisfy each required clause
(FR-218, FSpec FR-464). Over one supplied trace the replayed behaviour is a
finite run prefix, so each required clause is decided on it by the one
temporal evaluator (FR-327), the evaluator replay, monitors and the model
checker share, with positions and counted distance read by FSpec FR-431:
position 0 is the initial protocol state and position `i` the post-state of
step `i`, and only operation, event, send and receive steps count toward an
interval bound. The clause keeps its own profile, so a bounded profile reads
the replayed behaviour as a closed trace (FR-327) and the infinite-trace
profile reads it as a finite prefix (FR-328), where a clause that is neither
violated nor satisfied by the prefix is `pending`. No second evaluator and no
protocol-specific truth table exists. The record carries FSpec FR-352's
`disposition.temporal` (FR-621 "Temporal dispositions"), so a reader sees
the same disposition a monitor gives for the same clause over the same
observations.

### Formula claims

- The item SHALL bind each required clause's `over` parameter to the
  protocol's subject object of the same type, and SHALL evaluate the clause
  over the observations of the replayed positions.
- The item SHALL evaluate the required clauses in `requires` order and
  SHALL report, as its record, the first clause whose disposition is
  `violated` or `undefined`; when none is, the first clause whose disposition
  is not `holds`; when every clause holds, a `proved` record. A protocol run
  fails its claim when any required clause fails on it, so one refuting
  clause decides the item, and a fixed order makes the reported clause the
  same on every run.
- The reported clause's FSpec FR-352 kind SHALL settle the item as follows,
  with `disposition.temporal` that row's kind, members, `category`, `truth`,
  `basis` and `outcome`:

| FR-352 kind | Item `record` |
| --- | --- |
| `holds` (every required clause) | `proved` over the trace, naming the trace identity and the replayed step count, as a bare claim |
| `violated` | `refuted`; the counterexample is the supplied trace as an FSpec FR-433 protocol counterexample of `kind: formula` whose `trace_position` is the first false activation's position |
| `undefined` | `refuted`, cause `undefined-evaluation{where, cause}`, `where` the first undefined position and the atom's locus, `cause` the undefined value's catalogued reason; the counterexample is the supplied trace prefix through that position with FSpec FR-364's `undefined-evaluation` envelope |
| `insufficient-data` | `inconclusive`, cause `insufficient-data{count, min_count}` |
| `pending`, `pending-window`, `pending-next-event` | `inconclusive`, category `pending`, basis `unsettled`, exit 0 |
| `indeterminate` | `inconclusive`, category `indeterminate` |
| `missing-fairness-premise` | `unsupported`, `missing-fairness-premise` (FR-636 "Supplied trace refusals") |
| `incomplete`, `activation-incomplete`, `window-limit-reached`, `progress-limit-reached` | `incomplete`, `resource-incomplete`, cause `limit-reached{limit, value, setting}` naming the limit |
| `progress-contradicted`, `condition-refused`, `trace-refused` | `declined` `invalid-input`, with the refusal's code and cause |

- Every counterexample of a refuted formula item SHALL replay by FSpec
  FR-433 step 4 or step 6, with `reproduced` `true`.

### Undefined steps

- **Which expressions are read.** At each replayed state, position 0
  through the last, the item SHALL evaluate the protocol expressions the
  protocol system evaluates when it expands that state (FSpec FR-426
  "Undefined protocol expressions": constraints, guards, `check`s,
  captures, activation, retry and recovery predicates, `terminal when`),
  as FR-217's replay does, in the system's expansion order. A state a
  supplied trace reaches is a reachable state of the protocol, so an
  expression undefined there is a defect of the protocol, not of the trace.
- **The settlement.** The first such expression that evaluates to the
  kernel's undefined value SHALL settle the item `refuted`, cause
  `undefined-evaluation{where, cause}`, `where` `{position, state_key,
  locus}` with `locus` `protocol` and the expression's source region (FSpec
  FR-331), and `cause` the undefined value's catalogued reason. The
  counterexample SHALL be the supplied trace's steps up to that state,
  carrying FSpec FR-364's `undefined-evaluation` envelope at that
  `trace_position`, and SHALL replay by FSpec FR-433 step 6 to
  `reproduced-with-evaluated-witness`. The undefined value is never read as
  `false` or as a disabled step, so it never settles `declined`.
- **Order.** Replay SHALL stop at the first defect in position order: a
  step that is stale or not enabled settles `declined` `invalid-input` at
  that step and no later state is expanded, and an undefined expression at
  the state before a later stale step settles `refuted` at that state. Steps
  after the undefined state are not replayed, and no required temporal
  clause is evaluated. Wire-shape defects are still reported first, before
  any step replays ("Read").

## Acceptance Criteria

The order fixture is FR-636's.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-941-AC-1 | A bare `finite-replay` item over the supplied trace `Placed`, `Got`, `Ship` settles `proved` naming the trace and 3 replayed steps; over the prefix `Placed`, `Got`, which ends before `Seller`'s choice, it also settles `proved`. | Test (TC-2621) |
| FR-941-AC-2 | The trace of AC-1 with `send_record` removed from step 0 settles `declined` `invalid-input`, `missing-member`, `details.path` `/prefix/0/send_record`; with a `send_record` added to the `receive` step it settles `unknown-member` at that step; a trace whose step 0 is stale and whose step 2 lacks a required member reports the step-2 shape defect, not the stale step. | Test (TC-2621) |
| FR-941-AC-3 | A trace beginning with `Got` settles `declined` `invalid-input`, `invalid-value`, `details.position` 0. | Test (TC-2621) |
| FR-941-AC-4 | The order fixture with `requires temporal R;` and `R` a clause `using t0 over (k: M::Core) clock "model-steps" on origin` under the infinite-trace profile, over the supplied trace `Placed`, `Got`, `Ship`: with `R` `eventually holds(k.r1 = k.r1)` the item settles `proved` with `disposition.temporal` kind `holds` and 3 replayed steps; with `always holds(k.r1 != k.r1)` it settles `refuted`, kind `violated`, `position` 0, and a `kind: formula` counterexample that replays with `reproduced` `true`; with `always holds(k.r1 = k.r1)` it settles `inconclusive`, kind `pending`, category `pending`, basis `unsettled`, exit 0; with `always holds(1 mod (k.r1 - k.r1) = 0)` it settles `refuted`, kind `undefined`, cause `undefined-evaluation` with `cause` `division-by-zero` at position 0; with two requirements `requires temporal P; requires temporal R;`, `P` pending and `R` violated, it reports `R`'s `violated` disposition. | Test (TC-2965) |
| FR-941-AC-5 | The order fixture with the `Got` receive constraint `{ 1 mod (k.r1 - k.r1) = 0 }`, over the supplied trace `Placed`, `Got`, `Ship` recorded from the fixture with constraint `{ true }`: the bare item settles `refuted`, cause `undefined-evaluation`, `where.position` 1, `where.locus` `protocol`, `cause` `division-by-zero`, with a counterexample of the one step `Placed` that replays to `reproduced-with-evaluated-witness`, never `declined`; the same trace with step 2's post-state key changed is still `refuted` at position 1; the same trace beginning with `Got` settles `declined` `invalid-input`, `invalid-value`, `details.position` 0. | Test (TC-2965) |

## Dependencies

- FR-636 owns the `finite-replay` item kind; FR-217 owns protocol replay.
- FSpec FR-433 owns the trace spelling and its wire-shape rule.
