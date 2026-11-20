---
id: SR-20261120043542794295000
title: "code review of PR3140 claim_names row order"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit.rs claim_name_rows; fsl-package/src/emit/tests/hyper_clause.rs; fsl-package/src/emit/tests/protocol_controls.rs; PR3140 47f7c918a3c31cfe44f6bab0532e0df08e7b0c43"
review_set: subset
---

## Summary

Ticket: QSL-2659. `claim_name_rows` now ranks every row by `node_id(ids, key)` (the structural id with operands expanded) and then `CheckedOccurrence`, which is exactly the strict reader's key in filament-contract-ir c2146579 (`index.resolved_at(position, structural_keys)`, ordered by rank that agrees with `CheckedNodeId::cmp`, then occurrence). The key is applied uniformly to every row kind, so state clauses, refinements, state-graph, probabilistic, schedulability, step-relation and hyper rows cannot disagree. FR-322 "Claim declaration names" states the same rule (structural `NodeRef`, then occurrence). The compile fix passes the house `ZeroRewardEvaluator` fixture. One medium trace defect: both new tests are tagged to FR-488-AC-10 (align_skip binding), which neither asserts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both new tests are traced to FR-488-AC-10 (align_skip binding) which they do not assert; the ordering they prove is FR-488-AC-24 / FR-322-AC-457 | fsl-package/src/emit/tests/hyper_clause.rs:1333 |

## Verdict

Mergeable. No high findings. Ordering key matches reader and spec for every row kind; ties are impossible beyond the existing (declaration, occurrence) dedup; FSL consumers resolve `claim_names` by content or sort explicitly by `declaration_region`, never by row index; no recorded JSON fixture carries `claim_names`. quoin gap-analysis was not run (quick-pass brief); a manual AC-to-test check found the ordering rule covered by `state_temporal_and_hyper_claims_are_named_in_the_readers_order`.
