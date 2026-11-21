---
id: SR-2026112121341313320501
title: "QSL-1978 finite arguments code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3399; own 27-file finite/reference/enum/imported-location delta"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-1013
---

## Summary

Ticket: QSL-1978. Independent native Codex INITIAL review, maximum two fix rounds, same artifact IDs. Static review only at dd08c501fa4c1e7fe031e3688b52d3d98a44536a against ba280846beb6e8bf79c89b179c8841839c203090. No reviewer cargo/build/test/replay/gate execution. Author-reported production check and test-code compilation are compilation evidence only; no runtime pass is claimed. Exact native model id is unavailable in this harness; model metadata is native-codex-exact-id-unavailable. Source is frozen; dependency pins must be refreshed by the author after the shared pin window and the same reviewer must inspect that actual delta before final gates.

## Verdict

FAIL: duplicate type-extent traversal requires consolidation under the mandatory duplication rule.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new unbounded_parameter walker duplicates the Option/collection/map/composite traversal, cycle handling and first-component path construction already implemented by synthesis::first_unbounded_component at synthesis.rs:381-485. Searches for first_unbounded_component and unbounded classifiers found that existing home. Consolidate the traversal into one helper with explicit per-context leaf/reference policy, preserving the planning distinction between finite unsupported forms and unbounded forms. | fsl-semantics/src/check/plan_value.rs:160 | implementation-bug-despite-evidence |

## Coverage

All 27 changed files examined, including clean native retention, typed step identity equality/order, current-state reference filtering and exact deduplication, enum ranks, mixed operation identity, argument field presence, receiver union selection, independent native replay and closure derivation, command projection, shared strict locus decoder, public bounded diagnostic-only readback and inspect region avoidance. No spec/plan diff: spec review and its sub-analyses do not apply. No applicable AssuranceProfile found. Reverse mapping uses FR-1012/FR-1013 and imported-locus FR-267/FR-663; no new unowned production behavior or source/test stub was found in this delta. No gate or warning threshold was removed.

Plan completion: not assessed

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
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: fsl-semantics/src/check/plan_value.rs
  related_ids:
  - FR-1013
  path: fsl-semantics/src/check/plan_value.rs
  lines: 160-246
  excerpt: "fn unbounded_parameter<'a>(\n    types: &'a filament_semantic_value::declaration::TypeEnvironment,\n\
    \    value_type: &'a ValueType,\n) -> Option<String> {"
  finding: The new unbounded_parameter walker duplicates the Option/collection/map/composite
    traversal, cycle handling and first-component path construction already implemented
    by synthesis::first_unbounded_component at synthesis.rs:381-485. Searches for
    first_unbounded_component and unbounded classifiers found that existing home.
    Consolidate the traversal into one helper with explicit per-context leaf/reference
    policy, preserving the planning distinction between finite unsupported forms and
    unbounded forms.
bindings: []
```