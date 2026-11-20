---
id: SR-2026112013172122111220102
title: "Gap analysis of QSL-2716 PR #3188 package-route source model"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2716; PR3188 head 05eaf781303ac85284f19290c7267a5905c31d02; git diff origin/main...HEAD; spec/functional/FR-740, FR-940, FR-185, FR-569; spec/test-cases/TC-3012"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: reviews
---

# Gap analysis of QSL-2716 PR #3188

## Summary

Ticket: QSL-2716. This is a manual check of the acceptance criteria against the tests, with a probe. I did not run `quoin matrix`, because the host is thermally constrained and the matrix is a whole-repository run. Plan completion: not assessed.

Units examined:

- FR-740-AC-6, through the new fsl-eval `it` tests and the updated fsl-package read-back doc.
- FR-940-AC-6, the package-route and source-route parity for the model-check input. The PR removes a refusal that held this AC for source-object packages.
- QSL-2716's two acceptance bullets.

Context only: FR-569 (source objects are per unit; S3 keys a later unit's package as `source/<pkg>/<n>`) and FR-185-AC-14 (workloads on source subtypes).

The probe was a throwaway fsl-eval `it` module in the review worktree, never committed. It ran the production `PackageModelInput::of` route over single-unit, two-unit and stepping-shaped packages.

## Verdict

FR-740-AC-6 is met for a single-unit package. Two packages fall outside it.

1. For a two-unit package, the read-back is not equal to S3's (GAP-001).
2. With the refusal lifted, a package with source objects now builds a package-route model-check input that diverges from the source route. No test covers this, so FR-940-AC-6's parity no longer holds for these packages and no refusal stands in its place (GAP-002).

The ticket's first acceptance bullet asks for records "equal to S3's" for the package's declarations. That holds for single units only. The second bullet (a test through `PackageModelInput::of` for a random parameter and a workload) is met.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-740-AC-6 ("FSL's reader rebuilds a RandomParameter ... and a Workload equal to S3's") fails for a package of two units. The second unit's source types are rebuilt under `source/<pkg>`, but S3 declares them under `source/<pkg>/1`. Probe: `B::u` declaring is `ix://source/review-scratch/1/B` in S3 and `ix://source/review-scratch/B` rebuilt, and equality is false. Both new tests are single-unit, so the matrix shows AC-6 covered while a multi-unit package rebuilds unequal records without refusing. Add a two-unit test, with FIRST_UNIT/SECOND_UNIT minus their timed parts, and a package name such as `acme/v/2`. | fsl-eval/tests/it/source_object_probabilistic.rs:104-184; fsl-package/src/probabilistic_read.rs:335 |
| FND-002 | high | FR-940-AC-6 requires that a package request gives the same item records as a program request, and that no package-route record is wrong. Before this PR, a package with source objects was refused as `PendingFamily::ModelForm(source-object)` at the row's node. The PR deletes that refusal with no replacement and no test. The probe shows the package route now builds an input whose source types have no fields, whose source operations have empty effect frames, and which has no source populations. The source route has all three. No test asserts either that the package route refuses model checking over source objects or that it matches the source route. Add a test through the production route (`settle_package_run`, or `PackageModelInput::of` with `ModelSystem`) over a source object with a field and a `modifies` operation, asserting a located refusal or source-route parity. | fsl-package/src/model_input_form.rs:76-88; fsl-eval/src/simulation/model/input.rs:425-439 |
| FND-003 | medium | TC-3012 / FR-740-AC-6's literal unit declares `operation step(k: Int[1, 3])`, but both new tests use `x: Integer` or `bit: Boolean`. So the `resolved:Int[a, b]` parse added for this AC is not exercised by any tagged test (my probe shows it works). None of the new refusals (`UnprojectedParameter`, `MalformedOperation`, `UnknownSupertype`, `NoIdentityRows`) has a test. One of them, `UnprojectedParameter`, fires for any Text or Decimal operation parameter (reproduced) and settles in fsl-analyze as an unlocated internal fault. | fsl-eval/tests/it/source_object_probabilistic.rs:107-112; fsl-package/src/probabilistic_read.rs:236-258 |
| FND-004 | low | QSL-2716's problem statement names "a random parameter, workload or delay". Delays on source objects sit on timed source models, and the package route still refuses those (`UnreadModelForm::TimedModel`). Neither the PR body nor the ticket records the delay half as remaining work. Say so on the ticket with its owning ticket id, or narrow the problem statement. | fsl-package/src/model_input_form.rs:63-64 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test `the_package_route_rebuilds_each_unit_of_a_package_under_its_own_source_identity` (two units, package `acme/v/2`) asserts equality with S3, and the two identities are `source/acme/v/2` and `source/acme/v/2/1`. It passes. My probes of two units and of a first unit with no object are also equal. |
| FND-002 | fixed | The refusal is restored in `first_unread_model_form`. The test `the_model_check_input_over_a_stepping_source_object_is_refused` (TC-3012/FR-940-AC-6; a field, a population and a stepping operation) asserts the located `SourceObject` refusal at the row's node through `PackageModelInput::of`. It passes. |
| FND-003 | fixed | The AC-6 literal `k: Int[1, 3]` is now the first test's fixture. The unread-parameter refusal (with its node) and the malformed operation each have a test. All pass. |
| FND-004 | fixed | The PR body records the delay half as remaining work under QSL-2347. |
