---
id: SR-967175026302197768900474181941935084844
title: "Code review — filament-spec-language FR-282 certificate diagnostics"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/certificate.rs, fsl-replay/src/certificate/product.rs, fsl-replay/src/certificate/scc.rs, fsl-replay/src/certificate/simulation.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/spine/lifecycle/tests.rs, fsl-analyze/tests/it/closure_certificate.rs, fsl-analyze/tests/it/component_certificate.rs, fsl-analyze/tests/it/refinement_certificate.rs; FR-282-AC-6/7/8/21"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-282", type: references }
---

## Summary

Reviewed PR #3861 at `40059b6f92bbd0de433a9def225cc34b22dd2ca7` for the FR-282 AC-6/7/8/21 implementation, tests, Rust API/wire behavior, and trace tags. The principal rejection loci are implemented, but the AC-7 production mapping is not exercised by its tagged test and document parsing only observes cancellation after the parse has returned.

## Verdict

**CONDITIONAL** — two medium findings remain in the production-path evidence for AC-7 and the cancellation timing required by AC-21.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The cancellation poll runs only after the synchronous certificate parse returns, so AC-21's cancel-at-next-charge behavior during parsing is not implemented. | fsl-replay/src/probability_certificate/document.rs:922-935 |
| FND-002 | medium | The sole FR-282-AC-7 tagged test constructs `FailedStep::NoAbstractMatch` directly and never verifies that the real checker maps a missing abstract step to the diagnostic carrying its concrete transition. | fsl-analyze/tests/it/refinement_certificate.rs:1042-1061; fsl-replay/src/certificate/simulation.rs:801-811 |

## Review Methods

- Code review and Rust review: inspected changed public enums and serialization, panic invariants, checker control flow, test behavior and trace tags, changed crates, and the absence of CI workflow changes.
- Gap-analysis: run separately in `qsl-3251-gap-analysis.md`.
- No AssuranceProfile applies to these files.
- No tests or aggregate gates were run by the reviewer. The coder reported filtered AC-6, AC-7, AC-8 and AC-21 tests passing after rebasing. The code lead owns the post-review final gate.

## Finding Details

### FND-001 — AC-21 cancellation occurs after parsing

`read_document_profile` calls `quire_canonical::read_bounded` synchronously and polls `cancel` only after that call returns. The comment confirms the canonical reader has no cancellation hook. Consequently, a cancellation requested while a large valid document is being parsed cannot stop at the next parse charge; it is observed only after the complete parse. The added test cancels from the second `Cancel::observing` poll, which is the poll after `read_bounded` has returned. It verifies rejection before decoded fields are exposed, but not the AC's cancellation during parsing.

### FND-002 — AC-7 test bypasses the production mapping

`a_rejection_serializes_its_locus_and_verdict` constructs a `FailedStep::NoAbstractMatch` value with a synthetic transition and verifies its JSON shape. It does not call the simulation certificate checker. The new production branch in `Run` that converts `StepVerdict::Fails(RefinementFailure::NoAbstractMatch)` into this `FailedStep` is therefore not exercised by the only test bound to FR-282-AC-7. A production regression that emits the old failure shape would leave this test green.

## New findings (disposition pass 2)

Re-reviewed the whole diff `origin/main...83d70d88e5e1da26b88a170866a09f3fb6497631`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | FR-282-AC-7's stimulus (the certificate of a holding refinement with one relation pair removed) still rejects `PostDiffers` at the source edge, not a cause naming the unmatched concrete step; the only AC-7 binder substitutes a different stimulus (step rows rewritten to `any` over the lost-update variant, a refinement that does not hold), so the tag claims an AC the production checker does not satisfy. | fsl-analyze/tests/it/refinement_certificate.rs:225-249; fsl-analyze/tests/it/refinement_certificate.rs:1103-1163 |

### FND-003 — AC-7 binder does not exercise the AC's stimulus

FR-282-AC-7 reads: the simulation-relation certificate of a refinement that holds is accepted; "the same certificate with one pair removed from the relation is rejected with a cause naming the concrete step left without a matching abstract step." The pre-existing, unchanged test `a_removed_relation_pair_reports_only_the_concrete_source_edge` (tagged only `TC-556`) performs exactly that mutation (`certificate.positions[target].abstract_states.remove(0)` on the `safety_only()` certificate) and asserts the rejection is `PostDiffers` at the source edge, with doc text stating the checker "reports only the PostDiffers source edge and not the missing abstract match". The PR does not touch that path. The new AC-7 binder `an_unmatched_concrete_step_names_its_transition_in_the_checker_diagnostic` instead rewrites the commit step rows to `any` over `Variant::LOST_UPDATE` and lists the engine's certificate, so the refinement does not hold and no pair is removed. The production `FailedStep::NoAbstractMatch` mapping is now correctly exercised (FND-002 fixed), but the AC's stated stimulus and outcome remain unmet and the matrix now reports AC-7 as tagged. Either the checker must name the unmatched concrete step for a removed pair, or the spec AC must be amended to the stimulus actually served.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (Linear comment 6142e83f): cancellation polled on every canonical-reader reservation charge. |
| FND-002 | fixed | Round 1 (Linear comment 6142e83f): tagged integration test exercises the real checker conversion. |
| FND-003 | still-open | Round 2 at 83d70d88e5e1da26b88a170866a09f3fb6497631: new finding; removing a relation pair still rejects `PostDiffers` at the source edge, and the AC-7 binder uses a different stimulus. |
