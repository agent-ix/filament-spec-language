---
id: SR-1795275120196374126
title: "Base review of QSL-2501 FSL protocol-analysis outcome contract"
type: SpecReview
analysis: base
scope: "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md: protocol-analysis amendment and AC-33 to AC-37"
review_set: base
---

## Summary

PASS for the bounded pre-implementation serialization contract. The FR-286 amendment defines the canonical protocol-analysis counterexample body, preserves native evidence and originating-item association, requires the actual Source regression to produce violation/exit 10, and supplies positive and adverse criteria for both realizability variants and both projection decisions. No blocking specification findings were identified.

This is one independent base checklist review, as selected by Peter. The subject is FR-286 at commit 005cfb391230d39d175dc173096e473f25cec2c9 compared with a10af51. The read-only source worktree was not changed. This report is delivered in the authorized cache for the lead to copy into the repository's established spec/reviews root. The seven optional analyses, implementation acceptance, runtime verification, test matrix, Cargo and aggregate gates are outside this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No specification defects found in the bounded amendment; production repair and executable acceptance evidence remain outstanding as FR-286 Status states. | FR-286 protocol-analysis refutations; FR-286-AC-33 through FR-286-AC-37 |

## Checklist assessment

The new AC ids are unique and continue the existing sequence as AC-33 through AC-37. All 37 FR-286 criterion-row ids are unique and sequential; local Markdown file links resolve. FR-286 already traces to US-029, whose library/CLI outcome-equivalence user value supports this amendment. The new rows name the verification method Test without embedding TC ids. No added requirement creates a digest, pin, manifest, tracking record, file scan or tool-version prescription. Existing unrelated FR-286 text was context only.

Inputs and outputs are explicit: genuine typed realizability or local-projection Refuted results must become the one FR-331 ProtocolAnalysisCounterexample union alternative, as a separate counterexample member of the refuted analyze item. Source and checked-Package use the same record/item/spelling/counterexample-taking contract, preserving request index, declared claim and admitted request. FR-331's indexed entry retains request_index and backend; FR-286 incorporates that canonical contract rather than redefining its association. The item keeps its result and violation category, and the document keeps filament-outcome/1, analyze, ranked category and FR-621's completed-analysis stage.

The normative body descriptions agree with the concrete parallel FSpec FR-331 contract at 550d97404abd85ad3be22e1b449e2a83b8aceb2b and proposals/backend-provider-v1/schema.json: both histories/actions and obligation/witness for indistinguishable histories; branch-ordered play-only entries with top-level obligation/witness for environment defeat; and complete Choice or ActingInstance projection decisions. Native authority was inspected read-only in fsl-replay/src/protocol_analysis/strategy.rs (Play, PlayStep, BranchDefeat and RealizabilityCounterexample) and artifact.rs (ProjectionDecision and ProjectionCounterexample) in the QSL-2214 worktree. No native variant is silently omitted.

FR-331 incorporation preserves existing key JSON-u64 values, U64String play indices and acting-instance ordinals/maximum, all node/action/transition encodings and FR-351 witness fields. Send records are required exactly on send steps, omitted on non-send steps and never null. FR-352's record continues to exclude projected graphs, strategies and counterexample bodies; record serialization cannot substitute for the body. FR-286's published schema has an open item definition and does not model counterexample, so no local outcome-schema edit is needed to admit this body. Structural schema admission explicitly does not establish association or refutation truth.

The concrete existing Source request references Handover, handover-hidden-choice.fsl, lock.semantic-ir.json and lock-free.json, matching AC-36. That criterion requires one indexed refuted item, a genuine body, a non-null FR-621 stage and exit 10; the reported exit-30 runtime_invariant/items-empty/stage-null behavior cannot satisfy it. This review did not execute the reproduction or assert it repaired.

## Coverage rules and applicability

Criterion coverage by implemented test trace tags is deliberately unassessed: this is contract review before Code, and no matrix was run. AC-33 through AC-35 require genuine results through both intake routes and cover both realizability kinds and both projection decision forms. AC-34 checks loss, reordering and substitution of branch evidence. AC-37 incorporates FR-331's positive/adverse contract vectors, including numeric and pair-shape boundaries, and additionally rejects missing/unknown/mixed/null members, send-record misplacement, a missing body, a different request's body, a record-only substitute, graph substitution and result downgrades. AC-36 covers the real Source failure. Together these address option permutations, relevant constraint boundaries, error paths and edge cases for the serialization change. New state-machine behavior, performance targets and security mechanisms are not introduced by this amendment; existing replay/work-limit and association constraints are inherited from FR-331.

The installed catalog resolves AssuranceProfile to engineering-assurance. A repository Markdown search found no AssuranceProfile document and no review_policy declaration in this scope; therefore no applicable profile mandates an enforced spec-review run. This direct review does not invent a human acceptance gate or discharge code-review/gap-analysis operations.

## Validation

The SpecReview authoring pack was fetched once with quoin write . --types SpecReview --json from the subject repository; its installed skeleton and schema supplied this report's contract. Tools resolved to /home/peter/.npm-global/bin/quoin (0.36.8) and /home/peter/.npm-global/bin/quire (0.52.0, engine 0.62.2).

Targeted quire validate --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2501-outcome-contract on FR-286 exits 0. It reports two EARS grammar warnings at amended line 382, exactly matching the baseline's warnings at old line 331; no new structural error is reported. Fresh stderr is retained beside this report as subject-quire.stderr and baseline-quire.stderr. Full-repository validation and unrelated legacy specification findings are outside the authorized bounded review.

The cached report validates with exit 0 both with the standalone installed SpecReview module plus explicit repository scope and with composed repository-scope module discovery. Standalone validation reports existing UnknownEdgeType/UnknownRole registry warnings for ADR/TestMatrix-related definitions; composed validation emits no diagnostics. Fresh report-standalone.stderr and report-composed.stderr are retained beside it. The source worktree remains clean. No runtime checks or source edits were performed.
