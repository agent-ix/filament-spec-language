---
id: SR-967175026302197768900474181941935084878
title: "gap analysis of PR 4071: trace tags and implements claims for sampled timed runs on the reference evaluator (QSL-3493 part 1, QSL-3463)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze/src/statistical/, fsl-eval/src/temporal/, fsl-eval/src/simulation/model/timed/race.rs, fsl-eval/src/simulation/sampled/timed.rs, fsl-replay/src/statistical/witness.rs, fsl-analyze/tests/it/statistical_model.rs, fsl-eval/tests/it/timed_measures.rs, spec/functional/FR-254"
review_set: subset
---

## Summary

Ticket: QSL-3493. Head d7929ba052b4eb944083a3ffbd86f2f4963693ae, diff against the stacked base 1415370b19f. Plan completion: not assessed.
The computed matrix was run (quire matrix) and every FR-254 criterion, AC-1 to AC-9, has at least one tagged test.
The FSpec criteria of fsl-eval's readers are external to this repository's matrix and were checked by reading each tag against the FSpec text.
The added lines carry no todo, unimplemented, debug output or new ignore attribute. The six ignore attributes removed from the oracle belong to behaviour this change builds.
The unit test of the idle-tail fraction moved with the function to fsl-eval and keeps its tags.
Semantic review beyond the changed criteria was not run.

## Verdict

CONDITIONAL. The changed behaviour is owned by FR-254 and tested. Two trace claims are wrong or missing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | an_exponential_delay_over_a_bounded_window_is_unsupported is tagged FR-254-AC-3. AC-3 says nothing about an exponential delay over a bounded window, and FSpec FR-420 requires that draw to be conditioned on the window, so the matrix counts a test of a missing capability as evidence for AC-3. | fsl-analyze/tests/it/statistical_model.rs:1876 |
| FND-002 | low | FSpec-FR-407-AC-23 states both steps and accumulate. This change refactors timed_steps, which implements the steps half, and adds the implements attribute and test tag only on the accumulate half. The steps vectors in steps_within_a_time_horizon_are_censored_or_not_activated carry no tag, so a tool looking up AC-23 finds neither the steps code nor its test. | fsl-eval/src/temporal/timed_measure.rs:243 |
