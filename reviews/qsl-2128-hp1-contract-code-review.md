---
id: SR-20261121173001003
title: "QSL-2128 HP-1 package contract code-review"
type: SpecReview
analysis: code-review
scope: "FR-531 and FR-940, PR3351 at 81b3e47ada4a925b8cd1d0210e728573d8e572a2"
review_set: subset
---

## Summary

PASS: no language-independent code-review finding in this specification-only change. Reviewed the bounded HP-1 package form, evaluator port, declaration-order evidence and AC23–27. No applicable AssuranceProfile was found; this direct review uses base and integrity lenses. No Cargo or full-repository gate was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found in the scoped language-independent checks. | FR-531; FR-940 |

## Evidence

FR-940 says absence remains globally admissible and only selection of an HP-1 package form returns UnsupportedView, adding MissingSubjectDeclarationOrder to FR-390 reasons; Analyze settles only that item unsupported and a sibling remains available. FR-390 says unsupported view creation yields no partial view, orders its existing reasons, and after a view exists read_node returns a complete canonical read or an invariant fault, never late unsupported projection. The new reason is absent from the owner inventory and deterministic ordering. The contract must distinguish semantic form availability from complete evaluator-view admission.

The explicit alias/root permutation preserves two aliases sharing a model root and does not infer declaration order. Strict malformed-present versus legacy-absent behavior, source-free parity, independent certificate reconstruction, fresh tuple allowances, negative tampering and source removal are specified. AC23–27 remain in one contiguous acceptance table. Source certificate order is preserved. Contract IR reader agreement and its coherent dependency update remain implementation prerequisites; this review does not claim that implementation exists. The new member contract is explicitly requested by the owner; its normative home must remain coordinated with the owning reader contract. No second certificate checker, hash ledger, source fallback or fabricated checked typestate is specified.

## Validation

Author reports targeted Quire validation and diff check pass. Reviewer inspected strict reader execution-parameter and Boolean-body admission in the pinned step_relation reader and existing FR-179/163/184 semantics. New criteria intentionally precede implementation; no engine coverage or runtime PASS is claimed.
