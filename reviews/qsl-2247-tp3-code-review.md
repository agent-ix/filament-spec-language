---
id: SR-2026112115155546054652100
title: "code-review (with rust-review lane) of the QSL-2247 TP-3 monitor-configuration closure certifier"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-eval/src/automaton/{key,mod,monitor,translate}.rs; fsl-replay/src/model_check/item.rs; fsl-replay/src/certificate/closure.rs; fsl-analyze/src/model_check/check.rs; fsl-eval/tests/it/automaton_key.rs; fsl-analyze/tests/it/closure_certificate.rs; tests/it/plan_export_surface.rs (two TP-3 transfer tests only); QSL-2247 partial (TP-3 on origin over an ordinary model subject); branch terra/missing-engines-forms/qsl-2247-tp3 at d487f66c2ef9ce85d17454b52b1fe83a570d8549 on base d25728d74d830ccde8e0aae5c14d0b315af0db1e; PR agent-ix/filament-spec-language#3330"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: reviews
---

## Summary

Ticket: QSL-2247 (stays open: closure-only lasso witness/replay, TP-3 `on each`, TP-3 over protocol subjects). PR: agent-ix/filament-spec-language#3330 (head d487f66c, unchanged since this analysis). The frozen head d487f66c was checked: clean tree, its single commit sits on d25728d7, and the diff is 10 files, +666/-106. This review covers the code-review lane and the Rust lane (rust-review and rust-style) together. Repo Rust conventions come from AGENTS.md (trace-tag ids, `tests/it/` layout). Its repository idioms are followed, including `#[allow(clippy::too_many_arguments)]` on checker step helpers, which matches the existing TP-4 helpers.

Runtime evidence: **none**. Per the brief, no test, clippy or fmt gate was run by the reviewer. The author reports `cargo check --locked -p fsl-eval -p fsl-analyze -p filament-spec-language --test it` with exit 0 and 0 warnings (log `/tmp/qsl-2247-tp3-check4.log`). That is real compile evidence for the three test targets. It is not a runtime green.

Clean (examined, no defect):
- Soundness shape. The checker (`run_monitored`) rebuilds `translate(clause)` and `Monitor::bad_prefix` from the recompiled clause. It re-reads every letter (`when`, captures, atoms, uncounted) and walks its own product keyed by `(model StateKey, monitor StateId)`. It uses the certificate only as a membership set. Every reached product state is checked `is_rejecting` before membership, then for membership. So a wrong or non-injective key can cause a rejection (incompleteness) but never an acceptance of a bad run. Nothing is taken from the certificate.
- Same kernel. `Translator::configuration` is a pure extraction of the former `buchi_state` body; the activation wrapping is moved unchanged. TP-4's `BuchiState` result is byte-identical by construction. No second past or interval semantics is added, and no `NodeId` or `StateId` is serialized. `SafetyStates::key` performs the FR-338 normalization (earliest-deadline, offset-union, obligation-free subsumption) and self-admits before returning.
- Engine/checker agreement. The engine product node is `(model, monitor state per instance)` with terminal stutter (`stutters()` true for non-bounded clauses) until the product repeats. Its per-instance projection equals the checker's per-instance product, because the monitor does not constrain the model. The joint stutter covers each instance's stutter. The certified path uses `chooser: None`, so no POR prunes edges. Initial states are read with `counted=true, first=true` on both sides. Boundary (memory- or instance-cut) states are not stuttered on either side, matching `run_liveness`/`run_bounded`.
- Gating. `prepare` sets `monitored` only for `Safety`, `Counting::Every`, `Activation::Origin`, no invariant atom and `is_safety_translated`. `item.rs` builds `Monitored` under the same conditions. `PropertyForm::BoundedMltl` is impossible under `InfiniteTrace` (`property_form`) and `ReachableInvariant` always has an invariant atom, so the item-side omission of a form check does not widen the set. The protocol checker passes `instances_for(Protocol)`, so it never builds `Monitored`. A liveness-kind automaton reaching `Monitor::bad_prefix` is refused with `NotSafetyAutomaton`.
- Budgets. Windows keep full `u64` `remaining()`. Past memory is keyed under `max_key_bytes` with typed `KeyEncodingLimit`. The checker maps that to `ClosureRefusal::Encoding`, and the engine maps it to `ModelCheckRefusal::Translation`. No truncation path exists.
- FR-127. No empty-certificate `proved`, no provider self-certification and no `NotTranslated => Proved` were added. The `certified` widening affects only `Counting::Every` runs. `check_reduced` still discards it.
- Tests use `expect`/`panic!` only in test code. No `todo!`, `dbg!` or `unwrap` on a production path was added.
- PrevStutter (FR-1030-AC-5) is still asserted `inconclusive`/`premise-unproved`/slot 1 `refuted`/exit 22. It is not weakened, and it is a real semantic oracle for past memory at the repeated terminal state.
- No explicit slot-1 `certified` CLI assertion is required. FR-1030-AC-1 demands `proved`, `decisive-witness` and both premise identities, which the test asserts. FR-1030-AC-3's `uncertified` concerns supplied premises. This is not a finding.

## Verdict

**APPROVE WITH CHANGES (2 medium, 4 low).** The certifier is sound by construction and reuses the shared kernel. The mediums are test gaps: the new checker's bad-state path has no negative vector, and the negative for the still-unsupported TP-3 classes was removed. Neither is a wrong result in the code. Merge readiness additionally requires the focused runtime filters listed in the analysis report to pass, and tracker publication; neither has happened yet.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test exercises the new TP-3 checker's `BadState` paths: `reach_monitored`'s `is_rejecting(after)` rejection and `run_monitored`'s undefined-letter `product_rejecting` at an initial state. Every new closure vector offers a certificate for a clause that holds, and its negatives reject only by `SuccessorMissing`/`InitialMissing`. A checker that never consulted `is_rejecting` would pass all new tests while accepting a certificate for a violated TP-3 clause (FR-338 "hold no bad state"). Add a vector such as `always (once[1,1] p or historically p)` with `p` false at value 0, whose (even empty) certificate is rejected `BadState` at the origin's `["safety",[]]` product state, and one violated at a later position. | fsl-replay/src/certificate/closure.rs:1321-1338; fsl-replay/src/certificate/closure.rs:1214-1218; fsl-analyze/tests/it/closure_certificate.rs:709,776 |
| FND-002 | medium | `a_safety_proof_with_an_empty_certificate_settles_failed_never_proved` was deleted. It was the settle-level pin that an engine proof of an uncertified TP-3 shape carries an empty certificate and settles `failed`, never `proved`. Its replacement inside the interval test forges an empty certificate for a now-certifiable clause, which exercises the `Monitored` checker (`InitialMissing`), not the typed-unproved fallback. The remaining unsupported classes (TP-3 `on each`; TP-3 over a protocol subject) now have no "never proved" regression test. A future widening of `monitored`/`item.rs` gating to those classes would go undetected. Keep a negative over an `on each` TP-3 clause (Counter subject): `Holds` with empty certificate, which settles `Failed`. | fsl-analyze/tests/it/closure_certificate.rs:683-823; fsl-analyze/src/model_check/check.rs:1801-1810 |
| FND-003 | low | `reached_tp3_monitor_states_resolve_to_fr338_keys` documents "distinct live facts give distinct keys … including past history and interval windows". It asserts only key→admit self-round-trip, `configurations.is_empty() == is_rejecting`, and `distinct.len() > 1`. That last check is satisfied by the rejecting key (asserted reachable) plus any one other key. No assertion shows a past-memory or window member in any key, and no exact state body is checked. A projection that consistently dropped `past`/`windows` would pass this test. The closure tests' distinct-key assertions partly compensate. Assert a non-empty past/window member, or one exact body, for `once[1,1]` and `eventually[0,2]` after one position. | fsl-eval/tests/it/automaton_key.rs:1323-1380 |
| FND-004 | low | `item.rs` treats "no TP-4 context" (`tp4: None`) as an ordinary subject (`tp4.is_none_or(Ordinary)`), keying `SubjectKind::Ordinary`. But the protocol producer calls `request.item.instances(...)` (tp4 `None`) for protocol subjects. Today this is unreachable for TP-3 only because `prepare`'s `counting == Counting::Every` keeps `certified` false there. Two separately maintained gates must stay in sync, or a protocol run gets ordinary key algebra. Pass the subject kind explicitly, or require `tp4` to be `Some(Ordinary)`. | fsl-replay/src/model_check/item.rs:396-405; fsl-analyze/src/model_check/check.rs:839 |
| FND-005 | low | Misleading fault identities on the new TP-3 paths. `automaton_error` maps a TP-3 monitor's `UnknownState`/`Invariant`/`NotSafetyAutomaton` to `fault("tp4-automaton-transition")`. The now-shared `Translator::configuration` raises `buchi-obligation-/window-/shift-/past-/acceptance-has-source-occurrence` for safety-monitor projections. A runtime_invariant record from a TP-3 run would name TP-4/Büchi. Rename to form-neutral ids. | fsl-replay/src/certificate/closure.rs:1610; fsl-eval/src/automaton/translate.rs:1215 |
| FND-006 | low | `monitored_step`'s `missing: CertificateRule` parameter is `SuccessorMissing` at both call sites. At the stutter call site, `state`/`state_key` duplicate `pending.state`/`pending.at.model_state`. Together these push the helper to nine arguments under `#[allow(clippy::too_many_arguments)]`. Drop `missing` (and pass the stutter case through `pending`) to remove the dead degree of freedom. | fsl-replay/src/certificate/closure.rs:1238-1268,1275-1286 |

## Scope

- fsl-eval/src/automaton/translate.rs `PropertyAutomaton::configuration`, `Translator::configuration`, `Translator::buchi_state` refactor (examined; FND-005)
- fsl-eval/src/automaton/monitor.rs `SafetyAutomaton::configuration` default/forwarders, `Monitor::configurations` (examined, clean)
- fsl-eval/src/automaton/key.rs `safety_shape`, `is_safety_translated`, `SafetyStates::clause` refactor, `max_key_bytes`, `key` (examined, clean)
- fsl-eval/src/automaton/mod.rs re-export (examined, clean)
- fsl-replay/src/model_check/item.rs `Translation::Monitored`, `admits`, `build_instances` gate (examined; FND-004)
- fsl-replay/src/certificate/closure.rs `run_monitored`, `monitored_step`, `reach_monitored`, `product_monitored`, `product_rejecting`, `property_letter` rename, `automaton_error` (examined; FND-001, FND-005, FND-006)
- fsl-analyze/src/model_check/check.rs `prepare` `monitored`, `bounded_translations`, `certificate` Monitored arm, module docs (examined; FND-002, FND-004)
- fsl-analyze/src/model_check/check.rs `explore`, `stutters`, `product` (context_only)
- fsl-eval/tests/it/automaton_key.rs `reached_tp3_monitor_states_resolve_to_fr338_keys` (examined; FND-003)
- fsl-analyze/tests/it/closure_certificate.rs `a_past_clause_proof_carries_monitor_keys_the_core_accepts`, `an_interval_clause_proof_carries_monitor_keys_the_core_accepts`, removed `a_safety_proof_with_an_empty_certificate_settles_failed_never_proved` (examined; FND-001, FND-002)
- tests/it/plan_export_surface.rs `a_step_for_step_refinement_transfers_previous_and_interval_clauses_proved` (un-ignored), `previous_transfers_only_through_a_step_for_step_refinement` (examined, clean)
- FR-338 statement and state-algebra table, "Guarded `on each`" and "Protocol non-progress" sections (context_only); FR-127 settlement map and AC-1 (context_only); FR-1030 statement L119-141 and AC-1..AC-5 (context_only; see SR-2026112115155546054652101)
