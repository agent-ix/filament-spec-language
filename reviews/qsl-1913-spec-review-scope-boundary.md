---
id: SR-2026111414035505
title: "scope-boundary review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/test-cases/TC-1417-worker-count-hash-indexed-seen-set-and-packed-state-leave-results-unchanged.md; spec/test-cases/TC-1418-a-directly-solved-assignment-keeps-the-successors-and-counts-its-candidates.md; qsl-bench/BENCHMARKING.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. This review checks where each rule is
allocated and what each test has to reach. The worker rules sit in the
right places: FR-101 for the engine, FR-126 by reference, and FR-191 for
sampling. FR-126 builds its product on FR-101 state keys, so FR-126 inherits
the seen-set rule by reference.

The hash and pack ACs are bound to FR-101's generic `explore_request`, which
has no checked types and no hasher input. QSL-1833 and QSL-1837 change the
FR-126 explorers in qsl-analyze. The `mc/frame_candidates` axis does not
exist in qsl-bench yet. Its benches are parser, checker, cst, model,
evaluator and text_cluster, and QSL-1906 owns the `mc/*` axes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The packed layout is "derived from the checked types of the system's state", but FR-101's `TransitionSystem` exposes no checked types (`State` is opaque and `key` is `Serialize`). FR-101-AC-21 and AC-22 use `pack`, `unpack` and "with a packed layout and without one" without naming any API or switch. TC-1417 says tests go "through the public entries", which cannot reach them. QSL-1837 packs states in the FR-126 explorers, and no FR-126 criterion lets it tag that work. | FR-101:278-289, FR-101-AC-21, FR-101-AC-22, TC-1417 | missing-requirement |
| FND-002 | medium | FR-101-AC-20 needs "a test hash function that returns one constant", but `explore_request` has no hasher parameter and the spec names no seam. Through public entries the AC cannot be run. QSL-1833's code is in check.rs, graph.rs, refine.rs and hyper/single.rs, not FR-101's explorer (which already hashes). It has no criterion that covers its own explorers. | FR-101-AC-20, TC-1417 | missing-requirement |
| FND-003 | low | FR-120-AC-33 and TC-1418 step 5 describe the `mc/frame_candidates` axis in the present tense ("The ... axis of `qsl-bench` builds"), but it does not exist. QSL-1906 creates it. TC-1418 says its tests are in `qsl-eval/tests/it/`, while step 5 is a qsl-bench smallest-size test. "Generates 1 and 100 candidates" also needs a candidate counter that no output exposes. Name the owner, the location and the observable (for example the smallest `max_candidates` that completes). | FR-120-AC-33, TC-1418 | wrong-requirement |
| FND-004 | low | The solved-assignment rule says it is "the one rule for every generator", but does not say whether it covers other generators that share the subject's `max_candidates`. FR-141's hidden-field candidates follow QSpec FR-378, and FR-142 and FR-220 also expand under that limit. | FR-120:713-723, FR-141:222-232 | missing-requirement |

## Verdict

Mergeable on these points once FND-001 and FND-002 have a named seam (a
pack or hash strategy parameter or test-support hook) or an FR-126 criterion.
FND-003 and FND-004 are wording fixes.

## New findings (disposition pass 1)

Checked at 6d3ffe68e.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-101-AC-22 covers `explore_request` and `explore_request_parallel`. Those take a generic `TransitionSystem` with no checked types, so no `PackedLayout` exists for them, and `Packing::Auto` and `Off` are the same run. The AC can only fail on the ModelSystem-based FR-126 and FR-142 explorers. Restrict it to explorers that can derive a layout, or say how a generic system offers one. | FR-101-AC-22, FR-101:313-323 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-101 names a test-support seam: `SeenSetProbe { hash, packing }` and `PackedLayout` with `pack`/`unpack`, never in a request. AC-20 and AC-22 cover `explore_request`, `explore_request_parallel` and the FR-126 and FR-142 product explorers, so QSL-1833 and QSL-1837 have criteria to tag. The seam is consistent with the result-preserving claim, and AC-20 (constant hash, 100 states) and AC-22 can fail. See FND-005 for the generic-system case. |
| FND-002 | fixed | AC-20 now runs through `SeenSetProbe.hash` on each explorer. |
| FND-003 | fixed | AC-33 names QSL-1906 as the axis's creator and says it does not exist before then. TC-1418 places steps 1 to 4 in qsl-eval/tests/it and step 5 in qsl-bench. The observable is the smallest completing `max_candidates`, found by bisection. The charge values are wrong (base FND-007). |
| FND-004 | fixed | FR-120 says the rule covers expansions through `ModelSystem`, FR-120's own and the FR-126 and FR-142 explorers, and excludes FR-141 (QSpec FR-378), FR-220 and FR-315. Checked against the code: FR-126's `check.rs` builds `ModelSystem::new` (check.rs:529, 697) and expands through `successors_bounded` (default `successors`, subject.rs:205) or `successors_enforced` (footprints.rs:787). Both reach `walk::expand_state`, which charges `max_candidates` (model.rs:1608, walk.rs:389). FR-142's `refine.rs` expands the concrete `ModelSystem` through `successors` (refine.rs:1865) under `simulation.expansion.max_candidates` (refine.rs:247). |
