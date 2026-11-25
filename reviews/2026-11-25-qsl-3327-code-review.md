---
id: SR-20261125200206
title: "code-review (rust) of PR 3966 written binding with the admitted member type"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3966 head 0ac1e4e84eb4bac1ccaecc97b5c6c643588f7003; git diff origin/main...HEAD (fsl-eval simulation/model.rs, simulation/model/letters.rs; fsl-analyze analyze/{hyper_route,items,state_graph_item}.rs, model_check/state_graph.rs, tests/it/model_instances.rs; fsl-replay certificate/{state_graph,zone_certificate}.rs, model_check/graph_replay.rs)"
review_set: subset
---

## Summary

Ticket: QSL-3327. Code review with the rust-review lane folded in. The PR adds `ModelSystem::written_reference(declared, instance)`: the reference in `declared`'s universe with the type of the member record an initial state admits for the key (from the existing `admitted_types` map, now kept on the system), falling back to `declared` when no initial record admits the key. Every producer and checker of a written `over` binding on the temporal, state-graph (digital and timed), zone and HP-5 routes moves from `bound_reference` (declared type) or the population's least member type (HP-5) to it.

Specs read: FSL FR-125 "Instancing" (AC-8, AC-9, AC-15), FR-181, FR-172; FSpec (57c662b1) FR-361 "Instancing" (the wire reference of a binding carries the admitted type), FR-395 "Object parameters".

Producer/checker pairs traced: temporal `over_binding` (producers `clause_item.rs` via `letters.instance_reference`, `items.rs::bound_over` for the model item and zone routes); state-graph digital (producer `state_graph_item::run_digital` and `model_check/state_graph.rs::instances`, checker `certificate/state_graph.rs::bound_over`, reverse map `state_graph_item::bound_instance`); state-graph timed (producer `state_graph_item::run_timed`, checker `zone_certificate.rs` fed by `graph_replay::graph_over` in the timed replay branch); HP-5 (producer `hyper_route::single`; no checker re-reads the written key: `restore_over_binding` has no production caller). Remaining `bound_reference` callers: probabilistic (`exact_probabilistic/{build,digital}.rs`, `statistical/model.rs`, `probability_certificate.rs`, `probability_certificate/document/instances.rs`) and protocol (`protocol.rs`, `protocol/wire.rs`, `model_check/settle.rs`), both covered by QSL-3126; `fsl-replay/src/model_check/item.rs:353` discards the result (validity check only). Predicate evaluation re-binds by (universe, key) (`state_predicate.rs`), so the changed `graph_over` reference does not alter reads.

## Verdict

CONDITIONAL. No correctness defect found at the ten changed sites: every producer and its checker now call the same `written_reference`, so they agree on every route the PR touches, and no non-probabilistic, non-protocol producer or checker still writes or expects the declared or least-member type. The requirement demands the admitted type on each changed route (FSL FR-125 "Instancing" for temporal and state-graph claims; HP-5 through FSpec FR-395 "Object parameters", which makes HP-5 instances FR-361 instances). Rust: error mapping, `#[implements]` import form and the moved `admitted_types` field are fine; no new panic, unsafe or unbounded work.

The findings are test isolation (FND-001, FND-002) and one mirror route outside the diff (FND-003). The unadmitted HP-5 key fallback is recorded in the gap-analysis SR as unspecified behaviour.

Tests: logs under `worktrees/QSL-handoff-2026-11-24/gates-lead-conf/qsl-3327-*.log` ran on base `41ba1c8735` plus the uncommitted change (not on a commit); `cargo check --workspace --all-targets` ran on head `0ac1e4e84e` (exit 0). red1 selected 0 tests (wrong target), red2/red3 show the three new tests failing before the change; producer-only shows the state-graph test failing (`no-instance`) with the fsl-replay changes reverted together. The seven failures match their attributions: two `state_graph` undefined guard/constraint (QSL-3360, same panic sites), `an_undefined_terminal_predicate...` (QSL-3314, same check refusal, line drift 302 to 306), `zone_certificate::the_certificate_members...` (64 vs 65, QSL-3380), `liveness::the_model_only_pass...` (same message on 88eb6bc392, an ancestor of the base, in QSL-3273-gate-s4-base-2.log; no ticket owns it), and two `domain::tests::clock_reference_domains_*` (tests changed on main by #3961 after the base). None was rerun on current main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The HP-5 test does not isolate the admitted type at `hyper_route.rs`: `v: M::Lamp` with `l` held as `Lamp` gives the same reference for the declared and the admitted type, so `hyper_route` calling `bound_reference(declared)` would still pass; a parameter typed `M::Thing` (held `Lamp`) would separate them | fsl-analyze/tests/it/model_instances.rs:2456-2474, fsl-analyze/src/analyze/hyper_route.rs:312-318 |
| FND-002 | medium | Nothing tests the admitted type on the timed state-graph route (`state_graph_item::run_timed` producer, `zone_certificate.rs` checker, `graph_replay::graph_over` timed branch) or `items.rs::bound_over` (model item and zone temporal routes). Reverting any one of these sites alone would leave every new test green, and `graph_replay.rs` is not isolated from `certificate/state_graph.rs`: the only revert run (producer-only) reverted the fsl-replay files together | fsl-analyze/src/analyze/state_graph_item.rs:1111-1117, fsl-replay/src/certificate/zone_certificate.rs:4095-4100, fsl-replay/src/model_check/graph_replay.rs:885-909, fsl-analyze/src/analyze/items.rs:203-214 |
| FND-003 | medium | Mirror route not moved: the replay spine's `cross_evaluate` counterexample (`model_trace.rs`, `package_selection.rs`) writes `over_binding` from `letters.over_reference`, the held row whose type conforms to `T`, and drops the counterexample (`over: over?`) when the admitted record does not conform to `T`, where FR-125 "Instancing" writes (`UniverseId`, admitted type, key). It agrees with `written_reference` only when a conforming object is held under the key in the initial state | fsl-replay/src/spine/clause/model_trace.rs:268-330, fsl-replay/src/spine/clause/temporal/package_selection.rs:519-571, fsl-eval/src/simulation/model/letters.rs:619-627 |

## Coverage

Changed sites examined: 10 of 10 production hunks and the 3 new tests. Remaining `bound_reference` and `key_reference` callers examined: 14 production sites, of which 8 are probabilistic or protocol (QSL-3126), 1 is a validity check, and 5 are evaluation reads that re-bind by (universe, key). Overlapping open PRs: #3948 (head 8ed6f6aecf) and #3924 (head 2fc3434e27) change other hunks of `state_graph.rs`, `graph_replay.rs`, `certificate/state_graph.rs` and `simulation/model.rs`, and add no `bound_reference` or `written_reference` call. #3948 fixes the two QSL-3360 failures seen in this PR's run.
