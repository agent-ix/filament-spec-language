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

## Dispositions

Round 1, reviewed at frozen head a377cdea54b197ba9427768d6b1d1f627e83eafd.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The exact TRAIL_REACHES_KEY now records both self operands and both Boolean-type positions; the structural assert_eq! remains. The PR moved-ID inventory records this changed key. |
| FND-002 | fixed | Temporal computation paths now retain every operand position, addressing the measured min_count 0/2 source-order collision. The record-value subclaim is rejected: literal_as returns a literal NodeRef, and record_members binds that reference, so raw literal text cannot suppress a Text field-name segment. Both source record permutation tests passed before this production fix (record-red2.log: package 1, semantics 1). No value or bounded-domain encoding was changed. |
| FND-003 | fixed | The contradictory without-repeats claim is removed. Documentation explicitly distinguishes positional computation paths from retained compact value/hybrid/bounded-domain encodings and the decimal-range text exception. |
| FND-004 | fixed | The regression now requires the two checked claim identities to be distinct as well as stable across declaration order. |
| FND-005 | accepted-no-change | Lead decision: retain tests/fixtures/native-run-result-decisive.json as historical opaque native-run-result/2 reader contract data. Its expression binary/quantify IDs have the old single-Boolean path form; current writer keys differ. The moved-ID inventory lists them and native_run_result_root.rs now describes reader input rather than a current writer output. The only located fixture use reads/adversarially mutates the document; no regeneration path was found. This accepts historical IDs and does not claim that today's writer emits them. |

## Disposition evidence

Code-review and rust-review disposition inspection covers fix e446e9150106bd3fd2474381c05f6ebef7c7f827 and frozen head a377cdea54b197ba9427768d6b1d1f627e83eafd. Examined: walked_operands/derived_operands in lowering.rs; record lowering/literal_as; population_field_edge.rs exact key assertion; probabilistic.rs distinct/stable identities; temporal_aggregate.rs min_count permutation; golden.rs and node_keys.rs source record regressions; native_run_result_root.rs reader boundary; the unchanged historical fixture; and the refreshed GitHub PR body moved-ID inventory. No CI gate was narrowed; no unsafe, concurrency, public API or new persistence mechanism was introduced by this fix. No new finding was observed.

Evidence: /tmp/qsl-3266-repeated-red.log shows the temporal identity assertion failing before the temporal change (ordinals swap). Its record failures were setup failures and are not record collision evidence. /tmp/qsl-3266-record-red2.log shows corrected source record tests passing before the production fix: package 1 passed, semantics 1 passed, exit 0. The record string is represented by a literal NodeRef (literal_as -> value_node -> reference_node); LowerFrame::Record binds that NodeRef, so it cannot compare equal to a field-name Text segment. Reader-defined scalar/value/bounded-domain spellings remain intact. The original expression mutation evidence remains applicable.

No Cargo or full gates were run by this disposition reviewer under the dispatch's explicit restriction. Final frozen-head execution is pending the lead's Draco TEST-RUN evidence. This review does not claim that the historical fixture's captured IDs are emitted by the current writer.

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    reason: "The exact TRAIL_REACHES_KEY now records both self operands and both Boolean-type positions; the structural assert_eq! remains. The PR moved-ID inventory records this changed key."
    after_excerpt: |-
      /// The structural key of the unbounded `reaches(self, self, trail)` node over
      /// `Order`, retaining both operand positions (TC-1047 row 3).
  - fnd: FND-002
    outcome: fixed
    reason: "Temporal computation paths now retain every operand position, addressing the measured min_count 0/2 source-order collision. The record-value subclaim is rejected: literal_as returns a literal NodeRef, and record_members binds that reference, so raw literal text cannot suppress a Text field-name segment. Both source record permutation tests passed before this production fix (record-red2.log: package 1, semantics 1). No value or bounded-domain encoding was changed."
    after_excerpt: |-
              keep_repeats: matches!(content.node_tag, NodeTag::Expression | NodeTag::Temporal),
  - fnd: FND-003
    outcome: fixed
    reason: "The contradictory without-repeats claim is removed. Documentation explicitly distinguishes positional computation paths from retained compact value/hybrid/bounded-domain encodings and the decimal-range text exception."
    after_excerpt: |-
      /// repeat of its path takes the next producer ordinal. Expression and temporal
      /// computation nodes keep every walked operand, repeats included: operand
      /// positions distinguish their content, so dropping a repeated position would
      /// give different computations one path and source-order ordinals. Value,
      /// hybrid and bounded-domain nodes retain their compact first-occurrence
      /// spelling; this computation-key rule does not revise their encodings. A
      /// decimal range retains repeated text, so equal bounds repeat.
  - fnd: FND-004
    outcome: fixed
    reason: "The regression now requires the two checked claim identities to be distinct as well as stable across declaration order."
    after_excerpt: |-
          assert_ne!(
              forward[0].1, forward[1].1,
              "the two claims retain distinct identities"
          );
  - fnd: FND-005
    outcome: accepted-no-change
    reason: "Lead decision: retain tests/fixtures/native-run-result-decisive.json as historical opaque native-run-result/2 reader contract data. Its expression binary/quantify IDs have the old single-Boolean path form; current writer keys differ. The moved-ID inventory lists them and native_run_result_root.rs now describes reader input rather than a current writer output. The only located fixture use reads/adversarially mutates the document; no regeneration path was found. This accepts historical IDs and does not claim that today's writer emits them."
```

## Round 1 verdict

All existing findings have explicit outcomes; no finding remains open. Review dispositions are complete. Mergeability remains conditional on final Draco focused-test evidence for the frozen implementation and verbatim custody of these updated artifacts.

