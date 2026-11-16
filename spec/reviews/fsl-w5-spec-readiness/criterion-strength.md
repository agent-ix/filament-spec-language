---
id: SR-2026111608544804
title: "Criterion strength review of the FSL wave 5 spec readiness batch"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; PR #2744 branch fsl-spec-w5 at f38affa78, git diff origin/main...HEAD (FR-132, FR-255, FR-267, FR-501, FR-624, FR-625, FR-627, FR-640, FR-641, NFR-017, TC-1160, TC-1584, TC-1585, TC-1587, TC-2241..TC-2246; stubs in fsl-analyze, fsl-command and tests/it)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: reviews
---

## Summary

Ticket: QSL-2295. Jev is not installed on this host (`jev not found`), so the
Jev-calibrated judgement was skipped and the criteria were judged manually
from the spec text and stubs. Each new or amended AC can fail on a named
adverse case: FR-625-AC-3 on an `incomplete` horizon, FR-625-AC-4 on an
admitted `temporal-satisfaction`, FR-627-AC-1 on an inc-only refutation,
FR-640-AC-12 on a single row or the `n` row removed, FR-640-AC-13/14 on
unequal bytes or a missing conflict, FR-641-AC-6 on `half_open` 0 accepted,
FR-641-AC-7 on 0 or a `lasso` member accepted, and NFR-017-AC-5..7 on an
untagged fence or a pending fence that passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-641 stops the run "when reconstructing a step reaches that bound", but FR-120 stops an expansion when the count "would exceed `max_candidates`". A step with exactly N candidates under bound N is `incomplete` under one reading and decided under the other, so AC-7 cannot tell the two implementations apart. Fix: say "when a step's candidate count would exceed `max_candidates`" (FR-120's rule), and add the exactly-N case to AC-7. | spec/functional/FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md:158,188; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:683 |
| FND-002 | low | The FR-627-AC-1 stub asserts only that no `loop` step holds `value = 3`. The AC requires the loop to repeat the `setTo(0)` step at `value = 0`, so a loop stuttering at 1 or 2 would pass the stub. Fix: assert that each loop step is `setTo` with `n = 0` at `value = 0`. | fsl-analyze/tests/it/analyze_cli.rs:190-198 |

## Verdict

REQUEST CHANGES on FND-001; FND-002 is a stub tightening.

## Dispositions

Round 2, verified at head a158bccc0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-641 now reads "When a step's candidate count would exceed that bound (FR-120's rule; a step with exactly `max_candidates` candidates is expanded in full)". AC-7 and TC-2246 add the exactly-N case. |
| FND-002 | fixed | The stub now asserts that each loop step's `transition` names `setTo` and that its `post_state.state` holds `value` 0. Both spellings match the writer: `spell_states` in fsl-analyze/src/analyze/document.rs adds `state` beside the key, and FSpec FR-364 defines `{transition, post_state}`. |
