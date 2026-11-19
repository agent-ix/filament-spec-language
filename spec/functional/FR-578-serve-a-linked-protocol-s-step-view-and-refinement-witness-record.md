---
id: FR-578
title: "Serve a linked protocol's step view and refinement witness record"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-148
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-574
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-147
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
  - target: ix://agent-ix/filament-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-352
    type: references
---
# FR-578: Serve a linked protocol's step view and refinement witness record

## Description

FSL SHALL serve, for a protocol clause of a linked package, one read-only
view of its reachable steps under the one `fsl-eval` protocol transition
system, and one FSpec FR-351 record type for a decisive protocol refinement
counterexample. A protocol-level refinement or composition decided outside
FSL (filament-protocol FR-301, FR-302) reads its systems and writes its
witness through them. FSL decides no protocol refinement or composition
here; FR-147 and FR-530 are unchanged.

## Inputs

- The linked `CheckedPackageV2` and the `protocol_clause` node id.
- The protocol subject: the in-process checked package with the request's
  bindings, as realizability takes it (FR-571).
- The caller's state limit.
- For the witness: the claim's assessed root (FR-574), the deciding
  element's checked node id and the zero-based trace position.

## Outputs

`fsl_analyze::protocol_analysis::protocol_step_view` returns `fsl_replay::ProtocolStepView::Steps`, holding:

- the number of states, with each state named by its discovery index;
- the initial states;
- the final states: states with no step that a terminal declaration
  intends (FR-124 `Intended`); any other state with no step is a deadlock;
- every step, each with its source and target states, its `StepKind`, the
  checked node of its control where it has one, the checked node of its
  channel for a `send` or `receive`, its binder record in FR-181 typed
  canonical bytes, and the exact `fsl-eval` transition;
- the assumptions: FR-148's assumption set, the derived scheduler and
  memory fairness and the operation fairness of each directly required
  temporal clause.

Or it returns `ProtocolStepView::Unsupported`, listing the constructs that
keep the view from being exact, with no steps.

`fsl_replay::RefinementWitness` is the FR-351 record of the protocol family:
the deciding node's canonical text, the assessed root's value path with no
steps, and the trace position, with no quantifier and no index. It is the
same record ADR-036 PA-24 writes on `native-run-result/2`, written by the
same writer, with its RFC 8785 bytes and a strict reader.

## Behavior

- The view shall take every step from the `fsl-eval` protocol system's
  successors. It states no step rule, and it decides nothing about hiding,
  refinement or composition.
- The view shall list the constructs `fsl-eval` steps only up to a bound
  or not at all, in this order: an `activation on each` clause, each
  replicated role in view order, then a construct the protocol system
  refuses with `unsupported_construct`. When any is listed, the view returns
  no steps.
- The view shall refuse, rather than truncate, a protocol whose
  exploration reaches more than the state limit, or whose state is cut by
  the memory or live-instance bound. It shall also refuse a protocol whose
  reached state has any undetermined finding (an undetermined invariant,
  contract or protocol expression, or a weak-access shape) or an
  undetermined terminal class, whatever other findings the state has.
- The view shall refuse a step whose control or channel the linked package
  does not hold under exactly one name.
- The witness reader shall refuse a record outside the protocol family's
  closed shape, and bytes that are not already canonical, rather than
  repair them (FSpec FR-351-AC-5).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-578-AC-1 | For a protocol of sends, receives and a choice, every step of the view carries its kind; a checked node exactly when its transition names a control (the `finish` member for a `finish` step); a channel node exactly for a `send` or `receive`; and its own binder record, absent for a step with no record binder such as a `parallel` fork. The steps out of an initial state equal the `fsl-eval` system's successors of that state, in order, and two views of one subject are equal. | Test (TC-1422) |
| FR-578-AC-2 | The view's final states are non-empty for a protocol that finishes, no step leaves a final state, and the view's assumptions equal the protocol system's FR-148 assumption set. | Test (TC-1422) |
| FR-578-AC-3 | A protocol activated `on each`, a protocol with a replicated role, and a protocol whose step binder `fsl-eval` cannot enumerate each return `Unsupported` naming the clause, the role, or the `fsl-eval` construct, with no steps. | Test (TC-1422) |
| FR-578-AC-4 | A view whose state limit is one below the protocol's reachable state count is refused `StateLimit` naming the limit, and the same protocol at its reachable state count is a view. | Test (TC-1422) |
| FR-578-AC-5 | A refinement witness built for a node and trace position is the same record, on the wire, as the realizability FR-351 record for that node and position. Its bytes read back to an equal witness, carry the node's canonical text and the decimal position, and a different position is a different record. | Test (TC-1423) |
| FR-578-AC-6 | The witness reader refuses non-canonical spacing, a leading-zero trace position, an added `index` member, an absent `trace_position`, and a deciding element that is not a node id's canonical text. | Test (TC-1423) |
| FR-578-AC-7 | A protocol whose reached state has an undetermined finding is refused `Undetermined` naming the state; a protocol under `tso` whose store buffer the default memory bound cuts is refused `Cut`; and a protocol with two controls of one name is refused `UnnamedNode` naming the step's path. | Test (TC-1422) |

## Dependencies

- ADR-027 TS-1: the one protocol transition system.
- [FR-148](FR-148-decide-steps-against-an-abstract-protocol.md): the protocol system wrapper and its assumption set.
- [FR-574](FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md) and ADR-036 PA-24: the protocol family's FR-351 record.
- FSpec FR-351, FR-352.

## References

- filament-protocol FR-301 and FR-302 are the consumers.
- QSL-1961.
