---
id: SR-2026112007590400000002
title: "Gap analysis of QSL-2463 PR #3160 source-object workloads at S3"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@46bad2ca178dc0150936f3d4b0fe1245d0ad676f; git diff origin/main...HEAD (PR 3160); spec/functional/FR-185, FR-740, FR-569, FR-230; spec/test-cases/TC-2062; QSL-2463"
review_set: subset
date: 2026-11-20
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: reviews
---

# Gap analysis of QSL-2463 PR #3160

## Summary

Ticket: QSL-2463. This was a manual check of the ACs against the tests. The Quoin matrix tooling did not run here:
`quire trace --id FR-185-AC-12` did not finish within 120 s, so the trace bindings below come from reading the files.
I examined the FR-185 "Workloads" source-model clause (lines 154-160), FR-185-AC-3, AC-10 and AC-12,
FR-740-AC-6, the FR-569 name-collision clause (lines 292-297), FR-230's time-member resolution (lines 167-185),
TC-2062's scope, the arch ruling and the QSL-2045 repro on QSL-2463.

## Verdict

The S3 behaviour matches the FR-185 lookup-order sentence, and FR-569 prevents an alias and an object
from colliding. The partial framing is honest: the PR body says "Remaining work: QSL-2463", the ticket
carries a remaining-work comment, and the S4 emission tests stay ignored with reasons that name the real
blockers (#3157 pin, QSL-2645). There are gaps in three places: the trace tags, the spec text behind the per-type rule, and what
the remaining work will actually deliver for QSL-2045.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new tests are tagged `#[trace("TC-2062", "FR-185-AC-12")]`. FR-185-AC-12 covers the locus and message of an S4 refusal, but both tests check S3 admission and S3 refusal of a source-object workload, and neither runs S4. No AC states the FR-185 "Workloads" source-model clause (lines 154-160): AC-3 and AC-10 use the selected `Service` model. FR-740-AC-6 (TC-3012) contains the S3 half, but only together with emission. As a result, the matrix counts AC-12 as better covered than it is, and the clause this PR implements is still untraced | fsl-package/tests/source_probabilistic_emission.rs:235; fsl-package/tests/source_probabilistic_emission.rs:251; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:154-160 |
| FND-002 | medium | FR-185 does not say which operations a workload on a source object type weighs. The PR reads it as "only that type's operations": a weight on another source object is `unknown-operation`, and other objects' operations are not required. But FR-185 defines a workload as weighting "every operation of a model", and an omission as `incomplete-workload` against "its model", where the model is the source model. A second implementer could read it as requiring every source operation. FR-740-AC-6 and FSpec FR-640 (one `model` node) agree with the per-type reading, but the spec text does not state it, and it does not say whether inherited operations are included (see SR-2026112007590400000001 FND-001) | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:157-160; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:169-172 |
| FND-003 | low | The QSL-2045 repro shape will still fail at S4 after QSL-2463's remaining work lands, and the partial framing does not say so. The repro's random parameter `choose(bit: Boolean)` has Boolean support, and `random_parameter_node` refuses every Boolean support value regardless of the declaring type (`Value::Boolean(_) => return Err(unwritable())`, an IR reader limitation). Today the missing source model node hides that refusal. The PR body says it "Fixes the QSL-2045 `workload Input on Switch` refusal", which is true for S3 only. QSL-2045's stated goal, a successful S4 compile, needs a separate owner for Boolean support emission | fsl-package/src/emit/probabilistic.rs:249-255 |
| FND-004 | low | Two ignored tests in the touched file still cite QSL-2439, which is Done: `a_delay_on_an_untimed_source_model_is_refused_at_its_span` (it now fails with `invalid_timed_model`, not the asserted `invalid_probabilistic_model`) and `a_source_operation_entry_lists_its_parameters` (it now fails the strict read with `MalformedClockPlace`). The QSL-2463 readiness sweep lists both lines (:174, :209) for un-ignoring. The PR re-keyed only the two emission tests. These two now have no live owner, and one asserts a cause the checker no longer produces | fsl-package/tests/source_probabilistic_emission.rs:174; fsl-package/tests/source_probabilistic_emission.rs:209 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The re-keyed ignore reason on `a_delay_on_an_untimed_source_model_is_refused_at_its_span` says the expected cause "needs an arch ruling". FR-740-AC-5 already fixes it: "Without the `time` member, `delay O::step ~ uniform;` refuses `invalid_probabilistic_model`/`delay-on-untimed-model` at its span". S3 instead gives `invalid_timed_model`. This is a code defect against an existing AC, not an open question, so it should be ticketed as a defect and not parked with arch | fsl-package/tests/source_probabilistic_emission.rs:174; spec/functional/FR-740-emit-random-parameters-workloads-rewards-and-delays-as-v2-probabilistic-nodes.md:161 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The three tests are now tagged `#[trace("TC-3012", "FR-740-AC-6")]` (S3 half). The PR body and the test doc comment both say no AC states the FR-185 source-model Workloads clause |
| FND-002 | deferred | The ambiguity is recorded in the PR body ("Ambiguity for arch") and the coordinator is raising it with arch. It needs a spec ruling and cannot be fixed in this PR's code |
| FND-003 | fixed | The PR body now says the QSL-2045 Boolean repro will still refuse at S4 (Boolean support is an IR reader limitation) and needs a separate owner |
| FND-004 | fixed | Both ignore reasons are re-keyed to QSL-2463 with their current failure modes (`invalid_timed_model`; `MalformedClockPlace` until the #1082 pin). The wording of the first one is FND-005 |
