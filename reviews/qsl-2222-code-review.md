---
id: SR-202611210022442313144108430
title: "QSL-2222 loop proof code-review for PR 3270"
type: SpecReview
analysis: code-review
scope: "FR-218 AC-43/48; FR-730; PR 3270 changed Rust files"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `65dfdca3d24282967f6119274f64f6066ede53dd`, diff `origin/main...HEAD`. Read-only review; no Cargo, builds, tests or source mutations, per dispatch. `git diff --check origin/main...HEAD` passed. PR-reported 229 passing / 1 failing is unverified here; terminal_two_models::a_subject_reading_one_identity_derives_the_deadlock_item_of_that_identity has no main baseline evidence, so its failure is unresolved and is not classified as unrelated. QSL-2037, the structured blocker of QSL-2222/QSL-1971, is Done (verified via tracker UUID); the retained blocked_by edge does not create an active blocker.

Old #3141 findings: false-invariant fixture now uses decreasing `dec` with `got.left > 0` and initial 2, so it reaches 0 after body steps (source verified; execution not rerun). The no-proof trace finding remains open because the replacement AC-4 tag still names a different scenario. The old E0599 repair is source-inspected only; no independent compilation claim.

## Verdict

**FAIL** — wire-contract ruling and two implementation edge defects remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The producer emits loop-invariant and loop-termination as two additional temporal item_kind spellings while the normative contract admits exactly five. FR-730 requires these new items, so this is an unresolved cross-spec contract conflict, not permission to drop loop proofs. Obtain the architecture ruling and align the authoritative wire contract and consumers before merge. | FSpec-FR-368-AC-13; fsl-route/src/request.rs:119-140 |
| FND-002 | medium | The recursive tuple predicate admits tuple Pair(Integer,Integer); tuple Nested(Pair,Integer); variant Nested(Pair(1,2),3), although Nested has a tuple element rather than integer elements and FR-218 refuses every other type. The runtime recursively flattens it too. Restrict immediate tuple members to Integer/Int and add a nested-tuple refusal assertion. | FR-218; fsl-semantics/src/check/protocol_slots.rs:2157-2159 |
| FND-003 | medium | A recorded loop-proof refutation is lost when the while guard is undecided at the same decision. loop_proof_decision stores false/undefined in the Instance, then decide returns Settled::Undecided; initial_states discards that Instance (protocol.rs:1562-1563), and place returns false so a successor is not kept (steps.rs:761-763). A false invariant plus an incomplete/undefined guard therefore reports no loop-proof refutation. Preserve a reachable failing decision independently of whether the guard can advance, and test entry and post-body decisions. | FR-730; fsl-eval/src/simulation/protocol/settle.rs:288-290 |

## Coverage

Reconciliation: quoin matrix 0.36.8, quire 0.52.0 (engine 0.62.2), both exit 0. Whole-repository engine output retained in matrix.json and coverage.json. Static totals: tagged 4659; untagged 1161; tagged-by-ignored-test 156; method-without-symbol 104; retired 13. All 6093 rows have no run evidence. All six targeted criteria are tagged by active tests. Coverage extraction reports 22254 diagnostics and 5614 untracked symbols, with Rust census candidates 13121 / tagged 12563 / bound 12064; these are not a clean repository-wide assurance result. Whole-repository missing tags were not assigned to this PR without baseline attribution. The artifact verdict concerns the dispatched loop-proof subset; repository assurance does not pass.

Plan completion: not assessed

Reverse-gap/stub inspection covered changed public items and their callers across 36 files: slot type admission, derived items/selection/identity, proof-product evaluation and canonicalization, model checking, certificates, replay, result encoding and assertions. These behaviors have FR-218/FR-730 owners. No source stub found in the change. This is a bounded PR audit, not an exhaustive inventory of the repository's entire public API. Optional separate semantic-review fan-out was not run; ordinary code/test-intent review ran under the explicit dispatch scope.

## Examined files

- `fsl-analyze/src/analyze.rs`
- `fsl-analyze/src/analyze/document.rs`
- `fsl-analyze/src/analyze/items.rs`
- `fsl-analyze/src/analyze/protocol_item.rs`
- `fsl-analyze/src/claim_family.rs`
- `fsl-analyze/src/model_check/check.rs`
- `fsl-analyze/src/model_check/reduce.rs`
- `fsl-analyze/src/zone_check/certificate.rs`
- `fsl-analyze/src/zone_check/settle.rs`
- `fsl-analyze/tests/it/loop_proof.rs`
- `fsl-analyze/tests/it/protocol_composition.rs`
- `fsl-analyze/tests/it/protocol_system.rs`
- `fsl-eval/src/automaton/key.rs`
- `fsl-eval/src/simulation/model/encode.rs`
- `fsl-eval/src/simulation/protocol.rs`
- `fsl-eval/src/simulation/protocol/program.rs`
- `fsl-eval/src/simulation/protocol/settle.rs`
- `fsl-eval/src/simulation/subject.rs`
- `fsl-eval/tests/it/automaton_key.rs`
- `fsl-replay/src/certificate/closure.rs`
- `fsl-replay/src/lib.rs`
- `fsl-replay/src/model_check/counterexample.rs`
- `fsl-replay/src/model_check/item.rs`
- `fsl-replay/src/model_check/replay.rs`
- `fsl-replay/src/model_check/replay/timed.rs`
- `fsl-replay/src/model_check/smt_query.rs`
- `fsl-replay/src/outcome/counterexample.rs`
- `fsl-replay/src/spine.rs`
- `fsl-route/src/deadlock.rs`
- `fsl-route/src/model_request.rs`
- `fsl-route/src/request.rs`
- `fsl-semantics/src/check/protocol_clause/tests/refusal_corpus.rs`
- `fsl-semantics/src/check/protocol_clause/tests/rules.rs`
- `fsl-semantics/src/check/protocol_obligation/tests.rs`
- `fsl-semantics/src/check/protocol_slots.rs`
- `fsl-semantics/src/check/refusal.rs`
