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

**PASS** — all three findings are fixed at the disposition head; the remaining AC-4 timing gate needs evidence on this exact head.

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
- Focused rustfmt check: failed. `rustfmt --edition 2021 --config skip_children=true --check` over the changed Rust files reports numerous pre-existing formatting differences in the merged `fsl-eval/src/simulation/model.rs`; it also reports the two changed formatting sites in FND-002 and FND-003. The narrower liveness and bounded-successors check passed on the merged head according to the lead's focused check.
- Cargo checks/tests: not run by this reviewer because the shared build scheduler was not visible here. The lead's delegated merged-head focused cold-clone regression passed 1/1; AC-4's five-pass series on the preceding head measured 26.4 seconds median on draco (load ~20). No aggregate workspace gate was run.
- `cargo deny check` and workspace Clippy were not run; neither is an aggregate workspace gate requested for this review.




## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Product-key cache keys now include the exact caller byte limit, so a cache hit can only reuse bytes already admitted by the canonical encoder under that same limit; the manual layout/chunk accounting helper was removed. |
| FND-002 | fixed | The canonical key field now matches rustfmt's one-line form. |
| FND-003 | fixed | The observation imports now match rustfmt order. |
| FND-001 | fixed | Verified again at a43079bd: cache key includes caller input_bytes and cache misses use ProductKey::canonical_bytes_with_model_value with that same limit. |
| FND-002 | fixed | Verified again at a43079bd: ModelSimState.canonical_key remains on rustfmt’s one-line form. |
| FND-003 | fixed | Verified again at a43079bd: observation_facts imports remain in rustfmt order. |

### Round 1 evidence

| FND | Outcome | After excerpt |
| --- | --- | --- |
| FND-001 | fixed | `let key = (state.model_key.clone(), property, identity.input_bytes);` and cache misses call `canonical_bytes_with_model_value(&model, property, identity.input_bytes)`. |
| FND-002 | fixed | `canonical_key: std::sync::Arc<std::sync::OnceLock<std::sync::Arc<crate::simulation::StateKey>>>,` |
| FND-003 | fixed | `admit_initial_state, model_views, observation_facts, observe_state, state_fields, state_reference,` |

The former FR-120-AC-32 tag on `a_solved_size_integer_assignment_uses_its_domain_index` is removed; that test now covers the out-of-domain assignment case without claiming the temporal-query acceptance criterion. The two scenario tests for defined and undefined pre-state queries remain tagged to FR-120-AC-32. The memo regression now compares the mutated clone's key with a fresh changed state and checks that memo population leaves `Debug` output stable.

No new code defect was found in the capped FIFO caches, caller-limit keyed product cache, or expansion-payload cache removal. The current PR head has no recorded FR-851-AC-4 test-profile timing result in the issue/CI evidence I inspected; existing evidence is from earlier heads. Merge readiness therefore remains pending a current-head AC-4 measurement, separate from the resolved code review findings.

### Round 2 evidence

At `a43079bd8662cda4adf9ac281ee1cff0a25186a8`, the earlier code-review findings remain fixed. The new prefetch-window test counts expansions across overlapping windows and asserts only the five unique states were expanded; it also checks the retained map remains bounded by the worker count. The test passed 1/1. The locked workspace check passed at this head. The separate unchanged FR-851-AC-4 regression completed once on this host in 86.09 seconds and failed its 60-second assertion; draco's reference five-pass median remains 26.42 seconds.

| FND | Outcome | After excerpt |
| --- | --- | --- |
| FND-001 | fixed | `let key = (state.model_key.clone(), property, identity.input_bytes);` |
| FND-002 | fixed | `canonical_key: std::sync::Arc<std::sync::OnceLock<std::sync::Arc<crate::simulation::StateKey>>>,` |
| FND-003 | fixed | `admit_initial_state, model_views, observation_facts, observe_state, state_fields,` |
