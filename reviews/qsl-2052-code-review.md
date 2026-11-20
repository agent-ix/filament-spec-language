---
id: SR-1532
title: "QSL-2052 source qualification code and Rust review"
type: SpecReview
analysis: code-review
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

Reviewed the frozen test-only diff for PR #3197, ticket QSL-2052: published qualification adapters, real source intake/checking/evaluation, independently read expected results, loss/refusal comparison, test registration, and explicit incomplete-coverage reporting. No code defects found in this scope.

## Verdict

PASS for this diff. This is a partial source-qualification delivery, not acceptance of every published vector or every sub-obligation.

The adapters read the authoritative QUALIFICATION_VECTORS inputs and expectations. No standard fixture, expected-result table, wire package, or production evaluator implementation is copied into the repository. Decimal expectations extract authored exact results and rounding-loss fields. Text profiles compare actual source values; integer division retains the selected division definitions while checking independently Euclidean mod. IEEE cases compare actual result bits and flags. Unit cases admit or refuse genuine authored declarations; their untested identity and diagnostic payload details remain explicitly partial.

Each case reaches source recognition, forms assembly and CheckMode::Linked. Successful value cases link a checked package and invoke the checked function through the public evaluator. The explicit IEEE definition selection happens before checking. Admission-only unit/type cases still exercise real source and declaration validation. Refusals compare stable codes and authored causes; evaluator refusal also asserts that no partial losses escape. Panics are fixture/test assertions, not new production error paths.

The altered D02 oracle executes the real source and must report a vector-identified mismatch. The source-status test rejects a zero-sample runner, accounts for all published identities, distinguishes sampled-but-partial rows from unadapted rows, and rejects either under FSL_FULL_SOURCE_QUALIFICATION. It does not turn deliberately wire-only or backend-only obligations into successful source qualification.

The 35 sampled rows include 21 partial rows; 136 rows have no adapter. Per-row tests carry the repository's FSpec-prefixed external test-case tags. These tags are evidence for their sampled behavior, not a claim that the complete external test case is qualified. No production API, unsafe code, workflow, dependency, compatibility layer or vendored data is added.


## Trace bindings

The source_vectors macro assigns the corresponding FSpec-TC tag to each generated per-row test; each binding asserts only its sampled source behavior. The altered-expectation control binds to FSpec-TC-185. The runner-status control is ancillary accounting and makes no semantic trace claim.


## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

Accepted the author's 37 distinct focused passing tests, corrected-adapter receipts, expected full-qualification failure, formatting result and preserved original failures without rerunning them. Scoped Clippy remains blocked by 12 unchanged library diagnostics; this review makes no passing lint claim. The final complete affected-component gate remains with the code lead/coder after this review. No tests, builds, gates, matrix, or lint were run during review.

Repository AGENTS.md/CLAUDE.md, Rust lint configuration, Rust testing and implementation conventions, and the relevant published TC-185/186/187/192/193 rows informed the review. No applicable AssuranceProfile was identified in the bounded convention lookup. This is a diff review, not a repository assurance audit.

