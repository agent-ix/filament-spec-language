---
id: SR-202611211854276
title: "object review of FSL PR3373 QSL-2739 composition wire objects"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d; objects: composition item assumptions, component-guarantee slot key, interface-mismatch counterexample, planner mismatch, provider-answer-unconfirmed cause; FSpec #792 5423fcf482"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This is an object and cross-reference review. The method marker is spec-review/object-review; `analysis: base` is used because the installed SpecReview enum has no object-review value. Each wire object the PR names was checked against its upstream definition:

- **Composition item `assumptions`.** FSpec FR-331 Items 611–630 defines it as one strictly ascending array of `premises` indices per `component-guarantee` entry, in `premises` order, and empty when there is no assumption. FR-1044 Decisions lines 92–106 match. The sets are empty under the three non-AG rules (FSpec FR-491 "Assumption binding"), and an `H` that is not declared contributes no index. The item is identity-bearing, and FR-1044 adds no `assumptions` member to the ordinary component item, which matches FR-331 "by no other item".
- **`component-guarantee` slot key.** It is the item's identity plus `assumptions` (FSpec FR-492 "Request"). FR-1044 lines 107–122 and FR-530 Discharge match. `premise_items` deduplicates on `ItemKey`, which includes the set, so dedup uses the full identity and context. FR-1044-AC-8's second `Capped` at index 5 follows.
- **`interface-mismatch`.** FSpec FR-331 lines 1069–1074 give `{type, component, interface, premise, reason}`. FR-1044 lines 141–147 match: `component` is the peer name, `interface` the clause node of `H`, `premise` the index of the rely or dependency entry, and `reason` is `absent` or `weaker`. In AC-14, `SysAbsent`'s `premise` 1 is the `rely` entry at index 1 after the one `component-guarantee` entry.
- **Planner `mismatch`.** FSpec FR-492 Planner result gives `{component, interface, premise, reason}`, with no cause, no certification and no conclusion, and a complete `slots` list. FR-530 Settlement and AC-9 match.
- **`provider-answer-unconfirmed{premise}`.** FSpec FR-331 core-guard says it carries only `premise`, belongs to composition only and has no certification. FR-1044 lines 297–315 match.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder): no object-shape or identity defect. | - |

No object finding. The wire-object problems found in this PR are about how a rejection is classified (failure-domain FND-001/FND-002) and about which side owns admission (integrity FND-001). None concerns an object's shape or identity.

## Verdict

**Clean** for object identity and cross-references.

## Scope and evidence

Spec-only. A bounded code-seam check confirmed that the types named for the code PR exist:
- `CompositionTarget` (fsl-analyze/src/provider.rs:334, premises only).
- `ItemPremise` (fsl-route/src/declaration.rs:41).
- `ComponentGuaranteeKey` with its assumptions (fsl-semantics/src/check/composition.rs:349).
- `CompositionClaim::required` (fsl-analyze/src/analyze/document/composition_item.rs:58).

There is no `ProviderAnswerUnconfirmed` or `InterfaceMismatch` type yet. That is expected, because this PR makes no implementation claim.

- FR-1044 Decisions, Outputs, Registered provider: examined.
- FR-530 Discharge, Settlement, AC-9, AC-10: examined.
- FSpec FR-331, FR-491, FR-492 object definitions: context_only.

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | no_change_needed | Validator placeholder, not a defect. The object shapes are unchanged by the fix commits: interface-mismatch {type, component, interface, premise, reason}, provider-answer-unconfirmed{premise}, and assumptions/slot key. |
