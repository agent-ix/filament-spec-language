---
id: SR-2026112118004026158500
title: "code-review of PR3360 closure-stutter replay consumer contract"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3360 78e47c7ccb06352af03673036e9e24a37ecb1bb1 over base 158cc4328568a0c864b90b07105a4276f19d6f0e; spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
review_set: subset
---

## Summary

Ticket: QSL-2247. Spec-only diff of three Markdown files; no code, Cargo or runtime change. This review checks the contract's claims about real FSL sources and the buildability of its fixtures and oracles against base 158cc432. Source facts confirmed: ProductTransition::StutterClosure (fsl-analyze/src/model_check/product.rs:46), FSL-owned Lasso (fsl-replay/src/model_check/counterexample.rs:213), ModelTraceSubject without a closure field (fsl-replay/src/model_check/replay.rs:99), replay_checked_source_model_trace (replay.rs:1130), observed ModelStepWire (fsl-replay/src/spine/request.rs:498), the package adapter refusing any stutter_closure (fsl-analyze/src/analyze/clause_item.rs:884, 923), TRANSFER_DECLARATIONS StuckTwo exact syntax (tests/it/plan_export_surface.rs:1491) and the existing unsupported expectation (plan_export_surface.rs:1912). Three replay criteria are not buildable as written and two transfer statements are imprecise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-128-AC-10's two-atom implication clause cannot be evaluated by the lasso_replay harness, whose Letters reads every atom with one fn(i64); the AC must name a per-atom or production-letters oracle | spec/functional/FR-128-replay-a-model-counterexample.md:371 |
| FND-002 | medium | FR-128-AC-12's fair case ("no constraint enabled there") has no buildable fixture: the lasso_replay Counter has only inc, enabled at every nonterminal state, and the fair case's settlement is unstated | spec/functional/FR-128-replay-a-model-counterexample.md:373 |
| FND-003 | medium | FR-128-AC-13's mixed-loop limit names no fixture, clause, written lasso or re-indexed trace_position; the inc-only Counter has no real cycle, so no engine mixed loop exists to write | spec/functional/FR-128-replay-a-model-counterexample.md:374 |
| FND-004 | low | FR-1030-AC-6 says StuckTwo's only violating loop is the closure at the initial state; the closure at value 1 also violates it, and the empty prefix is EN-1's discovery-order choice | spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md:169 |
| FND-005 | low | FR-1030-AC-7 and TC-2920 step 7 give the new clause no name and use SoonTwo (an interval TP-3 clause) as its declaration template instead of StuckTwo | spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md:170 |

FND-001: fsl-analyze/tests/it/counter.rs:542-567 ignores the FormulaNodeId and applies one predicate to every atom. With is_three, holds(c.value = 1) reads value==3, so at value 1 the antecedent is false, the formula is true and replay settles ReplayParity, not reproduced. lasso_replay.rs already has production-letters routes (a_guarded_on_each_prefix_replays_through_production_letters, a_clause_with_a_capture_replays_through_the_production_letters); the AC should require one.

FND-002: document(false) (counter.rs:111-136) has only inc, and Level(3) makes 0..2 nonterminal, so any weak or strong constraint on inc is enabled at every closure state. The fair case needs a named second operation disabled at the loop state (for example a guarded setTo from document_with_ranged_parameter), and the AC should state that the fair lasso reproduces.

FND-003: the mixed-loop rule is FSpec FR-364-AC-29. To test it in FSL the AC must name a fixture with a real cycle (the setTo(0) document), a stutter-sensitive clause, the expected written lasso and the expected re-indexed trace_position. Otherwise "differs" and "re-indexed" have no oracle.

FND-004: Spec::Counter is value Int[0,2] with inc only (tests/support/refinement_cas.rs:306-309; CanInc value<2), so closures at 0 and at 1 both violate always eventually value=2. The empty prefix holds because EN-1 takes the first passing component in discovery order (fsl-analyze/src/model_check/liveness.rs:33-38). The AC should say that rather than "only".

FND-005: SoonTwo is declared at plan_export_surface.rs:1486. StuckTwo (1491) is the template for liveness with no fairness. Name the declaration and give its exact text so the test and the AC agree.

## Scope and evidence

No Rust lane: the diff has no .rs or Cargo files. No build or test was run; this is a spec-only review. Prep evidence predating #3330 was re-checked against base 158cc432. The upstream contract was read at the FSpec closure-contract worktree, FR-364 lines 300-340, 436-460 and AC-25..29.

- FR-128-AC-10 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, FND-001
- FR-128-AC-11 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, the pointers and order match FSpec FR-364 step 2
- FR-128-AC-12 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, FND-002
- FR-128-AC-13 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, FND-003
- FR-128-AC-14 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, ModelStepWire exists as cited
- FR-1030-AC-6 (spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md): examined, FND-004
- FR-1030-AC-7 (spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md): examined, FND-005; the formula forces a one-inc stem to value 1
- TC-2920 (spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md): examined, step 7 says honestly that the declaration is new

## Dispositions

Disposition pass 1, checked against the frozen fix 101574eb2b8a5a7102740deee9ef6dad08f5cc67 (delta from 78e47c7c). The fixtures are proposed: none is built or run, and no consumer test is claimed to pass.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-128-AC-10 now uses single-atom clauses on the one-fn Letters: FORMULA/is_three with an empty stem, and `always eventually holds(c.value = 0)`/is_zero with one inc to value 1 and the closure loop at 1. Both use hand-written lassos with trace_position 0. The two-atom clause stays only in FR-1030-AC-7, which runs through production letters in the native transfer. |
| FND-002 | fixed | FR-128-AC-12 uses document_with_ranged_parameter(). CONTRACT (counter.rs:33) guards only inc (value < 3), so setTo keeps value 3 nonterminal while inc is disabled there. That gives a buildable fair lasso (reproduced) and an unfair one at value 1 (UnfairLasso). `fair weak M::Counter::inc;` is the existing fixture form (lasso_replay.rs:150-152). |
| FND-003 | still-open | AC-13 is now concrete, but its clause `always (holds(c.value = 1) implies not once[1,1] holds(c.value = 1))` is in the safety fragment (temporal_clause.rs:2224-2264 admits always/implies/not/once), so its form is Safety. ClauseSubject::item refuses stutter_closure unless the form is Liveness (clause_item.rs:206), and FSpec FR-368 allows stutter_closure only on a liveness premise. "The engine's lasso is ..." also cannot come from EN-1, which writes a finite bad prefix for a safety clause. Fix: say it is a constructed engine lasso given to the writer (as AC-14 does) and use a liveness-form stutter-sensitive clause, for example the same conjunct `and always eventually holds(c.value = 2)`. |
| FND-004 | fixed | FR-1030-AC-6 now says the closures at value 0 and at value 1 both violate StuckTwo and the stem is EN-1's discovery order. TC-2920 step 6 agrees. |
| FND-005 | fixed | FR-1030-AC-7 and TC-2920 step 7 name `PendingTwo` with the exact declaration on the StuckTwo template, marked proposed. The claim that value 1 is its only violating loop checks out on Spec::Counter Int[0,2]. |
