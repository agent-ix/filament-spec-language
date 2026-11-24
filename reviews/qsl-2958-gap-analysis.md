---
id: SR-967175026302197768900474181941935084809
title: "QSL-2958 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2958; FR-364-AC-15/16; TC-1902; repository test matrix and Rust stub scan"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-364
    type: references
---

## Summary

Ticket: QSL-2958. Frozen head `5a6a6d55c67e8c6ad35130db809929a054efc0c5`, against origin/main `0cdc296949cdc3a0cc3ce989712ef8ab2f77edd1`. Quire matrix completed with exit 0 using `quire matrix --scope . --format json`; no plan was supplied, so plan completion is not assessed. FR-364-AC-15 and AC-16 are tagged in their changed Rust tests. The broad repository matrix reports 5,248 tagged, 1,110 untagged, 109 method-without-symbol, 30 tagged-by-ignored-test and 15 retired criteria. Reverse-gap/stub scan found 34 stub-marker occurrences across 15 Rust files repository-wide; changed production and test files contain no TODO/FIXME/XXX/todo!/unimplemented! markers. No claim is made that the full existing marker inventory is semantically resolved.

## Verdict

FAIL (repository coverage): the computed repository matrix has untagged criteria, including FR-001-AC-7. The changed QSL-2958 slice is tagged and has no discovered source/test stubs. This repository-wide finding predates and is independent of this PR's slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-001-AC-7 has no computed test binder, so its criterion is not covered by a tagged test. | spec/functional/FR-001-read-exact-source.md: - |

## Coverage

Matrix scope: full repository `.`; computation exit 0. Counts: 5,248 tagged; 1,110 untagged; 109 method-without-symbol; 30 tagged-by-ignored-test; 15 retired. Changed slice: FR-364-AC-15 has three test binders in `fsl-replay/src/spine/clause/tests/ordered_end_positions.rs`; FR-364-AC-16 has one binder in `fsl-semantics/tests/it/relationship_extents.rs`. Each test has a `#[trace("TC-1902", "FR-364-AC-15/16")]` tag and exercises the affected path. Matrix diagnostics include unmarked raw trace-id claims from comments/doc comments, including the changed tests, while the criteria themselves are status `tagged` due the function attributes; these are not counted as untagged changed criteria.

Reverse-gap and stub pass: scanned repository Rust files for `TODO`, `FIXME`, `XXX`, `todo!` and `unimplemented!` markers (34 hits in 15 files), then checked every changed production/test file. No new unowned code behavior or executable stub was found in the changed slice. The repository-wide marker hits were not individually adjudicated for semantic ownership, so no repository-wide reverse-gap PASS is claimed. Optional semantic review was not requested and was skipped. No plan was supplied; plan completion is not assessed.

## Examined scope

- FR-364-AC-15: snapshot position parsing, error pointers, admission order, per-group uniqueness, and ordered target navigation.
- FR-364-AC-16: model-layer position ordering, applicability, uniqueness, and refusal context.
- TC-1902: test procedures and expected snapshot/model-layer behavior.
- FR-106: context only for snapshot parsing and document pointers.
- FR-119: context only for semantic-value relationship extent representation.

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
      Verify FR-364-AC-15 and FR-364-AC-16: ordered multi-valued ends carry positions that order navigation, malformed snapshot positions refuse at admission, model-layer extents use the same position rules, and an unbounded end checks only its lower bound.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: context_only
    excerpt: Snapshot admission interprets relationship instance members and reports malformed input at document locations.
  - id: FR-119
    path: ix://agent-ix/filament-semantic-value/FR-119
    role: context_only
    excerpt: Relationship extents carry ordered positions and expose them through semantic-value reads.
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-001-AC-7
    related_ids: []
    path: spec/functional/FR-001-read-exact-source.md
    lines: "-"
    excerpt: >-
      Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: ["Point"], ordinal: 0}` each time: the source reference is no part of a node key.
    finding: >-
      The engine-computed Test Matrix marks FR-001-AC-7 `untagged` with no binders. Its required behavior therefore has no test carrying the criterion trace tag. This is an existing repository-wide coverage gap outside the QSL-2958 changed slice.
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-3118 tracks the unchanged FR-001-AC-7/TC-424 matrix gap. The criterion and document are identical on origin/main, outside this PR's changed scope, and the issue is related to QSL-2958. |
