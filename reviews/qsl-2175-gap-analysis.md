---
id: SR-2026111606410700000003
title: "Gap analysis of QSL-2175 source object member types"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2175; PR 2749 at 917b4df400cc59b8b128fad10efa498cbb4fd581; FR-569 AC-12/13; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/source_model.rs, fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
---

## Summary

The computed matrix binds both scoped criteria to active tests. AC-12's assertions cover admission and a generic Boolean mismatch, but do not establish the actual member or operation signature types.

## Verdict

**CONDITIONAL** — one medium evidence gap. The companion code review has high findings, so the PR is not mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-12 tests never inspect the checked member type or operation parameter/result types: self.f = self.f and returning a non-Boolean field from a Boolean function both behave identically if every rich member remains Integer. Add exact type assertions or type-specific valid uses, and a rich-type override refusal regression that exercises the normalized model consumed by conformance. | fsl-semantics/tests/it/source_objects.rs:1396; fsl-semantics/tests/it/source_objects.rs:1508; spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:566 |

## Scope

```yaml
scope:
  - id: FR-569-AC-12
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: "Source route. For each member type, an `object` field of that type, and an operation parameter or result of that type, assembles and checks exactly when a source `record` field of that type does, and `self.f` in a state clause on the object has the type the record field has:"
  - id: FR-569-AC-13
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: "Source route. A field typed `Population<Order>[3]`, and a field typed by the name of a source function, are each refused at the type with the code and cause a source record field of the same type gets, and the unit yields no checked package."
  - id: FR-569
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: context_only
    excerpt: |-
      The checked member type is the type the same form has as a record field: a
      source enum member reads as that enum, a record member as that record (FR-359
      for a child record at an ancestor-typed slot), a union member as that union
      (FR-316), and every value read through `self.f`, `deref(r).f`, frames,
      snapshots (FR-106) and the evaluator is the value the record field would
      hold.
```

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.38.3, engine 0.52.1) — no run evidence read. Repository-wide engine output was generated at this frozen head, then assessed within the explicitly dispatched FR-569 AC-12/13 scope. Both criteria are `tagged`: AC-12 has 16 active binders, AC-13 has two. There are zero untagged or ignored-only criteria in this subset. No test execution evidence was consumed by the matrix.

The newly active tests cover source enum, record, union, tuple, alias, decimal, rational, float, quantity, optional collection, and nested collection admission. Enum/record/union operation declarations are included, but their signatures are not exercised or inspected. The new negative table confirms a non-Boolean field cannot be a Boolean function result; it cannot distinguish a correct rich type from an erroneous Integer placeholder. The alias-cycle test asserts the actual `definition-cycle` cause. AC-13 compares concrete refusal code/cause lists for population and function-name types.

Reverse ownership: FR-569 owns the changed member typing paths. No source-object emission changes were introduced; QSL-1880 remains out of scope. No source/test stub remains in the newly activated cases, but FND-001 records coverage inflation from assertions that cannot distinguish the required types. The optional separate semantic-review fan-out was not run; ordinary code/test alignment and hollow-evidence checks were performed under the requested reviewer methods.

`quire coverage` reported 937 repository diagnostics, none on FR-569, and 4469 repository untracked symbols. Existing `FSpec-` references on the reviewed tests are external namespaced tags under the repo's documented convention, outside the local matrix's document root; no new local FR-569 tag gap was found. These global diagnostics limit any repository-wide claim; this verdict covers only the dispatched subset. Raw matrix and coverage outputs are retained beside this artifact. Quire emitted nonfatal inline-schema and duplicate-archetype warnings. No source edits, builds, tests, or aggregate gate were run.
