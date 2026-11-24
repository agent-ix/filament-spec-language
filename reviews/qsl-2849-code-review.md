---
id: SR-3639001
title: "code-review of PR 3639 (QSL-2849, QSL-2969): limits refuse before the work they bound"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3639 head bc7fa1b398, git diff origin/main...bc7fa1b398 (fsl-analyze schedulability, model_check initial/check/refine, exact_probabilistic unbounded/finite_horizon/fair, analyze/exact*)"
review_set: subset
---

## Summary

Ticket: QSL-2849 (also QSL-2969). Code review with the Rust review lane folded in, scoped
to the PR diff. Covers the EN-7 shared `Pace` (recurrence steps charge `max_demand_points`),
the EN-1 `admit_initial` early exit in `check_model` and `check_refinement`, and the EN-5
`dense_system` storage check with its `MaxWorkingBytes` -> `LimitReached` mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | EN-5 Bareiss elimination remains uncancellable and bounded only through the matrix-size cap; at the 1 GiB default a several-thousand-row system runs O(n^3) BigInt work with no cancel poll, which the ticket lists as a defect and FR-203-AC-16 now writes down as intended | fsl-analyze/src/exact_probabilistic/unbounded.rs:1660-1700 |
| FND-002 | low | dense_system meters only rows*(rows+1)*size_of::<Rational>() inline cells; solve_linear then builds a second, unmetered BigInt matrix and no heap digits are counted, so max_working_bytes does not bound the solver's working storage | fsl-analyze/src/exact_probabilistic/unbounded.rs:2571-2580 |
| FND-003 | low | The PR adds new rustfmt drift: `fn broken()-> EngineReport`, and two over-width `analyse` signatures | fsl-analyze/src/analyze/exact.rs:307 |
| FND-004 | low | The state-graph explorer still keys the whole initial set before checking max_states (the same EN-1 pattern), and it was not moved to admit_initial | fsl-analyze/src/model_check/state_graph/explore.rs:150-172 |

## Verdict

CONDITIONAL. No panic or wrong result was found. `dense_system` overflow is checked
(`checked_mul`, so `usize::MAX` refuses). `Pace::step` cannot overflow because
`used < limit`. `admit_initial` keeps the old first-index coalescing and key order. In
refine.rs the `keys` vector is filled in the same order as on main. In every existing
limit, the refusal code, value and cause are unchanged. The one change in precedence is
that an over-budget initial set with a later unkeyable state now reports MaxStates, not the
keying halt, and this is inherent to refusing early. Every `UnboundedError`/`FhError`
consumer that can reach a dense system maps `MaxWorkingBytes` to
`LimitReached{CertificateMaxWorkingBytes}`. The `measure.rs` -> Malformed arm is on the
acyclic `decide_measure_exact` path, which never builds a dense system. The FR-247-AC-5
test fails without the change on both policies: fixed priority had no limit, and the EDF
QPA sequence of Ctl is one point. The policy-evaluation test fails if `evaluate_policy`
drops the check.

## Dispositions

Round 1, reviewed head 1946b4382f (fix commits 2a68a88dc5 and 1946b4382f over the rebased 9309dd6832).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | solve_linear polls a Dense cancel handle at every matrix row and every pivot row; Cancelled settles Incomplete(Cancelled{source}) |
| FND-002 | fixed | DENSE_CELL_BYTES counts the Rational and the BigInt cell; FR-203-AC-16 now states that heap digits are not counted |
| FND-003 | fixed | The three named sites are formatted; new drift from the fix round is FND-005 |
| FND-004 | fixed | The state-graph explorer now runs admit_initial over initial_iter |

Round 2 reviewed `1ac4da2a725290e8986f265eeb45bf73917e7d0d`. The PR requires another fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | rustfmt 1.9.0 passes on exact.rs, model_check/mod.rs and all four test files named in this finding. |
| FND-006 | still-open | The protocol iterator constructs a full root before admit_initial checks distinct.len() > max_states. With four distinct protocol roots and max_states=1 the real check constructs two, as protocol_system.rs:726 confirms. The owner requires refusal before construction of the first distinct over-limit root. An in-budget construction fault is preserved and a fault strictly after the crossing root is skipped. |
| FND-007 | fixed | The four-row fixture cancels on poll five and asserts both the exact stop and its recorded cause. Removing pivot-row polling leaves four conversion polls and a successful solve. |
| FND-008 | fixed | The counter declaration, public reader, hot-path increment and re-export all require the test-support feature. |
| FND-009 | fixed | Dense::check calls poll and reads tripped. The observer test records Requested at the fifth poll. |
| FND-010 | still-open | A model check with partial-order reduction enters precheck even with no symmetry declarations. reduce::closure collects and keys the complete generated initial set before max_states admission. The empty group has no generators. A small state budget therefore still incurs every clock-range construction. Nontrivial symmetry also reaches this collector in state-graph exploration. |
| FND-011 | still-open | With a possible item and nonzero witness_samples, state-graph phase zero calls self.system.initial().len() before Exploration::run and allocates one witness slot per initial state per instance. A small max_states budget therefore permits the complete generated initial set and its sampling work before refusal. |
| FND-012 | still-open | With one sample for each of N initial states, Phase::run visits indices 0 through N-1. The observed walk now starts a fresh generated_initial iterator and calls nth(initial) each time. A clock-range odometer constructs the skipped states again, giving N(N+1)/2 generated states just to select the N starts, even when the entire set is within budget. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The fix round adds new rustfmt drift: 3 sites in exact.rs, one pub use ordering in model_check/mod.rs, and 45 diffs in four test files | fsl-analyze/src/analyze/exact.rs:410 |
| FND-006 | medium | EN-1 refuse-before-allocation is not met: ModelSystem::new enumerates the whole initial set (request documents x ranged/with_clock_counts cartesian expansion) at construction, unbounded by max_states; initial_iter saves only the clone and the keying | fsl-eval/src/simulation/model.rs:1521-1556 |
| FND-007 | low | a_cancelled_handle_stops_the_elimination cancels before the call, so the row-conversion poll catches it; deleting the Bareiss pivot-row poll, the one that bounds the O(n^3) work, leaves the test green | fsl-analyze/src/exact_probabilistic/unbounded.rs:1752-1756 |
| FND-008 | low | Test-only instrumentation in production: a thread-local KEYED counter incremented on every admitted state and exported as #[doc(hidden)] pub fn keyed_initial_states, not behind cfg(test) or a test-support feature | fsl-analyze/src/model_check/initial.rs:8-17 |
| FND-009 | low | Dense::check reads cancel.cause() rather than cancel.poll(), so a dense-solve stop is not recorded in tripped() and is invisible to Cancel::observing and charge counts, unlike Pace and the meters | fsl-analyze/src/exact_probabilistic/unbounded.rs:814-820 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | A model check with partial-order reduction enters precheck even with no symmetry declarations. reduce::closure collects and keys the complete generated initial set before max_states admission. The empty group has no generators. A small state budget therefore still incurs every clock-range construction. Nontrivial symmetry also reaches this collector in state-graph exploration. | fsl-analyze/src/model_check/reduce.rs:244 |
| FND-011 | medium | With a possible item and nonzero witness_samples, state-graph phase zero calls self.system.initial().len() before Exploration::run and allocates one witness slot per initial state per instance. A small max_states budget therefore permits the complete generated initial set and its sampling work before refusal. | fsl-analyze/src/model_check/state_graph/sample.rs:237 |
| FND-012 | medium | With one sample for each of N initial states, Phase::run visits indices 0 through N-1. The observed walk now starts a fresh generated_initial iterator and calls nth(initial) each time. A clock-range odometer constructs the skipped states again, giving N(N+1)/2 generated states just to select the N starts, even when the entire set is within budget. | fsl-eval/src/simulation/model.rs:3857 |
