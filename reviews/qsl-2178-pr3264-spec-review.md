---
id: SR-2026112105473203
title: "Spec review — FR-1040 refinement-row clock-read refusal (QSL-2178 PR #3264)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3264 commit bde7f42408; spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:122-145"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-430", type: references }
---

## Summary

Ticket: QSL-2178. Review of the one arch-ruled spec edit to FR-1040 "Refusals that remain" (integrity and EARS scope only: the edit is one bullet). The ruling itself (inside a refinement row the refusal is `unsupported_construct`/`expression-form`) is not reopened.

## Verdict

**Mergeable if fixed.** The sentence is written as system behaviour under the list's "each SHALL refuse" lead-in, and agrees with the implementation and with FR-430 "Timed refinement" (spec/functional/FR-430-...md:87-90). It cites the wrong document, has no acceptance criterion, and leaves the later clock-reading-call bullet without the same carve-out.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-1040 Refusals bullet 1 | examined | a population form outside a refinement row whose condition or body reads a clock value ... refuses invalid_timed_model/clock-read-outside-constraint at the read (FSpec FR-415); inside a refinement row, a clock read refuses unsupported_construct/expression-form at the read (FSpec FR-430, "Timed refinement"); |
| FR-1040 Refusals bullet 6 | examined | a call to a clock-reading declaration (FSpec FR-415) in a population form, or in any other data position, refuses invalid_timed_model/clock-read-outside-constraint at the call. |
| FR-1040-AC-6 | examined | negative population forms, all outside refinement rows |
| FR-430 Timed refinement | context_only | If the refinement's state mapping or a step row reads a clock field of the concrete model, then S3 SHALL refuse unsupported_construct/expression-form at the read |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Wrong cross-reference: "(FSpec FR-430, "Timed refinement")". FSpec FR-430 is "Apply protocol fairness" (filament-specification spec/functional/protocol/FR-430-apply-protocol-fairness.md); the "Timed refinement" rule is this repo's FR-430 (spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:81-90), cited bare as FR-1040 cites FR-230. Write `FR-430 "Timed refinement"` and add FR-430 to `relationships` (depends_on) and `## Dependencies`, which list neither. | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:128-130 |
| FND-002 | medium | The new refinement-row refusal has no acceptance criterion: AC-6 lists only the forms outside a refinement row, and the PR's negative test is tagged FR-1040-AC-3, a positive AC. Add the vector to AC-6 (for example: in a refinement row over `calls`, `sum(c in M::calls: c.x) >= 0` refuses `unsupported_construct`/`expression-form` at `c.x`), so the test has an AC to trace to. | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:128-130, spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:163 |
| FND-003 | low | The last bullet still says a clock-reading call "in a population form, or in any other data position" refuses `clock-read-outside-constraint`, with no refinement-row carve-out, while the row check passes the clock-reading declarations to `first_clock_read` and refuses `ClockRead` (`expression-form`) (refinement.rs:3433-3444, timed_model.rs:2158-2181). Scope that bullet "outside a refinement row" as the first one now is. Confidence medium: read, not run. | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:140-142 |
