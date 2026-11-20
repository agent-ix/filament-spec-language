---
id: SR-1527
title: "QSL-2054 property-generator code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2054; Cargo.lock; fsl-bench/Cargo.toml; fsl-bench/src/lib.rs; fsl-bench/src/prop/; fsl-bench/tests/it/; fsl-bench/tests/proptest-regressions/; fsl-cst/src/grammar.rs; fsl-cst/src/grammar/generate.rs; tests/fixtures/facade-contract-consumer/Cargo.toml"
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

Reviewed the frozen QSL-2054 diff for Rust behavior, representation, dependency containment, actual test assertions, and bounded property configuration. No code defects found in this scope. This is a pre-PR review at the branch and head above.

## Verdict

PASS for the reviewed diff. The generator library is default-off and exposes deliberate reusable test APIs. Value generation constructs independently inspectable types and expression trees before source rendering; it does not ask the checker to filter candidate programs. The bounded compound representation avoids duplicate named record/tuple declarations in generated shapes, and the expression constructors preserve their selected domain.

Grammar derivation walks the existing parser rule table using finite least-cost calculations. Its coverage assertion observes actual parsed CST productions instead of treating requested target labels as evidence. The only excluded production is ExtensionDeclaration, absent from the selected base grammar. The context specialization for ambiguous StepLabel forms uses an existing grammar production rather than a second grammar.

SmallModel has bounded one-to-three counters and a one-to-three maximum. Its independent transition description terminates under a product-state bound. Both source declarations and the library representation reach real intake/checking paths. The source declarations do not claim to carry the library's closed initial population; that difference is documented. Advanced model authoring or reduction agreement is not claimed.

Trace generation uses the evaluator's real finite/lasso representation, bounded proposition valuations, valid lasso indices, and explicit increasing timestamps for time-aware profiles. InfiniteTrace finite inputs are compatible with the evaluator's prefix mode; other profiles do not receive lassos. Near-miss expectations are authored before checking and assert code, cause and the complete intended source region.

The policy explicitly selects case counts, fixed ChaCha seed and bounded shrinking. Overrides are pure explicit arguments; the environment wrapper reads without mutation. Regression persistence writes replayable seeds and omits generated values. The negative sample really fails, observes a smaller counterexample within the bound, parses its saved seed, and removes its owned temporary directory. No production IPC or wire contract, unsafe code, new workflow trigger, vendored artifact, compatibility layer or new Make target is added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Applied code-review with the Rust lane and rust-style API, implementation, testing, safety and persistence rules. Repository AGENTS.md and CLAUDE.md conventions took precedence. The review covered every changed path, including seed-only regression files, Cargo.lock, test-target feature gating, and the existing-license metadata repair for the facade consumer fixture. Reviewed context included the existing all-feature Cargo test lane and evaluator finite-prefix/lasso dispatch.

Accepted the author's in-lane focused receipts without rerun: all eight acceptance tests have passing receipts; 1,000 Linked expression samples; all 228 grammar productions observed; 256 model, near-miss, package and trace samples; deterministic override and real false-property shrink/persistence tests. Scoped CST/bench Clippy and owned-file formatting passed. Dependency bans passed; the original combined license failure was preserved, and only licenses were rerun successfully after the authorized fixture metadata correction.

No tests, builds, lints, gates, matrix or upstream probes were run in this review. The final affected-component gate remains with the coder. No whole-repository assurance-profile discovery was performed under the explicit diff-only dispatch.

## Timing and acceptance limit

The accepted latest suite times sum to approximately 14.2 seconds, with each sample suite below 10 seconds. This satisfies the ticket's policy of at most 10 seconds per suite and 60 seconds for all property suites. A separate acceptance bullet says the combined sample time must be below 10 seconds; that wording conflicts with the policy and is not claimed as passed here. The review does not substitute a new timing measurement or silently change either limit.

## Publication

No PR number exists yet because the lead reserves one open language PR. Linear posting is deferred until the real number is supplied; pr:null and the reviewed branch/SHA are explicit. No placeholder PR number or comment was invented. The inherited exact model variant is unavailable and recorded as unknown. Quoin skill package version is recorded separately from any unqueried executable version.

## Supplemental Dispositions — Round 1

PASS, no correctness findings in the confined performance delta from d13ace0f1e5b1f769ee6db66906f7392e143aa3e to 5ab906af4fc855a37c974108c34ff67eb74a8e86. Public PR filament-spec-language#3193 now exists; the prior pre-PR publication paragraph is historical. Actual executable Quoin version for posting is 0.29.4.

Examined only fsl-cst/src/grammar/generate.rs, fsl-bench/src/world.rs, and the changed model sample setup in fsl-bench/tests/it/generators.rs. Grammar planning borrows one immutable grammar and normal-cost array, uses private dense work arrays indexed by the contiguous fieldless Production enum, and computes targeted costs over reverse context edges. The shared productions macro generates both enum variants and all() in the same order. Parent edges carry the containing production's expansion cost plus non-target sibling costs; the finite nonnegative shortest-path calculation preserves bounded derivation selection. Per-target rendering and the StepLabel context remain in place.

ModelSetupCache is caller-owned and keyed by exact library bytes plus exact model selections. It reuses only successfully admitted SelectedModels, including operation-clause parses. Each case still renders and parses its own unit, builds forms, assembles declarations, performs S3 Linked checking, constructs its own world/snapshot, and runs the independent enumerator. The stored admitted model set is cloned before each assembly; its normalization Meter is owned value state, so subsequent checking cannot mutate the cached initial meter or another case's state. Failed admissions and per-case checking results are not cached. Initialization remains inside the timed property closure.

Accepted changed author receipts without re-execution: 228 grammar derivations in 0.562363959 seconds and 256 model samples in 3.652177157 seconds, plus unchanged accepted suites totaling 6.017116437 seconds. Their sum is 10.231657553 seconds. The explicit combined-under-10-seconds acceptance condition remains unmet; this is not a Done/unblocking or aggregate-gate claim. Scoped Clippy and owned-file formatting receipts remain accepted. No tests, builds, profile runs, matrix, lint or full review was repeated.

One concrete optional optimization is visible directly in the changed path: World::build_using at fsl-bench/src/world.rs:381 calls package_input before looking up the exact-byte/selection cache, although packages is consumed only in the admit_unit cache-miss branch. That function reparses PackageDocument and clones the supplied bytes. Moving that immutable input preparation into the cache-miss path can avoid repeated work while keeping every source/S3 check. This is an optimization opportunity, not a correctness finding or a timing-success claim; no profiling or new measurement was used to identify it.

## Supplemental Dispositions — Round 2

PASS, no correctness findings in the eleven-line World::build_using delta from 5ab906af4fc855a37c974108c34ff67eb74a8e86 to 3d01aa9e3ba97919fd93ff706bdbd9dc1ad6499e on public PR filament-spec-language#3193. No other source path changed in this round.

PackageInput preparation now occurs only after an exact-document/selection cache miss. PreparedModel retains the successfully constructed package input together with the admitted models, and a hit clones both into the new World. Consequently, World.packages remains present for downstream model/snapshot resolution; the optimization does not replace it with an empty input or a reference to another case's mutable world. Cache misses still perform real package intake, admission and operation-clause parsing. Source parsing, form construction, declaration assembly, S3 Linked checking, the cloned normalization meter and independent sample enumeration remain unchanged for every case. Ordinary uncached World::build still takes the miss branch and performs intake.

Accepted the changed focused model sample receipt: 256 samples passed in 3.992065664 seconds; scoped formatting and Clippy passed. The latest sum with unchanged accepted suite receipts is 10.571546060 seconds. The earlier 10.231657553-second result remains historical, and the combined-under-10-seconds AC is still unmet. No timing retry, test, build, profile, lint, matrix or whole review was performed by this reviewer. No Done or dependent-unblocking claim is made.

Within the generator component, the directly affected sample remains model_generator_models_are_enumerable; the expression, grammar, near-miss, package-input, trace and two config sample paths have no changes in this delta. The lead's planned single final complete property-generator component gate remains appropriate; this disposition does not request a separate consumer audit or repeat passing authoring segments.

