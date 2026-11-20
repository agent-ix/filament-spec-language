---
id: SR-1529
title: "QSL-2048 finite-window source runner code and Rust review"
type: SpecReview
analysis: code-review
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
  - id: QSL-2048-source-runner
    path: "fsl-conformance/tests/it/fspec_finite_window.rs"
    role: examined
    excerpt: "Published fixture bytes supply expectations; generated native source reaches real parsing, Linked checking, authenticated emission and timestamped public temporal execution."
  - id: QSL-2048-negative-controls
    path: "fsl-conformance/tests/it/fspec_finite_window.rs"
    role: examined
    excerpt: "Altering an independent expected outcome reports expected and actual; missing or empty published corpus fails before a zero-case pass."
  - id: QSL-2048-required-routing
    path: "xtask/src/conformance.rs; xtask/src/conformance/tests.rs; xtask/tests/fixtures/checks.rs"
    role: examined
    excerpt: "The source window selection is required; successful Cargo completion without its positive source summary is refused; the process double follows the current list."
  - id: QSL-2048-test-registration
    path: "fsl-conformance/tests/it/main.rs; fsl-conformance/Cargo.toml; Cargo.lock"
    role: examined
    excerpt: "The existing integration target registers the new module; serde derive is a test-only dependency using the existing locked crate."
  - id: QSL-2048-compile-fix
    path: "fsl-replay/src/lib.rs"
    role: examined
    excerpt: "Remove only a duplicate closed-form re-export, retaining the earlier public exports."
findings_data: []
---

## Summary

Reviewed the frozen QSL-2048 diff for real source execution, independent fixture comparison, Rust boundary handling and required conformance integration. No defects found in the reviewed diff.

## Verdict

PASS. The runner reads the authoritative fixture through the existing pinned published-package API; it adds no ambient checkout requirement, compatibility loader or copied fixture. Typed expected/window records deny unknown fields. Missing bytes, empty windows, duplicate ids and unsupported operators or activation forms produce explicit failures instead of silent success.

Native temporal declarations derive from fixture inputs, not expected outcomes. Parsing, model selection, checking, emission authentication and temporal execution use public APIs. A typed Reading record carries actual Boolean observations; absence is conveyed through the public missing-valuation input. Progress, history boundaries and counter-cover uncertainty map from independent input fields. The published fixture header explicitly defines uncertainty as counter-cover uncertainty, so that translation is grounded.

Outcome, deciding activation when authored, pending deadline, indeterminate reason/position, optional settlement basis and support are compared against independent expectations. Evidence atom translation preserves the fixture's semantic proposition while naming the compiled holds node. Unrecognized execution dispositions fail; a missing finite-window report fails. The altered-oracle control runs real source before changing its expectation.

The required conformance selection uses the existing one-successful-test and positive-summary guards. The new regression refuses missing source execution evidence. The process-double list also includes the already-required clock tuple/union selection, keeping it aligned with the actual list without changing production gate policy. The one-line duplicate re-export deletion retains the original public API. No unsafe code, shared mutable environment writes, new task lifecycle or unbounded external service is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No AssuranceProfile document applies in this checkout. The review follows the repository's AGENTS.md and CLAUDE.md and the ordinary Rust, code, traceability and specification criteria. No workflow or automatic hosted qualification trigger changes.

## Evidence and Limits

Reviewed feature head: `14ea5cc2def156ce458fc9b0dc2ff584fda27a6f`. The existing review is attached to public PR #3192; this metadata update does not repeat the source review.

Accepted author receipts without rerunning: 39 of 46 window rows passed the first actual source execution; seven adapter defects were corrected, followed by eight changed rows passing (including the second changed counter-cover input). The altered-oracle and missing-input controls passed; the required-routing regression passed; scoped Clippy and exact-file formatting exited 0. Full qualification exited 101 as its intended negative control, naming the 31 unadapted cases before source execution. Initial compilation and adapter failures remain preserved.

Requirement FR-350 structural validation subsequently exited 0 through the repository-equivalent `quire validate --scope .` invocation. The missing canonical npm launcher and the earlier wrong-archetype validation remain recorded separately and are not claimed as successful validation.

No tests, builds, lints, gates, matrix or requirement validation were repeated by this reviewer. Final affected-component gate is still the coder's responsibility. This partial delivery does not complete QSL-2048, qualify the unadapted finite-window groups, or supply executable state-core domain packages.

