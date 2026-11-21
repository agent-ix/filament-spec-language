---
id: SR-202611210022442313144108538
title: "spec-review/base of FR-1036 plan candidate correction (QSL-1978)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR3279 head e28930774c7f3c2bf2c8665bd6ca03f24383a43e; FR-1036 candidate Behavior and AC2"
review_set: subset
---

## Summary

FR-1036 correction names generated plan-record candidate typing while preserving actual grammar G and domain D. The approved FSpec786 candidate 5932a58cea11de821e19c231a1eb9ff67bfcec85 supplies owning contract context despite its ordered later merge. No assurance profile applies to this requirement correction.

## Verdict

PASS for the scoped requirement wording; implementation coverage FND-009 belongs to the gap-analysis artifact and blocks PR merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-1036
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: examined
  excerpt: "- The writer SHALL write `candidate` as a reference to the `value`/`parameter`\n\
    \  node of the declaration's own name at level `0`, typed `D` for a value\n  synthesis\
    \ and the generated FR-800 plan-record type for a state-model or\n  hybrid plan\
    \ synthesis,\n  the binder S3 bound inside the block and the objective (FR-754).\
    \ The plan\n  candidate's `semantic_type` SHALL reference that generated plan-record\n\
    \  `composite_type`/`record` node defined by FSpec FR-322 \"Plan-record type\"\
    ,\n  with that section's field schema, applicable FR-819 extensions and\n  ordinary\
    \ generated-record identity, ownership and s"
- id: FR-1036-AC-2
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: examined
  excerpt: The plan synthesis `Reach` over `Config::ConfigVersion` with domain `Lengths`
    emits a node whose `grammar` is the actual `model`/`object_type` node of `ConfigVersion`,
    whose `domain` is the `bounded_domain`/`integer_range` node of `Lengths`, and
    whose `candidate` references the `value`/`parameter` node named `Reach` at level
    `0`, with `semantic_type` referencing the generated FR-800 plan-record type node;
    the emitted package is admitted by FSL's reader, which returns that same candidate
    and plan-record type reference, grammar, domain, objective and block. Retyping
    the candidate to the `ConfigV
- id: FR-322
  path: spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: context_only
  excerpt: The plan candidate is not typed by `G`; the `grammar` binding
```

## Evidence limits

Exact-head runtime remains pending. No cargo/runtime run. Base checklist applied only to the changed candidate schema and AC2; original surrounding Verification cells and dependencies were context, not a fresh baseline audit. No structure, relationship or domain-object change requires additional lenses. Criterion-strength unavailable without Jev; skipped. Native model ID unavailable.
