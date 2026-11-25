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
