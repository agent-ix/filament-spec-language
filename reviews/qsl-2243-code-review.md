---
id: SR-202611210022442313144108537
title: "code-review of PR3323 timed abstract hidden fields and protocol enabledness in refinement liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3323 head 2a2243f9bc3910c2a20533bf0ecf38f365be15e2; fsl-analyze/src/model_check/refine.rs, fsl-analyze/src/zone_check/refine.rs, fsl-analyze/src/zone_check/search.rs, fsl-analyze/tests/it/timed_refinement.rs, fsl-replay/src/model_check/refinement_replay.rs"
review_set: subset
---

## Summary

Ticket: QSL-2243. Soundness review of the diff (rust-review lane folded in). The narrowed timed path checks guard, carry and reset agreement across hidden candidates, but not invariant agreement. The independent checks (replay, and the FR-245 checker the PR makes reachable) do not apply the agreement rule. Protocol enabledness is read only at a trace's start configuration, which misses configurations inside the internal closure. Fix round needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Hidden candidates that disagree on abstract time invariants are not refused: the producer intersects every candidate's invariant and refutes when any candidate's invariant fails, and replay reproduces on any candidate, so a wrong Refuted is possible | fsl-analyze/src/zone_check/refine.rs:879-940, fsl-replay/src/model_check/refinement_replay.rs:1074-1091 |
| FND-002 | high | Replay judges CandidateSteps guards from the first candidate in the prior set that has any row's transition, with no row-to-candidate alignment and no agreement check, so it can confirm a refutation the producer must not emit | fsl-replay/src/model_check/refinement_replay.rs:1025-1069 |
| FND-003 | high | The FR-245 zone-certificate checker's CandidateSteps branch, now reachable for Holds certificates, uses the same first-candidate rule and does not reject disagreeing or mixed-stutter edges | fsl-replay/src/certificate/zone_certificate.rs:1997-2020 |
| FND-004 | high | Protocol enabledness is read only at a trace's start configuration. Configurations inside the internal closure, which FR-148 places in the candidate set and which the abstract run passes through, are ignored, giving a wrong Holds under strong fairness and a wrong Refuted under weak fairness. Replay repeats the same rule | fsl-analyze/src/model_check/refine.rs:3022-3045, fsl-replay/src/model_check/refinement_replay.rs:2026-2075 |
| FND-005 | medium | Replay records only the first application of a multi-step protocol trace as the taken transition, while the producer counts any step; an edit that takes the class on a later internal step still reproduces | fsl-replay/src/model_check/refinement_replay.rs:2012,2105-2120 |
| FND-006 | high | Removing the blanket refusal also opens the hidden-field timed-abstract liveness half, whose abstract enabledness comes from untimed expand_state with no abstract clock guard or F_A-guard split (FR-899 Timed subjects) | fsl-analyze/src/zone_check/refine/live.rs:637-688,1013-1031 |
| FND-007 | medium | TC-3084 now asserts only that the outcome is not HiddenFieldTimedAbstract; any other refusal (including the RequiresBound it exists to exclude) passes, and the run reaches MaxZoneBytes near 1 GiB | fsl-analyze/tests/it/timed_refinement.rs:478-489 |
| FND-008 | medium | No test pins the new behaviour: no protocol HiddenUnfair refutation, no disagreeing-candidate refusal, no mixed-stutter refusal, and no hidden timed-abstract Holds with an accepted certificate | fsl-analyze/tests/it/timed_refinement.rs, fsl-analyze/tests/it/refinement_protocol.rs |
| FND-009 | medium | The residual HiddenFieldTimedAbstract refusal contradicts FR-899 "Timed subjects" ("Neither case is settled unsupported") and the owner's QSL-2243 ruling (no narrowing); per-candidate abstract clock valuations are required scope | fsl-analyze/src/zone_check/refine.rs:1076-1090, fsl-analyze/src/zone_check/search.rs:581-586 |
| FND-010 | low | protocol_path_label re-expands the start configuration per edge and charges max_transitions again, though CandidateEnabled.transitions already holds the same list; this can trip MaxTransitions spuriously | fsl-analyze/src/model_check/refine.rs:3029-3034 |

## Scope and evidence

Read the full diff and these surrounding files: zone_check refine.rs auxiliary_step and invariants, refine/live.rs abstract_enabled, refinement_replay.rs replay_abstract_timed_failure and replay_lasso, zone_certificate.rs auxiliary_step, refinement/step.rs protocol_closure_traces and failure.rs ProtocolTrace. Governing text: FR-899 (Hidden fields, Timed subjects, Abstract protocol subjects, Replay, AC-9, AC-10), FR-900-AC-2, FR-148 Closure and Divergence-freedom, FR-430 Strong fairness over time, FR-127/ADR-018. No builds or test runs, per brief. The quoin gap-analysis method was not run. A manual AC-to-test check is recorded under FND-008. The diff has no Python and no logic-bearing shell. The changed tests keep their trace tags.

Out of scope (pre-existing, not a finding): the zone-certificate checker's auxiliary_step drops zone points that fail an abstract guard instead of rejecting the certificate. Both the producer and the checker use only the first non-empty guard part.

## Verdict

Fix round needed. The extraction for Taken::Abstract is behaviour-preserving. The guard, carry and reset agreement check in matched_auxiliary_step is sound as far as it goes, and refusing a mix of stutter and step is conservative. The first changed test is strengthened: it now asserts AbstractClockGuard and Reproduced. FND-001 to FND-006 are soundness defects on paths this PR opens.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The HiddenFieldTimedAbstractLiveness refusal returns before the safety run, so a timed-abstract hidden-field refinement with non-empty F_A loses its decidable safety half (a Violated or Holds safety verdict is never reported) | fsl-analyze/src/zone_check/refine.rs:320-334 |
| FND-012 | low | The new comment says untimed enabledness "is exact only without a hidden abstract field"; it is not exact without one either (no abstract clock guard, no F_A-guard split), which is a pre-existing gap the comment now hides | fsl-analyze/src/zone_check/refine.rs:320-325 |
| FND-013 | medium | New refusal tests are tagged to ACs they do not evidence: the liveness refusal test carries FR-899-AC-9, whose text says "None of the four settles unsupported", and the mixed-stutter refusal test carries FR-900-AC-2 (a guard refutation) | fsl-analyze/tests/it/timed_refinement.rs:272,310 |
| FND-014 | low | The FR-245 checker rejects disagreeing rows and invariants at at_node(0) instead of the node or edge being judged | fsl-replay/src/certificate/zone_certificate.rs:1952,2038 |
| FND-015 | low | clock_rows.rs module doc says the product reads the rule there; the producer (fsl-analyze matched_auxiliary_step, uniform_invariants) has its own copy, which is what keeps checker independence, so the doc is wrong | fsl-replay/src/refinement/clock_rows.rs:1-5 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | still-open | Protocol HiddenUnfair is expressible: unbounded `repeat ... while { true }` inside `parallel` branches is checked (CheckedRepeat.max is Option; analyze_cli.rs:645). No disagreeing-guard or disagreeing-invariant refusal test, and the checker's Disagree rejection is untested |
| FND-009 | deferred | PR body records per-candidate clock valuations as remaining work on QSL-2243 and the ticket stays open; QSL-2243 must not be closed on this merge |
| FND-010 | fixed | - |
| FND-008 | fixed | - |
| FND-011 | fixed | - |
| FND-012 | fixed | - |
| FND-013 | fixed | - |
| FND-014 | fixed | - |
| FND-015 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | low | A 1486-line backup copy of the test file is committed; it is not compiled and must be deleted | fsl-analyze/tests/it/timed_refinement.rs.bak |
| FND-017 | low | The two new protocol tests carry FR-899-AC-10, which names FR-147's CasTwice fixture; they evidence the FR-899 "Abstract protocol subjects" rule, not that AC | fsl-analyze/tests/it/refinement_protocol.rs:2558,2595 |
