---
id: SR-2026112010084800000002
title: "Gap analysis of QSL-2463 PR #3171 S4 source-owned probabilistic emission"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@16f5e64e77f9367e718b223185f0665b4853c8a4; git diff 0208154ee1...HEAD (PR 3171); spec/functional/FR-740, FR-185; FSpec FR-640 Reader; QSL-2463, QSL-2699, QSL-2347"
review_set: subset
date: 2026-11-20
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: reviews
---

# Gap analysis of QSL-2463 PR #3171

## Summary

Ticket: QSL-2463, with QSL-2699 partial. This was a manual check of the ACs against the tests. The Quoin matrix was not run, because the host is thermally constrained, so the bindings below come from reading the files and running the tests.

I examined FR-740-AC-5, FR-740-AC-6, FR-185-AC-12 and FSpec FR-640 Reader rules 1, 3 and 5. As context I read the arch ruling in QSL-2463's first comment and the FR-185 per-type ruling, and I checked the Linear tickets for each piece of remaining work: QSL-2347, QSL-2699, QSL-2678 and QSL-2465.

## Verdict

The S4 emission meets FR-740-AC-6 and the clock-free half of FR-740-AC-5: the strict read and the identity-row join admit both. Two items of the partial framing are not accurate.

1. "needs a contract-ir ticket": the ticket already exists. QSL-2347 seam 2, the source clock_place join, was ruled option (c) under W51 and its spec landed as contract-ir #1050. The code has not landed, and the pin 4c91eff still refuses a source Clock place even with no probabilistic declaration.
2. QSL-2699 does not record its partial state or the arch question.

The emission test tagged FR-740-AC-5 runs a different fixture from the criterion's literal one, and FR-185-AC-12's delay clause lost its only test. The removed test and case were otherwise justified.

The base failure of `fixtures_s_s_r_and_s_p_emit_source_objects` (SourceMapInvariant) is reproduced on base 0208154ee1, and QSL-2678 tracks it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_delay_on_each_source_object_of_a_timed_source_model_is_emitted` carries `#[trace("TC-3012", "FR-740-AC-5")]` and FSpec-FR-640-AC-6, but its `O` has `n: Integer` where FR-740-AC-5's literal fixture has `d: Clock`. The literal positive fixture still fails the strict read with MalformedClockPlace (reproduced), and no test, ignored or not, holds it. The matrix will report AC-5 covered while its stated input fails. Fix: add the literal AC-5 positive fixture as an ignored test keyed QSL-2347 with the AC-5 tag, and note in the passing test that it is the clock-free variant. | fsl-package/tests/source_probabilistic_emission.rs:141-166 |
| FND-002 | medium | The ignore reason on `a_source_operation_entry_lists_its_parameters`, the PR body and the QSL-2463 remaining-work comment all say the Clock-place gap "needs a contract-ir ticket". QSL-2347 already owns it: seam 2, the source clock_place join, arch option (c), W51 spec landed in contract-ir #1050, code not landed and QSL-2347 still in Coding. Re-key the ignore to QSL-2347, so the work is not filed twice or left orphaned. | fsl-package/tests/source_probabilistic_emission.rs:209 |
| FND-003 | medium | FR-185-AC-12 names S4 refusals of "a random parameter, a workload, a reward or a delay". The PR removed the delay case from `probabilistic_refusals_have_locus` and the test `a_delay_on_a_timed_source_model_refuses_emission_with_its_locus`. `ProbabilisticFeature::Delay` now appears only in the emitter, so no test exercises the located delay refusal (`delay_node`'s `unwritable`). Add a reachable delay refusal case, for example a delay whose declaring node is absent, or state why it cannot be reached. | fsl-package/src/emit/tests/probabilistic.rs:2303-2329 |
| FND-004 | medium | FR-740-AC-6 also states "FSL's reader rebuilds a `RandomParameter` with probabilities `1/4` and `3/4` and a `Workload` equal to S3's". `a_random_parameter_and_a_workload_on_the_source_model_are_emitted` asserts only the counts and the declaring and model references, so the read-back half of the criterion is untested. | fsl-package/tests/source_probabilistic_emission.rs:187-203 |
| FND-005 | low | QSL-2699 has no comment. Its description still says "fix the S3 check and un-ignore the test", but the PR shows the remaining case is a spec fixture vs FR-230 stage-order decision for arch. The partial state and the arch question are only in the PR body and on QSL-2463, so the decision has no owner on the ticket that tracks it. | fsl-package/tests/source_probabilistic_emission.rs:443-461 |
