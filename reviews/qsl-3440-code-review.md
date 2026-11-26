---
id: SR-20261126085141001
title: "code-review review of PR 4057 (QSL-3440)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #4057 diff against origin/main at 3a28a02778d615958f7b732979de19e9d0f86e3f"
review_set: subset
---

## Summary

Ticket QSL-3440, PR #4057 at 3a28a02778d615958f7b732979de19e9d0f86e3f. Code review with the Rust lane over the 17 changed Rust files. Every writer of a written transition found by search goes through WrittenOperation; no wrong value, wrong accept or new panic found.

## Verdict

**CONDITIONAL** - one low traceability finding.

Writers: WrittenTransition (From and read), scheduled_transition and encode_transition_in all reach one rule. ModelTransition canonical bytes still carry identity text where they are used as identity (state and product keys, orderings, Schedule::read_transition policy hints, the weak-access-shape cause text, inspect transition_hex). Operations at those places are declaration identities, so the name rule cannot differ there. model_based_test invocations and inspect model-check edges write an operation identity without a transition type and are outside FR-120's rule.

Producer and checker: certificate source-transition reading is injective over object and name identities, and same_source_transition compares those spellings; no path found that lets a checker accept a different transition. Identity-byte producers are unchanged (no fsl-eval or filament-semantic-value change).

Not a defect of this PR: a name identity beginning ix:// is written bare and rebuilt as {node, package} by the temporal reader only. Main already wrote source-step names bare; arch ruling Q2 removes the form in QSL-3517. The codec restore still restates the name-to-JSON-string step through its metered encoder instead of calling into_identity_text; behaviour agrees.

Gates run by this review at head: fsl-replay library filters written_operation, outcome::, certificate::task_product, certificate::simulation, certificate::product, reference_codec, spine::request, native_result::temporal, task_transition: 112 passed, 0 failed. filament-spec-language it package_model_native: 5 passed. git diff --check clean. rustfmt differences seen in changed files are present on main. Clippy, arch-lint, string-edge, route-lint, checked-input, canonical-types and cargo deny were not rerun; the PR body's results are the only record.

Recipe lines not finished by the coder: execute:: is the fsl-replay execute module's library tests, including the slow 100000-term TC-736 test (QSL-3433). execute.rs and execute/ reference none of the changed writers or readers, so the change cannot reach them. fallback:: matches no test; it comes from the synthetic path fsl-replay/src/fallback.rs in xtask's recipe.

The 39 it_monitor_protocol failures: the same 39 names (all SourceMapInvariant in the main run) appear in the rolling check of main dc71332c3d, an ancestor of this head, and in the qsl-3282-base log. Pre-existing (QSL-3314). No dependency version moves.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-120-AC-40 names the temporal counterexample reader as one that reads each written identity back, and this PR changes that reader to call WrittenOperation::into_identity_text, but read_document's source_transition carries only #[implements("FR-120-AC-39")]. quire trace --id FR-120-AC-40 lists no implements entry for the temporal reader, so a later change to it is not linked to AC-40. | fsl-replay/src/outcome/counterexample.rs:438-442 |
