---
id: SR-1528
title: "QSL-2054 property-generator scoped gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2054; fsl-bench/src/prop/; fsl-bench/tests/it/; fsl-cst/src/grammar/generate.rs; Cargo feature and test-target integration"
review_set: subset
pr: 3193
branch: codex/qsl-2054-property-generators
head: d13ace0f1e5b1f769ee6db66906f7392e143aa3e
model: unknown
quoin_skill: 0.31.1
module: spec-artifacts-process@0.2.0
clean: true
reviewed_units:
  - id: QSL-2054-value-generator
    path: fsl-bench/src/prop/value.rs
    role: examined
    excerpt: "A generator returns the source of a function together with the declared parameter types and the expression tree it was built from."
  - id: QSL-2054-grammar-generator
    path: fsl-cst/src/grammar/generate.rs
    role: examined
    excerpt: "A generator walks Grammar and produces unit source text that the parser admits, covering every named production."
  - id: QSL-2054-model-generator
    path: fsl-bench/src/prop/model.rs
    role: examined
    excerpt: "Small models as both library values and source text, with an independent description a test can enumerate."
  - id: QSL-2054-trace-generator
    path: fsl-bench/src/prop/trace.rs
    role: examined
    excerpt: "Finite traces and lassos over a small proposition alphabet, for each trace profile."
  - id: QSL-2054-round-trip-inputs
    path: fsl-bench/src/prop/unit.rs
    role: examined
    excerpt: "Admissible units for the package round trip."
  - id: QSL-2054-near-miss
    path: fsl-bench/src/prop/near_miss.rs
    role: examined
    excerpt: "Every near-miss source is refused with a located typed refusal, never accepted and never a panic."
  - id: QSL-2054-policy
    path: fsl-bench/src/prop/policy.rs
    role: examined
    excerpt: "Default case count is a per-suite argument; fixed ChaCha seed; environment overrides; bounded shrinking; seed-only persistence."
  - id: QSL-2054-config-tests
    path: fsl-bench/tests/it/prop_config.rs
    role: examined
    excerpt: "A deliberately false sample property fails, shrinks to a smaller input within the shrink bound, and writes a regression file."
  - id: QSL-2054-generator-tests
    path: fsl-bench/tests/it/generators.rs
    role: examined
    excerpt: "Generated source reaches the real checker; grammar coverage comes from parsed CST nodes; models have a terminating independent enumerator."
  - id: QSL-2054-dependency-feature
    path: fsl-bench/Cargo.toml
    role: examined
    excerpt: "property-generators enables optional proptest and fsl-cst/test-support; the property_generators test target requires that feature."
findings_data: []
---

## Summary

Examined requirement-to-test-to-code alignment for the QSL-2054 foundation diff, including reverse ownership, missing implementation and hollow evidence. No implementation or evidence defects found in the reviewed scope. This scoped result is not a repository-wide traceability claim.

Plan completion: not assessed

## Verdict

PASS for the scoped diff. All five generator categories exist with real consumer-facing representations and sample tests. Typed expressions retain the independent expression tree and declared domains, and generated programs reach real parse/select/Linked checking. Near misses assert independently constructed code, cause and region against actual refusals. The grammar sample asserts successful admission and the union of actual CST productions; it does not count desired target labels as parser coverage.

Models have independent finite-state descriptions and a terminating enumerator, plus real source and library checks. That test is a generator-validity sample, not engine cross-check or reduction correctness evidence. Trace samples establish the nonempty bounded shape/profile invariants requested by this foundation. Package inputs reuse the typed expression generator and reach real checked-package emission; strict-reader round-trip agreement is left to its separately planned suite.

Shared configuration supplies deterministic ChaCha seeds, scoped overrides, bounded shrinking and seed-only persistence. The false-property negative test exercises actual TestRunner failure and persisted replay seeds. Optional feature gating keeps these reusable test helpers outside normal production builds; the existing all-feature Cargo lane selects the required-feature integration target.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Requirement and evidence ownership

QSL-2054 explicitly calls for a testing foundation without a new language specification. The changed modules are ticket-owned test support, not unowned new language semantics. The tests use Provenance lines for QSL-2054; no FR criterion coverage is asserted merely from those ticket references, and no trace ID was fabricated. There is no new placeholder implementation, checker-based candidate filtering, expected-output snapshot, copied corpus, source-substring proof, or successful assertion of a missing language capability.

The public grammar helper is limited to the existing test-support feature, and the property module is limited to property-generators. The facade consumer license line repairs metadata for an existing workspace fixture using the repository's declared license; it does not introduce a license-policy exception.

## Accepted execution evidence

Accepted the author's focused evidence as true: all eight acceptance tests have passing receipts, with 1,000 typed expressions, 228 parsed grammar productions, and 256 samples each for models, near misses, checked package inputs and traces. The original construction failures and subsequent changed-test passes remain separate receipts. Two configuration tests cover deterministic sequences, scoped overrides, a real deliberately false property, bounded shrinking and seed persistence.

Accepted scoped Clippy/format results and dependency bans success plus the separate successful licenses-only correction receipt. No aggregate PASS is inferred from those partial commands. No test, build, gate, matrix, audit or reported result was repeated.

## Limits

The accepted sample suite times total approximately 14.2 seconds. Every sample suite meets the policy's individual 10-second bound, and the total is below its 60-second program bound. The conflicting acceptance bullet requiring the combined samples below 10 seconds remains unclaimed; the review does not reinterpret 14.2 as less than 10.

This review does not certify all future QSL-2055 through QSL-2063 properties, model-engine agreement, every operator's execution semantics, strict-reader round trips, fuzz or mutation strength, or gated advanced model authoring. Those are downstream consumers of this foundation. The coder must run the one final affected-component gate after review. No whole-repository matrix or assurance-profile scan was performed under the dispatch's explicit scope.

## Publication

Pre-PR artifacts use pr:null with the actual branch and reviewed SHA. The same artifacts will be posted to Linear only when the lead supplies the real public PR number. No repository-wide PASS or completed-plan claim is made.

## Supplemental Dispositions — Round 1

PASS for the confined performance delta d13ace0f1e5b1f769ee6db66906f7392e143aa3e to 5ab906af4fc855a37c974108c34ff67eb74a8e86 on public PR filament-spec-language#3193. The prior pre-PR publication paragraph is historical; posting now uses the real PR and actual Quoin executable version 0.29.4.

The optimization does not replace generated semantic checks with cache hits: grammar coverage still observes all 228 actual parsed productions; every model case still parses source, constructs forms, performs Linked checking and independently enumerates its finite description. Shared immutable grammar planning and caller-owned, exact-input admitted-model setup do not cache the checker verdict or independent model enumeration. Initialization stays within measured work. The original expression, near-miss, package and trace sample paths are unchanged.

Accepted the two changed focused receipts and unchanged prior receipts without rerun. Counts remain 1,000 expressions, 256 samples for each other property family, and 228 grammar productions. The actual combined total is 10.231657553 seconds; the combined-under-10-seconds AC remains unclaimed despite the improved grammar/model times. No ticket Done, dependent unblocking, aggregate-gate PASS or new semantic coverage is inferred. The original artifact's finding placeholder remains unchanged because this supplement found no correctness defect.

The redundant package_input preparation before a cache hit is a concrete optimization opportunity from the three-file diff, recorded in SR-1527 and sent to the lead. No broad search, profile, benchmark or repeated timing run was used. The coder owns any actual source optimization and the one final affected gate. Plan completion: not assessed.

## Supplemental Dispositions — Round 2

PASS for the confined delta from 5ab906af4fc855a37c974108c34ff67eb74a8e86 to 3d01aa9e3ba97919fd93ff706bdbd9dc1ad6499e on public PR filament-spec-language#3193. Only fsl-bench/src/world.rs changes: package_input moves to the exact-input cache-miss branch, and PreparedModel retains the successful PackageInput so cache hits preserve each World's owned packages as well as its admitted models.

No semantic case or property has been removed. Every model sample still parses and checks source and library forms and runs its independent enumerator. Per-case S3 checking and normalization-meter isolation are unchanged. The directly affected generator sample is model_generator_models_are_enumerable; the other seven property-generator tests have unchanged paths in this delta. No additional generator consumer search or audit was performed.

Accepted the coder's focused model receipt, 256 passing samples in 3.992065664 seconds, and scoped formatting/Clippy success without rerun. The current total from changed and accepted unchanged receipts is 10.571546060 seconds. The prior 10.231657553-second total is preserved as history, not substituted for the latest evidence. The combined-under-10-seconds AC remains unmet and no Done/unblocking claim is made. The one required final complete property-generator component gate is still pending with the coder; its actual outcome and combined time must be recorded honestly, without a timing-variance retry. Plan completion: not assessed.

