---
id: SR-1530
title: "QSL-2048 finite-window source runner scoped gap analysis"
type: SpecReview
analysis: gap-analysis
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

Examined the changed requirement-to-test-to-code chain, reverse ownership, stubs and coverage inflation for the QSL-2048 diff. No defect found in this bounded scope.

Plan completion: not assessed

## Verdict

PASS for the scoped diff, not a repository-wide assurance claim. FR-350-AC-20 is bound to actual source-to-verdict comparisons and the altered-oracle/missing-input controls. FR-350-AC-21 is bound to accounting, the full-qualification refusal path and required routing. The timestamped-valuation behavior exercised by the standard windows is consistent with the existing FR-501-AC-4 contract.

The semantic evidence is real compiler and evaluator behavior rather than copied expected output or a source-substring assertion. Emission is authenticated before temporal execution. No branch manufactures a successful refusal to stand in for a missing producer. All 46 supported window ids execute in the normal source selection; the remaining 31 are explicitly identified as unadapted and cannot satisfy full qualification.

The new implementation has clear ownership in FR-350. Its private test helpers are not exposed as new production interfaces. The duplicate-export compile repair preserves the existing closed-form API. No stubs, placeholder verdicts, vendored standard data or fabricated model semantics were found.

State-core abstract judgments lack the authoritative executable domain inputs needed for the requested runner. The PR and local ruling question preserve that gap, rather than claiming those fixtures were exercised. QSL-2048 therefore remains partial after this delivery.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No AssuranceProfile document applies in this checkout. The review follows the repository's AGENTS.md and CLAUDE.md and the ordinary Rust, code, traceability and specification criteria. No workflow or automatic hosted qualification trigger changes.

## Scope Boundary

The user and lead explicitly prohibit a repository-wide matrix rerun or audit. This review therefore checks changed trace tags and actual semantic assertions directly; it does not claim that all repository criteria are tagged, all source has been inventoried, or the program plan is complete.

## Evidence and Limits

Reviewed feature head: `14ea5cc2def156ce458fc9b0dc2ff584fda27a6f`. The existing review is attached to public PR #3192; this metadata update does not repeat the source review.

Accepted author receipts without rerunning: 39 of 46 window rows passed the first actual source execution; seven adapter defects were corrected, followed by eight changed rows passing (including the second changed counter-cover input). The altered-oracle and missing-input controls passed; the required-routing regression passed; scoped Clippy and exact-file formatting exited 0. Full qualification exited 101 as its intended negative control, naming the 31 unadapted cases before source execution. Initial compilation and adapter failures remain preserved.

Requirement FR-350 structural validation subsequently exited 0 through the repository-equivalent `quire validate --scope .` invocation. The missing canonical npm launcher and the earlier wrong-archetype validation remain recorded separately and are not claimed as successful validation.

No tests, builds, lints, gates, matrix or requirement validation were repeated by this reviewer. Final affected-component gate is still the coder's responsibility. This partial delivery does not complete QSL-2048, qualify the unadapted finite-window groups, or supply executable state-core domain packages.

