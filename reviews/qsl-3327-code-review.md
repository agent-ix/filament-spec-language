---
id: SR-20261125200206
title: "code-review (rust) of PR 3966 written binding with the admitted member type"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3966, review pass at head 0ac1e4e84eb4bac1ccaecc97b5c6c643588f7003, disposition pass 1 at head 80764c452acd233ecadd192eeb3c0237d6caa725; git diff origin/main...HEAD (fsl-eval simulation/model.rs, simulation/model/letters.rs; fsl-analyze analyze/{hyper_route,items,state_graph_item}.rs, model_check/state_graph.rs, tests/it/{model_instances,timed_state_graph}.rs; fsl-replay certificate/{state_graph,zone_certificate}.rs, model_check/{graph_replay,replay}.rs, spine/clause/temporal/package_selection.rs; tests/it/request_surface_s1_16.rs)"
review_set: subset
---

## Summary

Ticket: QSL-3327. Code review with the Rust review folded in. The PR adds `ModelSystem::written_reference(declared, instance)`. It returns the reference in `declared`'s universe with the type of the member record an initial state admits for the key, taken from the existing `admitted_types` map, which is now kept on the system. When no initial record admits the key, it falls back to `declared`. Every producer and checker of a written `over` binding on the temporal, state-graph (digital and timed), zone and HP-5 routes now uses it. Before, they used `bound_reference` (the declared type) or, for HP-5, the population's least member type.

Specs read: FSL FR-125 "Instancing" (AC-8, AC-9, AC-15), FR-181, FR-172; FSpec FR-361 "Instancing" (the wire reference of a binding carries the admitted type), FR-395 "Object parameters".

Producer/checker pairs traced:

- **Temporal `over_binding`.** Producers: `clause_item.rs` and `digital_item.rs` through `letters.instance_reference`, and `items.rs::bound_over` for the model item and zone routes.
- **State-graph digital.** Producers: `state_graph_item::run_digital` and `model_check/state_graph.rs::instances`. Checker: `certificate/state_graph.rs::bound_over`. Reverse map: `state_graph_item::bound_instance`.
- **State-graph timed.** Producer: `state_graph_item::run_timed`. Checker: `zone_certificate.rs`, fed by `graph_replay::graph_over` in the timed replay branch.
- **HP-5.** Producer: `hyper_route::single`. No checker re-reads the written key: `restore_over_binding` has no production caller.

Remaining `bound_reference` callers:

- Probabilistic: `exact_probabilistic/{build,digital}.rs`, `statistical/model.rs`, `probability_certificate.rs`, `probability_certificate/document/instances.rs`. QSL-3126 covers them.
- Protocol: `protocol.rs`, `protocol/wire.rs`, `model_check/settle.rs`. QSL-3126 covers them.
- `fsl-replay/src/model_check/item.rs:353` discards the result; it is a validity check only.

Predicate evaluation re-binds by (universe, key) (`state_predicate.rs`), so the changed `graph_over` reference does not alter reads.

## Verdict

Review pass: CONDITIONAL. No correctness defect was found at the ten changed sites. Every producer and its checker call the same `written_reference`, so they agree on every route the PR touches. No producer or checker outside the probabilistic and protocol routes still writes or expects the declared or least-member type. The requirement demands the admitted type on each changed route: FSL FR-125 "Instancing" for temporal and state-graph claims, and for HP-5 FSpec FR-395 "Object parameters", which makes HP-5 instances FR-361 instances. On the Rust side, the error mapping, the `#[implements]` import form and the moved `admitted_types` field are fine, and there is no new panic, unsafe code or unbounded work. The findings were test isolation (FND-001, FND-002) and one mirror route outside the diff (FND-003). The review-pass gate ran on the base commit plus the uncommitted change. Its seven failures matched their tickets (QSL-3360, QSL-3314, QSL-3380, and the liveness budget test, since filed as QSL-3413) and #3961.

Disposition pass 1, at 80764c452a: FND-001, FND-002 and FND-003 are fixed. Two new low findings (FND-004, FND-005) are text-only.

- **Mutation runs.** Each written-binding site was reverted alone on the pre-merge head bdc6188ffc. Every revert fails at least one new test, so `graph_replay.rs` and `certificate/state_graph.rs` are now isolated from each other.
- **Merge commits.** They bring in origin/main only and do not touch the reviewed sites.
- **Gate at 892279a673.** It ran 72 steps and recorded 45 failed results over 44 distinct tests. None is caused by this PR:
  - 39 `monitor_*` failures, all `SourceMapInvariant`. The same set fails on main 3e150e7552 (QSL-3314, introduced by #3845).
  - `certificate_kinds::value_bounds_certificates_are_accepted`, counted twice. It fails at `certificate_kinds.rs:368` with shifted 0.4 and `ExactProbMaxStates` 10, also on main 3e150e7552. This is the remainder QSL-3371 records as QSL-3352, not the `:209` failure in QSL-3371's description.
  - `zone_certificate::the_certificate_members_are_the_reconciled_schema_set`: 64 vs 65 (QSL-3380). The FSL pin move for it is not merged, although the ticket is marked Done.
  - `state_graph::an_undefined_terminal_predicate_is_reported_without_a_claim_predicate` (QSL-3314). It is a `CheckRefusal` from checking the unit (`6 mod (3 - self.value)`), raised before any analysis. This PR changes no checking code, and the same panic was recorded before it.
  - `liveness::the_model_only_pass_spends_the_budget_the_next_instance_gets` (QSL-3413). The same message was recorded on 88eb6bc392, which does not contain this PR. `written_reference` is called only after a counterexample or on evidence, never in exploration or budget accounting.
  - `timed_state_graph::a_state_graph_certificate_over_a_zeno_only_graph_needs_no_divergence_witness` (QSL-3438). Two tests shared the workspace `timed-sg-zeno-only`.
- **Head 80764c452a.** It renames the second workspace to `timed-sg-zeno-certificate`, test-only. No other `Workspace::new` name is duplicated in `fsl-analyze/tests`. `timed_state_graph` passed 14 of 14 in three runs, and the workspace check exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The HP-5 test does not isolate the admitted type at `hyper_route.rs`: `v: M::Lamp` with `l` held as `Lamp` gives the same reference for the declared and the admitted type, so `hyper_route` calling `bound_reference(declared)` would still pass; a parameter typed `M::Thing` (held `Lamp`) would separate them | fsl-analyze/tests/it/model_instances.rs:2456-2474, fsl-analyze/src/analyze/hyper_route.rs:312-318 |
| FND-002 | medium | Nothing tests the admitted type on the timed state-graph route (`state_graph_item::run_timed` producer, `zone_certificate.rs` checker, `graph_replay::graph_over` timed branch) or `items.rs::bound_over` (model item and zone temporal routes). Reverting any one of these sites alone would leave every new test green, and `graph_replay.rs` is not isolated from `certificate/state_graph.rs`: the only revert run (producer-only) reverted the fsl-replay files together | fsl-analyze/src/analyze/state_graph_item.rs:1111-1117, fsl-replay/src/certificate/zone_certificate.rs:4095-4100, fsl-replay/src/model_check/graph_replay.rs:885-909, fsl-analyze/src/analyze/items.rs:203-214 |
| FND-003 | medium | Mirror route not moved: the replay spine's `cross_evaluate` counterexample (`model_trace.rs`, `package_selection.rs`) writes `over_binding` from `letters.over_reference`, the held row whose type conforms to `T`, and drops the counterexample (`over: over?`) when the admitted record does not conform to `T`, where FR-125 "Instancing" writes (`UniverseId`, admitted type, key). It agrees with `written_reference` only when a conforming object is held under the key in the initial state | fsl-replay/src/spine/clause/model_trace.rs:268-330, fsl-replay/src/spine/clause/temporal/package_selection.rs:519-571, fsl-eval/src/simulation/model/letters.rs:619-627 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The committed copies of this review and the gap review in a public repository cite local machine log paths (the review-pass Verdict prose). They also carry a date prefix in their file names, where the convention is `reviews/qsl-3327-<method>.md`. Replace both files with this version, which has no local paths | reviews/2026-11-25-qsl-3327-code-review.md, reviews/2026-11-25-qsl-3327-gap-analysis.md |
| FND-005 | low | `hyper_instances_are_the_keys_of_the_resolved_population_alone` now declares `v: M::Thing`, but its doc comment still reads "`v: M::Lamp` resolves to `P1`, the one population that overlaps `Lamp`" and says `Q` "does not overlap `Lamp`". The assertions are no weaker, because `Animal` overlaps neither type, but the comment is stale | fsl-analyze/tests/it/model_instances.rs:2346-2357 |

## Coverage

- **Review pass.** 10 of 10 production hunks and the 3 new tests examined. Of 14 remaining `bound_reference` and `key_reference` production callers, 8 are probabilistic or protocol (QSL-3126), 1 is a validity check, and 5 are evaluation reads that re-bind by (universe, key).
- **Disposition pass 1.**
  - Fix commits examined: 96117f57b2 (tests), bdc6188ffc (replay-spine binding), 09f4eddcd9 (review files) and 80764c452a (QSL-3438).
  - New tests: `zone_state_graph_evidence_binds_the_admitted_member_type`, `digital_state_graph_evidence_binds_the_admitted_member_type`, `a_zone_temporal_over_binding_names_the_admitted_member_type`, `an_hp5_witness_of_an_unadmitted_key_binds_the_declared_type`, `a_model_counterexample_keeps_a_non_conforming_admitted_binding`.
  - Each of the ten single-site reverts fails at least one new test: hyper_route, items bound_over, letters, run_digital, run_timed, bound_instance, model_check state_graph, certificate state_graph, zone_certificate, graph_replay.
  - The replay-spine test fails before bdc6188ffc and passes after it.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `an_hp5_witness_binds_the_admitted_member_type` declares `v: M::Thing` with `l` held as `Lamp`, so the expected reference (Thing universe, `Lamp`, `l`) differs from both the declared type and the least member type. Reverting `hyper_route.rs` alone fails it |
| FND-002 | fixed | `timed_state_graph.rs` adds zone, digital-clocks and refuted zone-temporal tests over `Derived extends Base`. Reverting run_timed, zone_certificate or graph_replay alone fails `zone_state_graph_evidence_binds_the_admitted_member_type`. Reverting items bound_over fails `a_zone_temporal_over_binding_names_the_admitted_member_type`. Reverting certificate/state_graph alone fails `state_graph_evidence_binds_the_admitted_member_type`, separately from graph_replay |
| FND-003 | fixed | `evaluate_model_trace` now takes `bound_over` from `letters.instance_reference` (the written binding) for every named instance. `over_reference` is removed. The package route returns no decisive record, instead of a fault, where no conforming object is held. `a_model_counterexample_keeps_a_non_conforming_admitted_binding` (an admitted `Published` under `c: Draft`, on the source and package routes) fails before the change and passes after it |
