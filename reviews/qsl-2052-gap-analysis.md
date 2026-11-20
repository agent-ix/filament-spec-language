---
id: SR-1533
title: "QSL-2052 diff-scoped qualification gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2052; fsl-eval/tests/it/main.rs; fsl-eval/tests/it/qualification_vectors_from_source.rs"
review_set: subset
pr: 3197
branch: codex/qsl-2052-qualification-source-vectors
head: fa2abc9f6ff2f147a76295266ca6d88e644c7ca9
model: unknown
quoin_skill: 0.31.1
module: spec-artifacts-process@0.2.0
clean: true
reviewed_units:
  - {"id":"QSL-2052-decimal-source","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"\"D02\" | \"D04\" => {"}
  - {"id":"QSL-2052-text-source","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn text_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {"}
  - {"id":"QSL-2052-unit-source","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn unit_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {"}
  - {"id":"QSL-2052-division-source","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn division_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {"}
  - {"id":"QSL-2052-ieee-source","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn float_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {"}
  - {"id":"QSL-2052-real-evaluation","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn observed(case: &SourceCase) -> Expected {"}
  - {"id":"QSL-2052-independent-comparison","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn compare(vector: &QualificationVector) -> Result<usize, String> {"}
  - {"id":"QSL-2052-oracle-negative","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn an_altered_standard_expectation_reports_the_vector_identity() {"}
  - {"id":"QSL-2052-partial-accounting","path":"fsl-eval/tests/it/qualification_vectors_from_source.rs","role":"examined","excerpt":"fn every_published_vector_has_an_explicit_source_qualification_status() {"}
  - {"id":"QSL-2052-registration","path":"fsl-eval/tests/it/main.rs","role":"examined","excerpt":"mod qualification_vectors_from_source;"}
findings_data: []
---

## Summary

Examined the QSL-2052 diff's source-to-expectation links, real behavioral assertions, trace ownership, missing/partial vector accounting and potential coverage inflation. No defects found in the delivered partial scope. The parent ticket remains incomplete.

Plan completion: not assessed

## Verdict

PASS for the delivered diff and its explicitly partial claims. This is not a full repository matrix or complete qualification verdict.

The changed code is test-only and is owned by QSL-2052's source qualification work and the external FSpec-TC-185, FSpec-TC-186, FSpec-TC-187, FSpec-TC-192 and FSpec-TC-193 behaviors. The actual decimal, text, unit declaration, integer-division and IEEE assertions can fail when FSL recognizes, checks, evaluates or refuses their source incorrectly. They do not use emitted package metadata or a second wrapper of the same evaluator as the result oracle. The new module is registered in the existing test harness.

The mutation control changes an independent expected result and checks the real comparison's identified failure. The accounting control is ancillary runner evidence; it is not counted as new language semantics. Pending source adapters and missing sub-obligations are explicit rather than represented as passing refusal tests. Deliberately non-source obligations are classified as boundaries, not invented FSL defects.

Coverage remains incomplete: the pinned published table has 171 rows, with 35 sampled rows, 136 unadapted rows and 21 sampled rows still partial. All unit-declaration samples remain partial because richer diagnostic identity/payload requirements are not checked. Decimal provenance/accounting, retained text normalization/provenance, and additional IEEE rounding directions or nonfused comparison remain unclaimed where applicable. Full-source qualification fails while any of these remain. QSL-2052 must remain partial rather than Done.


## Trace bindings

The source_vectors macro assigns the corresponding FSpec-TC tag to each generated per-row test; each binding asserts only its sampled source behavior. The altered-expectation control binds to FSpec-TC-185. The runner-status control is ancillary accounting and makes no semantic trace claim.


## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

Accepted in-lane focused execution evidence without remeasurement. The production-backed source assertions and their independently read published expectations were inspected directly, including adverse refusal and altered-expectation paths. No stubs, fabricated diagnostics, copied oracle corpus or successful assertions of absent source capability were found in this diff.

The user prohibited matrix reruns and repository-wide audits. Consequently this artifact is confined to changed tests, their relevant specification expectations and their scope claims; it does not represent a fresh repository-wide quoin gap audit. No Test Matrix was authored or regenerated. No tests, builds, lint, gate or requirement validation were repeated. The parent acceptance gaps listed above are retained as unclaimed scope, not new measurement-only tickets.

