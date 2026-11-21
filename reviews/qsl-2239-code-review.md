---
id: SR-202611210022442313144108429
title: "code review (with Rust lane) of PR 2957 finite hyper and refinement liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2957 head abd6935cc8c29737ed1c8be4618543a6b35ec101 vs origin/main d84ed5685b; fsl-analyze, fsl-replay, fsl-eval, fsl-semantics, fsl-package, fsl-forms, fsl-command"
review_set: subset
---

## Summary

Ticket: QSL-2239 (PR record) and QSL-2243. Static review only (no build on terra by order). The untimed hidden-field liveness decider (per-candidate rows, loop-summary antichain search, U(v) Emerson-Lei confirmer) is sound in the directions the FND-010 ruling required, and the earlier FND-001..010 rows hold at this head. The timed path is not: it ignores the per-candidate rows when checking abstract clock guards, and it decides each fair SCC from one greedy loop. The constraint naming rule implements only the fallback half of FR-899. The fix-round oracle changes are warranted (strong-fairness `proved`) except the FR-488-AC-18 edit, which contradicts the AC text it is tagged with. No Decimal intake (QSL-2451) remnant is in the diff. The PR is CONFLICTING against current main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Timed abstract side with a hidden field: `auxiliary_step` handles only `Taken::Abstract`, but with a hidden field every mapped step yields `Taken::CandidateSteps`, so abstract clock guards and resets are never applied and an `AbstractClockGuard` violation settles the safety half `proved` | fsl-analyze/src/zone_check/refine.rs:962-1031; fsl-replay/src/refinement/step.rs (judge_operation, decide_step) |
| FND-002 | high | Timed hidden-field liveness decides each fair concrete SCC from one greedy loop: if that loop has a fair candidate run the whole SCC is skipped (`Ok(true) => continue`), so a violating sub-loop in the same SCC is never examined and the half returns a false `holds` | fsl-analyze/src/zone_check/refine/live.rs:266-307 |
| FND-003 | medium | HiddenUnfair naming implements only the prefix fallback (least j with F_A[0..=j] unsatisfiable); FR-899's primary rule (first constraint every candidate cycle violates) is not applied, so with F_A=[c1,c2,c3], c1/c2 jointly unsatisfiable and c3 violated by every cycle the engine names c2 where the spec names c3; replay uses the same rule and agrees with the wrong name | fsl-analyze/src/model_check/refine.rs:3194-3235; fsl-replay/src/model_check/refinement_replay.rs:1633-1645; fsl-analyze/src/zone_check/refine/live.rs:266-300 |
| FND-004 | medium | Abstract-protocol candidates: enabledness is read from the abstract model state (`abstract_system.expand_state`), not from the protocol configuration as FR-899 "Abstract protocol subjects" requires, so a class the protocol cannot fire at a rest point counts as enabled and can yield a spurious HiddenUnfair refutation; the U(v) confirmation is skipped for protocols, so nothing catches it | fsl-analyze/src/model_check/refine.rs:2456-2520, 2718-2760 |
| FND-005 | medium | FR-488 refusal 2 (alias that binds no object parameter) removed in emission.rs and the AC-18 test's alias-only assertion deleted, but FR-488 "Clauses S4 does not emit" item 2 and FR-488-AC-18 ("the alias case alone is refused at the clause too") still require it; FR-631-AC-4 requires the opposite; the spec conflict is unresolved and the test tagged FR-488-AC-18 now asserts behaviour that AC contradicts | fsl-semantics/src/check/hyper_clause/emission.rs:46-56; fsl-package/src/emit/tests/hyper_clause.rs:1145-1163 |
| FND-006 | medium | Finite-hyper population refusals name no trace: `invalid_population()` carries only the catalog code, while FR-898 "Request" requires `invalid_runtime_input`/`invalid-value` naming the trace for a nonconforming record or a conflicting identity | fsl-analyze/src/analyze/finite_hyper_item.rs:107-156, 339-345 |
| FND-007 | low | Finite-hyper body evaluation maps every evaluator error, including meter exhaustion, to `Incomplete`, so a budget stop settles `inconclusive TemporalIndeterminate` instead of a stop naming the limit (FR-625) | fsl-analyze/src/analyze/finite_hyper_item.rs:175-221 |
| FND-008 | low | Open population with an all-`forall` prefix settles `refuted`; FR-898 "Result" lists an open population under `inconclusive` without that exception. The code's reading is logically sound but the spec text and AC-4 ("An open population settles inconclusive") do not state it | fsl-eval/src/hyper/finite.rs:173-183 |
| FND-009 | low | Dead unsupported paths left after FR-899 ("never settled unsupported"): `TimedLivenessHalf::Unsupported`, `UnavailabilityCause::HiddenFieldLiveness` / `StrongAbstractFairness` have no producer; stale docs claim hidden fields are unsupported | fsl-analyze/src/zone_check/refine.rs:166-181; fsl-replay/src/proof_result.rs:625-638; fsl-analyze/tests/it/timed_refinement_liveness.rs:521-522 |
| FND-010 | low | `unreachable!()` on a non-test path, guarded only by an earlier match arm in the same function | fsl-semantics/src/check/timed_model.rs:4385 |

## Scope and evidence

- FR-898 (all ACs), FR-899 (AC-1..10), FR-900 (AC-1..8), FR-488-AC-18, FR-631-AC-4 examined against the diff `git diff origin/main...HEAD` (71 files).
- Prior history: FND-001..010 of SR-2026111612172400000050 re-checked. Open-population exists refutation inconclusive (finite.rs:183); production path routes `ItemForm::FiniteHyper` (request.rs, document.rs); per-candidate `CandidateStep` rows replace the union; joint F_A check in the summary search; per-candidate enabledness. All hold. Weak-fair antichain order (SR-2026111911313600000003 FND-001) holds (refine.rs `at_least_as_fair`).
- Fix-round oracle changes: `a_strong_abstract_fairness_refinement_settles_the_library_result` -> `proved` is warranted: the abstract `inc` has `SpecCanInc` (`value < 3`), the only concrete cycle is the terminal stutter at value 3 where abstract `inc` is disabled, F_C is empty and there is no implicit non-terminal stutter (FR-143), so no unfair loop exists. The precedence-test change is FND-005. Finite emit clock selection, profile selection in the lock and `None`/`source_unit: None` keys are consistent with FR-898 S4 and FR-640.
- Interdiff of the engine files against the arch-reviewed head 20c05342 is formatting-only, except routing and `clocks` wire changes in analyze/document.rs and fsl-command analyze.rs, which are correct.
- `cargo fmt --check` lists 31 PR files, but main already fails it on about 180 other files, so it is not counted as a PR finding.

## Verdict

Not mergeable on review grounds: two high soundness defects in the timed hidden-field path (FND-001 false safety `proved`, FND-002 false liveness `holds`) and four medium findings. The untimed engine, finite-hyper evaluator and replay are otherwise sound.

## New findings (disposition pass 1)

Reviewed head `ed3f278e4e3ece4f49f6520019d5cc3f14c18d68` (main integration 4570ddc42d, compile repair c6b276d0ac, FR-488 amendment ed3f278e4e). Static review; no cargo run by this reviewer, and the focused TEST-RUN batch was still pending when this was written.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The FND-002 guard accepts a timed hidden-field `holds` only when every member of the fair component has exactly one outgoing edge inside it. In a dense zone product each vertex usually carries a step arc as well as `Wait`/`Diverge` arcs inside the component, so the guard almost never fires and a timed hidden-field liveness half that holds now settles undecided (`LassoNotConcretized`). The unchanged FR-899-AC-9 test `timed_hidden_free_initial_valuation_can_disable_weak_send`, which asserts `Holds`, is expected to fail. This is sound but leaves the AC-9 holds case unmet. Either the test is changed to assert undecided, with that shortfall recorded on QSL-2243, or the exact per-component check is built. Pending TEST-RUN 1 confirms or refutes this | fsl-analyze/src/zone_check/refine/live.rs:295-317, 1317-1326; fsl-analyze/src/zone_check/refine/live.rs:486-543; fsl-analyze/tests/it/timed_refinement_liveness.rs:586-597 |
| FND-012 | low | `RefinementCause::TimedAbstract` has no producer since the S3 timed-abstract refusal was removed (main constructed it at fsl-semantics/src/check/refinement.rs:2389). It is the same dead-path class as FND-009 and was left in place, with its code/cause arms and the fsl-replay spine arm | fsl-semantics/src/check/refinement.rs:270-276, 364, 390; fsl-replay/src/spine/refinement.rs:317 |

## Dispositions

Round 1, at head `ed3f278e4e3ece4f49f6520019d5cc3f14c18d68`. Each fix was checked against the code at this head, independent of the coder report.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 523292e89c: `check_refinement_with_time_source` returns `ZoneCheckRefusal::HiddenFieldTimedAbstract` before the product is built whenever the abstract side is timed and any object has a hidden field. This covers both the dense and the free-delay entry points (refine.rs:310-317), and items.rs:997-1001 settles it unsupported/not-yet-implemented. No run, outcome or counterexample is produced, so a skipped guard cannot settle `proved`, and replay is never reached. In production `request.rs:934-938` keeps timed packages off EN-1, so no untimed path bypasses the refusal. Test: `a_timed_abstract_side_with_a_hidden_field_is_refused` (timed_refinement.rs:368-403, still pending a run). The exact fix is recorded on QSL-2243 (comment 9e16007f) |
| FND-002 | fixed | 523292e89c/27a43a517b: `Ok(true)` ends the component only when `single_loop` holds. Counting edges in the unfiltered `model.graph` restricted to `members` makes the component one simple cycle, so the greedy loop is its only loop. Otherwise the code sets `not_concretised` and settles `LassoNotConcretized` undecided, while a later genuine violation still returns `Violated`. No false `holds` is left from an unexamined loop. The exact per-component check is recorded on QSL-2243 (9e16007f). The cost of this conservative cut is FND-011 |
| FND-003 | still-open | b3a5f57a8b fixes the untimed engine (`first_unsatisfied_fairness_prefix` tests `FairScope::Only` first) and replay (`replay_lasso` primary rule), and 683c7039fc tests both. The timed path named in this finding's Refs (live.rs:266-300) is unchanged. `decide_live` iterates classes and `lasso()` names `HiddenUnfair{class constraint}` for the first class whose prefix `F_A[0..=k]` has no fair run, so it still applies only the prefix rule. With F_A=[c1,c2,c3] (c1,c2 jointly unsatisfiable, c3 universally violated) the timed engine names c2, while the fixed replay computes c3 and does not reproduce it. Engine and replay now disagree on the timed path. Needed: the primary rule in the timed loop (test each constraint alone first), or a conservative undecided when the alone-test names a different constraint |
| FND-004 | deferred | 9f694a968e: a hidden-unfair refutation over an abstract protocol now returns `Undecided(UndecidedSuccessor)` before naming or counterexample construction (refine.rs:2736-2745). Model-state enabledness over-approximates protocol enabledness, so the remaining `HoldsReduced` answers stay sound. FR-899 protocol refutations are therefore never produced. Remaining: read enabledness from the protocol configuration, on QSL-2243 (recorded in this round's QSL-2243 cross-link, since 9e16007f omits it) |
| FND-005 | fixed | ed3f278e4e follows the arch ruling (FR-631-AC-4 wins): item 2 is removed from FR-488 "Clauses S4 does not emit", the alias case is stated as emitted with the ruling cited, and FR-488-AC-18 is amended openly. Code (emission.rs) and the AC-18 test now agree with FR-488. The cross-spec restatement in FR-362 that still lists the alias case is SR-202611210022442313144108500 FND-001 |
| FND-006 | fixed | 300f688441: `invalid_population(Some(&named))` for a non-object record, a missing field or a nonconforming value, and `Some(identity)` for `EmptyTrace`/`ConflictingTraceIdentity`, emitted as `DeclinedWithDetail` with `detail.trace`. `None` remains only where no trace is at fault (missing/undecodable document, wrong format/record type, empty identity). finite_hyper_request.rs asserts `detail.trace == "a"` |
| FND-007 | fixed | 300f688441: `Outcome::Incomplete(incomplete)` (filament-exact: "a named charge was unavailable") records `IncompleteCause::LimitReached{EvaluationMeter{counter}}`, and an otherwise inconclusive result settles `TerminalRecord::incomplete` naming the limit. A misaligned tuple stays `inconclusive` (no meter_stop), preserving FR-898-AC-5. No dedicated meter-stop test was added |
| FND-008 | fixed | 300f688441: the all-forall open-population refutation shortcut is removed, so an open population settles `Inconclusive` as FR-898 Result and AC-4 state. The eval test now expects `Inconclusive` |
| FND-009 | fixed | 78481093e3: `TimedLivenessHalf::Unsupported`, `UnavailabilityCause::HiddenFieldLiveness` and `StrongAbstractFairness`, their settle/outcome arms and the stale docs are removed, and a grep at this head finds none. The analogous `RefinementCause::TimedAbstract` is FND-012 |
| FND-010 | fixed | da13e1566c: the `unreachable!()` arm returns `TemporalCause::AmbiguousTimedDeclaration{span}`, and a grep finds no `unreachable!()` left in timed_model.rs |

## Dispositions (round 2)

Round 2, at frozen head `db0f80895f16a0cc6c0926b99321180ee1c571fb`. The fixes are 307050eff1, 37824e252a, e257f8f4c7, the main integration 4deb0ee47b (main ab98ea12d8) and 2cf9515da4. db0f80895f differs from 2cf9515da4 by one test expression in fsl-package/src/emit/tests/hyper_clause.rs. Static review; no cargo run by the reviewer. This round adds no new code-review finding.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-003 | fixed | 307050eff1: on the timed path, an `Ok(false)` loop is now named by `first_unfair_constraint`. It tests `FairScope::Only(i)` for every constraint in source order, falls back to the least `FairScope::Through(j)`, and raises a fault if neither applies. `lasso()` names `abstract_operation_fairness().nth(named)`. `candidate_loop_has_fair_run` numbers every candidate-enabled and taken transition before the scope filter and indexes constraints by their position in `abstract_operation_fairness()`, as `class.constraint()` does, so the scoped constraint sets share one identity numbering. The rule is the one `replay_lasso` applies, so the timed engine and replay agree. No timed three-constraint fixture exercises it (gap-analysis FND-009) |
| FND-011 | deferred | 307050eff1/37824e252a: both dense timed hidden-field cases that hold now assert `Undecided{Zone(LassoNotConcretized)}`, are traced to `FR-899` only, and state in their docs that FR-899-AC-9 is not met. Both passed on Draco at 37824e252a. The cut is sound, never a false `holds`. The exact per-component check, which is the remaining FR-899-AC-9 holds case, stays open on QSL-2243 (comment ee20b33f). AC-9 is not met |
| FND-012 | fixed | 307050eff1: `RefinementCause::TimedAbstract` and its `code`, `cause` and fsl-replay spine arms are removed; a grep at this head finds none |

## Round 3 note

Round 3 (authorized test-only completion, head `aed94771b48007ac1269258193ded2e7bef90b8b`) changes no production code. No code-review finding was outstanding after round 2, and none is added. The round-2 outcomes stand: FND-004 and FND-011 are deferred to QSL-2243, and FR-899-AC-9's holds case is not met.
