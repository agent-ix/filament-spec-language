---
id: SR-967175026302197768900474181941935084731
title: "QSL-2821 QSL-2823 gap-analysis of gate repairs"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3482; QSL-2821 QSL-2823; changed gate, package boundary and fixture paths"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `04212f15809eef6cfdbf00935fbf955bef3369fb`. Tickets: QSL-2821 and the assigned xtask portion of QSL-2823. All ten original QSL-2821 assertions are unchanged; checked-input scanner, ignored-reason assertion, scan-root equality, import-graph assertion, registry expectation, consumer-facade assertion and feature assertion retain their controls. No local build, test, repository executable or titan job ran. Runtime acceptance remains pending the lead's 31 exact named tests and merged-head compile gate. `git diff --check` passed. No applicable AssuranceProfile was found (the two textual matches are prior review prose).

The package boundary reads the same strictly admitted package through BarePackage; the temporary owner views do not read/construct a world or admit observations, and ClauseIdentity still reads the original package's own rows. The codec constructor becomes crate-visible. All five caller sites were followed. S6aRewardEvaluator is the existing production evaluator, not a copied helper; the lockfile adds only the consumer's default dev dependency. The fifteen import changes retain their existing defining types. Both new scan roots are real workspace members. QSL-2691 is the actual completed explicit-limit qualification ticket. The registry reuses the existing operation-contract exclusion.

The external-export change resolves only declared normal or target-specific dependency names, aliases by Cargo dependency key, and Rust builtins; local missing/private/member negative controls remain. Actual external declarations stay the Rust compiler's authority. The exact admitted_bytes, TaskSetRefusal and three raw-wire field classifications were compared with their implementations and FR-087/088; no suffix/pattern exemptions were introduced. The valid population fixture drops only obsolete model.version. X19 and upstream pins are unchanged.

## Verdict

FAIL — new control does not bind; inherited repository assurance gaps also prevent a repository-wide PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new cfg scanner control uses `#[ix_trace_rs::trace(...)]`, which the installed declared trace grammar does not bind. Static coverage lists `tests::cfg_setting_scan_distinguishes_flags_from_named_test_filters` at line 1646 under unmatched_tags for FR-063-AC-3; that criterion remains untagged. Import the trace macro in this test module and use the declared `#[trace(...)]` form so the new control binds. | xtask/src/seam_probe.rs:1646 |
| FND-002 | high | Repository assurance remains incomplete: the static matrix reports 1081 untagged and 68 ignored-only criteria, with extraction diagnostics; a full reverse inventory of every repository behavior has not been established in this PR review. This is inherited baseline debt, not a new source regression or an instruction to repair unrelated requirements in this fix round. The gap audit cannot certify repository-wide PASS. | spec/spec.md:1 |

## Coverage

Reconciliation: static `quire matrix --scope /home/peter/dev/worktrees/qsl-2821-gates --format json` (quire 0.52.0, engine 0.62.2), exit 0; no run evidence read. `quoin matrix --repo /home/peter/dev/worktrees/qsl-2821-gates --json` using /home/peter/.npm-global/bin/quoin 0.36.8 exits 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`, matching the previously reported limitation. This is not a native matrix PASS.

Engine status counts: tagged 5020; untagged 1081; tagged-by-ignored-test 68; method-without-symbol 107; retired 15. Coverage exit 0 reports 6209 untracked symbols, 7084 unmatched tags, 23504 diagnostics: 6 section-matches-nothing, 1 archetype-matches-nothing, 22866 trace-id-outside-marker, 608 untracked-id-has-minted-children, 22 uncatalogued-verification-method, 1 catch-all-universal. Rust binding census: 14446 candidates, 13871 tagged, 13272 bound. These are tool totals, not verified new defects or runtime evidence. FR-270, FR-489, FR-761, FR-911, FR-868 and NFR-013 changed obligations have active binders; FR-088 inspection criteria are method-without-symbol. FR-063's qualified markers do not bind; the new control is separately identified below.

Plan completion: not assessed

The reverse check follows the changed package identity boundary and five callers, registry candidate query, scan roots, import ownership, export resolver, feature manifest, seam settings, exact stage/identity/wire classifications and fixture admission (12 behavior groups). No new unspecified behavior, production stub, copied implementation, untracked skip or vacuous new assertion was found in those groups. Existing todo! strings in typestate fixtures are deliberate scanner adversaries. Full repository reverse coverage is not established; see baseline finding. Optional separate semantic gap lens skipped; requirement/code/test checks in the mandatory Rust review were performed. No spec/plan changes require spec-review.

Changed files examined:

- `Cargo.lock`
- `fsl-analyze/src/analyze/clause_item.rs`
- `fsl-analyze/src/model_check/liveness.rs`
- `fsl-package/src/emit/tests/deep_bodies/explicit_limit.rs`
- `fsl-replay/src/model_check/identity.rs`
- `fsl-replay/src/model_check/item.rs`
- `fsl-replay/src/model_check/replay.rs`
- `fsl-replay/src/reference_codec.rs`
- `fsl-semantics/src/check/assemble.rs`
- `fsl-semantics/src/check/check.rs`
- `fsl-semantics/src/check/check/integer_division_range.rs`
- `fsl-semantics/src/check/check/typing.rs`
- `fsl-semantics/src/check/ir.rs`
- `tests/fixtures/facade-contract-consumer/Cargo.toml`
- `tests/fixtures/facade-contract-consumer/tests/it/main.rs`
- `tests/fixtures/facade-contract-consumer/tests/it/protocol.rs`
- `xtask/src/ci_feature_lane_tests.rs`
- `xtask/src/consumer_facades.rs`
- `xtask/src/refinement/layering.rs`
- `xtask/src/seam_probe.rs`
- `xtask/src/string_edge.rs`
- `xtask/src/typestate_scan.rs`
- `xtask/tests/qualification_adapters.rs`

## Examined scope

- FR-270-AC-1 — spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; role: examined; excerpt: Planted signatures fail the gate: over a copy of the workspace's scanned trees with one planted item each, a `pub fn` in `fsl-eval` taking `&fsl_forms::Expression`; the same through `use fsl_forms::Expression as Expr`; the same through a `type` alias of a CST node; the same through a `pub use` of `fsl_forms::Expression` in `fsl-semantics`; a `pub` method in `fsl-route` taking `Option<Vec<&PackageDeclarations>>`; and a `pub fn` in `fsl-replay` outside `spine` taking a `filament-contract-model` package; and a `pub fn` in `fsl-eval` taking `&CheckedNodeId` beside a `&ContractPackage`, which gives
- FR-270-AC-2 — spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; role: examined; excerpt: Planted reconstruction fails the gate: a private function in `fsl-eval` with a checked-only signature that calls a `fsl-cst` parse function, one in `fsl-route` that calls `PackageDeclarations::check`, one there that calls `<PackageDeclarations>::check`, and one in `fsl-eval` that calls `.check(..)` on the result of `checked_dispatch_operation(..)?`, each give one `reconstruction` finding naming the file, line, function and the called stage function. The same parse call planted in `fsl-replay`'s `spine` module gives no finding.
- FR-270-AC-3 — spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; role: examined; excerpt: No false findings: a `pub fn` in `fsl-eval` taking `StateClauseKind`, a function in `fsl-replay` outside `spine` that constructs `fsl_cst::Limits`, a `pub fn` in `fsl-eval` taking `fsl_cst::Limits`, a `pub fn` in `fsl-eval` taking `filament_contract_model::CheckedNodeId`, a `pub fn` in `fsl-replay` outside `spine` taking `filament_contract_model::SourceIdentity`, and a `#[cfg(test)]` module in `fsl-eval` that takes a `fsl_forms::Expression`, give no finding. Two planted signatures in one run give two findings.
- FR-270-AC-4 — spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; role: examined; excerpt: Over the FSL workspace as it is, `cargo xtask checked-input` reports nothing and exits 0, and with one FR-270-AC-1 signature planted in a copy of `fsl-eval`'s shipped code the command fails with a finding exit status.
- FR-489-AC-2 — spec/functional/FR-489-build-the-production-linked-provider-registry.md; role: examined; excerpt: Candidate queries over the production snapshot include EN-1 and EN-6 for `temporal-satisfaction` and EN-4 for `probabilistic-satisfaction` when the query supplies the evidence kind EN-4 advertises, as FR-075-AC-13 requires; the FR-345 `ValueValidity` witness capability gives exactly `value-checker` and the `OperationContract` witness capability gives exactly `contract-checker`. An `operation-contract` item naming an `invariant` state clause in a request that names a model subject gives exactly EN-1; the same item with no model subject, and an `operation-contract` item naming an operation's `pr
- FR-064-AC-11 — spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; role: examined; excerpt: FSL's adapter passes its roots and rules to the standalone library, with no second detector implementation. On clean, violating and invalid-input fixtures, its report content and success/failure agree with the library; its existing lint prerequisite still propagates failure. Extraction adds only the scanner crate's own tests to FSL verification.
- FR-068-AC-6 — spec/functional/FR-068-split-expression-checking-into-check-stage.md; role: examined; excerpt: Every shipped (non-`#[cfg(test)]`) import under `fsl-semantics/src/check/` into this crate's own modules or the workspace crates `filament_exact`, `filament_semantic_value` and `fsl_foundation`, whether a `use` line or a `crate::`-rooted inline path, resolves into a module `check` MAY import (Behavior, "`check`'s import rule: modules by layer"): `filament_exact`; `filament_semantic_value`; `fsl_foundation`; `forms`; `check` and `family`; `model` and `value::model_query`; `library`; the §6.2 `semantic_value` modules `value::definition`, `value::enumeration`, `value::unit`, `value::semantic_node
- FR-761-AC-1 — spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md; role: examined; excerpt: `cargo xtask consumer-facades` over the committed bindings exits 0 and its report lists every inventory fact id, `CF-IR-04` with `fsl_package::SchedulabilityRoot::obligation_identity`, `CF-CG-04` with `fsl_replay::check_closed_form`, `CF-RT-02` with `fsl_analyze::monitor_root::admit_progress_view` and `CF-OB-01` with `fsl_package::checked_views::subject_kinds` among them.
- FR-761-AC-2 — spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md; role: examined; excerpt: With a bindings file whose `CF-PR-03` accessors are `["no_such_accessor"]`, the check exits 1 and names `CF-PR-03` and `fsl_package::protocol_view::no_such_accessor`; a crate-private item named as an accessor fails the same way.
- FR-911-AC-1 — spec/functional/FR-911-call-every-bound-consumer-fact-from-an-outside-crate.md; role: examined; excerpt: `cargo metadata` lists the workspace member `facade-contract-consumer` at `tests/fixtures/facade-contract-consumer`; each of its dependencies on `fsl-package`, `fsl-replay` and `fsl-analyze`, normal and development, enables no feature beyond the default, and no dependency of the crate enables a `test-support` feature of an `fsl-*` crate.
- NFR-013-AC-5 — spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md; role: examined; excerpt: A package is excluded only if it and everything below it are the same compile units with the same feature sets in the default workspace resolve as in the `--all-features` resolve on the host platform, including when `cargo metadata` gives every dependency the same merged feature set in both.
- FR-063-AC-3 — spec/functional/FR-063-exhaustive-family-extension-seam-probe.md; role: examined; excerpt: A test inspects the repository for anything that could set `seam_probe` outside `xtask seam-probe`'s own build invocation -- a `[features]` table entry, `build.rs`, `.cargo/config.toml`, or a `RUSTFLAGS`/`rustflags` setting in any `Makefile` target or CI workflow -- and asserts none exists
- FR-063-AC-6 — spec/functional/FR-063-exhaustive-family-extension-seam-probe.md; role: examined; excerpt: The checked-in seam-function list contains at least one entry for each of: the `FamilyKind` `catalog_code()` prefix arm; the S6a seam's `match` over the S6a family kind (`evaluate_declaration`); the parser's leading-token-kind entry table and the check seam over the parsed form enum; the checked node enum's evaluator, v2 emitter and requirement-derivation matches; and each family `Cause` enum's `catalog_code()`. A checked-in list missing the entry for any one of these categories, run against a real build that still has an `E0004` at that category's location (the source is unchanged), causes `x
- FR-063-AC-7 — spec/functional/FR-063-exhaustive-family-extension-seam-probe.md; role: examined; excerpt: A `match` at an S1 to S4 seam that carries a `_ => unsupported(...)` fallback arm produces no `E0004` for that seam under `RUSTFLAGS=--cfg seam_probe` and is therefore invisible to `xtask seam-probe` alone; `cargo clippy` over that seam's module fails on `clippy::wildcard_enum_match_arm` (or `clippy::match_wildcard_for_single_variants`, for the enum that trips it instead), so the lint gate, not the seam probe, is what catches this case. Every checked-in seam function carries the relevant `#[deny(...)]` attribute, and a test walks the real, checked-in tree source of each one and asserts the att
- FR-087-AC-2 — spec/functional/FR-087-typestate-and-cross-package-node-key.md; role: examined; excerpt: No conversion function exists from `ParsedSource` to `CheckedGraph` or `CheckedPackage` other than the S3 checker's own entry and the S4 link step's own entry; no conversion function exists from `ImportView` or any `protocol_artifact`-read value to `CheckedGraph` or `CheckedPackage`. A source scan of every function returning `CheckedGraph` or `CheckedPackage` confirms each is defined inside the S3 checker or the S4 link step (Inspection, primary evidence, since this rules out an in-crate bypass a doctest's external vantage cannot see); a `compile_fail` doctest per forbidden path (Behavior, "R-
- FR-088-AC-6 — spec/functional/FR-088-clause-name-and-type-identity.md; role: examined; excerpt: `QualifiedName` is used only as a declared component of an identity preimage (for example inside `DeclarationKey`'s or a checked node's own preimage where a name participates); no equality or hashing implementation on any identity type treats a `QualifiedName` as the sole identity-bearing field where a node id is available instead.
- FR-088-AC-8 — spec/functional/FR-088-clause-name-and-type-identity.md; role: examined; excerpt: `ValueTypeRef` is exactly the five-member union `{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String), Resolved(Box<ValueType>)}`; no sixth variant and no second definition exists, and no model field type is represented by a bare `NodeKey` or a raw string type name. `Clock` carries no fields (a place in a timed model, FR-230), and the `String` of `Unit` is a unit name the assembler resolves (FR-056-AC-42), not a type name. No production code outside the source-object assembler (FR-569 "The source model") constructs `Resolved`, domain package intake never does, and no serializ
- FR-868-AC-3 — spec/functional/FR-868-refuse-a-version-or-digest-in-an-observation-document-s-model-header.md; role: examined; excerpt: An invocation document whose `model` carries `version` refuses the same way, naming `model.version`; and a compile request's `models` entry and an observation's `model` member each refuse an unknown member, so neither input accepts a member the other refuses.
- spec/spec.md — spec/spec.md; role: examined; excerpt: name: filament-spec-language

## Dispositions

Round 1; inspected delta `04212f15809eef6cfdbf00935fbf955bef3369fb..1b5f2dd9189b7bc5c91bd61c31dd5ac79d320509`. Original findings above are unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test module imports ix_trace_rs::trace; both cfg controls use the unqualified declared marker. The supplied Quire 0.52.0 trace result resolves FR-063-AC-3 with structural verifies claims for both exact test symbols. This is structural trace evidence, not runtime success. |
| FND-002 | deferred | Inherited repository-wide traceability/extraction debt and exhaustive reverse inventory are outside this bounded red-main repair. The earlier repository audit remains FAIL; no new full matrix sweep was run and no repository-wide assurance PASS is claimed. This delivery only disposes the scoped source and new-control defects. |

Current scoped disposition: **CONDITIONAL pending runtime**. No unresolved scoped source finding and no new finding in this confined fix delta. The ten original gate assertions and exact file/type/field classifications are unchanged. The delta touches only seam scan helpers, their controls and imported trace marker, plus custody copies of the original review artifacts. `git diff --check` passes. All builds, no-run compilations and the 32 exact named cases remain the lead/arch titan lane; none ran locally. No second full review or matrix sweep ran.

The initial repository assurance verdict remains **FAIL** for the separately deferred baseline. The focused trace result is `/tmp/qsl-2821-fix-fr063-trace.json`; it has `resolved: true` and two structural verification claims. It establishes binding only.

### Authorized runtime-fix round 2

Authorized runtime-fix round 2, confined delta `1b5f2dd9189b7bc5c91bd61c31dd5ac79d320509..7e488e3f78111c940d9dc80024237a42e6ab9dc5`. No new finding in this delta. Earlier multiline cfg scanning and both declared trace markers remain unchanged; their prior fixed outcomes stand. Exact stage/diagnostic/wire exceptions are unchanged.

- The profile call names its existing `value::definition` owner; the aggregate re-export points at that same function. The import gate is unchanged.
- The full registry expectation adds exactly `en-7`, required by FR-489-AC-1/5 and the real Engines registrations. The separate exact OperationContract witness assertion remains `["contract-checker"]`; no capability rule changes.
- The seam list names `convert_outcome_resolving`, which owns the exhaustive FamilyOutcome match and wildcard-denial attribute. `convert_outcome` delegates to that owner.
- The qualification adapter sets the existing Call::canonical_values flag. FR-350 requires the public entry's typed result; FR-100/650 select the FR-181 owner writer. The existing runtime writes canonical completed values, and the existing adapter still takes the typed `result` from OutcomeDocument while retaining its complete native bytes separately for repeated-execution determinism. No translator, copied writer or new output grammar is introduced. The eight independently authored local expected-result literals and three integer value lookups follow the canonical Integer/Boolean shape. CallInput kind/decimal argument tags, wrong-result detection, retained bytes and determinism assertions remain intact. With no wire_arguments, the existing source call admission uses the already decoded native arguments; selecting canonical output does not reinterpret these corpus arguments.

Current scoped disposition: **CONDITIONAL pending runtime** for the 39 exact queued cases and titan compile checks. No local builds, tests, repository executables or full matrix sweep ran. `git diff --check` passes. The earlier repository-wide gap verdict and deferred baseline remain unchanged. This round adds no new finding and no new finding-outcome row; every original finding already has a fixed or deferred disposition.

Examined additional requirement scope:

- FR-489-AC-1; spec/functional/FR-489-build-the-production-linked-provider-registry.md; role: examined; excerpt: A production snapshot built from `fsl_analyze::Engines::fsl()` contains exactly the engine descriptors that collection exposes, including EN-1, EN-4, EN-6, EN-7, `value-checker` and `contract-checker`, and no descriptor without its engine implementation.
- FR-489-AC-5; spec/functional/FR-489-build-the-production-linked-provider-registry.md; role: examined; excerpt: `Engines::registrations()` yields exactly the descriptors owned by the EN-1, EN-4, EN-5, EN-6, EN-7 (`en-7`, advertising `schedulability`), `value-checker`, `contract-checker` and `protocol-analysis` engines in `Engines::fsl()`; `xtask` declares `fsl-analyze` as a normal dependency and its production factory registers each yielded descriptor once.
- FR-350; spec/functional/FR-350-run-the-conformance-corpus-against-fsl.md; role: examined; excerpt: Settlement compares the subject's typed result with `expected.value`.
- FR-100; spec/functional/FR-100-run-a-named-function-through-the-spine.md; role: examined; excerpt: A completed value is serialized in FSpec FR-181's tagged canonical form,
- FR-650; spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md; role: examined; excerpt: The public `fsl_command::command::run` family SHALL read function-call arguments and write completed call values in FSpec FR-181's tagged canonical form.

## Final acceptance

**COMPLETE for this scoped delivery.** The code lead supplied final arch/titan evidence for source `7e488e3f78111c940d9dc80024237a42e6ab9dc5`, merged as `b3c06de73787bb573a354adfc15b01e4ffa94b10`: all compile checks exit 0; all 39 named cases pass (20 library, 2 consumer-facade, 1 facade-feature, 9 qualification-adapter and 7 outside-consumer cases); the merged workspace compile gate exits 0. This supersedes the earlier runtime-pending conditions. Earlier scoped findings remain fixed; no additional review, matrix sweep or local test ran for this evidence-only update.

The separately deferred repository-wide assurance baseline remains unchanged; scoped completion does not claim a repository-wide gap-analysis PASS.
