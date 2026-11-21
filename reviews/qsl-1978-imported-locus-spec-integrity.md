---
id: SR-2026112117335577103729
title: "spec-integrity-analysis of QSL-1978 imported-locus wire amendment"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@720f29572ab49e9bc6d05953f8c57f35f2b1037f; FR-267 Refusal locus wire and Wire member cross-check, FR-267-AC-33, FR-267-AC-34, FR-663-AC-4, TC-1746, TC-3081"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: reviews
---

## Summary

Ticket: QSL-1978 (PR filament-spec-language#3352). Integrity checks scoped to the diff:
identifier uniqueness, cross-reference resolution, trace scope lines, and consistency between
the new prose and the tables it edits.

## Verdict

**FAIL (one medium finding).** Identifiers are sound. FR-267-AC-33 and AC-34 each occur once,
and AC-30 to AC-32 keep their own text, so the renumbering past 30 and 31 is correct.
FR-663-AC-4 occurs once. TC-3081 `verifies` FR-267 and FR-663, and its scope line lists AC-33
and AC-34. TC-1746 `verifies` FR-663, and its scope lists AC-4. The FR-663 link to
FR-267#refusal-locus-wire resolves. No FSpec amendment is made, which is consistent with FSpec
FR-352 delegating the locus table to FSL. The one inconsistency is in the member cross-check
table the PR edited.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR widens the `Outcome locus` / `span` row of the wire member cross-check, which is FR-100's untagged `{file, span}` evaluation-outcome locus and not the tagged `disposition.refusal.locus`, to admit "imported-point endpoints … without bytes". It adds no `sourceIdentity` row to the `Outcome locus` class. As written, the table lets an outcome locus carry a byteless imported point with `file` but no URI. That drops the exact identity the prose makes the imported point's identity, and it is the shape the prose has the `located` reader refuse ("an imported point without `sourceIdentity`"). Either the row edit is unintended and should be reverted, or the class needs a conditional `sourceIdentity` member under the same rules. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:619 |

## Gates

`quire validate --scope .` over the four changed artifacts at their exact paths exited 0. No
runtime, cargo or full gate was run.
