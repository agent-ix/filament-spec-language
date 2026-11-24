---
id: SR-967175026302197768900474181941935084808
title: "QSL-2958 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2958; fsl-semantics/src/model/observation/{relationships,extents}.rs; fsl-semantics/tests/it/relationship_extents.rs; fsl-replay/src/spine/clause/tests/ordered_end_positions.rs; FR-364-AC-15/16; TC-1902"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-364
    type: references
---

## Summary

Ticket: QSL-2958. PR #3684 at `5a6a6d55c67e8c6ad35130db809929a054efc0c5`; reviewed against origin/main `0cdc296949cdc3a0cc3ce989712ef8ab2f77edd1`. Rust implementation and test changes reviewed, including the affected semantic-value API callers and wire restore/projection paths. No changes made. The duplicate semantic-value dependency route through filament-mltl is documented in the PR and tracked as QSL-3116; it is a separate gate blocker.

## Verdict

FAIL: two scoped defects remain in malformed-position diagnostics and model-layer refusal context. Apart from these findings and the explicitly tracked QSL-3116 workspace-check blocker, no other merge blocker was identified in the reviewed slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Model-layer position refusals incorrectly include a snapshot document pointer, contrary to FR-364-AC-16's model-layer diagnostic contract. | fsl-semantics/src/model/observation/extents.rs:265 |
| FND-002 | low | Unknown position keys containing `.` or `/` are converted into incorrect document pointers. | fsl-semantics/src/model/observation/relationships.rs:619 |

## Examined scope

- FR-364-AC-15: snapshot ordered position parsing, admission ordering, duplicate-group validation, pointer reporting and ordered navigation.
- FR-364-AC-16: model-layer position validation, ordering, uniqueness groups and refusal fields.
- TC-1902: test procedure and changed tagged tests for snapshot and model-layer positions.
- FR-119: context only, semantic-value `RelationshipExtent` ordered positions API.
- FR-106: context only, snapshot admission diagnostics and document pointers.

FND-001: the model-layer admission path reuses `extent_refusal`, which adds `document_pointer` for missing, unexpected and duplicate positions. AC-16 requires model-layer refusals to name the relationship/end without a snapshot pointer. Consumers can otherwise mistake a synthesized `/relationships/...` path for a location in a snapshot document. Preserve the relationship/end context without `document_pointer` for model-layer errors; keep precise pointers on snapshot admission errors. Add an assertion that the model-layer refusal has no `document_pointer`.

FND-002: unknown keys are encoded as `positions.{name}` and later converted by replacing every dot with `/`; slash and tilde are not JSON Pointer escaped either. For a supplied unknown key `odd.name`, this reports `/positions/odd/name` instead of `/positions/odd.name`; `odd/name` also yields a malformed pointer. Preserve the original member name and apply JSON Pointer escaping so the refusal locates the actual unknown key.

## Verification

`cargo fmt --check` failed on unrelated existing files outside the PR diff (including `xtask/src/merge_gate_recipe.rs`, `xtask/src/refinement/compare.rs`, and `xtask/tests/grammar_drift.rs`). No formatting changes were made. The PR's locked workspace all-target check failure is separately documented as the QSL-3116 duplicate identity route through filament-mltl. Filtered test execution was not repeated during this review; the PR body records the focused compile check, and the tagged tests were inspected for assertion intent.

## Scope data

```yaml
clean: false
scope:
  - id: FR-364-AC-15
    path: spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md
    role: examined
    excerpt: >-
      For a relationship whose target end is ordered `{0,3}`, a snapshot entry whose instances for source `o1` carry `positions.target` `"7"`, `"2"` and `"5"` navigates from `o1` to the three targets in ascending numeric position, whatever order the entry lists them in. If the second instance omits `positions.target`, condition 5 refuses `invalid_runtime_input`/`invalid-value` at `/relationships/0/instances/1`; if it supplies `positions.source` for the unordered source end, the refusal is at `/relationships/0/instances/1/positions/source`; and if the second of two instances of `o1` repeats target position `"2"`, the refusal is at `/relationships/0/instances/1/positions/target`. A non-object `positions` refuses `wrong-value-kind` at `/relationships/0/instances/1/positions`; a JSON-number target position refuses `wrong-value-kind` at `/relationships/0/instances/1/positions/target`; a noncanonical or out-of-range decimal string refuses `invalid-value` at that target member; an unknown key `other` refuses `unknown-member` at `/relationships/0/instances/1/positions/other`. The missing, extraneous and duplicate position cases are condition 5, after condition 3 end-type and condition 4 repeated-instance checks.
  - id: FR-364-AC-16
    path: spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md
    role: examined
    excerpt: >-
      A model-layer `RelationshipExtent` on the same ordered `{0,3}` relationship with target positions 7, 2 and 5 navigates in ascending position, regardless of instance-list order. Equal positions on instances with different source references admit; equal positions on instances sharing a source refuse `invalid_runtime_input`/`invalid-value` naming the relationship and `target`. Omitting a required position refuses the same code and cause naming the missing end; supplying one for the unordered source end or for a single-valued ordered end refuses the same code and cause naming that end. The source-group uniqueness, required-position and applicability decisions run after the current instance's end-type and repeated-instance checks and before admission of the next instance.
  - id: TC-1902
    path: spec/test-cases/TC-1902-ordered-end-positions-and-unbounded-end-counts.md
    role: examined
    excerpt: >-
      Verify ordered-end positions are admitted, used for navigation in ascending numeric order, and rejected when malformed, missing, duplicated within an opposite-end group, or supplied for an inapplicable end; also verify unbounded-end counts.
  - id: FR-119
    path: ix://agent-ix/filament-semantic-value/FR-119
    role: context_only
    excerpt: Relationship extents carry optional ordered positions and reads use their numeric ordering.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: context_only
    excerpt: Snapshot admission owns document-pointer diagnostics for snapshot inputs.
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-364-AC-16
    related_ids: [FR-119]
    path: fsl-semantics/src/model/observation/extents.rs
    lines: "265-271"
    excerpt: >-
      A model-layer `RelationshipExtent` on the same ordered `{0,3}` relationship with target positions 7, 2 and 5 navigates in ascending position, regardless of instance-list order. Equal positions on instances with different source references admit; equal positions on instances sharing a source refuse `invalid_runtime_input`/`invalid-value` naming the relationship and `target`. Omitting a required position refuses the same code and cause naming the missing end; supplying one for the unordered source end or for a single-valued ordered end refuses the same code and cause naming that end. The source-group uniqueness, required-position and applicability decisions run after the current instance's end-type and repeated-instance checks and before admission of the next instance.
    finding: >-
      Model-layer missing-position refusals add `document_pointer` through the shared extent_refusal mapper, even though FR-364-AC-16's model-layer refusal contract names the relationship/end and does not expose a snapshot document pointer. Keep the model-layer refusal context-free of document pointers and assert the field is absent in its test.
  - fnd: FND-002
    severity: low
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-364-AC-15
    related_ids: [FR-106]
    path: fsl-semantics/src/model/observation/relationships.rs
    lines: "619-620"
    excerpt: >-
      For a relationship whose target end is ordered `{0,3}`, a snapshot entry whose instances for source `o1` carry `positions.target` `"7"`, `"2"` and `"5"` navigates from `o1` to the three targets in ascending numeric position, whatever order the entry lists them in. If the second instance omits `positions.target`, condition 5 refuses `invalid_runtime_input`/`invalid-value` at `/relationships/0/instances/1`; if it supplies `positions.source` for the unordered source end, the refusal is at `/relationships/0/instances/1/positions/source`; and if the second of two instances of `o1` repeats target position `"2"`, the refusal is at `/relationships/0/instances/1/positions/target`. A non-object `positions` refuses `wrong-value-kind` at `/relationships/0/instances/1/positions`; a JSON-number target position refuses `wrong-value-kind` at `/relationships/0/instances/1/positions/target`; a noncanonical or out-of-range decimal string refuses `invalid-value` at that target member; an unknown key `other` refuses `unknown-member` at `/relationships/0/instances/1/positions/other`.
    finding: >-
      The member path is reconstructed by replacing dots with slashes and does not escape JSON Pointer special characters. An unknown key `odd.name` is reported at `/positions/odd/name` rather than `/positions/odd.name`; preserve the raw key and RFC 6901-escape it.
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `extent_refusal` now returns only relationship and end for model-layer position errors; the test asserts `document_pointer` is absent. |
| FND-002 | fixed | Unknown-key pointers escape `~` and `/` with JSON Pointer rules; filtered wire-shape coverage asserts dot, slash, and tilde key locations. |
