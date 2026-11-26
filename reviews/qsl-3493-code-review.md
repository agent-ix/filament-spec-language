---
id: SR-967175026302197768900474181941935084877
title: "code review of PR 4071: sampled timed runs read by the fsl-eval reference evaluator (QSL-3493 part 1, QSL-3463)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/statistical.rs, fsl-analyze/src/statistical/model.rs, fsl-analyze/src/statistical/model/window.rs, fsl-analyze/tests/it/statistical_model.rs, fsl-analyze/tests/it/timed_oracle/probability.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/timed.rs, fsl-eval/src/simulation/model/timed/race.rs, fsl-eval/src/simulation/sampled/timed.rs, fsl-eval/src/temporal/mod.rs, fsl-eval/src/temporal/timed.rs, fsl-eval/src/temporal/timed_idle.rs, fsl-eval/src/temporal/timed_measure.rs, fsl-eval/tests/it/timed_measures.rs, fsl-replay/src/proof_result.rs, fsl-replay/src/statistical.rs, fsl-replay/src/statistical/witness.rs"
review_set: subset
---

## Summary

Ticket: QSL-3493. Head d7929ba052b4eb944083a3ffbd86f2f4963693ae. Only what this PR adds over the stacked base 1415370b19f was reviewed. Rust review is folded into this file.
Checked by hand against FSpec FR-407, FR-416 and FR-420 (origin/main) and FSL FR-254 and ADR-038: the elapsed clock start and censoring, window bounds, horizon, positions read over a time-lock, the idle-tail completions, reward and delay weighting with the crossing step, the new accumulate and weighted fraction vectors, and the replay step and value comparison. No wrong estimate or verdict was found in them.
The no-mass time-lock is right: with guard x >= 4, invariant x <= 4 and discrete{0}, the window is [4, 4], 0 is outside it, no identity races and delay is bounded, which FSpec FR-420 "The race" calls a time-lock.
Replay still re-runs the sampler; that is part 2 of the ticket.
Run by the reviewer at this head: cargo test --locked -p fsl-analyze --test it_probabilistic -- review_probe_, where review_probe_exponential_bounded_window_is_sampled (FND-002, not committed) fails with Unsupported NotStochastic.
The arch-lint site at fsl-eval/src/simulation/model.rs:4342 is a thread_local from abf8e9de1fa (2026-11-24), not from this change. The import reordering in fsl-eval is what the pinned rustfmt wants.

## Verdict

Mergeable once the pull request description is replaced (FND-001) and the exponential gap is either built or ticketed (FND-002). Neither produces a wrong verdict.
Notes, not findings: the time-locked state key reported for a no-mass lock is the state delayed to the invariant bound, where the identity is enabled with delay 0; an elapsed sample whose first predicate never holds in a cyclic model runs to max_sample_steps, as FSpec FR-420 states, retaining every position; the public replay_timed_sample and replay_timed_sampled_witness, which no product code calls, still sample from the run start and so cannot reproduce a sample made by sample_timed_from for elapsed, which part 2 should settle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The pull request description is that of PR 4061 (starts "Refs QSL-3491", says no verdict path, route or wire format changes). It states none of this change's behaviour changes (elapsed clock start, time-locked samples read, no-mass delay now a time-lock, exponential bounded window unsupported), none of its public API changes (SampledWitnessRefusal::TimedSample removed, WitnessReadStop::TimedNotRegenerated added, WitnessReader::read_timed signature, TimedSample fields, TimedAdvance::Race), and its criteria, related-code and test lists are for another change. A merge would record that description. | fsl-replay/src/statistical/witness.rs:36 |
| FND-002 | medium | An exponential delay over a bounded or split window with positive length now settles unsupported not-stochastic. FSpec FR-420 conditions an exponential draw on the identity's window, and its not-stochastic cases are only an unbounded uniform window, a racing free delay and the idle tail. With send ~ exponential(rational(1, 1000)) under invariant x <= 4 ms, probability >= 0.5 of eventually[0 ms, 8 ms] phase = 1 has value 1 and should be accepted; a run at this head gets Unsupported NotStochastic, and no ticket tracks building it. | fsl-eval/src/simulation/sampled/timed.rs:74 |
