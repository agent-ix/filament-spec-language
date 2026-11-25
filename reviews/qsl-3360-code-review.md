---
id: SR-20261125172632
title: "code-review (rust) of PR 3948 undefined state-graph guard, terminal and constraint"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3948 head e2e458067d7d47f92708b2ebfb029090d3d342b0; git diff origin/main...HEAD (fsl-analyze model_check/state_graph{,/explore,/decide}.rs, fsl-eval simulation/model/{constraint,input,terminal}.rs, fsl-replay model_check/{graph_replay,graph_settle,state_graph}.rs, fsl-inspect render.rs, fsl-analyze tests)"
review_set: subset
---

## Summary

Ticket: QSL-3360. Code review with the rust-review lane folded in, focused on verdict soundness. The PR makes the untimed state-graph engine (`check_state_graph`) treat the subject's own guard, `terminal when` predicate and selected state constraint as evaluated at every explored node (FR-168 "Undefined evaluation"), keeps the first such node as `subject_undefined`, and lets `decide.rs` return `Undefined` from it in place of `Witnessed`, `Holds`, `HoldsReduced` or a later trap. A selected constraint now resolves before expansion (`UnknownConstraint`), cuts at false states (new `OpenCause::ConstraintBoundary`, never expanded, never a deadlock), and its reads join the POR visible set. `graph_settle` maps a `ConstraintBoundary` open cause to `ConstraintReached{boundary_states}` after `UndecidedSuccessor` (FR-169 V-6), `graph_replay` resolves a constraint locus to a `constraint` clause (was `invariant`), and `fsl-inspect` spells the new cause.

Specs read: FSL FR-158, FR-167, FR-168 (AC-5, AC-11..15), FR-169 (V-6, certificate rules, settlement method), FR-170 (subject-expression replay, AC-7), FR-852; FSpec (57c662b1) FR-391 ("Well-definedness is global", AC-14), FR-393 (`undefined-evaluation` locus), FR-712, FR-364.

## Verdict

FAIL at this head: two medium findings of the same cause (FR-168's subject expressions not carried through the whole state-graph route), to be fixed in this PR.

Sound in the producer as far as traced:

- Boundary states are open, so `complete()` is false: no `Witnessed`, `Holds` or `HoldsReduced` from a cut run; traps and `unique path` known nodes exclude open nodes and `E+` includes them, so a trap or path pair found under the cut is real in the full graph. A witness to a boundary node is a real path and settles only `WitnessedUnchecked`.
- An undefined constraint node is open `UndecidedSuccessor`, not expanded, labelled, and refutes every item; the deadlock-freedom item takes it as an `UndefinedEvaluation` violation. `ConstraintReached` for the deadlock item comes after `Stopped` and before `BoundReached` and `HoldsReduced`.
- `decide.rs` uses `first_undefined` (subject vs claim predicate, earlier node first) in all three forms and in `settle_with`, so `Undefined` replaces a proof and beats a trap at the same or a later node.
- POR: guard findings come from the full `successors` expansion before ample selection; FR-168 limits the refutation to items "whose exploration reaches that node", so a reduced run that never reaches an undefined guard is within the requirement. Symmetry: an undefined value found at a canonical node is at a reachable concrete state.
- Replay (`graph_replay`): the guard arm needs the same clause, transition and cause first at the stem's last state; the terminal arm calls `terminal_class(state, has_successor)`; the constraint arm refuses a stem through a boundary state and an earlier undefined state. Reading the constraint from the locus rather than the request is what FR-170 states (AC-7), not a defect.
- The changed test `an_undefined_selected_state_constraint_is_not_a_boundary` is not weakened: the old `versions()` read object `b` in a `{a}` universe; the new assertion `[0, 1, 2]` over `a` is the AC-13 stem `(0) -upd(a)-> (1) -upd(a)-> (2)`, and the state-key, predicate, transition and open-cause assertions are unchanged.

Dependents: producer `fsl-analyze` state_graph and checker `fsl-replay` graph_replay are changed together; `graph_settle` and `StateGraphInstanceOutcome.boundary_states` are changed; the certificate checker `fsl-replay/src/certificate/state_graph.rs` is not (FND-001); the analyze route refuses `reductions` on state-graph items (`document.rs:2313`), so the constraint half is library-only; the wire writer `graph_evidence_wire` is unchanged; the digital route (`explore_digital_state_graph`) and timed zone route ignore expansion findings, outside FR-852/FR-712's stated rules; `hyper/single.rs` keeps its own open-cause rule. Tests run: see the QSL-3360 review comment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The independent certificate checker never evaluates a `terminal when` predicate, so it accepts a `Witnessed` or `Holds` certificate over a subject whose terminal predicate is undefined (or refused) at a reachable dead-end state, which FR-169 says an accepted certificate rules out ("well-definedness included") and FSpec FR-391 forbids settling `proved` | fsl-replay/src/certificate/state_graph.rs:350-427 |
| FND-002 | medium | A refused `terminal when` reading (`TerminalClass::Undecided`) is dropped for state-graph items: the node stays closed and unopened, so the item can settle `Witnessed`/`Holds` without the terminal predicate being evaluated; and since FR-124 ranks a refused declaration above an undefined one, a dead-end node with one refused and one undefined declaration returns a proof where FR-168 requires `Undefined` (Unconfirmed item 7) | fsl-analyze/src/model_check/state_graph/explore.rs:420-436 |
