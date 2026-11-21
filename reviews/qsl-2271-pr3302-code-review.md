---
id: SR-20261121003302001
title: "code-review of PR 3302 generic dependency artifact closure"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2271; frozen PR3302 changed paths and FR-517/850/1039 generic closure, FR-071/940 context"
review_set: subset
---

## Summary

Ticket: QSL-2271. Independent review of the new generic dependency artifact closure, including the Rust code-review lane.

## Verdict

**FAIL** — 2 findings require disposition in the original PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Imported composite dependencies are pruned before the closure type environment is assembled. | fsl-package/src/bare_world.rs:285 |
| FND-002 | medium | Directory native-run changes the established extraneous-model refusal and drops its request pointer. | fsl-command/src/command/handoff.rs:451 |

### FND-001

Each flat dependency view has no dependency_views of its own (bare.rs:140-155). world_of first calls closed_composites against only that package’s represented handles, then asks every dependency for an already closed world. An imported composite handle is therefore treated as missing at lines 105-109, and a local record/union containing it is discarded before the authenticated imported declaration is merged. A library Z containing a record field of test/a::A (or the equivalent root record) loses that declaration even though A is in the retained closure; constructing/reading it cannot preserve source behavior. Collect the closure’s authenticated candidate declarations before closure filtering/admission, and add a source-deleted imported nested-composite regression. Static control-flow finding; no runtime reproduction was authorized.

### FND-002

admit_files now delegates directory requests to handoff::admit. Its unused supplied model branch always returns AnalyzePackage invalid_package/conflicting-definition with pointer None, whereas the preserved file branch returns invalid-request at /request/models/<i>. FR-1039’s existing AC-3 requires the latter for a model not selected by the package; adding dependency-only model acceptance must retain it when neither root nor dependency selects the model. Keep the shared admission diagnosis typed so the run adapter can retain its established code and indexed pointer; analyze should retain FR-850’s different mapping.

## Coverage

Scope is the new generic dependency artifact port only. The entire repository matrix was computed, then the seven new criteria were selected: all seven tagged; all seven have no run evidence in Quoin. No whole-repository PASS is claimed. Static coverage extraction exited 0 with repository-wide symbol-collision diagnostics outside this port; no untracked symbols were reported in dependency_handoff.rs or command/handoff.rs. Reverse-gap inventory maps writer/manifest, shared loader, retained owner views, imported projection, replay admission and encoded-byte extension to FR-517/850/1039/071/940; no new unowned surface or live stubs found in changed code. Optional standalone semantic gap sub-analysis was not run. Plan completion: not assessed.

## Evidence and limitations

Reviewed frozen head 75a4f4a04fc4046ec15a830f34b56221d55859d7 against integrated main 2dd3b2f41f93bde8be3e2644db5df89d693e73c6. No source/branch changes or Cargo/build/gates were run, as directed by the lead. Target remains absent. Coder evidence: 32 focused tests across five invocations, only 19 at final head; final command-only Clippy passed. Earlier package/analyze/output tests are not exact-head proof. Aggregate final gate remains with lead/Draco. CI/Cargo/lock/lint-policy files are unchanged. Composition/derived-slot work QSL-2739 and atom/refset follow-ups are excluded. No selected AssuranceProfile was found.

## Examined scope

```yaml
scope:
- id: FR-517-AC-2
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: The package read from `checked-package.json` with `read_checked_package`
    and evidence assembled from the written files is admitted, and `Selection::check`
    passes; the sidecar's clause ids, names, requirement entries, activation forms,
    profile selections, definition authority and `TemporalBinding` rows, those of
    clauses and those of compensation templates, equal the values read from the checked
    unit held in memory by the writer.
- id: FR-517-AC-27
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: A root importing `test/z`, which imports `test/a`, writes the manifest
    vector of "The dependency artifact closure" and exactly one artifact/witness triple
    in each indexed directory. Each triple is admitted with that package's own direct
    dependencies and witnesses, and the root is admitted with the retained closure.
    Reordering the caller's library provisions does not change any written byte. A
    unit with no imports writes the required manifest with an empty list and no dependency
    directory.
- id: FR-517-AC-28
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: For a diamond whose root imports `test/left` and `test/right`, both importing
    `test/shared`, the manifest contains three unique rows ordered `test/left`, `test/right`,
    `test/shared`, with directories `dependencies/0`, `dependencies/1`, `dependencies/2`;
    the shared package and its witnesses occur once. Two writes give equal trees.
    A conflicting `test/shared` selection is refused under the existing dependency
    conflict rule and leaves no output directory, never a tree containing one arbitrarily
    chosen shared artifact.
- id: FR-850-AC-4
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: AC-1's package request with `program` also present, with a `libraries`
    entry, and with `integer_division` `floor` each refuse `invalid-request` at the
    request stage naming that member, exit 20; with `models` empty it refuses `missing_import`/`missing-selection`
    naming `test/counter`; with an extra `models` entry the package does not record
    it refuses `invalid_package`/`conflicting-definition` naming that entry.
- id: FR-850-AC-12
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: A compiled root calls `test/z::answer`, which calls `test/a::answer` returning
    Integer `42`. Running the root function from source and from its FR-517 directory
    returns `42` on both routes. Its Counter temporal claim `always holds(c.value
    <= z::answer() - 40)` is analyzed with the same models/documents/items on both
    routes and gives equal item records, counterexamples and reproduced replays except
    the request selection. Deleting all unit/library source files and the handoff's
    `sources/` before the package run and analyze/replay changes neither result. Removing
    `dependencies.json` refuses `inva
- id: FR-850-AC-13
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: FR-517-AC-28's diamond is admitted once per unique library and settles
    the source route's result. A missing transitive artifact refuses before an item
    runs; a manifest entry for a foreign or conflicting library refuses `conflicting-definition`;
    an unreachable row, a duplicate identity, reversed rows, an unknown row member,
    a nonindexed path, `../escape`, an absolute path and a selected symlink/non-regular
    file each refuse by the dependency admission table at their own file or pointer.
    A lock cycle refuses `definition-cycle`. The malformed-manifest and omitted-selection
    mutations in the supplem
- id: FR-850-AC-14
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: For AC-12's transitive handoff, removal of either dependency's `identities.json`
    or `equality.json` refuses `missing-file` naming that indexed file even when the
    equality lists would be empty. Swapping non-empty witness files between distinct
    admitted libraries refuses their original identity/equality join error at the
    foreign file, before any item runs; no dependency receives the root's witness.
    The unchanged handoff completes.
- id: FR-850-AC-15
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: For AC-12's handoff, compute `B` as the bytes of exactly the root, manifest,
    unique dependency package/witness files and selected model/runtime files read.
    At `dependent_bytes = B` intake completes; at `B - 1` it settles `incomplete`,
    `resource_exhausted`, exit `22`, naming `dependent_bytes` and its configured bound.
    A handoff whose manifest expands the selected-file total from 64 to 65 stops at
    the same phase naming the shared file limit before opening file 65. A diamond
    charges its shared triple once. For strict-read/native-join work and storage,
    an allowance equal to the completed closure p
- id: FR-850-AC-16
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: A library in AC-12's closure selects domain document `test/library-domain`,
    which the root does not select, and the request supplies both that document and
    the root's domain documents. Admission binds each document to the selecting package
    and completes without an extraneous-model refusal. Removing the library-only document
    refuses `missing_import`/`missing-selection` naming its identity; replacing it
    by content the library selection refuses preserves that reader refusal at the
    dependency package. No witness from the root is substituted.
- id: FR-1039-AC-3
  path: spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md
  role: examined
  excerpt: 'Each defect of "A package that cannot run" refuses at stage `request`,
    exit 20, with the code and locus its row states: a missing file; a document whose
    `contract_version` is `filament.checked-package/v1` (`unknown_wire`/`unsupported-wire`
    with `actual` `filament.checked-package/v1` at `/request/package/contract_version`,
    never `invalid_package`); a package with an unknown top-level member (`invalid_package`
    at the reader''s pointer); a spine-model package run with no `models` entry for
    its recorded selection (`missing_import`/`missing-selection` at `/request/models`);
    and the value package run'
- id: FR-071-AC-7
  path: spec/functional/FR-071-implement-typed-replay-request.md
  role: context_only
  excerpt: A request whose encoded size exceeds the configured reader bound refuses,
    and no truncated or partially-populated request is returned.
- id: FR-940-AC-22
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: context_only
  excerpt: AC-4's Counter request over source with the items `Bounded` and `AtMostTwo`,
    run with `replay.input_bytes` set below the length of the package's emission (the
    up-front read-back is bounded by `i2.input_bytes` and admits the package), settles
    `AtMostTwo` `inconclusive` with cause `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded`
    naming `replay.input_bytes`, never `refuted`, never `failed` with `analyze`/`package-emission`,
    and settles `Bounded` `proved` as without the setting; the document's category
    and exit follow its items (category inconclusive, never `internal-failure
- id: TC-1312
  path: spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: Verify the checked-package handoff writer on the ported IT-001 recipe **R**.
- id: TC-2410
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: Verify that a `native-analyze/1` request selecting a v2 checked package
    is
```

## Source custody

- fsl-analyze/tests/it/clause_item.rs
- fsl-analyze/tests/it/model_world.rs
- fsl-analyze/tests/it/terminal_two_models.rs
- fsl-command/src/checked_handoff.rs
- fsl-command/src/checked_handoff/selection.rs
- fsl-command/src/command.rs
- fsl-command/src/command/analyze.rs
- fsl-command/src/command/dependency_manifest.rs
- fsl-command/src/command/export.rs
- fsl-command/src/command/handoff.rs
- fsl-command/src/command/output.rs
- fsl-command/src/command/package_run.rs
- fsl-command/src/command/synthesis.rs
- fsl-command/tests/it/dependency_handoff.rs
- fsl-command/tests/it/main.rs
- fsl-package/src/analysis_view.rs
- fsl-package/src/bare.rs
- fsl-package/src/bare_slot.rs
- fsl-package/src/bare_state_graph.rs
- fsl-package/src/bare_synthesis.rs
- fsl-package/src/bare_term.rs
- fsl-package/src/bare_terminal.rs
- fsl-package/src/bare_world.rs
- fsl-package/src/checked.rs
- fsl-package/src/checked_v2.rs
- fsl-package/src/dependency_artifact.rs
- fsl-package/src/lib.rs
- fsl-package/src/package_admission.rs
- fsl-package/src/reader_protocol_projection.rs
- fsl-replay/src/execute.rs
- fsl-replay/src/model_check/graph_replay.rs
- fsl-replay/src/model_check/refinement_replay.rs
- fsl-replay/src/model_check/replay.rs
- fsl-replay/src/outcome.rs
- fsl-replay/src/proof_result.rs
- fsl-replay/src/request.rs
- fsl-replay/src/spine/lifecycle.rs
- tests/it/checked_handoff.rs


## Dispositions

Round 1 independently inspected at `477b0027670c8c046e131401eebc111ea20b34a1`, against original reviewed head `75a4f4a04fc4046ec15a830f34b56221d55859d7`. Original findings remain unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | Rejected: the hypothesized cross-library composite field cannot reach this consumer. FR-396 Behavior explicitly forbids imported record/tuple type names, ADR-015 D-5 restricts imported signatures to package-independent types, and assemble.rs:3390-3401 raises ImportedTypeName first. The prototype red log confirms SOURCE assembly refusal rather than a consumer failure. The permitted local-nested-composite/scalar-import control passes unchanged bare_world code from source and source-deleted strict artifacts with Integer 42. The original finding assumed an unsupported source type combination; no consumer fix or typing expansion is required. |
| FND-002 | fixed | Private typed admission failure preserves run/analyze mapping separately; complete file/directory refusal parity and the analyze mapping are asserted. |

## Disposition verdict

**PASS — no open review findings after round 1.** This supersedes the initial verdict for mergeability; final exact-head testing remains with lead/Draco. No new bounded defect was found in the fix diff.

## Round 1 evidence

The reviewer inspected all six changed paths, original artifact custody, source boundary clauses and actual test code. Coder logs at the exact frozen head show six focused tests passed (one per command) and confined command-only Clippy passed; no reviewer Cargo/build/gates ran. The earlier source-assembly failure is counterevidence to the original hypothesis, not a consumer red. The valid nested-local-record control passed with no bare_world change. Target remains retained and the worktree is unchanged.

Additional boundary context: FR-396 Behavior, spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md:86: “A unit cannot name an imported record or tuple as a type (the assembler refuses `ImportedTypeName`), so an own declaration never has a field of a dependency's composite type.” ADR-015 D-5, spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:218, restricts imported signature types to package-independent types. fsl-semantics/src/check/assemble.rs:3390-3401 enforces that boundary before byte admission.

```yaml
head: 477b0027670c8c046e131401eebc111ea20b34a1
dispositions:
- fnd: FND-001
  outcome: rejected
  reason: 'Rejected: the hypothesized cross-library composite field cannot reach this
    consumer. FR-396 Behavior explicitly forbids imported record/tuple type names,
    ADR-015 D-5 restricts imported signatures to package-independent types, and assemble.rs:3390-3401
    raises ImportedTypeName first. The prototype red log confirms SOURCE assembly
    refusal rather than a consumer failure. The permitted local-nested-composite/scalar-import
    control passes unchanged bare_world code from source and source-deleted strict
    artifacts with Integer 42. The original finding assumed an unsupported source
    type combination; no consumer fix or typing expansion is required.'
- fnd: FND-002
  outcome: fixed
  after_excerpt: "Self::UnselectedModel { index, identity } => RunCause::ClauseRequest\
    \ {\n                member: format!(\"/request/models/{index}\"),\n         \
    \       reason: format!(\"the package records no model selection {identity}\"\
    ),\n            },"
  reason: Private typed admission failure preserves run/analyze mapping separately;
    complete file/directory refusal parity and the analyze mapping are asserted.
```
