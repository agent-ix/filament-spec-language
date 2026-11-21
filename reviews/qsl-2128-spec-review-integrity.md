---
id: SR-20261121173001002
title: "QSL-2128 HP-1 package contract integrity"
type: SpecReview
analysis: integrity
scope: "FR-531 and FR-940, PR3351 at 81b3e47ada4a925b8cd1d0210e728573d8e572a2"
review_set: subset
---

## Summary

FAIL: the legacy item-local refusal conflicts with the existing view-admission contract. Reviewed the bounded HP-1 package form, evaluator port, declaration-order evidence and AC23–27. No applicable AssuranceProfile was found; this direct review uses base and integrity lenses. No Cargo or full-repository gate was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | MissingSubjectDeclarationOrder is added as an UnsupportedView reason returned for one selected relation after a view exists, but FR-390 defines unsupported projection at all-or-nothing view creation, requires a complete reason ordering, and prohibits late unsupported projection. Define a typed relation-form selection refusal mapped to the existing item outcome independently of view-admission UnsupportedView, or amend the owning contract explicitly without weakening complete read_node or sibling availability. | FR-940 declaration-order member contract; FR-390 ViewFailure and view creation |

## Evidence

FR-940 says absence remains globally admissible and only selection of an HP-1 package form returns UnsupportedView, adding MissingSubjectDeclarationOrder to FR-390 reasons; Analyze settles only that item unsupported and a sibling remains available. FR-390 says unsupported view creation yields no partial view, orders its existing reasons, and after a view exists read_node returns a complete canonical read or an invariant fault, never late unsupported projection. The new reason is absent from the owner inventory and deterministic ordering. The contract must distinguish semantic form availability from complete evaluator-view admission.

The explicit alias/root permutation preserves two aliases sharing a model root and does not infer declaration order. Strict malformed-present versus legacy-absent behavior, source-free parity, independent certificate reconstruction, fresh tuple allowances, negative tampering and source removal are specified. AC23–27 remain in one contiguous acceptance table. Source certificate order is preserved. Contract IR reader agreement and its coherent dependency update remain implementation prerequisites; this review does not claim that implementation exists. The new member contract is explicitly requested by the owner; its normative home must remain coordinated with the owning reader contract. No second certificate checker, hash ledger, source fallback or fabricated checked typestate is specified.

## Validation

Author reports targeted Quire validation and diff check pass. Reviewer inspected strict reader execution-parameter and Boolean-body admission in the pinned step_relation reader and existing FR-179/163/184 semantics. New criteria intentionally precede implementation; no engine coverage or runtime PASS is claimed.

## Dispositions

### Round 1

PASS for the bounded specification delta at 25634322fcdcf4b27b3abb9f2141376fb681ccf7. FND-001 fixed: ModelRelationFormRefusal::MissingSubjectDeclarationOrder is a typed semantic form-selection result independent of ViewFailure/UnsupportedView. It maps to existing item UnsupportedWithDetail and retains owning package/relation-key diagnostics. FR-390 view creation, read_node completeness and deterministic projection reason inventory/order remain unchanged. No other semantic change or new finding in this narrow disposition. Implementation remains conditional on the owned Contract IR member contract and coherent dependency update; no code/runtime/coverage pass is claimed.

### Owner contract alignment

Inspected the exact reader-owner-approved path refinement e61a3cc9 and citation correction f0a8e7f69406305c31152c1a8ac7c372e4aedaa4. The specified invalid_semantic_graph/no-cause diagnostics, fifth-member paths, supplied-entry precedence and omission-last rule match the owner agreement. The body ownership citation now explicitly names filament-contract-ir FR-351; unrelated seven-field prose was removed. No changed semantic finding or additional full review. The bounded contract remains clear; runtime and implementation review remain separate.
