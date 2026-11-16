---
id: SR-2026111606410700000002
title: "Code review of QSL-2175 source object member types"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2175; PR 2749 at 917b4df400cc59b8b128fad10efa498cbb4fd581; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/source_model.rs, fsl-semantics/tests/it/source_objects.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
---

## Summary

Reviewed QSL-2175's three-file diff at the frozen head, including callers that retain the normalized source model. The late type-resolution pass updates the checking tables but leaves the canonical model with placeholder types; another early admission arm still rejects model-owned variants.

## Verdict

**FAIL** — two high findings prevent merging PR 2749.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Placeholder Integer types survive in the normalized source model after the checking tables are repaired, so operation variance checks compare Integer with Integer and accept incompatible rich-type overrides. Resolve and retain the real member types in the model consumed by conformance as well as in the checker tables. | fsl-semantics/src/check/source_model.rs:865; fsl-semantics/src/check/assemble.rs:3342; fsl-semantics/src/check/assemble.rs:5363; fsl-semantics/src/check/model_conformance.rs:181; fsl-semantics/src/model/conformance.rs:355 |
| FND-002 | high | A qualified model-owned enum or payload variant still refuses UnsupportedType in the synthetic-model prepass, before the new resolver can use the valid record-field type. Admit these named value types through the same resolution rule as source record fields. | fsl-semantics/src/check/source_model.rs:827; fsl-semantics/src/check/assemble.rs:1400; spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:309 |

## Evidence

FND-001: `scalar_type` now returns `Native(Integer)` for local enum/composite/alias and other rich type forms. `source_model` normalizes that package before `resolve_source_object_members` runs. The latter changes only `object_types` and `operations`; `package.model_inputs = Some((models, model_meter))` and `package.models = admitted` retain the earlier model. `check::model_conformance::check` obtains `selection.view.model_index()` and passes its original operation records to `check_operation_redefinition`, which compares `dp.value_type` and `rp.value_type`.

A concrete counterexample from that call path is:

```text
record R { x: Integer; }
record S { y: Boolean; }
object Base { operation f(p: R): R; }
object Child extends Base { operation f(p: S): S redefines Base::f; }
```

The resolved operation table distinguishes `R` and `S`, while both normalized signatures remain `Integer`. Their identical placeholder types satisfy the parameter and result axes; the required `variance-parameter`/`variance-result` refusals are lost. This is a source-route conformance failure, independent of the deferred QSL-1880 emission work. The counterexample is established by source tracing; no execution was run under the review brief.

FND-002: with an admitted model `M` declaring an enum `Tier`, `record R { f: M::Tier; }` resolves through `ModelTypes::package_names`, which includes `variant_types`. `object O { f: M::Tier; }` instead reaches the earlier qualified-name match in `source_model::scalar_type`. Its permitted records are only `ScalarType`, `AliasType`, `RecordValueType`, and `TupleValueType`; `Enum` reaches `Some(_) => Err(unsupported())`. The new late resolver cannot repair a unit already refused by this prepass. FR-569's member rule explicitly includes model-owned variant types.

## Scope

```yaml
scope:
  - id: FR-569
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: |-
      The assembler and S3 SHALL admit, for a field, operation parameter and
      operation result of a source object type, every type a source `record` field
      admits, with the same checks and the same refusals (FSpec FR-610 "Member
      types"):
  - id: FR-569
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: |-
      a redefinition that
      breaks an FSpec FR-151 conformance axis: that axis's refusal, as FR-151
      gives it for the same domain declaration (`ill_typed`/`variance-parameter`,
      `variance-result`, `type-mismatch` for arity, `multiplicity-narrowing` or
      `effect-escape`);
  - id: FR-569-AC-12
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: "Source route. For each member type, an `object` field of that type, and an operation parameter or result of that type, assembles and checks exactly when a source `record` field of that type does, and `self.f` in a state clause on the object has the type the record field has:"
  - id: FR-569-AC-13
    path: spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md
    role: examined
    excerpt: "Source route. A field typed `Population<Order>[3]`, and a field typed by the name of a source function, are each refused at the type with the code and cause a source record field of the same type gets, and the unit yields no checked package."
```

## Coverage

Methods: code-review with rust-review lane, including Rust style, exhaustive conversion behavior, error propagation, existing callers, code/test alignment, and requirement fidelity. No CI workflow or gate changes occur in the diff. No applicable AssuranceProfile was found under `spec/`.

The common type resolver is reused rather than introducing another scalar-domain implementation. The new wildcard-to-Integer conversion nevertheless violates the Rust rule against mapping an unknown domain variant to a valid but wrong value; FND-001 records its concrete consequence. The field and operation checking table changes retain existing identity keys and effect frames. AC-13's population and function-name refusal comparisons and the alias-cycle refusal are present; the separate gap-analysis artifact records insufficient type assertions for AC-12.

No source edits, builds, tests, or aggregate gate were run, as directed. The coder reported 44 source-object tests, the full semantics suite, formatting, workspace checks/Clippy in both feature configurations, and qualified-core lint passed; those results were not rerun. Quire 0.38.3 (engine 0.52.1), Quoin 0.29.4, spec-artifacts-process v0.29.2 were used for review metadata. Quire emitted nonfatal inline-schema and duplicate-archetype warnings.
