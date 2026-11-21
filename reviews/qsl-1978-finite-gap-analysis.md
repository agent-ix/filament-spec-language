---
id: SR-2026112121341313320502
title: "QSL-1978 finite arguments gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3399; own 27-file finite/reference/enum/imported-location delta"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-1013
---

## Summary

Ticket: QSL-1978. Independent native Codex INITIAL review, maximum two fix rounds, same artifact IDs. Static review only at dd08c501fa4c1e7fe031e3688b52d3d98a44536a against ba280846beb6e8bf79c89b179c8841839c203090. No reviewer cargo/build/test/replay/gate execution. Author-reported production check and test-code compilation are compilation evidence only; no runtime pass is claimed. Exact native model id is unavailable in this harness; model metadata is native-codex-exact-id-unavailable. Source is frozen; dependency pins must be refreshed by the author after the shared pin window and the same reviewer must inspect that actual delta before final gates.

## Verdict

CONDITIONAL: bounded changed-surface coverage gap; no repository assurance PASS.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new retention test denies the first runtime Applied retention before Completed (began=1, completed=0), so it cannot exercise PlanStep::from_native retention or its newly added replay/certificate stop propagation. Add bounded tests that allow runtime retention and deny the additional plan payload retention, and independently exercise replay and certificate derivation with the named accounting cause. Positive certification and the early search denial do not cover those new error paths. | fsl-analyze/tests/it/plan_search.rs:1794 | correct-requirement-no-evidence |

## Coverage

All 27 changed files examined, including clean native retention, typed step identity equality/order, current-state reference filtering and exact deduplication, enum ranks, mixed operation identity, argument field presence, receiver union selection, independent native replay and closure derivation, command projection, shared strict locus decoder, public bounded diagnostic-only readback and inspect region avoidance. No spec/plan diff: spec review and its sub-analyses do not apply. No applicable AssuranceProfile found. Reverse mapping uses FR-1012/FR-1013 and imported-locus FR-267/FR-663; no new unowned production behavior or source/test stub was found in this delta. No gate or warning threshold was removed.

Plan completion: not assessed

Computed static matrix completed exit 0 with quire 0.52.0/engine 0.62.2. The 7.3 MB result and 29,843 diagnostic lines are not a repository assurance pass; scoped extraction confirms FR-267-AC-33/34 and FR-663-AC-4 tagged. Source-only review binds these tests to their asserted portions, not full criterion qualification. FSpec tags require cross-repository qualification and are not established by this single-repository matrix. No run evidence store was evaluated. Semantic intent inspection is bounded to this authorized PR delta. Optional/Option parameters, other finite families, creation and receiverless authored operations remain outside this cut.

## Examined scope and finding data

```yaml
clean: false
scope:
- id: fsl-analyze/src/plan_search.rs
  path: fsl-analyze/src/plan_search.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-650 and FR-800 to FR-804: bounded, metered, certified plan

    //! search over a native state model (FR-755, FR-1011, FR-1012).

    //!

    //! The search is separate from `fsl-replay`''s proof-composition planner and'
- id: fsl-analyze/src/plan_search/certificate.rs
  path: fsl-analyze/src/plan_search/certificate.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-650''s `potential` and `closure` certificates: the writer the

    //! search calls after a complete exploration, and the core checker

    //! ([`check`]), which re-derives every entry''s successors through

    //! [`ModelSystem`] and re-evaluates goal, policy and objective itself.'
- id: fsl-analyze/src/plan_search/replay.rs
  path: fsl-analyze/src/plan_search/replay.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-650''s native replay: every reported plan is re-applied from the

    //! bound initial state through [`ModelSystem`], each step enabled, every

    //! state''s invariants checked, every policy and the goal re-evaluated and

    //! the objective recomputed, before it is reported. A plan that fails replay'
- id: fsl-analyze/src/plan_source.rs
  path: fsl-analyze/src/plan_source.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-800 to FR-803: the goal, objective and policy rules of a plan

    //! synthesis written in FSL source, evaluated over the checked package.

    //!

    //! [`SourcePlan`] is the [`PlanRules`] of one checked `synthesis` declaration'
- id: fsl-analyze/tests/it/model_world.rs
  path: fsl-analyze/tests/it/model_world.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Model subjects for the EN-1 safety tests (`crate::model_check_safety`):

    //! a domain package with one population, a native unit selecting it as `M`

    //! with its contracts and temporal clauses, the initial snapshots, and the

    //! letters of each clause''s `holds` atoms.'
- id: fsl-analyze/tests/it/plan_search.rs
  path: fsl-analyze/tests/it/plan_search.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! QSL-1978: the plan search engine (FR-755, FR-1011, FR-1012) over FSpec

    //! FR-650''s published counter fixture: model `M`, `Counter.value` in

    //! `Int[0, 20]`, `inc` then `double`, one counter `c` with `value = 1`.

    //!'
- id: fsl-command/src/command/output.rs
  path: fsl-command/src/command/output.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-026: construct typed JSON views of native outcomes and actual provenance.


    mod types;

    '
- id: fsl-command/src/command/output/types.rs
  path: fsl-command/src/command/output/types.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-026: typed native result views; upstream identities retain their serializers.


    use filament_contract_model as ir;

    use fsl_foundation::{LocatedSpan, SourceIdentity};'
- id: fsl-eval/src/simulation/model.rs
  path: fsl-eval/src/simulation/model.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-120: [`ModelSystem`], the FR-101 [`TransitionSystem`] of a checked

    //! package''s state family, and the entries that explore and sample it.

    //!

    //! A model state is one closed population state. Its successors are every'
- id: fsl-eval/src/simulation/model/frames.rs
  path: fsl-eval/src/simulation/model/frames.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-120 "Post-states": the applications of one object''s operations and the

    //! candidate post-states inside each operation''s frame, generated lazily.

    //!

    //! A candidate is one choice of three things, made in this order: which'
- id: fsl-foundation/src/lib.rs
  path: fsl-foundation/src/lib.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! `fsl-foundation`: the ADR-011 §6.1 layer **F** foundation crate

    //! (ADR-011 §7.3 X-2).

    //!

    //! Module order, per ADR-011 §6.1''s F row: [`backend`] < [`json_number`] < [`serde_object`]
    <'
- id: fsl-foundation/src/source.rs
  path: fsl-foundation/src/source.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-001: immutable source text, the caller-named source reference and

    //! checked scalar coordinates.

    //!

    //! [`provenance`] holds ADR-013 O-07/O-12''s occurrence key, source region'
- id: fsl-inspect/src/result_views.rs
  path: fsl-inspect/src/result_views.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Typed projections of producer results that are not package or node views.


    use alloc::{string::String, vec::Vec};

    use filament_exact::Cancel;'
- id: fsl-replay/src/lib.rs
  path: fsl-replay/src/lib.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! `fsl-replay`: the ADR-011 §6.1 layer **6** crate (ADR-011 §7.3

    //! X-10) -- the CG-facing replay facade (ADR-011 §6.1, ADR-013 O-24 to

    //! O-27, C-13). It depends on layers 1 to 5, F and K, as §6.1 allows.

    //!'
- id: fsl-replay/src/outcome.rs
  path: fsl-replay/src/outcome.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-286 (ADR-029 CB-4): the one `filament-outcome/1` JSON outcome document

    //! every FSL lifecycle operation''s outcome serializes to, and its one

    //! serializer.

    //!'
- id: fsl-replay/src/spine.rs
  path: fsl-replay/src/spine.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-027, FR-098 and FR-278: the spine (ADR-011 §5, §7.3 M-6a; ADR-013

    //! O-26). Complete-V1 source enters S1 and goes through the stage APIs in

    //! DAG order: S1 (`fsl_cst::parse`), S2 (`fsl_forms::build_unit`), I1

    //! (`model::intake::admit_unit`, the unit''s `model` declarations against'
- id: fsl-replay/src/spine/native_result.rs
  path: fsl-replay/src/spine/native_result.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-267: canonical clause-run result production from the FR-109 report.


    use std::cell::RefCell;

    use std::collections::BTreeMap;'
- id: fsl-replay/src/spine/refinement.rs
  path: fsl-replay/src/spine/refinement.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-341 and FR-342: the refinement class of a spine compile result and of a

    //! clause-run disposition (ADR-017 RF-2), and [`CompileRefusal::causes`], the

    //! all-causes accessor FR-341 reads.

    //!'
- id: fsl-semantics/src/check/check.rs
  path: fsl-semantics/src/check/check.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Name resolution and typing of Complete-V1 value expressions and function

    //! declarations into the typed tree, before any definedness or termination

    //! judgment.

    //!'
- id: fsl-semantics/src/check/lowering/model.rs
  path: fsl-semantics/src/check/lowering/model.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-094: model-owned nodes, the `Reference<T>` and

    //! `Population<T>[N]` type nodes, clause-function owners and quantity type

    //! nodes.

    //!'
- id: fsl-semantics/src/check/mod.rs
  path: fsl-semantics/src/check/mod.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! ADR-011 §7.3 M-5 (FR-068): the layer-3 checking half of what was

    //! `value::expression`. This module owns name resolution, typing, static

    //! definedness and termination checking, and the checked-output types

    //! ([`CheckedGraph`], [`CheckedExpression`], `CheckedFunction`) whose'
- id: fsl-semantics/src/check/plan_value.rs
  path: fsl-semantics/src/check/plan_value.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-800 "The plan value": the S3 types of the plan a plan

    //! synthesis''s `satisfies` block and objective read.

    //!

    //! The declaration''s name denotes a record with `length`, `final`, `initial`'
- id: fsl-semantics/src/check/refusal.rs
  path: fsl-semantics/src/check/refusal.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Located checking refusals and their closed codes and causes.


    use super::protocol_memory::AccessClass;

    use filament_checked::node::EndSide;'
- id: fsl-semantics/src/check/region.rs
  path: fsl-semantics/src/check/region.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-096: a check [`Location`] resolves to a [`SourceRegion`] of the unit

    //! its declaration was read from.

    //!

    //! `Origin::Body{index}` starts at the body of function `index`, and'
- id: fsl-semantics/src/model/intake.rs
  path: fsl-semantics/src/model/intake.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FR-154: lift a spec-bundle to Semantic IR 2.0.0 bytes, admit them as a

    //! domain package selection, and read the admitted document''s IR nodes into

    //! [`super::domain_package::DomainPackageRecord`]s.

    //!'
- id: fsl-semantics/tests/it/synthesis_plan_value.rs
  path: fsl-semantics/tests/it/synthesis_plan_value.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! FSpec FR-800 to FR-802: a plan synthesis types the plan value its goal and

    //! objective read (`Reach.length`, `.final`, `.initial`, `.steps`), refuses a

    //! plan length that is not a non-negative integer range, and refuses a read

    //! of a member the plan value does not have.'
- id: tests/it/plan_export_surface.rs
  path: tests/it/plan_export_surface.rs
  role: examined
  excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    //! Request-surface tests for the plan outcome document, the plan search

    //! limits, plan refusals and the reference planning examples (FR-1010 to

    //! FR-1014), and for the TLA+ and Lean export outcome, mappers and golden

    //! corpus (FR-1015 to FR-1018). Each feeds source and a `native-run/1`'
- id: FR-1013
  path: spec/functional/FR-1013-refuse-plan-declarations-and-requests-by-stage.md
  role: examined
  excerpt: FSL SHALL raise FSpec FR-806's check-time refusals at S3, located at the
    source name, and its request-time refusals when the run request is admitted, located
    at the request member's pointer, and SHALL build the plan space only from the
    checked-package nodes FSpec FR-807 lists.
- id: FR-663-AC-4
  path: spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md
  role: examined
  excerpt: Test |
- id: FR-267-AC-33
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: Test |
- id: FR-267-AC-34
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: Test |
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: fsl-analyze/tests/it/plan_search.rs
  related_ids:
  - FR-1012
  path: fsl-analyze/tests/it/plan_search.rs
  lines: 1794-1873
  excerpt: fn finite_argument_retention_denies_the_exact_named_occurrence_bound()
    {
  finding: The new retention test denies the first runtime Applied retention before
    Completed (began=1, completed=0), so it cannot exercise PlanStep::from_native
    retention or its newly added replay/certificate stop propagation. Add bounded
    tests that allow runtime retention and deny the additional plan payload retention,
    and independently exercise replay and certificate derivation with the named accounting
    cause. Positive certification and the early search denial do not cover those new
    error paths.
bindings:
- test_id: tests::imported_source_locus_stays_byte_free_in_inspection
  ac_id: FR-267-AC-33
  trace: correct
- test_id: refusal_locus_tests::imported_loci_and_native_controls_preserve_distinct_wire_forms
  ac_id: FR-267-AC-33
  trace: correct
- test_id: refusal_locus_tests::outcome_diagnostic_reader_limits_its_claim_and_rejects_invalid_diagnostics
  ac_id: FR-267-AC-33
  trace: correct
- test_id: finite_imported_parameter_origins_preserve_uri_endpoints_and_flat_details
  ac_id: FR-267-AC-33
  trace: correct
- test_id: refusal_locus_tests::imported_loci_refuse_mixed_missing_and_malformed_members
  ac_id: FR-267-AC-34
  trace: correct
- test_id: refusal_locus_tests::imported_command_details_are_closed_without_native_labels_or_bytes
  ac_id: FR-267-AC-34
  trace: correct
- test_id: refusal_locus_tests::outcome_diagnostic_reader_limits_its_claim_and_rejects_invalid_diagnostics
  ac_id: FR-267-AC-34
  trace: correct
- test_id: refusal_locus_tests::imported_command_details_are_closed_without_native_labels_or_bytes
  ac_id: FR-663-AC-4
  trace: correct
- test_id: finite_imported_parameter_origins_preserve_uri_endpoints_and_flat_details
  ac_id: FR-663-AC-4
  trace: correct
- test_id: finite_missing_parameter_origins_remain_unavailable
  ac_id: FR-663-AC-4
  trace: correct
```: correct
```

## Disposition pass 1 review scope

Frozen source head `a3780d3668af4477c348c37e6a9dc123a7b031f2`, own delta against current-main base `cf8d85826db560569cd8806cce07007d4b493e0c`. The initial Findings rows and initial scope above remain unchanged. This pass rechecked both original findings and all six observed runtime corrections, including source fixtures and imported diagnostic readback.

The source now has one iterative `finite_domain::first_unbounded_component` traversal. Context-specific leaf policy preserves standalone synthesis behavior, planning state-relative references, finite-but-unsupported bounded Rational and existing union path spellings. Old recursive synthesis traversal and planning traversal were removed; their wrappers delegate to the shared implementation. Declaration order, map-key-before-value order, first back-edge and unknown-composite refusal remain explicit.

New retention tests allow the native seven-unit Applied result and deny the extra four-unit PlanStep payload at a ten-unit bound. The independent checker uses a twenty-one-unit bound after the search's eleven units; replay separately uses a ten-unit bound. Cancellation fixtures observe real Cancel polls and trigger Requested/Deadline at the payload charge; no production meter or evaluator is mocked. Source assertions check named stops, retained consumption and absence of certification. Runtime confirmation is pending at this static checkpoint.

Runtime corrections inspected: typed Boolean snapshot wrappers; schema-valid generated origin without source coordinates; context-specific S3 plan-parameter refusal category; and imported flat command details decoded from the retained canonical arena rather than the shallow-envelope placeholder. Closed source-coordinate and native byte-bearing locus forms remain shared by native and diagnostic-only readers, with existing negative mutation checks retained. No whole outcome payload reader is introduced.

The true rebase inherits main's IR `96554ea506a19b16ff9863ad3d30066578bc609c` and FSpec `f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0` pins in both manifest and lockfile, identically to the reviewed base; no private pin workaround appears in the own diff. Parent composition/refinement/API changes are base context, outside this finite PR's own delta. The original own finite production surfaces are unchanged by the rebase before the explicit fix commit.

No new static finding. No reviewer runtime, build, matrix, index/ref/source edit, commit, push or merge. Final disposition verdict awaits focused pinned runtime evidence and does not replace fresh-base exact-final custody gates.

Disposition round 1 continuation: frozen head `e362e5ea9caeeb740e498b3f78a06ac94e8daa86` adds only the new traversal unit test's pin-compatible `EffectiveId::new(None, OriginalDeclarationKey::new("test", "ix://test/Object"))` reference fixture. The production source, policies and dependency pins remain identical to `a3780d3`; the static conclusions above remain applicable. Focused runtime evidence is still pending.

Disposition round 1 integration checkpoint: frozen head `f8b98cc5a2f7ca0994efb851d05f83d38191e656` has actual merge base `7fe342358da787b1e3c9d57dae3bbc60bc899f32`, including the merged inline producer. Comparing every own-file blob with `e362e5e`, only three differ: `plan_search.rs` tests retain the incoming inline/named-range four-outcome and out-of-range controls; `check/mod.rs` retains incoming PackageHyper exports; the unavailable-source fixture supplies a nonempty generated input identity taken from its actual parameter source before replacing the origin. All other own code, tests and original SR blobs are unchanged. No borrowed operation origin, fabricated source coordinates or admission weakening is added.

At `e362e5e`, the author reported exact eight-package test-code compile PASS and 28/29 focused runtime PASS. The sole failure was the generated-origin fixture's invalid empty input list at intake; the integrated fixture now preserves a genuine input identity without source coordinates. The 24 unaffected runtime cases retain their actual PASS provenance at `e362e5e`, including both new payload-retention paths, all compatibility cases, strict imported codec and inspection tests, and three traversal regressions. The five affected semantics/root cases are pending at `f8b98cc`; no integrated result is asserted yet.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 at integrated source f8b98cc5a2f7ca0994efb851d05f83d38191e656: New real Completed-before-denial tests pass, including independent replay/checker named accounting stops and Requested/Deadline cancellation at payload retention. |

## Disposition round 1 verdict

PASS for this bounded PR review: the original finding is fixed and no new finding was identified. The original Findings row remains immutable above. This is neither whole-repository assurance nor whole-ticket acceptance.

Focused evidence supplied by the lead: five affected tests at exact integrated `f8b98cc5a2f7ca0994efb851d05f83d38191e656` passed (two finite semantics integration and three root integration), zero failures/ignored. Twenty-four unaffected tests retain their actual passing provenance at `e362e5ea9caeeb740e498b3f78a06ac94e8daa86`: sixteen analyze cases including both added retention cases and seven compatibility cases, three shared-traversal unit cases, four strict replay codec cases and one inspect case. Together these cover 29 unique cases across two recorded heads; this is not a claim of 29 tests run at one head. Eight touched packages' test code compiled at `e362e5e`; final integrated exact-custody checks remain the lead's mandatory separate gate. No reviewer runtime was executed.

The final integrated source inherits current main and the shared pins. Preserve latest disposition artifacts in source custody before the fresh current-main exact-final all-targets gate for foundation, semantics, eval, analyze, replay, inspect, command and root. Any subsequent source/base change requires inspecting its actual delta; no assurance or merge claim is made by this artifact.

### Round 1 machine-readable disposition

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "    assert_eq!(\n        system.evaluation_meter().consumed(LimitKind::ResultUnits),\n\
    \        7,\n        \"payload denial retains no invalid candidate\"\n    );"
```
