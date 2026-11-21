---
id: SR-202611210022442313144108430
title: "Gap analysis of filament-spec-language PR 3255 (QSL-2657 check dependent_bytes)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; git diff origin/main...be60a060ddc8cebe5c6e1d9b21e4a5611414d7b8 (fsl-command/src/command.rs, fsl-command/tests/it/check_imports.rs)"
review_set: subset
evaluated_revision: "be60a060ddc8cebe5c6e1d9b21e4a5611414d7b8"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: reviews
---

## Summary

Ticket: QSL-2657. Gap analysis of FR-866's Behavior and AC-7 against the
diff. Review pass ran on another machine; this file is reconstructed from its
posted comment and carries the round 1 dispositions.

## Verdict

Round 1: the source half of FR-866's Behavior now has a test. The pre-existing
AC-2/AC-6 trace gaps are outside this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Behavior 'source read under s1.input_bytes' has no check_outcome test | spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md:153-156 |
| FND-002 | low | Pre-existing: FR-866-AC-2 untagged, AC-6 tagged-by-ignored-test | spec/functional/FR-866-check-a-unit-against-the-models-and-libraries-it-imports.md:163 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | check_source_over_s1_input_bytes_refuses_at_s1_like_compile (TC-2457) checks a source one byte over and exactly at s1.input_bytes |
| FND-002 | deferred | Pre-existing trace gaps on AC-2/AC-6 outside this diff; the PR touches neither tag, so they are out of scope here |
