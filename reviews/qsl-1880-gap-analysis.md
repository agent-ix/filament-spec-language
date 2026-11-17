---
id: SR-2026111612172400000017
title: "Scoped gap analysis of filament-spec-language PR #2812"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2812 partial source-object emission for QSL-1880 and target emission for QSL-2314"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-569", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-390", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-088", type: references}
---
# Scoped gap analysis of PR #2812

## Summary

Ticket scope: QSL-1880 and the target-emission slice of QSL-2314. Exact reviewed head: `e5f37571c847c9df735ea3c645f4e1fa73d9ead5`. The full Quoin gap-analysis method could not run: installed `quoin` is 0.29.4 and has no `gap-analysis` command. A manual scoped check used `quire matrix --scope . --format json` and inspected the tagged tests. It covered source object emission/identity ACs, source-operation identity rows, FR-088-AC-13, and FR-531 identities/4 encoding. The implementation's reader-dependent ACs are explicitly blocked on QSL-2347; that dependency is recorded as a blocker, not a code finding.

## Verdict

FAIL. The source-object identity encoding has one high implementation defect, and FR-569-AC-7 has no trace-tagged test. The QSL-2347 reader joins remain pending and are outside this PR's FSL ownership slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The source-object identity rows emitted into identities.json diverge from the required identities/4 schema, so tagged emission/roundtrip tests do not establish conformance for bytes-only consumers. | fsl-package/src/bare.rs:584-603 |
| FND-002 | medium | FR-569-AC-7 is untagged: no test in the computed matrix verifies source object node identity stability and rename behavior. | fsl-package/src/emit/tests/source_objects.rs |

## Coverage

Plan completion: not assessed.

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

- The matrix reports FR-569-AC-7 `untagged`. FR-569-AC-6, FR-569-AC-8 and FR-390-AC-70 are tagged. FR-531-AC-8 is tagged by a general handoff identity-file test, but the source-object-specific roundtrip test uses the same local encoding and decoder, not an exact JSON-schema assertion.
- The reported targeted source-object tests passed 7 and ignored 4 reader-dependent cases. FR-390-AC-71/72/73 have no tagged tests in the matrix and remain explicitly pending QSL-2347's source dispatch, operation-body and timed-place reader joins; no separate defect is assigned to that external dependency.
- Quoin's `gap-analysis` command is missing: `quoin --version` reports 0.29.4, and `quoin gap-analysis --help` exits 1 with `command gap-analysis not found`. Manual scoped matrix review was used. `quire matrix --scope . --format json` is repository-wide, so it was filtered to the relevant FR rows and not used to claim whole-repository coverage.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new FR-569-AC-7 test does not verify the required distinction between source-owned and selected-model `Order` node ids; its TC tag also points to the reader-operation case. | fsl-package/src/emit/tests/source_objects.rs:416-443 |

### FND-003 — FR-569-AC-7 coverage is incomplete

The added test checks source-node stability under field addition, operation rename and declaration movement, plus a changed id on object rename. FR-569-AC-7 also requires that the source-owned `Order` node id differ from the selected model-owned `Order` node id; no assertion compares those ids. The test tags `TC-1407`, whose scope is operation-reader evaluation, rather than `TC-1408`, which covers FR-569-AC-7.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Head `7a0fe577fd510f47da4f11d64e37143b2d0fac5c` emits the required abstract key and source-record-shaped member keys; the focused identity-row roundtrip test passed. |
| FND-002 | fixed | The new test is tagged to FR-569-AC-7 and the computed matrix now reports that criterion as tagged. |
| FND-003 | still-open | The new test omits the selected-model/source-owned id comparison required by AC-7 and uses the unrelated TC-1407 tag. |

FND-001 after excerpt:

```rust
assert!(object_json.get("abstract").is_some());
assert!(object_json.get("is_abstract").is_none());
```

FND-002 after excerpt:

```rust
#[trace("FR-569-AC-7", "TC-1407")]
#[test]
fn source_object_node_identity_tracks_only_the_object_declaration_identity() {
```
