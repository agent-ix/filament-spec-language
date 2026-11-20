---
id: SR-1531
title: "QSL-2048 FR-350 source conformance criterion review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2048; fsl-conformance/tests/it/fspec_finite_window.rs; fsl-conformance/tests/it/main.rs; fsl-conformance/Cargo.toml; Cargo.lock; xtask/src/conformance.rs; xtask/src/conformance/tests.rs; xtask/tests/fixtures/checks.rs; fsl-replay/src/lib.rs; spec/functional/FR-350-run-the-conformance-corpus-against-fsl.md"
review_set: subset
pr: 3192
branch: codex/qsl-2048-standard-source-fixtures
head: 14ea5cc2def156ce458fc9b0dc2ff584fda27a6f
model: unknown
quoin_skill: 0.31.1
module: spec-artifacts-process@0.2.0
clean: true
reviewed_units:
  - id: FR-350-AC-20
    path: "spec/functional/FR-350-run-the-conformance-corpus-against-fsl.md"
    role: examined
    excerpt: "The pinned published FSpec package supplies the authoritative fixture bytes; the finite-window `window` group is translated to native temporal source and run through public parse, check, emission authentication and temporal execution. Each case asserts its independently authored outcome, activation, deadline, reason, evidence position and, when present, settlement basis and decision support."
  - id: FR-350-AC-21
    path: "spec/functional/FR-350-run-the-conformance-corpus-against-fsl.md"
    role: examined
    excerpt: "The finite-window source runner accounts for every fixture case by id as executed or lacking an adapter. Unsupported groups do not count as executed. The window runner is a required conformance selection with a nonzero execution summary; requesting full qualification with `FSPEC_FULL_QUALIFICATION` set fails while any fixture case lacks a source adapter."
  - id: FR-501-AC-4
    path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md"
    role: examined
    excerpt: "Stamps 0, 2, 5, `eventually[0,5] holds(p)`, `W` 5, `Complete { through: 5 }`: with `p` false at 0 and 5 and its valuation absent at stamp 2, the result is `Indeterminate { where: 0, reason: MissingValuation { position: 1, atom: p } }`; with `p` true at 5 and absent at 2 it is `Completed(true)`;"
findings_data: []
---

## Summary

Reviewed the two added FR-350 criteria in the same pass as their implementation and tests. No requirement defect found in this confined change.

## Verdict

PASS. AC-20 states the authoritative input source, real compiler and temporal execution path, compared output fields, altered-expectation failure and missing-input failure. AC-21 distinguishes executed windows from unsupported groups, requires observable nonzero source execution, and defines an explicit full-qualification failure while adapters are missing. The criteria are falsifiable by incorrect language results, an altered independent expectation, missing input, missing execution evidence or incomplete requested qualification.

The new ids continue the existing FR-350 sequence as AC-20 and AC-21, and their verification cells name Test without inventing a new test-case id. Both have real trace bindings in the changed tests. The specification does not claim every QSL-2048 acceptance criterion is delivered. Normal partial execution and full qualification are distinguished explicitly, consistent with the PR's unserved state-core and other finite-window groups.

The reference to the existing pinned published package identifies the repository's authoritative dependency access. It adds no secondary hash ledger, copied schema, version record, manifest or file-tracking system. No new requirement statement, dependency edge or domain object is introduced, so separate EARS, dependency or object sweeps are unnecessary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No AssuranceProfile document applies in this checkout. The review follows the repository's AGENTS.md and CLAUDE.md and the ordinary Rust, code, traceability and specification criteria. No workflow or automatic hosted qualification trigger changes.

## Review Selection

Base checklist scoped to the two added acceptance criteria. Structural requirement validation is accepted from the author's successful confined validation, without rerunning it. No separate analysis sweep is required for unchanged requirement statements, relationships or domain objects.

## Evidence and Limits

Reviewed feature head: `14ea5cc2def156ce458fc9b0dc2ff584fda27a6f`. The existing review is attached to public PR #3192; this metadata update does not repeat the source review.

Accepted author receipts without rerunning: 39 of 46 window rows passed the first actual source execution; seven adapter defects were corrected, followed by eight changed rows passing (including the second changed counter-cover input). The altered-oracle and missing-input controls passed; the required-routing regression passed; scoped Clippy and exact-file formatting exited 0. Full qualification exited 101 as its intended negative control, naming the 31 unadapted cases before source execution. Initial compilation and adapter failures remain preserved.

Requirement FR-350 structural validation subsequently exited 0 through the repository-equivalent `quire validate --scope .` invocation. The missing canonical npm launcher and the earlier wrong-archetype validation remain recorded separately and are not claimed as successful validation.

No tests, builds, lints, gates, matrix or requirement validation were repeated by this reviewer. Final affected-component gate is still the coder's responsibility. This partial delivery does not complete QSL-2048, qualify the unadapted finite-window groups, or supply executable state-core domain packages.

