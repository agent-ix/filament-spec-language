---
id: SR-967175026302197768900474181941935084808
title: "Code review — QSL-2959 composite nested-reference lift"
type: SpecReview
analysis: code-review
scope: "QSL-2959 diff from origin/main, fsl-semantics, fsl-replay, fsl-eval, fsl-analyze, tests/support"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Static review of FSL PR #3846 at `58888086c035b728b637edeebd88ef00e064d323` covered the composite lifting implementation, its tests, and the current owning criteria. It found one type-checking bypass for unmapped nested references and one missing end-to-end proof case; cargo gates were not run because the build guard had stopped a build.

## Verdict

**CONDITIONAL** — fix the nested-reference type-check bypass and cover tuple/union refinement settlement before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `composite_conforms_lifted` returns immediately for nominally identical or inherited record/tuple types, bypassing recursive reference-lifting checks. A shared `Box { target: Reference<Ghost> }` declaration used on both sides can therefore produce a `CheckedRefinement` without any object row for `Ghost`; `map_state` later reaches `lift_reference` and returns `Lifting::Unmapped`. Recurse through the composite even when nominal conformance holds, and refuse the row when a nested target is not mapped. | fsl-semantics/src/check/refinement.rs:2274; fsl-replay/src/refinement/map.rs:602; FR-135:171-186 |
| FND-002 | medium | The analyzer-level certificate test covers `Box` and `Parcel`, but not the Pair tuple or Holder union. FR-630-AC-8 and AC-9 therefore have no tagged end-to-end test proving the checked refinement item settles certified with the lifted tuple/union value. | fsl-analyze/tests/it/refinement_replay.rs:729; FR-630-AC-8; FR-630-AC-9 |

## New findings (disposition pass 1)

Reviewed head `a2d980afb9a2f31f5e3a60c43986f0a1c1d43c56` (fix `58888086c0..a2d980afb9`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A reference outside any composite, at top level or inside an option, collection or map, still passes with no object row when both sides share its object type. `lifted_mismatch` keeps the unmapped reference when `component` is false, and `lift` leaves unmapped references unchanged, so `Reference<Ghost>` (or `Sequence<Reference<Ghost>>`) conforms to itself under `abstract Q concrete Q` with no `Ghost` row. FR-135 says a `Reference<T_A>` field accepts `Reference<T_C>` "exactly when an object row maps `T_A` from `T_C`", and FR-135-AC-6 refuses an unmapped nested reference "as a top-level one does". Replay `lift_reference` returns `Lifting::Unmapped` for the same value. This is the FND-001 defect class (a `CheckedRefinement` that replay cannot lift), so it belongs in this PR. The PR body lists it as not covered. Fix: refuse an unmapped reference wherever it occurs, not only inside composites, and add the top-level and carrier cases to the shared-declaration test. | fsl-semantics/src/check/refinement.rs:2283-2290; fsl-semantics/src/check/refinement.rs:2415-2418; fsl-semantics/src/check/refinement.rs:3507-3514; fsl-replay/src/refinement/map.rs:774-793; FR-135:171-186; FR-135-AC-6 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `composite_conforms_lifted` and its nominal short-circuit were removed. `LiftedPairing::composite_mismatch` walks every composite pair, shared ones included, and inside a composite (`component = true`) an unmapped reference fails. The test `a_shared_composite_holding_an_unmapped_reference_is_refused` covers the shared `Box`, `Pair` and `Holder` with `Reference<Ghost>` and refuses at `box.target`, `[0]` and `holder.variant[0]`. It is red on the pre-fix checker and green on the head. |
| FND-002 | deferred | QSL-3263 (blocks QSL-2959). Pair and Holder analyzer settlement needs snapshot tuple and payload-union carriers. On origin/main d041ae422a, `observation/tree.rs` has no `tuple` arm and `admit_union_case` admits only enum (nullary) cases, so this is not a test-only fix. |
| FND-003 | fixed | Round 2, head `f04c86bb5cf67a5227c0061c74d57cf948478396`. The `component` flag is gone, and `LiftedPairing::mismatch` accepts a reference only through `self.lifting`, so an unmapped reference inside a composite fails at every depth. Non-composite rows now check the unlifted declared type with `references_mapped` (refinement.rs:2403-2431) and refuse `ill_typed`/`type-mismatch` at the expression (refinement.rs:3717-3724). This matches FR-135 lines 171-173 and FR-135-AC-3/AC-6. The step receiver and step-argument expressions go through the same `check_row` (FR-136 "checked as FR-135 checks a field row"). Node arguments and result binding go through `conforms_lifted`, which uses the same walk. `a_shared_reference_without_an_object_row_is_refused` covers top level, option, collection, map, record, tuple and union rows, with the mapped `Queue` admitted and the unmapped `Ghost` refused. |
