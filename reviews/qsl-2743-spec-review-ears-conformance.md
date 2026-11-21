---
id: "SR-17952890000005"
title: "QSL-2743 spec-review-ears-conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2743/QSL-2718; FR092/531/533 enum carrier and returns; terminal target admission"
review_set: subset
---

## Summary

The modified requirement statements have concrete subjects and externally verifiable responses; the engine reports no scoped grammar warnings.

Ticket: QSL-2743; secondary QSL-2718. Frozen head 999023232ed624fe22b98739e4cac161883e6b2f, source base 19c53934c2e710e8d17de59a862108b4f5dd66e0. One existing worktree; no code changes, Cargo commands, checkouts, pushes or extra worktrees. Quoin 0.36.7, Quire 0.52.0 (engine 0.62.2), spec-artifacts-process@v0.29.2. No active AssuranceProfile document applies; matches were quoted review prose rather than type: AssuranceProfile frontmatter.

## Verdict

PASS. No new defect found in this analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Engine check: explicit quire validate of FR092/531/533 --summary, exit0: 3/3 docs grammar-clean (100%); 0 grammar findings. Semantic checks covered selected payload-free enums, retaining actual declaration/case nodes, literal canonical variants/ranks, equal source/Bare returned values and semantic charges, strict no-default/refusal behavior, and the prohibition on erasing authenticated enums because another declaration is unsupported. These name concrete actors, conditions and responses. The known table-extraction defect belongs to integrity/base and is not mislabeled a grammar success for AC42/43/44.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-092",
      "path": "spec/functional/FR-092-key-type-parameter-and-declared-nodes.md",
      "role": "examined",
      "excerpt": "For every payload-free model-owned enum of a selected domain model, including\nan unused declaration, lowering SHALL retain the real declaration and case\nnodes. The `Emission` SHALL carry one\n`SelectedModelEnumShapeRow` beside the model enum declaration and member\nrows. The row holds the declaration's emitted structural id and key, the\nselected document and the declaration's original model identity, the checked\n`EnumShape` of the variant type (unordered, with the cases' variants in\nascending byte order of the case names), and one entry per case in that\norder: the case label, the case's emitted "
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "These three model-enum channels carry only payload-free selected model variant\ntypes, including unused declarations and every real case node. A case with a\nClock payload is not payload-free. The writer retains the checked shape and\nvariant identities, never derives them from case keys, and orders cases by the\nshape's canonical rank, independently of authored source positions or model\naliases. Contract IR's `SelectedExactIdentities::with_admitted_model_enum_rows`\njoins all three channels against this package's selected model documents under\nthe invocation's cumulative native work and storage bu"
    },
    {
      "id": "FR-533",
      "path": "spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md",
      "role": "examined",
      "excerpt": "Payload-free selected model enum declarations and all their members SHALL be\navailable through the three required identities/5 channels of FR-531. The native\npackage-local join authenticates their complete canonical shapes before use,\nincluding selected declarations no slot reads. Enum literals SHALL retain the\noriginal variant and its canonical rank; source and bare enum-returning calls SHALL return the\nsame value and semantic charge sequence. Enum equality and inequality remain\nunmet until the native equality reader admits held selected-model declaration\nfacts independently of their original"
    }
  ]
}
```

