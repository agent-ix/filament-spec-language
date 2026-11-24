---
id: SR-2026112402191
title: "gap-analysis of PR3636 fair eventual refutation against FR-200, FR-202 and FR-413-AC-5"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/functional/FR-200-decide-every-scheduler-claims-over-fair-schedulers.md, ADR-028 FS-9/WS-1/WS-5, filament-specification FR-413-AC-5; PR3636 f6b1a23f1e6c9d5b3ae279aa2233b14f27cb61ac"
review_set: subset
---

## Summary

Ticket: QSL-2830. This is a manual acceptance-criteria-to-tests check scoped to the PR diff. It is not a full quoin gap audit: matrix and reverse-gap runs were not executed because the brief made the worktree read-only with no builds. The PR implements the fair refutation route and the replay fairness check. The negative half of FR-413-AC-5 (an unfair witness settles `inconclusive`, `replay-parity`) has no test. The re-enabled tests trace only to the package-parity ACs, not to the fairness requirements they now exercise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-413-AC-5's unfair-witness branch (replay-parity) has no test; the PR states none is possible through the analyzer seam, but fsl-replay accepts any untimed subject, so a cyclic-subject unit test with a hand-built unfair witness is feasible | fsl-replay/src/probability_certificate/witness.rs:786-802 |
| FND-002 | low | The two fair tests trace FR-940-AC-6, FR-741-AC-3 and FR-203-AC-8 only; the refutations they now assert (fair minimum 24/25, FR-200-AC-4; fair witness replay) carry no FR-200 trace | fsl-analyze/tests/it/package_probabilistic_refutation.rs:155-210 |

## Verdict

Partial coverage. The positive route is covered: a fair refutation reproduces with source and package parity, under weak and strong, whole and each fairness. Holding verdicts, cyclic subjects, and the QSL-2778 and QSL-2779 profiles are declared Remaining in the PR and are not counted as gaps here.
