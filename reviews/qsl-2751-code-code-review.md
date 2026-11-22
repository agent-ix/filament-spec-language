---
id: SR-20261121235237000
title: "Code review and Rust review of PR filament-spec-language#3414 (QSL-2751 slice 1)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-2751; PR3414 head 3ad91680d07d88c5a79205e532fa6906ae1ab76d base ad79956e5546c69cdeab413a5ecf9228d0f05e06; fsl-analyze/src/model_check/check.rs, fsl-replay/src/certificate/closure.rs, fsl-replay/src/model_check/item.rs, fsl-command/src/command/transfer.rs, fsl-analyze/tests/it/closure_certificate.rs, fsl-analyze/tests/it/closure_certificate/tp3_each.rs, fsl-analyze/tests/it/analyze_tp3_package.rs, fsl-analyze/tests/it/counter.rs, fsl-analyze/tests/it/main.rs, fsl-analyze/tests/it/model_check_safety.rs, fsl-analyze/tests/it/protocol_bounded_lasso.rs, fsl-analyze/tests/it/protocol_counted_steps.rs, tests/it/plan_export_surface.rs; context fsl-eval/src/automaton/translate.rs, fsl-eval/src/automaton/key.rs, fsl-eval/src/automaton/monitor.rs, fsl-replay/src/planner/settle.rs, fsl-replay/src/planner/from_package.rs"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1031
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: references
---

## Summary

Ticket: QSL-2751. This reviews draft PR filament-spec-language#3414 (Refs, not Closes), frozen head 3ad91680d07d88c5a79205e532fa6906ae1ab76d against base ad79956e5546c69cdeab413a5ecf9228d0f05e06. The owning contract is the spec merged in #3410: FR-338-AC-29..31 and its new sections on unguarded `on each` TP-3 and on TP-3 over a protocol subject, FR-126, FR-127, FR-337, FR-1030-AC-8..11 (TC-2920), FR-1031-AC-5 (TC-2921) and TC-525 steps 17 and 18. Methods: dev-tools 0.3.2 `/code-review` with its Rust lane `/rust-review` plus the `rust-style` baseline and references, folded into this one artifact. The review is static only. Every runtime test is unrun because of an explicit architecture hold on Titan requests, so no runtime pass is claimed. The author's report and the ticket text were read as data. Tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0. Model claude-opus-5-5, run b6d48eeb-9843-473a-b193-e46c54e9b8f7.

## Verdict

**FAIL**: there is one high finding. When the PR removes the `on origin` gate, every `on each` TP-3 clause is routed to the monitored `["safety", branches]` keying. That keying cannot project the synthetic activation `always` node, so on a static trace both the producer certificate and the core checker fault for every `on each` vector of FR-338-AC-30, the Echo vector of FR-338-AC-31 and FR-1030-AC-8..10.

These parts are clean:

- The `fsl-command` refusal of a protocol abstract subject. It spells `<alias>::<protocol>` from `AbstractSide::Protocol`, `require_selected_model` runs first, and the `invalid-value` row at `/refinement_transfer/abstract/model` is preserved.
- The protocol route's switch to `instances_for(.., SubjectKind::Protocol, ..)`. Liveness clauses are refused earlier by `prepare`, and a bounded `on each` closure shape cannot reach the monitored branch, because `closure_shape` admits `on each` only under a bounded profile.
- The `ZeroEdges`/`fair_zero_states` generalisation. The TP-2 caller keeps the same least-`bounded_ref` locus over the same `found.fair_vertices`, so it is behaviour-preserving for TP-2 and the QSL-2247 closure-only semantics are untouched.
- The checker's zero-progress rule, which records uncounted edges between owing product states, the terminal stutter counted, and cut boundary states not stuttered. It matches the producer's pre-existing `zero_progress` rule.

The 'any member owes' rule is not the literal negated-automaton auxiliary construction. Because `Node::Or` excludes the state-formula operand (translate.rs:1611-1623), an owing member is only opened where the implication's antecedent holds. No wrong result was demonstrated, so this is recorded as a residual limit and not as a finding.

Rust style: no new `unwrap` or panic on caller input, no unsafe code, no new public items, and doc comments match the behaviour, except that the doc and comment claims in check.rs:1797-1802 and closure.rs:47-55 rest on FND-001. Gates: the coder's `cargo check` logs were read, and fmt, clippy, tests and deny were not run because of the hold.

## Assurance Context

No `type: AssuranceProfile` document applies under `spec/`. This is a direct delegated review, and no enforced workflow or human gate is inferred. None of the selected architecture, impact, measurement, exception, independence, structural-gap or producer-reliance evidence is available, and there are no active exceptions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR sends every infinite-trace `on each` TP-3 clause to `Translation::Monitored` (item.rs:396-404) and certifies it (check.rs:1806-1808), but its key projection cannot handle the activation wrapper. `translate_formula` wraps an `on each` body as a synthetic `always[0,*] (not guard or body)` with no source occurrence (translate.rs:832-850), and it sets `activation_root` only for `AutomatonKind::Liveness` (translate.rs:869-871). The `Always` rule then inserts that wrapper into `next.must` (translate.rs:1678-1681), and `Translator::configuration` skips only `activation_root`, returning `invariant("state-obligation-has-source-occurrence")` for it (translate.rs:1208-1215). As a result, the producer's `certificate()` errors with `ModelCheckRefusal::Translation` instead of returning `Holds` with a certificate (check.rs:2093-2100), and the checker's `product_monitored` faults at the first product state instead of returning `Accepted` or `BadState` (closure.rs:1414-1427). This affects every `on each` vector: FR-338-AC-30, the Echo vector of AC-31 and FR-1030-AC-8..10. | fsl-eval/src/automaton/translate.rs:869 |
| FND-002 | medium | The PR does not implement or test FR-1030-AC-11's package-route transfer. The AC requires `planner::settle` over the checked package to write the same `proved` transfer, a clause root that the recompiled package lacks to settle `failed`, and the emitted `temporal_clause` node to carry its `on each` activation. Only claim parity from source and package is tested (analyze_tp3_package.rs:49). The author's statement that `planner::settle` is only the composition settler is incomplete: `planner::settle` (planner/settle.rs:289) builds a `PlannerDeclaration::from_package` that also settles implementation guarantees (from_package.rs:125-131), while transfer.rs:349 uses `settle_transfer`. Whether a package-backed transfer is reachable through it is unresolved, and the AC is open. | fsl-analyze/tests/it/analyze_tp3_package.rs:49 |
| FND-003 | medium | The FR-1030-AC-10 test never asserts `InitialNotAbstract`, which is the AC's distinguishing reason. It checks only `cause.premise == 0`, `slots[0] == refuted` and `step_for_step.holds`, so a slot-0 refutation for any other refinement failure passes too. The failure kind is serialized as `{"kind":"InitialNotAbstract"}` (simulation.rs:251-252, counterexample.rs:1191-1192). | tests/it/plan_export_surface.rs:2521 |
| FND-004 | medium | The FR-338-AC-30 failure-domain test does not exercise two clauses of the AC. The AC requires cancellation with source `Requested` "after its first charge", settled `incomplete`. The test cancels before the call, so only the first `poll` is reached, and it never asserts that the cancel settles `incomplete`. The undefined-atom case also asserts `UndefinedEvaluation` without the stated position 2. | fsl-analyze/tests/it/closure_certificate/tp3_each.rs:451 |
| FND-005 | medium | Several FR-338-AC-31 branches are untested or tested in a different form. Requests whose `max_store_buffer`, `max_messages` or `max_live_instances` differs from the run's (AC-21 and AC-22) are untested. The Echo `on each` clause has no package-route parity test. The 'protocol components are cycle keys' claim is not asserted: the helpers use `with_cycle_keys(BTreeSet::new())`. The stated post-attempt-to-pre-attempt `SuccessorMissing` tamper is replaced by a two-attempt-to-one-attempt tamper (protocol_counted_steps.rs:1455-1467). | fsl-analyze/tests/it/protocol_counted_steps.rs:1455 |
| FND-006 | low | FR-1030-AC-8 requires each transfer to carry both premise identities, but the test asserts only that `transfer.premises` has length 2, never which identities it holds. | tests/it/plan_export_surface.rs:2475 |
| FND-007 | low | The FR-1031-AC-5 test accepts `details` from either `value.details` or `value.error.details`, so it does not pin the refusal's wire location. A move of the member between the two locations would pass unnoticed. | tests/it/plan_export_surface.rs:2570 |

## Dispositions

Round 1 is a blocker-only pass at frozen head bf991901a04c7652c6fd5153eb90a7beed953346, under the latest user policy (one review and one fix round; only a wrong result, wrong refusal, unsound proof or broken build blocks). FND-002..007 are non-blocking and sit in the PR body's single follow-up list, so they are not re-audited here.

The fix in `fsl-eval/src/automaton/translate.rs` is projection-only. It does not change transitions, acceptance or `owes_eventuality`, so it cannot make the producer or checker accept a behaviour the automaton rejects. It makes three changes:

- `synthetic_root` skips only the one synthetic `on each` safety `always` wrapper (`target`, set only for `AutomatonKind::Safety` with `Activation::Each`). Every reachable configuration carries that wrapper, so dropping it merges no two keys, and every other occurrence-less node still faults.
- `shifted_cores` relabels a positive-lower core's counter as `offsets`.
- `protocol_safety_debts` adds the finite `F`/`U` acceptance members, only for protocol safety.

Both of the last two are injective functions of the configuration, and the producer and checker share the one projection.

Runtime evidence was read as data:

- the wrapper regression `automaton_key::reached_on_each_tp3_monitor_states_resolve_to_fr338_keys` passed at 3030396f and fe4ed0f2;
- ordinary `on each` source and package parity passed at fe4ed0f2;
- the positive `on each` certified transfer passed at c5aa8d21;
- at c5aa8d21, PrevJumpEach and OlderDebtEach are refuted and PrevStutterEach is inconclusive, with every transfer premise-unproved, so there is no false proof.

At review time the bf991901 runs of EVAL2 (the protocol normal-form and positive-lower keys) and Echo2 were still pending. They must pass before merge.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The synthetic `on each` safety wrapper is projected away, so producer and checker keys resolve (translate.rs:937-939, 1236). No new blocker was found statically. Runtime qualification at bf991901 is pending. |
