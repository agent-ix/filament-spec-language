---
id: SR-2026112601100001
title: "Code review — filament-spec-language PR #3997 unit bounds and observation loci"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3997 head 48a91b77c05386368d69af981f31493ca6709f5d; fsl-semantics/src/model/observation.rs, fsl-semantics/src/model/observation/document.rs, fsl-replay/src/outcome.rs, tests and spec rows"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-106", type: references }
---

## Summary

Tickets: QSL-3437 (Closes) and QSL-3356 (Refs, partial). Reviewed head `48a91b77c05386368d69af981f31493ca6709f5d`. Rust-review lane folded in. FSpec FR-154 "Unit bounds" read at filament-specification origin/main `0d3d4ca8`.

The invocation paths the PR names (PreCall, Invocation pre/post, Context post, Frame) now enforce parameter and result bounds, with literal-value tests. Two further admission paths still admit a value beyond a unit bound, both proved by failing probes at this head (log `gates-lead-reqs/qsl-3437-review-probe.log`, probe source `gates-lead-reqs/qsl-3437-review-probe-machine-unit-bounds.rs`).

## Verdict

**FAIL** for QSL-3437: it cannot be closed while FND-001 and FND-002 stand. The QSL-3356 locus changes are sound: the producer (`fsl-semantics`) and consumer (`fsl-replay` `snapshot_admission_locus`) are both changed and tested with hand-written pointers and members. The three merge-check reds are QSL-3402's: same three tests and values as the ticket; on a trial merge with main `dc71332c3d` (#4004) they pass along with the PR's edited suites (286 passed, 0 failed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A state-machine (context) field typed by a bounded unit is admitted beyond its bound on `Context` pre and post: `member_bounds` reads field bounds only for object-type owners and `admit_contexts` checks no unit bound | fsl-semantics/src/model/state.rs:199-206 |
| FND-002 | high | A `min`/`max` unit constraint declared on a `state_machine` type is accepted and dropped: a `4 ms` argument and result under it admit | fsl-semantics/src/model/intake.rs:3797 |
| FND-003 | medium | Sibling-route sweep for the same cause is incomplete: context fields, the fsl-replay call route and the simulation are not listed with a result | fsl-replay/src/spine/call.rs:1426-1434 |
| FND-004 | low | The value-invariant selection is claimed fixed but has no test | fsl-semantics/src/model/observation.rs:3705 |
