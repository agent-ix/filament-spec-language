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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-740-AC-6's read-back now passes only when the test hands the reader S3's own source packages. Its doc says "The join runs over S3's own source packages, which the bytes do not carry". The package route that reads emitted bytes, fsl-eval `PackageModelInput::of` (simulation/model/input.rs:418-432), builds its `StateModel` with `ModelSelections::new(&model_selections, &[])`. With no source packages, a source-object random parameter or workload still cannot be rebuilt on that route. This was found by reading the code and not run, because fsl-eval tests do not build on main (QSL-2710). Either track source packages on the package route (a ticket), or say in AC-6's test and in the PR body that the read-back is over S3's source packages and not over the bytes alone. | fsl-package/src/emit/tests/probabilistic.rs:3098-3172; fsl-eval/src/simulation/model/input.rs:418-432 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `a_delay_on_each_source_object_beside_a_source_clock_is_emitted` holds FR-740-AC-5's literal fixture (`O { d: Clock; .. }`). It is tagged TC-3012/FR-740-AC-5, ignored on QSL-2347, and fails with MalformedClockPlace under `--include-ignored`. The passing test and it share `assert_delays_emitted`. |
| FND-002 | fixed | Both clock ignores now cite QSL-2347. The PR body and the QSL-2463 remaining-work text name QSL-2347 as the owner of the source clock_place join. |
| FND-003 | still-open | The claim "No S4 delay refusal is reachable from a unit S3 admits" is false. In a two-unit package (`assemble_units`), unit b's `delay B::u ~ uniform` (B its own timed root, `time B dense unit s`) and `delay C::poke ~ uniform` pass S3 and are refused at S4 with "delay `u` cannot be written to a checked-package/v2 graph yet" (code-review FND-004). FR-185-AC-12 requires every S4 refusal concerning a delay to carry the declaration's source reference and label span in its locus, never null, and a message that names the declaration in words with no debug rendering. That refusal is reachable and needs a test that asserts the locus and the message, as the workload and random-parameter cases do. If FND-004's fix makes multi-unit delays emit, a test is still owed for the refusal that remains: the dense-unit disagreement or an absent declaring node. |
| FND-004 | fixed | `a_source_random_parameter_and_workload_are_read_back_equal` (lib, tagged TC-3012/FR-740-AC-6) rebuilds the record through `BarePackage` and `StateModel::probabilistic_model` and asserts probabilities `1/4`, `3/4` and the workload's name, weights and model key equal to S3's. It passes. The caveat about the route it reads over is FND-006. |
| FND-005 | fixed | QSL-2699 now has a comment (2026-11-20T10:12Z). It records the partial state and the arch question: change the AC-5 fixture or the FR-230 stage order. |
| FND-003 | accepted-no-change | I found no unit that S3 admits and S4 then refuses for a delay. A tick source, an untimed model, two time sources, an inherited op of a timed subtype, and a cross-unit delay in either direction (timed unit a on unit b's untimed C, timed unit b on unit a's A) are each refused at S3 (`delay-on-untimed-model`, `duplicate-time-source`) or emitted. `source_dense_unit` now reads only the declaring type's own owner, so its 'units differ' branch cannot fire within one unit, because FR-230 allows one time member per unit. FR-185-AC-12 is conditional: any S4 delay refusal must carry the declaration's source reference and label span (never null) and a message in words with no debug rendering. The remaining delay refusals are defensive and are built by the same `refusal()` helper as the workload and random-parameter refusals, whose locus and message are tested, including in a later unit. The PR body now states this. |
| FND-006 | fixed | QSL-2716 exists (Specced, project FSL probability from source). The read-back test's doc and the PR body cite it for fsl-eval `PackageModelInput::of` passing no sources. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `a_declaration_on_another_units_source_type_is_refused` is tagged `TC-620`/`FR-185-AC-14`. AC-14 states the source-object workload weights and refusals within one unit, not cross-unit visibility. Unit scoping is FR-569's rule, which the repo binds as `#[trace("FR-569-AC-2", "TC-1408")]` (fsl-semantics/tests/it/source_objects.rs:1081). Retag to the FR-569 binding, or add it beside the current tag. | fsl-package/tests/source_probabilistic_emission.rs:798 |
| FND-008 | low | `the_source_map_region_of_a_later_units_declaration_names_that_unit` is tagged `TC-2062`/`FR-185-AC-12`. AC-12 states the locus and message of an S4 refusal, but the test asserts the `source_map` region of an emitted node (FR-096 / FSpec FR-322 source_map). Bind it to the AC that states source-map regions. | fsl-package/tests/source_probabilistic_emission.rs:843 |
