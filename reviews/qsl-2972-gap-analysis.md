---
id: SR-967175026302197768900474181941935084814
title: "Gap analysis of QSL-2972 host allocation propagation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; changed fsl-package, fsl-replay, fsl-analyze, fsl-command paths; FR-096-AC-24 and FSpec FR-330-AC-17"
review_set: subset
---

## Summary

Targeted acceptance-criterion-to-code/test gap analysis for PR #3687 at `0e56a371e7ca83b7ff366347e41017a22d9cb484`. The FSL matrix has no local FR-096-AC-24 test in fsl-command, model-check replay violates the no-package-refusal obligation, and two tests' FSpec tags bind to an unrelated local criterion.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Model-check replay maps HostAllocationFailed to malformed-package refusal | fsl-replay/src/model_check/replay.rs:905-909 | implementation-bug-despite-evidence |
| FND-002 | medium | fsl-command's reader consumer lacks an injected-failure criterion test | fsl-command/src/command.rs:5667 | correct-requirement-no-evidence |
| FND-003 | medium | FSpec tags in hybrid and command tests are unprefixed and bind locally | fsl-replay/src/hybrid/process_request.rs:619; fsl-command/src/command.rs:5665 | correct-requirement-no-evidence |

## Verdict

Targeted gap analysis is conditional. `quire matrix --scope . --format tsv` succeeded; FR-096-AC-24 is tagged by tests in fsl-analyze, fsl-package, and fsl-replay, but not fsl-command. The relevant hybrid test exercises the enum-to-mapper path. The model-check consumer remains an uncovered and failing path. The local FR-330-AC-17 matrix row is incorrectly credited with the two tests whose comments claim FSpec FR-330-AC-17.

`quoin matrix --repo . --json` was attempted and refused with `CORE_REFUSED: request exceeds the accepted size`; the documented static matrix fallback (`quire matrix`) was used. `quire coverage --scope . --json` completed but reports repository-wide baseline diagnostics and many untracked symbols; no new FSpec-prefixed trace tags are present in the changed tests. The repository already has many untagged matrix criteria, so this is a scoped result, not a whole-repository coverage claim. Plan completion: not assessed (no plan supplied).

## Scope examined

- FR-096-AC-24, requiring injected host allocation failure evidence for every consumer in fsl-package, fsl-replay, fsl-analyze, and fsl-command.
- FSpec FR-330-AC-17, requiring no package/feature-accounting record and host-error reporting.
- Changed Rust tests and production branches in all four listed crates, including hybrid and model-check replay.

## Matrix and reverse-gap evidence

`quire matrix --scope . --format tsv` reports FR-096-AC-24 tagged at `fsl-analyze/src/analyze.rs:5082`, `fsl-package/src/package_admission.rs:419`, and `fsl-replay/src/request/package.rs:652`; it has no fsl-command tag. The FSpec FR-330-AC-17 criterion has no valid cross-repository test tag in this FSL matrix. The bare tags at `fsl-replay/src/hybrid/process_request.rs:619` and `fsl-command/src/command.rs:5665` resolve to local `FR-330-AC-17`, which is a different temporal-run requirement.

A `quire coverage --scope . --json` run completed. It reports 24,247 diagnostics and 6,386 untracked symbols repository-wide, reflecting existing baseline state; static inspection of the changed tests confirms the erroneous local binding and absence of FSpec-prefixed tags. No changed files include a stub implementation or a new Cargo dependency.

## Required closure

1. Explicitly preserve host/internal failure through `fsl-replay/src/model_check/replay.rs`, with a consumer-level test.
2. Add fsl-command injected-failure coverage through the consumer path, tagged to FR-096-AC-24.
3. Prefix cross-repository tags as `FSpec-FR-330-AC-17`; retain local criterion tags where the test establishes them.
4. Ensure the analyze diagnostic pair is cataloged/owned or use an already registered host/internal diagnostic pair.
