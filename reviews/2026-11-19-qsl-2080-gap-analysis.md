---
id: SR-1476
title: "Scoped acceptance gaps in protocol source contracts"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2080; PR3154 at 0176c72bdaee28097b79b3558aee54a00e6900fd; fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs and tests.rs registration"
review_set: subset
---

## Summary

Compared the new accepted/refused S3 source scenarios with current protocol and located-refusal requirements. Await currently has an inverted-window refusal but lacks the requested undeclared-event/after-anchor refusal with its precise name locus.

## Verdict

**PASS** at `847b6cfc28b793261798626e7139e1c426d8e901` — the missing-await-anchor counterpart is added with its exact catalog and source locus. Original finding remains below as the review record.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The await pair only rejects an inverted window. Add a syntactically admitted undeclared `after` anchor to the existing valid fixture and assert its grounded missing-name code/cause and exact offending name span, rather than the await or protocol declaration. This exercises the currently omitted anchor-resolution branch required by FR-113 and the ticket's await counterpart. | fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs:187 | correct-requirement-no-evidence |

## Examined scope

- `FR-113` Description: examined — S3 resolves each scoped anchor and refuses a missing target with a catalog code at that reference's span.
- `FR-218-AC-35`: examined — valid receive-matching awaits, recorded temporal data and interval refusals.
- `FR-884-AC-5`: examined — each rejected construct has an accepted counterpart and a refusal region smaller than and within its protocol.
- `FR-218-AC-5` and `FR-218-AC-33`: examined — a repeat without a maximum is admitted under the current contract; the old ticket's blanket no-bound rejection is not imposed.
- `FR-218-AC-10`, `FR-218-AC-12`, `FR-218-AC-13`, `FR-218-AC-17`, `FR-218-AC-19`, `FR-218-AC-23`, `FR-218-AC-28`, `FR-113-AC-3`, `FR-884-AC-2`: examined for the current channel, key-binder, capacity-preservation, read-visibility, choice-partition, join, replicated-population, compensation and role-duplicate pairs.
- `FR-208-AC-1`: context only — runtime spawn/retire behavior is not an S3 source-admission obligation; its incorrect trace binding is recorded in SR-1475, not used to invent an S3 spawn syntax.

## Coverage

Plan completion: not assessed

This is the user-authorized diff-scoped gap analysis, including semantic requirement-to-test inspection of actual source paths. The repository-wide matrix, inventories, reverse audit and old Done-ticket verification were intentionally not repeated. No repository-wide completeness claim is made. The symbolic capacity is intentionally unresolved at S3 under FR-218-AC-13; no unsupported missing-capacity refusal was invented. Choice partition failure is correctly located at the choice header under FR-884, rather than forced onto an individual guard. The existing lower-layer ordering grammar is not relabeled as a new S3 rule.

The author's nine focused passing functions and the accepted blocked-lint receipt were trusted. No tests, builds, lint, product probes or baseline measurements ran. A single applicable final component gate remains the original coder's responsibility after these confined fixes.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The existing valid receive-matching await fixture now has an additional undeclared `MissingDispatch` after-anchor counterpart. It asserts missing_declaration/missing-name and the exact written name span through assert_locus. |

Round 1 inspected only the correction diff from `0176c72bdaee28097b79b3558aee54a00e6900fd` to `847b6cfc28b793261798626e7139e1c426d8e901`, including the new `await_after_an_undeclared_anchor_refuses_at_the_written_name` function. The paired valid await is retained in the existing source contract. The author's changed-case passing receipt was accepted; no execution, broad review, matrix or baseline work was repeated. No new findings remain in the assigned diff; the original coder owns the one final component gate.
