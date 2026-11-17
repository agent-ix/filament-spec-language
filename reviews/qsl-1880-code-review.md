---
id: SR-2026111612172400000016
title: "Code review of filament-spec-language PR #2812"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2812: fsl-package/src/bare.rs, fsl-package/src/emit.rs, fsl-package/src/emit/identity_rows.rs, fsl-package/src/emit/monitor_plan.rs, fsl-semantics/src/check/**; QSL-1880 and QSL-2314 partial batch"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-569", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-390", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-088", type: references}
---
# Code review of PR #2812

## Summary

Ticket scope: QSL-1880 and the target-emission slice of QSL-2314. Reviewed exact head `e5f37571c847c9df735ea3c645f4e1fa73d9ead5`. The source object graph nodes, source operation rows, effects and clause provenance are present. The explicit QSL-2347 reader joins remain an external blocker and are not treated as implementation defects here. The Rust review found a source-object identity-file schema mismatch that the in-process encode/read roundtrip does not catch.

## Verdict

Changes requested: one high finding. Targeted checks reported on the PR pass; no aggregate gate was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `source_objects` JSON does not match the FR-531 identities/4 wire shape: it serializes `is_abstract`, and its member encoding drops the required original identity, admitted name, presence and exact value type. | fsl-package/src/bare.rs:584-603; fsl-package/src/emit.rs:7101-7114 |

## Review scope

- Examined the following acceptance criteria and identity-file contract:

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-569-AC-6 | examined | S4 over fixture S emits `Order`, `Sub` and `Shape` as `model`/`object_type` nodes |
| FR-569-AC-7 | examined | `Order`'s node id is unchanged when a field is added, an operation is renamed or the declaration is moved, and changes when `Order` is renamed |
| FR-569-AC-8 | examined | The emission over S carries one source-object row per type: `Order` with `abstract` false |
| FR-390-AC-70 | examined | Source-operation identity rows |
| FR-390-AC-71 | examined | Reader evaluation of a slot that reaches a source-declared operation anchor |
| FR-390-AC-72 | examined | Witness refusals for source-operation rows at `Package` with no partial view and no meter charge |
| FR-390-AC-73 | examined | Source-route evaluation passes; reader view creation waits on QC-30 |
| FR-088-AC-13 | examined | The in-process `PlaceRow` member type is the FR-322 identity rendering of the target node |
| FR-531-AC-8 | examined | The identity file equals, byte for byte, the encoding of the in-process unit's rows |

- Reviewed all changed Rust files and repository guidance. The change does not modify CI workflows.
- The reported in-process `ReadIdentityRows` roundtrip uses the same private `SourceObjectJson` type for writing and reading. It cannot establish byte compatibility with the required FR-531 shape.
- The source-operation, dispatch and timed-place reader checks ignored for QSL-2347 are explicitly pending that IR ticket; they are not findings against this FSL slice.

## Findings detail

### FND-001 — source-object identity JSON shape diverges from identities/4

FR-531's identity-file contract requires source-object rows with the `abstract` key and member entries in `source_records` member encoding (`original_member_identity`, `admitted_name`, `presence`, `type_node`, `value_type`). `SourceObjectJson` instead serializes `is_abstract`, while `SourceObjectMemberJson` contains only `name`, `type_node` and `optional`. `source_object_rows` also constructs only that reduced member row from the node binding. The resulting file is not the declared identities/4 contract and omits exact member type and identity data required by a bytes-only consumer. Since the private reader uses this same reduced struct, the existing roundtrip test can pass while the external wire contract is broken.

## Test evidence

The PR description reports passing targeted checks: `cargo check --locked -p fsl-semantics -p fsl-package`; source object emission tests (7 passed, 4 ignored); the clock-place identity test; and source record/union row tests. No aggregate gate was run. Quoin's gap-analysis command was unavailable in the installed CLI; a separate manual scoped traceability check is recorded in the gap-analysis artifact.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The public source-row constructor cannot retain source-object member facts, so `to_bytes` refuses rows created through this documented API. | fsl-package/src/bare.rs:1323-1343, 1468-1469 |

### FND-002 — public source-row construction is not serializable

`ReadIdentityRows::of_rows_with_source_rows` accepts source-object rows and documents that it constructs rows including source-object witnesses, but delegates with an empty `source_object_member_facts` vector. `to_bytes` rejects whenever the object rows and fact rows differ in count. The facts type and the constructor that accepts facts are crate-private, so an external caller cannot produce a serializable source-object file through this public constructor.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Head `7a0fe577fd510f47da4f11d64e37143b2d0fac5c` emits `abstract` and the required source-record-shaped member keys; the focused identity-row roundtrip test passed. |
| FND-002 | still-open | At the reviewed head the public constructor still supplies no member facts, while `to_bytes` returns `Malformed(SourceObjects)` for that row-count mismatch. |

FND-001 after excerpt:

```rust
assert!(object_json.get("abstract").is_some());
assert!(object_json.get("is_abstract").is_none());
```
