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
