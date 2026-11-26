---
id: SR-20261126014317001
title: "code review of PR 4017 admitted-type instance bindings"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #4017 head 859955ed3db61f5784b4ef741e30c3d39a992dcb; fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/measure.rs, fsl-analyze/src/exact_probabilistic.rs, fsl-analyze/src/exact_probabilistic/build.rs, fsl-analyze/src/exact_probabilistic/digital.rs, fsl-analyze/src/statistical/model.rs, fsl-eval/src/model_subject.rs, fsl-eval/src/simulation/protocol.rs, fsl-eval/src/simulation/protocol/wire.rs, fsl-replay/src/lib.rs, fsl-replay/src/model_check/settle.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document/instances.rs, fsl-replay/src/probability_certificate/document/measure.rs, fsl-replay/src/proof_result.rs, and the tests the PR adds"
review_set: subset
---

## Summary

Ticket: QSL-3126. Code review with the Rust review lane folded in. The PR moves the probabilistic (exact, digital, statistical, certificate checker) and protocol `over` bindings from `ModelSystem::bound_reference` (declared type) to `ModelSystem::written_reference` (admitted member record's type), changes `ExactEntry.binding`/`.instance` to the typed reference, removes `ExactBinding`, and drops the population from the zero-delay cycle state identity. Governing text: FR-125 "Instancing" and FR-125-AC-9; FR-741 "Binding"; FR-364 reference equality; FR-208 replicated roles.

Checked: every remaining `bound_reference` caller in production code (four), the `instances_with_meter` route, the footprint expansion, the wire shape against the consumer's committed expected files, the trace and `implements` tags, and the coder's "cannot be written" claim for the digital route.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Replicated role objects are still written with the role's declared type (`role_universes` calls `bound_reference`), while the protocol `over` value is now written with the admitted type. Reference equality compares the whole (universe, type, key) (FR-364; FR-125 "Instancing": `x = r` treats the bound object as any reference to it), so with `Node extends Base`, `protocol ... over (view: M::Base)` and `role w each M::Base from nodes` over the `Node` `node-1`, `w = view` was true before this PR and is false after it. This is the paired code of the `over` binding, it is not listed in the PR's paired-code table, and its sweep row says "unchanged, not tested" | fsl-eval/src/simulation/protocol.rs:1945-1966; fsl-eval/src/simulation/protocol.rs:1683; fsl-eval/src/simulation/protocol.rs:1875; fsl-eval/src/simulation/protocol/roles.rs:263-283 |
| FND-002 | medium | The claim that no digital route subtype test can be written is not supported. The `ambiguous-name` refusal comes from clock ownership resolution and fires only when two distinct original declarations claim one clock member. The existing `settle_over_base` harness already runs a timed subject with a unit-declared `Derived extends Base`, and the zone and digital-clocks state-graph tests use it. The `written_reference` change in `digital.rs` therefore has no test with a subtype | fsl-analyze/src/exact_probabilistic/digital.rs:103-108; fsl-analyze/tests/it/timed_state_graph.rs:441-560; fsl-semantics/src/model/clock_reference/normalization.rs:684-696 |
| FND-003 | medium | The PR's same-shape sweep omits `compute_universe_declaring`. It writes footprint objects with the type that declares the field (`bound_reference`), and footprint admission compares `Location::Field` objects by whole reference (`location == read`). A field read through a binder that now holds an admitted-type reference may not be covered by those expanded locations. The PR must list this site with a result, and either show that the admitted-type reference is covered or change it | fsl-eval/src/simulation/model.rs:2896-2913; fsl-eval/src/simulation/model/footprints.rs:257-272; fsl-eval/src/simulation/model/footprints.rs:665-680 |

## Verdict

Request changes on head 859955ed3db61f5784b4ef741e30c3d39a992dcb (FND-001 high).

FND-001 must be fixed in this PR. The fix is `written_reference` in `role_universes`. Both constructors (source `new`, package `new_package_with_loop_proof`) call it, so the test must cover both routes with a hand-written expected `object_type` for the spawned role object and a `w = view` predicate that holds.

FND-002 must be fixed in this PR: a digital probabilistic claim `over (c: Base)` through the `settle_over_base` harness, with the expected `Derived` type written by hand. If that harness refuses the claim, report the exact refusal and the input that produced it.

FND-003 needs a stated result in the PR: changed and tested, or shown to be covered.

What is right:
- The exact producer and the certificate checker share `claim_binding`, and the checker's `instances_of` derives the `on each` rows independently. Both changed.
- `ExactEntry` serializes through the same canonical `value_json` reference spelling as `counterexample.binding`.
- The new tests assert hand-written types (`ix://test/link/Msg`, `ix://test/fill/Cell`) and the member set, never values copied from output.
- The bounded translation (`model_check/item.rs`) and the FT-3 footprint helper still call `bound_reference`. The PR's own sweep table accounts for the first; the second is FND-003.
- The wire listing is correct. Exactly 15 committed expected files in the consumer carry `entries[].binding` as `{population, key}`, and these are the 15 the PR body names. The rover monitor `entries` carry a different binding shape that this PR does not change. The follow-up ticket exists.
- The public type `ExactBinding` is removed; no other repository references it.
- Trace and `implements` tags: the seven new tests carry `TC-520` and `FR-125-AC-9`. Each `implements` marker uses the bare attribute with `use ix_trace_rs::implements;`, and `quire trace` lists all eleven new markers, `written_binding` included.

Not a finding for this PR, judged:
- `ModelSystem::instances_with_meter` neither de-duplicates by key nor filters to admitted keys, unlike `instances`. That is FR-125-AC-10 and FR-125-AC-8 instancing on the FR-1012 synthesis policy route (`plan_source.rs`). Its contexts already carry the key alone, the shape this ticket fixes does not occur there, and this PR does not touch it. It is outside this ticket's scope. A failing input would be a synthesis `satisfies` clause `over (x: Thing)` whose overlapping populations list `[d, u]` and `[u, a]`, with `u` held by no initial record: `instances` gives `d, a`, while the metered list gives `d, u, u, a`. This was not run. Per intake, it needs a failing test before a ticket is filed (Must-fix: yes if it fails, as a wrong answer).

Tests: fsl-replay lib `execute::` and `fallback::` with `--skip 100000` on this head: 220 passed, 0 failed, 5 ignored (STD-652, already on main), 1563 filtered out. The coder's earlier logs on this head and on code-equivalent heads were not rerun.
