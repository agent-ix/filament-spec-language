---
id: SR-96745
title: "Code review: object-set cross-route comparison test"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/object_sets.rs, fsl-analyze/tests/it/model_instances.rs, fsl-analyze/tests/it/model_check_binary.rs"
review_set: subset
---

## Summary

Ticket: QSL-3512. PR agent-ix/filament-spec-language#4063 at a15cf1752bf3c00c2687be92123ee401bf6f22d8, base main (merge base 499e6c5d1d). Test-only change; code-review with the rust-review lane. Expected values were compared with FR-125, FR-172, FR-201, FR-315, FR-896, FR-1012 and FSpec FR-361, and with the arch ruling of this review (a subject with an initial state and no admitted instance is NoInstance on every route; FR-315 refusals follow FR-125 order: RequiresBound, NoInitialState, conflicting-identity, NoInstance). Gap analysis was not run: the diff has no production code. No test result was obtained by this review: a run of the 18 ignored tests stayed queued for a build lane and was cancelled, so ignore causes and findings rest on reading.

## Verdict

**FAIL**: three high findings. An SMT row expects the old refusal order, the component checker column is never driven, and a shared unit-header helper is copied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | smt_temporal_refuses_in_its_table_order expects the old FR-315 order: ONLY_UNADMITTED gives ObjectLifecycle and NO_UNIVERSE gives NoInitialState. The ruling gives NoInstance and RequiresBound. Current behaviour is encoded as expected in a passing test; both rows must be ignored rows with the FR-125 cause and a ticket | fsl-analyze/tests/it/object_sets.rs:1514 |
| FND-002 | high | check_components is never executed. check_certificates("eventually", CAUSES) gets no certificate and falls to `None => check_closure`, and "eventually" is never run over INSTANCES. The component checker column reports 8 ok rows for a checker that is not called, and the test of that name repeats the closure checker | fsl-analyze/tests/it/object_sets.rs:849 |
| FND-003 | high | untimed_unit is a second copy of the shared counter::unit_text_in header (language, profile v, profile t0, model M, function Noop), with one extra profiles argument; counter is already imported. Extend the shared helper | fsl-analyze/tests/it/object_sets.rs:436 |
| FND-004 | medium | Statistical rows for empty universe, only unadmitted keys and Orphan with an initial state are omitted (footnote 4). The ruling fixes NoInstance, so each must be an ignored row naming a ticket; none is filed | fsl-analyze/tests/it/object_sets.rs:1459 |
| FND-005 | medium | The no-universe rows of en-1 digital and the timed state graph are dropped because they fail established-invariant-broken today (footnote 8). The correct value RequiresBound is known; no ignored row, no ticket | fsl-analyze/tests/it/object_sets.rs:226 |
| FND-006 | medium | Two further observed wrong behaviours are removed from the assertions instead of recorded red: the liveness final proved probe (probes.pop, footnote 1) and the component certificate of an eventually claim over instances (footnote 2). Neither has a ticket | fsl-analyze/tests/it/object_sets.rs:557 |
| FND-007 | medium | Global conformance rows over subtype-only, created, unadmitted-across and two-population fixtures are omitted as "no ticket" (footnote 10). FR-896 requires every admitted key, and QSL-3507 (PR 4062) owns the first-key defect. They must be ignored rows naming QSL-3507 | fsl-analyze/tests/it/object_sets.rs:1480 |
| FND-008 | medium | Cells are "-" with no footnote and no assertion: exact over empty initial, empty universe with empty initial, and Orphan with empty initial (NoInitialState is required), and the timed state graph over a key listed by two populations | fsl-analyze/tests/it/object_sets.rs:1447 |
| FND-009 | medium | closure_checker_accepts_the_certificate_over_the_instances asserts only that some certificate is accepted for `x.age <= n`. A producer and checker that share an instance set missing an instance or in the wrong order still pass, yet the test is tagged FR-125-AC-8, AC-10, AC-14 and the table shows ok | fsl-analyze/tests/it/object_sets.rs:1202 |
| FND-010 | medium | For the conflicting-identity case check_certificates stops at the ModelSystem::new error and never calls a checker (and check_state_graph_library likewise never reaches the graph checker), while the table shows closure and component checker ok | fsl-analyze/tests/it/object_sets.rs:802 |
| FND-011 | medium | liveness_binds_the_instances_in_order drops the final probe, so an extra instance after the expected ones (the excluded key z of CREATED) passes. The FR-125-AC-14 and exact-set AC-8/AC-10 tags claim more than is asserted | fsl-analyze/tests/it/object_sets.rs:1164 |
| FND-012 | medium | Plan probes `x.age <= i` turn infeasible whenever the highest-aged (last) instance is monitored, so a search that monitors only the last key passes every probe. "Monitors each admitted key" (FR-1012-AC-8) is not asserted | fsl-analyze/tests/it/object_sets.rs:988 |
| FND-013 | medium | hyper_product_checker_accepts_nothing_over_no_instance passes an empty `instance` for a clause with one parameter, which is refused Input("instance") for every population. It cannot fail for the fixtures and is tagged FR-125-AC-21 | fsl-analyze/tests/it/object_sets.rs:1120 |
| FND-014 | medium | hyper_smt_takes_only_instances_of_several_populations expects Ok for TWO_POPULATIONS, but SymbolicSubject refuses ConstantName for a key held by two populations, which smt_temporal asserts for the same subject. The row will still fail after QSL-3408 | fsl-analyze/tests/it/object_sets.rs:1355 |
| FND-015 | medium | Package-form routes and the hyper replay and settlement checkers are neither driven nor listed under "Not driven"; only source routes are compared, although source and package are paired routes | fsl-analyze/tests/it/object_sets.rs:1 |
| FND-016 | medium | exact_is_unsupported_over_no_instance expects unsupported (FR-201-AC-30) where the ruling says NoInstance on every route, and asserts no cause, so any unsupported reason passes. Needs an arch answer on whether the ruling covers the exact route and FR-201-AC-30 | fsl-analyze/tests/it/object_sets.rs:510 |

## Examined and clean

model_instances.rs only widens visibility of existing fixture helpers, and object_sets.rs reuses them. model_check_binary.rs only registers the module. No production code changes. The temporal, state graph, state graph library (instance cases), HP-3/4/6, zone, digital and timed state graph instance-set expectations match the hand-written FR-125 values.
