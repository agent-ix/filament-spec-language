---
id: SR-967175026302197768900474826
title: "Code and Rust review of QSL-2815 AC-4 performance follow-up"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2815; PR #3816 at 5d267c34b1f45bc3e6871e506e1c2870f8fdd99f; 19 changed Rust files; FR-101-AC-2, FR-120-AC-32, FR-851-AC-4"
review_set: subset
date: 2026-11-24
---

## Summary

Reviewed the 19-file Rust diff against `origin/main`, including the state-key memo, product-key and expansion caches, observation-sharing changes, and the three changed trace bindings. The AC-4 test is tagged to the correct requirement and the focused behavior appears sound, but the new product-key cache duplicates canonical-encoding size semantics and the touched Rust files do not pass focused formatting.

## Verdict

**FAIL** — the new cached-key path hardcodes the canonical writer's serialization layout, creating a second source of truth for key-byte limits.

## Scope

- Examined the PR diff at frozen head `5d267c34b1f45bc3e6871e506e1c2870f8fdd99f` against `origin/main` after merge commit `d4ad338ea0f8710721623a01020fc7f231391745`.
- Inspected all 19 changed Rust files, with behavior tracing through `fsl-analyze/src/model_check/liveness.rs`, `fsl-analyze/src/model_check/product.rs`, `fsl-eval/src/simulation/model.rs`, `fsl-eval/src/simulation/model/frames.rs`, `fsl-semantics/src/model/state.rs`, and `fsl-replay/src/certificate/closure.rs`.
- Read the complete changed/relied-on criteria FR-101-AC-2, FR-120-AC-32, and FR-851-AC-4; checked the three modified/new trace bindings.
- No applicable AssuranceProfile was found under `spec/`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `copy_product_key` re-states `PropertyKey`'s canonical JSON layout and writer chunk accounting as hard-coded lengths (`7`, `11`, and manual punctuation). The typed `ProductKey` encoder already owns that encoding. If its representation changes without this helper changing, cache hits enforce a different `CanonicalBytes` limit/error requirement than cache misses. Keep limit accounting in the canonical encoder instead of duplicating its format. | fsl-analyze/src/model_check/liveness.rs:1869-1898 |
| FND-002 | low | The new `ModelSimState.canonical_key` field does not match rustfmt output, which wraps it on one line. | fsl-eval/src/simulation/model.rs:263 |
| FND-003 | low | The changed `observation_facts` import order does not match rustfmt output. | fsl-semantics/src/model/state.rs:28 |

## Method Status

- `code-review` with Rust lane: completed. The cache keys preserve full `StateKey` bytes; the expansion cache is local to one binding/pass, and cached expansion results preserve their bounded result and cut. The `model_state_key_memo_tracks_clones_and_cell_mutation` test exercises sharing, mutation detachment, and unchanged original state. `FR-120-AC-32`'s added test exercises undefined/incomplete equation behavior under the narrowed path.
- Trace review: the changed test binds to FR-120-AC-32; the memo regression binds to FR-101-AC-2; the AC-4 rover test binds to FR-851-AC-4.
- Focused rustfmt check: failed. `rustfmt --edition 2021 --config skip_children=true --check` over the changed Rust files reports numerous pre-existing formatting differences in the merged `fsl-eval/src/simulation/model.rs`; it also reports the two changed formatting sites in FND-002 and FND-003 and FND-003. The narrower liveness and bounded-successors check passed on the merged head according to the lead's focused check.
- Cargo checks/tests: not run by this reviewer because the shared build scheduler was not visible here. The lead's delegated merged-head focused cold-clone regression passed 1/1; AC-4's five-pass series on the preceding head measured 26.4 seconds median on draco (load ~20). No aggregate workspace gate was run.
- `cargo deny check` and workspace Clippy were not run; neither is an aggregate workspace gate requested for this review.

## Dispositions

Not applicable in this review pass.
