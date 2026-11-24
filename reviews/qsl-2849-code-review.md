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

Round 3 reviewed `bb649d2006b756bcbd7358a4fbd1474760ab5c86`. Open findings block merge.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | still-open | The deadlock/safety protocol check now constructs one distinct root at max_states=1 and preserves the admitted fault. The model SubjectSystem still inherits initial_for_check that ignores its limit (subject.rs:131). Its iterator yields full ModelSimState values before admit_initial checks the distinct count (initial.rs:47–54). The reduced model and state-graph tests count two generated states at limit one (reduction.rs:194, state_graph.rs:1493 and :1539). Mapped refinement likewise pulls a full model state before checking its key (refinement/subject.rs:363). The owner requires refusal before constructing the first distinct over-limit state. |
| FND-010 | fixed | An empty generator list returns before the iterator is pulled. With symmetry the closure consumes one state at a time and stops on the first over-budget orbit. The reduced-model counts confirm that the remaining roots are skipped. FND-006 tracks the crossing state itself. |
| FND-011 | fixed | Initial::admit now runs before closure, witness-slot allocation and phase-zero walks. Over-budget state-graph tests verify an empty sample set with and without nontrivial symmetry. |
| FND-012 | fixed | Phase zero passes each retained start by reference to the observed walk. Phase one consumes the same admission. The four-start test counts four generated states and checks the retained witness index. |
| FND-013 | still-open | Protocol liveness bypasses initial_for_check. check_protocol_constrained dispatches liveness separately (check.rs:952). LassoSystem::start_states pulls from initial_iter, settling the protocol root before it checks the product-state limit (liveness.rs:1165–1204). For a binding with one initial product state per root and max_states=1, starting a second distinct protocol root can return its construction fault before MaxStates. The counter test exercises deadlock freedom. |
| FND-014 | still-open | ProtocolSystem::reachable also pulls the unrestricted protocol iterator. It settles and keys each root before search checks nodes.len() >= max_states. With a nonempty subject and max_states=0, a first-root settlement fault therefore returns ReachRefusal::Initial before the required Limit(States(0)). The guarded safety iterator does not participate in this entry point. |

Round 4 reviewed `b05ca318826032f4fb39bf61918f9e3993f51c72` against `608e201178a28f96a0b4e738114ce7227cb66c7b`. BLOCKED by outstanding Rust and command-contract findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | still-open | Plain model and protocol admission now satisfy the counter, but two bounded model paths still construct the crossing root. A nontrivial model symmetry precheck pulls generated_initial() at fsl-analyze/src/model_check/reduce.rs:266, then canonicalizes and counts the orbit at lines 292–300. Even max_states=0 constructs the first full state. Mapped refinement similarly consumes a full generated model state at fsl-replay/src/refinement/subject.rs:363 before checking the limit at line 368. At limit one a second distinct candidate is already constructed in both paths. |
| FND-013 | fixed | Protocol liveness uses the bounded iterator and preserves construction refusals. The focused protocol liveness regression passed. |
| FND-014 | fixed | Direct protocol reachability obtains its roots from the guarded iterator and maps limit, encoding, expansion and construction failures distinctly. The max_states=0 direct reach test passed. |
| FND-015 | still-open | Symmetric model admission returns a permuted representative and reuses that same state for equivalent recipes (lines 313–315, 332–345). State-graph explore.rs:76–88 saves these as concrete starts. state_graph.rs:632–640 then checks their closure under the declared generators. With a swap of two unbound objects and two initial assignments (0,1) and (1,0), the closed set becomes two copies of one asymmetric state. The check reports state-graph-symmetry-initial-closure. Preserve concrete identities and permutations separately from the canonical exploration nodes. |
| FND-016 | still-open | The reader constructs the typed target with the caller’s activation bound, then stores it in private Inputs._target at line 701. Nothing reads that field. Lines 862–863 open or resume the same session regardless of the target, and the timed request is rejected at lines 648–653. The FSL command exposes no decoded target plus capacity handoff to the driver. An accepted non-timed request ignores its target. Driver plan derivation and preparation are separately assigned under QSL-2986. |
| FND-017 | still-open | The six-member target allows max_event_rate to be explicitly null. Serde Option also maps an omitted max_event_rate to None. Deleting only that member admits a malformed target and silently selects the default instead of raising the required request refusal. |
| FND-018 | still-open | Malformed target input uses ClauseRequest. Its Display adds “clause” to the published message (command.rs:886), and the details.path arm at command.rs:1605–1624 handles only /call pointers. A malformed target therefore lacks the required diagnostic details.path=/request/target and exact message. The CLI test asserts only a substring. |
| FND-019 | still-open | For F initial range factors the first recipe computes F suffix products by rescanning the remaining factors, costing F(F−1)/2 multiplications. With many binary holders even a zero-state request incurs quadratic work before it can refuse. Compute the strides once or maintain the existing mixed-radix odometer. |
| FND-020 | still-open | New target-wire validation and new recipe projection code fail the project edition-2021 rustfmt check. Changed examples are wire.rs:1041 and :1116, monitor.rs:612, and fsl-eval/src/simulation/model/refine.rs:210, :248 and :287. This finding covers only new lines. |

Round 5 reviewed `b0078a167be4aa35c8fce8d1ef8bfcbec4094c6e` against `dccaf11c3d00f51738761f26cb72ef3124415e02`. BLOCKED by outstanding monitor target and diagnostic findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The symmetry precheck uses initial_for_check_with_symmetry and bounded mapped refinement consumes concrete.initial_for_check(limit) at fsl-replay/src/refinement/subject.rs:369. Distinct projected identities are refused before cloning the concrete root. The bounded reduction/state-graph and zero-budget refinement assertions passed. |
| FND-015 | fixed | The iterator reserves canonical identities while yielding original concrete roots. The state-graph regression explicitly supplies a two-member swapped orbit and succeeds. |
| FND-016 | fixed | The public monitor_with_target_preparer entry point delivers the checked package, typed target with caller-supplied activation bound, and caller limits before the timed-kind refusal or first record. A target without a preparer is explicitly refused. The driver owns deriving and executing the prepared plan. |
| FND-017 | fixed | The rate field is required; explicit JSON null still selects the default. Omitting max_event_rate in the public request now yields invalid MonitorTarget. A separately reported defect affects repeated names within a non-null rate. |
| FND-018 | still-open | Target diagnostics now include details.path=/request/target. The required exact message is still wrong: command.rs:887 formats “invalid clause request member” and command.rs:1793 plus command/output.rs:572 publish it. FR-644-AC-17 requires “invalid request member”. A missing max_event_rate in a public command::run request returned the extra word. |
| FND-019 | fixed | The recipe plan computes strides in a single reverse pass, then uses them directly for each recipe digit. |
| FND-020 | fixed | Edition-2021 rustfmt with skip_children=true passes on monitor.rs, wire.rs and the recipe projection implementation. |
| FND-021 | still-open | The decoder calls the derived struct visitor directly, which admits a positional JSON array as well as an object. A target ["tick","step",16,zeroRational,null,1] passes target validation and reaches the caller-capacity check in command::run. FR-644 requires a closed object. Require an object at this boundary and verify the normalized refusal before any callback or record. |
| FND-022 | still-open | Parsing max_event_rate through serde_json::Value drops repeated members before the exact rational decoder runs. A non-null rate containing numerator "9" followed by numerator "1" was accepted by command::run as a valid rate and reached caller-capacity validation. Decode a required nullable rate from its original JSON tokens so malformed numeric objects receive the specified invalid-target refusal. |

Round 6 reviewed `2dc5f4817dfad4059ac12990eff97e0176311dba` against `16c724b092132cce8283860c9f67fd327f3db064`. BLOCKED by the EN-5 solver settlement regression at three adapters. All round-5 monitor code findings closed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-018 | fixed | The new MonitorTargetRequest variant supplies the published exact message. The native error and outcome diagnostic use the same request path, and the public malformed-target probes confirm the message. |
| FND-021 | fixed | The object-only visitor rejects a positional target before invoking the driver. The same array that succeeded in round 5 now receives the exact target refusal and callback=false. |
| FND-022 | fixed | The required rate is retained as raw JSON until typed decoding, so repeated rational members reach the serde duplicate-name check. Public command probes confirm callback=false and exact refusal; explicitly null still reaches the callback. |
| FND-023 | still-open | The shared unbounded failure adapter lost its MaxWorkingBytes and Cancelled arms during the rebase. The wildcard returns an engine fault. Long-run workload and every-scheduler routes use this adapter (exact/long_run.rs:416); the workload pivot-cancellation test returns Failed rather than Incomplete(Cancelled{Requested}). Restore the bound with CertificateMaxWorkingBytes and the originating cancel cause. |
| FND-024 | still-open | The time-bounded adapter handles PolicyIterationLimit but maps other FhError values to broken(). A Bareiss pivot cancellation in a timed deadline returns Failed instead of Incomplete(Cancelled{Requested}); a dense working-storage limit loses CertificateMaxWorkingBytes. The second bounded-event path has the same catch-all at line 1609. Restore the bounded-solver settlement at both entry points. |
| FND-025 | still-open | The independently implemented expected-elapsed adapter drops MaxWorkingBytes and Cancelled from decide_expected_reward through its wildcard. A dense reward solve that exhausts document.storage_bytes or observes cancellation consequently reports Failed. The prior reviewed implementation handled both variants immediately after MaxRationalBits. This branch needs its own fix even after the other two settlement adapters are corrected. |

Round 7 reviewed `e77768d387d01eee1ed13483835e60b2ce249000` against `ca74568372bdda843b1808c6fee4e3c5c56a8ee7`. PASS. All prior findings, including FND-023 through FND-025, are fixed. The three EN-5 storage-bound cases and three pivot cancellation cases cover the settlement paths. The graph worker fixture uses an infallible initial iterator yielding one state; the two protocol tests set one worker.

| FND | outcome | reason |
| --- | --- | --- |
| FND-023 | fixed | The unbounded adapter now settles MaxWorkingBytes with the actual document.storage_bytes bound and Cancelled with its original cause. Workload and scheduler byte limits and workload pivot cancellation are independently exercised. |
| FND-024 | fixed | Both the timed deadline and ordinary bounded-event paths now dispatch their FhError through bounded_solve_failure (lines 668 and 1634). The byte-limit and pivot-cancel arms preserve the correct terminal values. |
| FND-025 | fixed | run_timed_mean now explicitly handles the dense byte-limit and original cancellation cause. The timed working-byte integration checks Deadline and MeanTime, and timed_mean_dense_elimination_cancellation_settles_incomplete independently exercises its pivot. |

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

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | Protocol liveness bypasses initial_for_check. check_protocol_constrained dispatches liveness separately (check.rs:952). LassoSystem::start_states pulls from initial_iter, settling the protocol root before it checks the product-state limit (liveness.rs:1165–1204). For a binding with one initial product state per root and max_states=1, starting a second distinct protocol root can return its construction fault before MaxStates. The counter test exercises deadlock freedom. | fsl-analyze/src/model_check/liveness.rs:1165-1168 |
| FND-014 | medium | ProtocolSystem::reachable also pulls the unrestricted protocol iterator. It settles and keys each root before search checks nodes.len() >= max_states. With a nonempty subject and max_states=0, a first-root settlement fault therefore returns ReachRefusal::Initial before the required Limit(States(0)). The guarded safety iterator does not participate in this entry point. | fsl-eval/src/simulation/protocol/reach.rs:152-155 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | high | Symmetric model admission returns a permuted representative and reuses that same state for equivalent recipes (lines 313–315, 332–345). State-graph explore.rs:76–88 saves these as concrete starts. state_graph.rs:632–640 then checks their closure under the declared generators. With a swap of two unbound objects and two initial assignments (0,1) and (1,0), the closed set becomes two copies of one asymmetric state. The check reports state-graph-symmetry-initial-closure. Preserve concrete identities and permutations separately from the canonical exploration nodes. | fsl-eval/src/simulation/model/refine.rs:332-335 |
| FND-016 | high | The reader constructs the typed target with the caller’s activation bound, then stores it in private Inputs._target at line 701. Nothing reads that field. Lines 862–863 open or resume the same session regardless of the target, and the timed request is rejected at lines 648–653. The FSL command exposes no decoded target plus capacity handoff to the driver. An accepted non-timed request ignores its target. Driver plan derivation and preparation are separately assigned under QSL-2986. | fsl-command/src/command/monitor.rs:593-594 |
| FND-017 | medium | The six-member target allows max_event_rate to be explicitly null. Serde Option also maps an omitted max_event_rate to None. Deleting only that member admits a malformed target and silently selects the default instead of raising the required request refusal. | fsl-command/src/command/wire.rs:1026-1030 |
| FND-018 | medium | Malformed target input uses ClauseRequest. Its Display adds “clause” to the published message (command.rs:886), and the details.path arm at command.rs:1605–1624 handles only /call pointers. A malformed target therefore lacks the required diagnostic details.path=/request/target and exact message. The CLI test asserts only a substring. | fsl-command/src/command/monitor.rs:608-612 |
| FND-019 | low | For F initial range factors the first recipe computes F suffix products by rescanning the remaining factors, costing F(F−1)/2 multiplications. With many binary holders even a zero-state request incurs quadratic work before it can refuse. Compute the strides once or maintain the existing mixed-radix odometer. | fsl-eval/src/simulation/model/refine.rs:185-192 |
| FND-020 | low | New target-wire validation and new recipe projection code fail the project edition-2021 rustfmt check. Changed examples are wire.rs:1041 and :1116, monitor.rs:612, and fsl-eval/src/simulation/model/refine.rs:210, :248 and :287. This finding covers only new lines. | fsl-command/src/command/wire.rs:1041-1045 |

## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-021 | medium | The decoder calls the derived struct visitor directly, which admits a positional JSON array as well as an object. A target ["tick","step",16,zeroRational,null,1] passes target validation and reaches the caller-capacity check in command::run. FR-644 requires a closed object. Require an object at this boundary and verify the normalized refusal before any callback or record. | fsl-command/src/command/wire.rs:1033-1038 |
| FND-022 | medium | Parsing max_event_rate through serde_json::Value drops repeated members before the exact rational decoder runs. A non-null rate containing numerator "9" followed by numerator "1" was accepted by command::run as a valid rate and reached caller-capacity validation. Decode a required nullable rate from its original JSON tokens so malformed numeric objects receive the specified invalid-target refusal. | fsl-command/src/command/wire.rs:1039-1046 |

## New findings (disposition pass 6)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-023 | high | The shared unbounded failure adapter lost its MaxWorkingBytes and Cancelled arms during the rebase. The wildcard returns an engine fault. Long-run workload and every-scheduler routes use this adapter (exact/long_run.rs:416); the workload pivot-cancellation test returns Failed rather than Incomplete(Cancelled{Requested}). Restore the bound with CertificateMaxWorkingBytes and the originating cancel cause. | fsl-analyze/src/analyze/exact.rs:382-389 |
| FND-024 | high | The time-bounded adapter handles PolicyIterationLimit but maps other FhError values to broken(). A Bareiss pivot cancellation in a timed deadline returns Failed instead of Incomplete(Cancelled{Requested}); a dense working-storage limit loses CertificateMaxWorkingBytes. The second bounded-event path has the same catch-all at line 1609. Restore the bounded-solver settlement at both entry points. | fsl-analyze/src/analyze/exact.rs:634-638 |
| FND-025 | high | The independently implemented expected-elapsed adapter drops MaxWorkingBytes and Cancelled from decide_expected_reward through its wildcard. A dense reward solve that exhausts document.storage_bytes or observes cancellation consequently reports Failed. The prior reviewed implementation handled both variants immediately after MaxRationalBits. This branch needs its own fix even after the other two settlement adapters are corrected. | fsl-analyze/src/analyze/exact.rs:1087-1092 |
