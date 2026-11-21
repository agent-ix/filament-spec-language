---
id: SR-202611210637401
title: "code-review and rust-review of PR 3266 arrangement-independent expression node ids"
type: SpecReview
analysis: code-review
date: 2026-11-21
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/lowering.rs, fsl-package/src/emit/tests/probabilistic.rs; PR 3266 head ece4405b1b over origin/main 7b3d64fbce"
review_set: subset
---

## Summary

Ticket: QSL-2656. The PR makes `walked_operands` keep every walked operand, repeats included, for `expression` nodes only, and adds TC-1288 test `probabilistic_claims_with_repeated_expression_operands_keep_identity_when_reordered`. The fix is correct for the ticket's example: a one-line mutation restoring first-occurrence dedup for expressions (`keep_repeats: false`) turns the new test red at probabilistic.rs:409, so the test is a real oracle. But the change re-keys every expression node (result type and semantic type are now both kept, every repeated operand is kept), and one pinned structural key in the same crate's lib tests now fails. Rust lane: no idiom or safety defects; the change is small and auditable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Branch is red: `an_unbounded_field_edge_keeps_its_member_and_node_identity` fails, its pinned `TRAIL_REACHES_KEY` now differs (self param and Boolean type repeat in the expression path); ticket requires moved recorded ids to be re-recorded and listed, coder reported none | fsl-package/src/emit/tests/population_field_edge.rs:19, fsl-package/src/emit/tests/population_field_edge.rs:269 |
| FND-002 | medium | Same-operand-different-arrangement collisions remain on temporal and value nodes, which still dedup: a window aggregate whose `min_count` equals `upper` or `lower` drops it, so `positions[0,2] min_count 2` and `min_count 0` walk to one path and take source-order ordinals; a `record_value` whose string field value equals a later field name collides the same way | fsl-semantics/src/check/lowering.rs:8040, fsl-semantics/src/check/lowering/temporal.rs:691 |
| FND-003 | low | `derived_operands` doc still says value, expression, temporal, hybrid and bounded-domain nodes walk "without repeats", then appends that expressions keep repeats; it never states why the other walked tags keep dedup, so the expression-only scoping reads as unexplained | fsl-semantics/src/check/lowering.rs:7713 |
| FND-004 | low | New test asserts equality across orders only; it never asserts the two claims get distinct ids from each other (the sibling test does with `assert_ne!`) | fsl-package/src/emit/tests/probabilistic.rs:409 |
| FND-005 | low | Recorded expression node ids in the native-run-result fixture are now in the pre-change path form (single trailing Boolean on `binary`/`quantify`); not read structurally, but not what the writer emits any more and not listed as moved | tests/fixtures/native-run-result-decisive.json:1 |

## Evidence

- HEAD run (`cargo test -p fsl-package --lib` filtered): new test ok, `probabilistic_claim_identity_survives_reordering` ok, `an_unbounded_field_edge_keeps_its_member_and_node_identity` FAILED at population_field_edge.rs:269 (left path carries the `self` parameter node twice).
- Mutant run (`keep_repeats: false`): new test FAILED at probabilistic.rs:409 (claim identities differ across orders), pinned-key test ok. The mutation was reverted; worktree clean at ece4405b1b.
- FND-002 traced from `aggregate_argument` (temporal.rs:691-697: `lower`, `upper`, `min_count` are inline integer literals in one group) through `Walk::push` (lowering.rs:8040-8045), which still drops a repeated text for `temporal` nodes. Not run.
- Scoping: the code comment cites no ticket. Keeping dedup for `integer_range`/value paths is a reader-compatibility choice (coder reports the pinned contract-ir reader refused repeated nodes there), not a derivation rule; the comment does not say so.

## Verdict

Not mergeable until FND-001 is fixed (re-record the pinned key, or delete the pin if it is judged ceremony, and list moved ids). FND-002 is a real residual of the same FR-498 defect class on non-expression walked tags; fix or record a decision on the ticket. FND-003 to FND-005 are small.
