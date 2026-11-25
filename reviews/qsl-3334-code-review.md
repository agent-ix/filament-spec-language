---
id: SR-202611251425031
title: "code-review of PR 3927 partial-product lower check and red-main fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3927 head 345c3498d9c3a1a60bc9f6af7872ad4fb256f85e; git diff origin/main...HEAD (fsl-replay probability_certificate, fsl-analyze exact.rs and items.rs, fsl-replay outcome.rs, fsl-probability-check tests)"
review_set: subset
---

## Summary

Ticket: QSL-3334 (with QSL-3331, QSL-3274 item 3). Code review with the rust-review lane folded in. The main question was soundness of the new partial-product `lower` check in `fsl-replay/src/probability_certificate/document/partial.rs` and its wiring in `check_document_bytes`. The review was done by reading the code. No cargo run happened: MemAvailable was 17,322,504 kB, below the 30 GiB floor of lock C and the 20 GiB floor of locks A and B.

## Verdict

CONDITIONAL. I found no soundness defect. After a `max_states` stop, the explored-part check accepts only documents whose named-state conditions would also hold over the whole graph. Each check below was traced:

- Every named state is expanded with all of its successors (`enumerate_model_graph_visiting`, `expand`).
- A named state that cannot be reached through named states is refused, because `explored_subject` returns `exhausted` when a named key is not in the graph.
- An unnamed successor is never expanded and is valued 0 (`certificate_values`, Partial). It takes part in no equality and no progress check, and it cannot carry a rank (`explored && ranks.keys() ⊄ named` is malformed).
- Prob0 and closed non-target components on the explored part are supersets of the whole graph's, so they only force more named values to exactly 0.
- Progress at a positive named state needs a named, positive successor of smaller rank. This excludes greatest-fixed-point cycles.
- A reward's least value of 0 is sound because negative rewards are refused at model admission (FR-185, FR-187-AC-4).
- Identity is compared before enumeration.
- A measure `shifted` base never reaches this path, because the Measure dispatch keeps `partial_lower: false`. Level shifting is therefore not reachable here.
- The path is taken only for a claim-side `lower` with no fairness. Counterparts, `upper`, exact and fair documents keep the state stop.
- The whole-graph and explored attempts share one meter and one storage budget. This can only add refusals.

All four red-test resolutions agree with the spec, and nothing was weakened:

- `items.rs`: a production change that copies only `smt_encoding`, so FR-281-AC-4's altered request is judged.
- `outcome.rs`: the fixture now expects `type-mismatch` and the catalog hint, per FR-495-AC-9 for `1 / x > 0` in a Boolean body.
- `measure.rs`: the fixture now names `retained` at 0, and a new InequalityFails assertion covers both successors unnamed, per FR-1042 Document.
- The reward partial path: production.

Two findings remain open. The first is a half-finished `expected-reward/v1` wrapping that leaves its `upper` always rejected. The second is a dangling trace id on a test this PR turns green.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `expected-reward/v1` was made a wrappable value-bounds base, but `wrap` and `unwrap` keep `ranking` on an `upper` only for the cyclic reward profiles. An acyclic expected-reward `upper`, which FR-1042 requires to carry a properness ranking, therefore always rejects `Malformed` at `ranking`. | fsl-replay/src/probability_certificate/document/value_bounds.rs:26, fsl-replay/src/probability_certificate/document/value_bounds.rs:225-235, fsl-replay/src/probability_certificate/document/value_bounds.rs:314-318 |
| FND-002 | low | `partial_reward_lower_certificate_uses_infinite_upper_bound` is tagged `FSpec-FR-1042-AC-11`. FSpec at the pinned revision 691ac5a has no FR-1042, so the id is dangling. The test is turned green by this PR but proves only the reward clause of FSL FR-1042-AC-11. | tools/fsl-probability-check/tests/it/reward.rs:107 |

## Scope and evidence

- partial.rs `explored_subject`, `partial_lower_probability_proof_document`, `progress_ranks` (examined).
- document.rs `check_document_bytes` lines 2325-2527, `check_document_equalities`; probability_certificate.rs `enumerate_model_graph_visiting`, `certificate_values`; cyclic.rs `enumerate`, `analyze`; reward.rs `enumerate` terminal check (examined).
- exact.rs producer changes (sink targets, `reward_subject_stopped`, the partial document builder) (examined).
- items.rs `run_model_item` (examined), outcome.rs test fixture (examined), measure.rs `unnamed_shifted_successor_uses_negative_level` (examined).
- value_bounds.rs `is_wrappable_base`, `wrap`, `unwrap` (examined).
- FSL FR-1042 Document, Conditions and AC-11, AC-18, AC-19 (examined); FR-201:155-171, 982-1032 (context_only); FSpec FR-411 "Evidence on a partial product" and FR-412 Checker, AC-5 at 691ac5a (context_only).
- Rust: no new unwrap, expect or panic in production paths; the arithmetic in the changed code is checked or saturating; cancel polls and meter charges are present in the new loops.
