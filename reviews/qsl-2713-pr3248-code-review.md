---
id: SR-20261121031333
title: "code review of PR3248 protocol-subject liveness under a state constraint"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/items.rs, fsl-analyze/src/model_check/check.rs, fsl-analyze/src/model_check/liveness.rs, fsl-analyze/tests/it/tso_fairness.rs; PR3248 head 20f2f57197f570180310eb8fd000d22bf2efda29"
review_set: subset
---

## Summary

Ticket: QSL-2713 (remainder: a state constraint on a liveness claim over a protocol subject). Code-review with the rust-review lane folded in. Diff only, read at the PR head with `git show`, no build. The constraint routing is sound. The channel-bound deviation is what FR-225 requires, but turning the bound on in the liveness engine exposes two defects that were unreachable while the bound was always `None`. First, a send cut by the bound is left out of the state's `Enabled` set, so the fairness filter can accept a lasso that starves the cut sender. That is a wrong `refuted`, against ADR-025 MF-4. Second, the model-only pass never records a send that the bound cut, so a run can settle `holds` even though part of the graph was never explored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A send cut by the channel bound is missing from `Finding::Enabled`, so a lasso that starves the cut sender passes the fairness filter and settles `Violated`, against ADR-025 MF-4 / FR-225 ("a cut step ... exactly as a store FSpec FR-439 cuts") | fsl-analyze/src/model_check/liveness.rs:1218-1234, 1748-1752 |
| FND-002 | high | The model-only (dead-root) pass records `cut.memory` but ignores `cut.messages`, so a send cut there is never counted and the run can settle `Holds` over an unexplored region instead of `ChannelBoundReached` | fsl-analyze/src/model_check/liveness.rs:2244-2248, 1732-1737 |
| FND-003 | low | `constraint_reached_settles_before_a_channel_bound` is tagged FR-158-AC-5, but AC-5 is the `SB` sc/tso/ra constraint case. Nothing runs a constrained protocol liveness claim under `tso` or `ra` | fsl-analyze/tests/it/tso_fairness.rs:1259 |
| FND-004 | low | `Bound::constraint`'s doc still reads "`None` for a protocol subject", which is stale now that protocol liveness passes the constraint | fsl-analyze/src/model_check/liveness.rs:425 |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed. Both are small:

- FND-001: when the bound cuts a send, list that send's transition identity in the state's `Enabled` finding. The fairness filter then reads a cut send as enabled, as MF-4 requires.
- FND-002: insert the state's key into a dead-phase message-limited set when `cut.messages` is set, and union that set into `message_limited`.
- FND-003: retag the test to the FR-225 constraint-before-bound rule, and add one constrained `tso` liveness case (a buffered store is not a boundary state).
- FND-004: is a one-line doc edit.

Checked clean:

- **Unknown constraint.** `UnknownConstraint` refuses in `check_protocol_liveness` before any `ProtocolSystem` is built or expanded. The same `has_constraint` check is used on the safety path.
- **Boundary states.** A boundary product state returns no successors, no `Enabled` and no stutter edge (liveness.rs:1189-1208). So no lasso passes through or ends at one, and no stem continues past one.
- **Undefined constraint.** It refutes with `UndefinedEvaluation` at locus `Constraint(name)` in both the product and the model-only pass, and the state is not counted as a boundary state.
- **Outcome ordering.** A run with no violation and at least one stored boundary state settles `ConstraintReached` before the memory, channel and instance bound outcomes (liveness.rs:1748-1790). So a claim cut at a boundary never settles `Holds`.
- **tso/ra reading.** The constraint is read as `self.model().constraint_value(state.model(), name)`. Every memory model's `atom_values` returns the model state unchanged, so the constraint reads exactly what a `holds` atom reads. That is memory values, with buffered stores and non-mo-last messages not read, as FR-158 "Protocol subjects" states. It is the same read the merged safety path uses (check.rs:1735), so a buffered write cannot make a state look constraint-true or constraint-false here in a way it would not for safety.

Channel-bound deviation. Spec lines:

- FR-225 "Decision": "`max_messages` SHALL also bound the occupancy of each protocol channel, counted per channel, of every channel kind, on every subject whose model holds a channel, RA or not."
- FR-225 "Inputs": "`None` means 4".
- FR-126 says `max_messages` is a method bound for which "FR-225 specifies their meaning, their published defaults and the `MemoryBoundReached` outcome they settle". FR-126 does not exempt liveness.

So applying the bound to protocol liveness conforms to the spec, and leaving it unset was the defect. The default is `DEFAULT_MEMORY_BOUND` = 4 (fsl-eval tso.rs:52), the value the safety arms use (check.rs:845-871). Under `ra` it equals `ra.max_messages()`.

Before this change, a channel queue was still finite at its declared capacity, so liveness did not diverge. It explored the full graph, subject to `max_states`. After the change, a run with no counterexample whose product graph holds a message-limited state settles `ChannelBoundReached`, never `Holds` (liveness.rs:1775-1782). A previously sound `holds` becoming `ChannelBoundReached` is acceptable. A refutation inside the bound is still reported, because the lasso check runs first. Two cases break this: FND-001, where the reported lasso may not be a real fair behaviour, and FND-002, where the model-only pass's cuts are not counted.

Out of scope (pre-existing, same mechanism): `tso` stores cut by `max_store_buffer` are also left out of `Enabled`. The module doc's claim that such a run settles `MemoryBoundReached` "in place of ... a counterexample" does not match `decide`, which returns the lasso first.

Tests: TC-580/TC-581 cases 1-5 assert the right verdicts and loci. Case 1 is `Holds` without the constraint and `ConstraintReached{1}` with it. Case 2 is `ConstraintReached{1}` with `product_states == 1`. Case 3 is `Violated` before the boundary. Case 4 is `UndefinedEvaluation` at `Constraint("Odd")`. Case 5 is `UnknownConstraint("Missing")`. All run under `sc`.

## Scope and evidence

Read the PR diff and, at the PR head, liveness.rs (`successors`, `decide`, the model-only pass, `LiveSubject for ProtocolSystem`), check.rs (`check_protocol_constrained`, `check_protocol_liveness`, the safety protocol arms, `Reducer for ProtocolSystem`), fsl-eval protocol/channel.rs `send` (the channel cut), steps.rs `expand_bounded`, every `atom_values`, fair_walk.rs `FairModel`, FR-158, FR-225, FR-126 and ADR-025 MF-4/MB-4. No cargo. Gap-analysis not run (one proportionate pass per the brief).

- FR-158 Protocol subjects (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-158-AC-2 (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-158-AC-3 (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-158-AC-5 (spec/functional/FR-158-cut-the-search-with-a-state-constraint.md): examined
- FR-225 Decision (spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md): examined
- FR-126 method bounds (spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md): context_only
