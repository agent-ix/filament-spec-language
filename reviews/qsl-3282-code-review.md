---
id: SR-20261126055136001
title: "code review of PR 4039 bind abstraction remembered emission"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #4039 head 5bb1c4475ad785773df85a9fe765e6075fd955b4; fsl-package/src/checked.rs, fsl-package/src/emit.rs, fsl-package/src/emit/tests/abstraction.rs, fsl-package/src/export/tests.rs, and the check-time relation consumers fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/guarantee.rs, fsl-semantics/src/check/lowering/hyper.rs, fsl-semantics/src/check/abstraction.rs"
review_set: subset
---

## Summary

Ticket: QSL-3282. Code review with the Rust review lane folded in. The PR makes `CheckedPackage::bind_abstraction_at` drop the remembered emission on success, routes `bind_abstraction` through it, replaces the test that asserted the defect with two tests that write their expected occurrences by hand, renames `emitted_package_id` to `emitted_package_name` in TC-1111, and adds `#[implements]` tags for NFR-015-AC-9 and AC-10 in a separate commit.

Checked: both binding methods (each tested); the refused-binding path (it returns before anything changes); every reader of the remembered emission (`emit_checked`, `emit.rs:5515`, `checked.rs:740`, `checked_v2.rs:1449` and `:1620`); every writer (`emit_checked_with_cancel`, `emit_dependency_with_temporal_selections_and_cancel`); and every check-time consumer of `CheckedGraph::abstraction` (guarantees, step relations, compositions). The lead's arch ruling (fix the stale guarantee verdict in this PR) is taken as given.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | After a successful rebind, the guarantee item verdict held in `CheckedGraph` is stale. `check_declared_guarantees` computes `refusal` (`unbound_elements`) once, at check time, against the source-declared relation. `bind_abstraction_at` replaces only `CheckedPackage::abstraction`. A rebind without the `Counter::inc` frame therefore still reports `item_refusal() == None` and emits, which contradicts FSpec FR-490-AC-6. Failing test: `3282-guarantee-stale-test.diff`, log `3282-guarantee-stale.log`, head f805ade0f9 plus the fix. Per the arch ruling it is fixed in this PR | fsl-package/src/checked.rs:646-659; fsl-semantics/src/check/guarantee.rs:1215-1245; fsl-semantics/src/check/mod.rs:8851-8865 |
| FND-002 | high | Two more check-time results derive from the relation and stay stale after a rebind (read only, not run). (a) Step relations: `finish_step_relations` computes `bound_implementations` from `graph.abstraction`, and for each bound execution it writes an `implementation` reference to the relation node key. A rebind that adds or drops a frame leaves executions claiming, or missing, an implementation. (b) The relation node key is derived from the relation's targets (`AbstractionTerm::targets`, the object types and populations), so a rebind that changes the bound object types or populations changes the key. The guarantee's `relation` field and every step-relation `implementation` reference then name a key the emitter no longer writes. Compositions do not read the relation | fsl-semantics/src/check/mod.rs:8938-8990; fsl-semantics/src/check/lowering/hyper.rs:738-810; fsl-semantics/src/check/guarantee.rs:1218-1227; fsl-semantics/src/check/abstraction.rs:1270-1283 |
| FND-003 | medium | `remember_emission` keeps the first emission (`OnceLock::set`, result ignored). `emit_dependency_with_temporal_selections_and_cancel` says that a later `emit_checked` returns the emission made under the request's clock selections. For a package that `emit_checked` has already emitted, the selection emission is returned to the caller but not remembered, so a later `emit_checked`, and the import and link readers of the remembered emission, return the older bytes made without the selections. This is the same cache as this PR and is reachable through the public API, but it is absent from the sweep table. Confirmed by reading; no failing test with selections that change the bytes | fsl-package/src/checked.rs:670-672; fsl-package/src/emit.rs:5789-5804; fsl-command/src/command/monitor.rs:838-854 |
| FND-004 | medium | The PR body says "no requirement says what a rebind does to it" about the stale guarantee verdict. FSpec FR-490-AC-6 (a subject operation with no frame binding refuses only the guarantee item; the other items are derived unchanged) and FR-306 "Rebinding is a new package value" both own it. The spec search the rules require was not done before the claim | PR #4039 body "Not fixed here" item 1; spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md:55-64 |

## Verdict

Request changes on head 5bb1c4475ad785773df85a9fe765e6075fd955b4.

FND-001 and FND-002 are fixed in this PR, together. Recompute every relation-derived check-time result in the bind, with the same functions check uses: the guarantee `refusal` and `relation` key, and the step-relation `implementation` references and `bound_implementations`. Tests use hand-written values for both routes. Bind, then compare the item refusal, the relation key and the step-relation implementation with a fresh check of a unit that declares the rebound relation. Cover a rebind that drops a frame and one that changes the target set. The graph is shared through `shared_graph()` (an `Arc` that importers hold), so the recomputed results must not mutate a graph another package already reads.

FND-003: add a failing test (emit_checked, then emit_dependency_with_temporal_selections_and_cancel with a selection that changes the bytes, then emit_checked), then fix it in this PR or list it with its result in the sweep table.

FND-004: correct the PR text.

What is right:
- One binding path. `bind_abstraction` delegates to `bind_abstraction_at`, and the reset is inside the `std` gate where the `OnceLock` exists.
- The refused binding returns before `abstraction` or `emission` changes. The AC-10 test checks this, and that the kept emission still has no relation node.
- The new tests write their expected occurrences (`[{"role":"declaration","ordinal":0}]`, empty) by hand. Both fail without the fix (`3282-red.log`: 7 passed, 2 failed).
- `use ix_trace_rs::implements;` with the bare attribute. Tags are in a separate commit (9d58af5237). No new interface type is added.
- The no-std check and clippy for `fsl-package` pass on the final head.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The latest emission now replaces the remembered one, but a dependent's remembered emission is not dropped. An importer emitted earlier keeps the `dependency_clock_selections` it read from the dependency's first emission. After the caller re-emits the dependency (an `Arc` it kept from `Import`) under other selections, `emit_checked` of the importer returns bytes that name the first selections, while the closure readers (`checked_v2.rs` closure read, `checked_handoff::acquire_evidence`) hand out the dependency's second emission. Under first-wins the closure stayed consistent. Read only; this is a public-API path, not a shipped one (the lifecycle emits each library once per request, before its importer links) | fsl-package/src/checked.rs:677-682; fsl-package/src/emit.rs:5509-5530; fsl-package/src/checked_v2.rs:1442-1456 |
| FND-006 | medium | `with_abstraction` maps every `finish_step_relations` refusal to one `AbstractionRefusal::Internal("step-relations-derive-for-a-bound-relation")`. One of them is an `abstraction_nodes` refusal that user input causes: a frame bound to a Rust path with an empty segment (`update::`) gives `EmptyRustSegment`, which `check_abstraction` does not refuse. Before this PR such a bind succeeded and emission refused with the typed `EmitRefusal::Abstraction(EmptyRustSegment)`. Now a package with a step relation over that frame refuses the bind as an internal fault and loses the cause, and a package with a guarantee refuses it as `Guarantee(RelationNode)`. The fresh-check route has the same InternalFault mapping (mod.rs:9036). Read only, not run | fsl-semantics/src/check/mod.rs:8939-8962; fsl-semantics/src/check/mod.rs:9020-9040; fsl-semantics/src/check/abstraction.rs:1176-1180 |
| FND-007 | low | FR-306 prose and TC-804 step 3 say a rebind derives the step relation's occurrences again, but no test compares the bound graph's occurrences (or the emitted node's `occurrences`) with a fresh check. `retract_step_relations` removes every occurrence of the claim key (`OccurrenceMap::remove`), not only the two `finish_step_relations` records, so a role recorded earlier under that key would vanish without a test noticing | fsl-semantics/src/check/mod.rs:8966-8978; fsl-package/src/emit/tests/step_relation.rs |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 42c5b1f808: `bind_abstraction_at` installs `CheckedGraph::with_abstraction`, which re-runs `check_declared_guarantees`; `a_rebind_that_drops_a_frame_refuses_the_guarantee_item` asserts `missing_declaration`/`missing-name`, equality with a fresh check, and the shared graph unchanged. |
| FND-002 | fixed | 42c5b1f808: `with_abstraction` retracts and re-finishes the step relations with `finish_step_relations`, and the guarantee relation key is derived again. Tests cover dropping a frame, adding one, a first binding and a changed target set; implementations reference the new relation node_id. The claim key is `declared_node_key` of the declared name (lowering.rs:1297-1312). Requirement records are re-ranked, the emission is dropped, and `occurrences_by_location` is rebuilt; no other cache is keyed on it. |
| FND-003 | fixed | 42c5b1f808: `RwLock<Option<Arc<Emission>>>`, and the latest `remember_emission` wins; `a_later_dependency_emission_replaces_the_remembered_one` asserts that the bytes differ and `authority-second`. The dependent consequence is FND-005. |
| FND-004 | fixed | The PR body now cites FSpec FR-490-AC-6 and FR-306 "Rebinding is a new package value". |
