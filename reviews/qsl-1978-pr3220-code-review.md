---
id: SR-2026112021460200000000002
title: "code-review of PR 3220 certified plan search engine (QSL-1978)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3220 head 10d523df99d408e9fbd2b783c100942ab3d04ace; fsl-analyze/src/plan_search.rs, fsl-analyze/src/plan_search/certificate.rs, fsl-analyze/src/plan_search/replay.rs, fsl-analyze/tests/it/plan_search.rs, fsl-command/src/command/plan_request.rs"
review_set: subset
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3220. This is a diff-only code review with the rust-review lane folded in. Nothing was built or run. Quoin gap-analysis was not run. The engine cannot yet be reached from source, so its ACs were checked by hand against FSpec FR-650, FR-800 to FR-806 and FR-755/FR-1011/FR-1012, read-only from filament-specification.

### What was checked and holds

- `potential`, by soundness argument. `descent` carries a `None` (no goal reachable) value backward. `goal` is checked in both directions. `initial` checks `pi(initial) = plan value`. With `closure` this makes `pi(initial)` a lower bound on every admissible plan reachable within `max D`, so a non-optimal claim is rejected.
- `closure`. It checks that every re-derived, policy-admitted step within `max D` has a record naming the right node. The checks are state key, monitors, depth and the accumulated value when one is kept, plus the parent when prefixes are not merged. So an `infeasible` claim covers every reachable node.
- `goal-free`.
- The `max_check_work` meter.
- The FR-801 layered frontier and the order within a layer.
- Dominance keeping the earlier prefix on equal values.
- `best()`'s FR-804 order.
- Native replay of the steps, policies and goal.
- The plan request reader's closed member list and pointers.
- The `filament.plan-outcome/v1` writer.

### Coder's spec point (a), the FR-650 potential `goal` rule

The FSpec text is "pi(n) <= 0 only at a node whose length is in D and at which the goal holds". It is defective in both directions.

- As written it states only "pi <= 0 implies goal". That is not enough for soundness. Without "goal implies pi <= 0", a certificate can set a large `pi` at a cheap goal node and certify a more expensive plan.
- The "only" direction cannot be met whenever a zero-cost step enters a goal node from a non-goal node. `descent` forces `pi(n) <= 0 + pi(g) <= 0` at that non-goal `n`. With an objective of 0 from a non-goal initial node, it also contradicts `initial`.

The code checks both directions (`at_most_zero != goal`, certificate.rs:397-399). That is sound, but it turns such optimal plans into `feasible`. The spec should be corrected to "pi(n) <= 0 at every node whose length is in D and at which the goal holds", with the converse dropped. The code should then drop the converse and keep the goal-implies direction. This is not filed as a PR finding.

### Coder's spec point (b), the FR-801 charge for a disabled step

FR-801 "Charge order" step 4 is unambiguous: one `max_work` for applying each step, and "a disabled step stops here and yields no successor". So a disabled step is charged. `ModelSystem::successors` returns only enabled steps, so the implementation never charges one. That is an implementation defect, not a spec question (FND-007).

## Verdict

NOT MERGEABLE. Three highs (FND-001 to FND-003) and four mediums (FND-004 to FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Search, checker and replay ignore `Expansion::findings`. A violated model invariant is a finding, not a pruned successor, so a plan through or ending at a state that breaks an invariant is reported as admissible (optimal or feasible). FR-650 requires every state clause and invariant to hold at every state of the plan. | fsl-analyze/src/plan_search.rs:734-749; fsl-analyze/src/plan_search.rs:751-845; fsl-analyze/src/plan_search/certificate.rs:300-310; fsl-analyze/src/plan_search/replay.rs:49-67 |
| FND-002 | high | A search stopped by cancellation, deadline or an evaluation-meter denial replays its best plan under the same tripped cancel or exhausted meter. `check_rules` maps every `Err(RuleStop)` to `ReplayRejected`, so the run becomes a method failure (exit 30) instead of FR-801's `incomplete` carrying the best plan. | fsl-analyze/src/plan_search.rs:946-952; fsl-analyze/src/plan_search.rs:1047; fsl-analyze/src/plan_search/replay.rs:118-146 |
| FND-003 | high | The checker treats a node where the goal holds but the objective is undefined as a goal node. The search excludes such plans, as FR-802 requires. When every goal plan has an undefined objective, the closure is rejected `goal-free` and the result is `incomplete`/`certificate-rejected` instead of `infeasible`. | fsl-analyze/src/plan_search/certificate.rs:390-404; fsl-analyze/src/plan_search.rs:909-925 |
| FND-004 | medium | For a `closure` certificate over a per-step sum (`maximize` a step sum, or after a negative step), `improvement` compares the certificate-supplied `entry.value` instead of re-evaluating it. Under cost dominance the key keeps no accumulated value, so nothing checks that value. | fsl-analyze/src/plan_search/certificate.rs:414 |
| FND-005 | medium | The checker re-derives from the certificate's own `state` and `parent` fields. It never checks that an entry's `state` encodes to its `key.state`, and with merged prefixes never checks `parent`. A forged entry state or prefix is used as given, and a parent cycle loops forever in `prefix`. | fsl-analyze/src/plan_search/certificate.rs:287-293; certificate.rs:357-371; certificate.rs:459-466 |
| FND-006 | medium | Replay does not recompute the objective or check that the plan length is in `D`, and `settle` reports `nodes[best].goal` as the objective of whatever plan replay returned. With the plan substitution seam, an admissible but worse plan (`inc, inc, inc, double` under `length`) is reported `optimal`, certified, objective 3, with 4 steps. | fsl-analyze/src/plan_search/replay.rs:28-69; fsl-analyze/src/plan_search.rs:1047-1048 |
| FND-007 | medium | The FR-801 charge order differs from the code. Disabled steps are never charged `max_work`, because only enabled successors are enumerated. The initial node's policy-monitor reads are charged before its `max_states` admission, where step 4 lists admission first. `explored` counts and stop points therefore differ from the canonical exploration on any model with disabled invocations. | fsl-analyze/src/plan_search.rs:668-684; fsl-analyze/src/plan_search.rs:751-757 |

### FND-001

`walk::expand` evaluates the state's invariants first (`contracts::invariants`, fsl-eval/src/simulation/model/walk.rs:434). A false invariant becomes `ModelFinding::InvariantViolated` and an undecided one `InvariantUndetermined`, and the successors are still returned. The plan search reads only `expansion.successors`. Leaves at `max D` are never expanded, so their invariants are never evaluated. FR-650 "admissible": "every state clause and invariant of the model holds at every state of the plan". Fix: evaluate every admitted state's invariants (its expansion's findings, or `state_findings` for a leaf) and treat a state with a violated invariant as inadmissible. Settle undetermined invariants as FR-650/FR-801 say. Apply the same rule in the search, in `certificate::check` and in `replay`, and add a counter-fixture test with an invariant.

### FND-002

`PlanRules` evaluations return `RuleStop::Cancelled` and `RuleStop::Accounting` (plan_search.rs:237-249), and FR-801 builds one meter per request. After the search stops on one of them, `stopped` calls `replayed(best)`. Replay re-runs `policy_start`, `policy_advance`, `policy_accepts` and `goal`, which deny again. `Err(_)` then becomes `ReplayRejected`, and `search` returns `Err(PlanSearchError)`. The same applies to the replay in `settle` when the meter runs out exactly there. Fix: have `check_rules` return the `RuleStop` separately from `Ok(false)`/`Reject`, and settle the run as `incomplete` with that cause. Report no plan if it cannot be replayed. Never report a method failure. Add a test with goal 1 or 2 and a small `work_units`, and one that is cancelled after a plan is found.

### FND-003

`goal_holds` (certificate.rs:434-456) checks the goal and the non-safety policies only. `evaluate_goal_value` sets `goal = None` and counts `undefined_objectives` for an undefined objective. FR-802: "A plan whose objective evaluation is undefined SHALL NOT be admissible". Fix: in `check`, re-evaluate the objective at a goal node of a `General` objective and treat an undefined value as not a goal node, both for `goal-free` and for `potential`'s `goal`.

### FND-004

FR-650: the core checks every certificate "by exact evaluation of the model's operations, policy, goal and objective". Fix: carry the representative prefix's step-cost sum through the checker's own re-derivation (it already evaluates `step_cost` per record), or keep `accumulated` in the key for closure certificates, and compare that value.

### FND-005

For entry 0 only `key.state` is compared with the initial key, and `first.state` is never compared. For successors, `successor.key.state` is compared with the derived key, but `successor.state` (which the next expansion and `prefix` read) is not. Fix: check `system.state_key(&entry.state) == entry.key.state` for every entry. Check that each non-initial entry's `parent` names an entry that has a record for this step to this entry. Bound the `prefix` walk by `key.depth`.

### FND-006

FR-650 test seams exist "so that the replay and certificate checks can be shown to reject a bad plan". FR-650 `optimal`: "the plan, its objective value". Fix: in `replay`, evaluate the plan's objective (the step-cost sum or `objective`) and require its length to be in `D`. Reject a plan whose value differs from the certified value, and report the replayed value as `objective`.

### FND-007

Fix: enumerate candidate invocations in FR-804 step order, enabled or not, and charge one `max_work` per candidate before testing whether it is enabled. This needs an enumeration from fsl-eval: the `expand_enabled` candidate list or a candidate count per operation. For the initial node, admit (`max_states`) before the monitor reads, or get an FR-801 ruling on whether initial monitor reads are charged. Then pin counts on a model with a disabled step (the counter's `double` at a value above 10).

## Dispositions

Round 1, head 19750b3aec7252092ae209e22ce3b8957e06760e.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Invariants are now evaluated through `ModelSystem::invariant_findings_at` at the start state, at every admitted successor (including states at the maximum length) and at every replayed state (plan_search.rs:780-785, certificate.rs:357, replay.rs:78-86). New tests cover a mid-plan invariant, a maximum-length invariant and replay. An undecided invariant is handled as a prune, which is FND-008. |
| FND-002 | fixed | Replay now reports rule stops as `RuleStopped` rather than `ReplayRejected`. Both `stopped` and `settle` settle `incomplete` and never a method failure (plan_search.rs:1038, settle). `begin_replay` gives replay its own allowance, and a test cancels after the first plan is found. A plan that cannot be replayed is dropped, not reported unreplayed. |
| FND-003 | fixed | The checker's closure rule re-evaluates the objective, and an undefined value is not a goal node (certificate.rs:460-466). |
| FND-004 | fixed | The closure checker computes the step-cost sum along the verified representative prefix itself (certificate.rs `objective_value`). |
| FND-005 | fixed | The checker now verifies that each entry's state encodes to its key, that the parent is an earlier entry one step shallower that records the step (certificate.rs:535-546), and that the record count equals the number of re-derived admissible steps (certificate.rs:439). The prefix walk is bounded by depth (certificate.rs:577). The new `ForgeEntryState` and `ParentCycle` seams are library-only: `PlanSearchRequest.fault` is not read by plan_request.rs, and the request member list stays closed. |
| FND-006 | fixed | Replay recomputes the objective and checks the length is in D. A value that differs from the node's certified value is `ReplayRejected` (replay.rs `objective_value`, plan_search.rs:1081). |
| FND-007 | fixed | `candidates` merges `transition_identities` with the enabled successors in FR-804 order and charges one `max_work` per candidate, disabled ones included (plan_search.rs:789-822, 836). The initial node's `max_states` is now charged before its monitors are read (plan_search.rs:705-713). |

The checker now enforces only goal => pi <= 0 (certificate.rs:454). That is the sound direction, and it no longer rejects valid potentials that involve zero-cost steps. FSpec FR-650's text still needs the matching correction.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | An invariant the model cannot decide at a state counts as not holding. `invariant_findings_at` returns `InvariantUndetermined` as a finding, and `findings.is_empty()` / `invariants_hold` treat it like a violation. The state is pruned silently, consistently in the search and the checker, so the closure or potential certificate is accepted and the result can be `infeasible` or `optimal` when a plan through that state may exist and may be better. Fix: tell `InvariantUndetermined` apart from `InvariantViolated`, record that an undecided prune occurred, and cap the result at `incomplete` with a cause naming the undecided invariant (never `optimal` or `infeasible`). The checker must refuse to accept, or report not-certified, when it skipped a step for an undecided invariant. Add a test. | fsl-analyze/src/plan_search.rs:777-785; fsl-analyze/src/plan_search/certificate.rs:357-361, 491-502; fsl-analyze/src/plan_search/replay.rs:78-86 |

Round 2, head a912259602ffc115215ccd4842daf136bd3597b3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | `admission()` classifies a state as Admitted, Violated or Undetermined, and a violation outweighs an undetermined invariant (plan_search.rs:602-616). `admissible` records the first undetermined prune and does not expand the state (plan_search.rs:826-840). `settle` checks `undecided` before writing any certificate and returns `stopped(InvariantUndetermined)`, which is `incomplete`, carries any replayable best plan and has no certificate (plan_search.rs:1144-1149). Every other path is safe. The stop paths already return `incomplete`. The negative-cost restart starts a fresh search that records again. The checker ends with `PlanStopCause::InvariantUndetermined`, not `CertificateRejected` (certificate.rs:494-514). Replay returns `PlanSearchError::InvariantUndetermined`, which `stopped` and `settle` turn into `incomplete` with no plan, so the substitution seam is covered too (replay.rs:78-91, plan_search.rs:1090-1094, 1235-1240). The writer emits `{"type":"invariant-undetermined","invariant":name}` (plan_request.rs:556-557). This matches FR-1012 Behavior, FR-1012-AC-7 and FR-1010's cause list. Both new tests drive the case with the cube invariant at `integer_bits` 5. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | TC-2862's Test Procedure lost step 6 (`Plan over counter with max_states 2 and with max_work 1, each twice.`). The new line that replaced it is numbered `.` instead of `7.`, while Expected Results still lists Step 6 and adds Step 7. The pinned-count procedure is gone from the test case. Fix: restore step 6 and number the new step `7.`. | spec/test-cases/TC-2862-plans-keep-policies-exact-costs-and-canonical-order.md:22 |
