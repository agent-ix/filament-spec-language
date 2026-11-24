---
id: SR-967175026302197768900474181941935084805
title: "code-review of PR3641 QSL-2923 rare-failure always regression test"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3641 commit 50b1afcf3e; fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs"
review_set: subset
---

## Summary

Ticket: QSL-2923. Test-only commit. The conclusion is plausible: under `{true: 999, false: 1}` the down state is reached almost surely, so unwindowed `always holds(s.up)` has exact value 0; the 0.2.0 candidate (86acfdee) predates QSL-2921's subsystem refutation (5b9c3ec3d1), so its refutation fell into the path-set search with masses (999/1000)^n. The test asserts exact certificate values, ranks, empty scheduler and paths, and replays, so it would fail on the candidate's incomplete result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The regression pins a minimal one-service model only; the ticket names the filament-examples architecture and engines systems, and the engines case is not pinned or re-run | fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:618-667 |

## Scope and evidence

- FR-202-AC-36 (spec/functional/FR-202-replay-a-probabilistic-witness.md): examined; the test mirrors its lower-certificate shape (x=1 at both states, ranks 1 and 0).
- TC-3162: examined; tag correct.
- Test run result: see the posted comment.

## Verdict

Plausible and discriminating; one low.
