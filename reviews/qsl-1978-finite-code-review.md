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
| FND-001 | fixed | Round 1 at integrated source f8b98cc5a2f7ca0994efb851d05f83d38191e656: Both callers delegate to one iterative structural traversal with explicit ValueSynthesis/PlanParameter policies; declaration order, cycle refusal, historical paths and bounded Rational distinction are preserved. |

## Disposition round 1 verdict

PASS for this bounded PR review: the original finding is fixed and no new finding was identified. The original Findings row remains immutable above. This is neither whole-repository assurance nor whole-ticket acceptance.

Focused evidence supplied by the lead: five affected tests at exact integrated `f8b98cc5a2f7ca0994efb851d05f83d38191e656` passed (two finite semantics integration and three root integration), zero failures/ignored. Twenty-four unaffected tests retain their actual passing provenance at `e362e5ea9caeeb740e498b3f78a06ac94e8daa86`: sixteen analyze cases including both added retention cases and seven compatibility cases, three shared-traversal unit cases, four strict replay codec cases and one inspect case. Together these cover 29 unique cases across two recorded heads; this is not a claim of 29 tests run at one head. Eight touched packages' test code compiled at `e362e5e`; final integrated exact-custody checks remain the lead's mandatory separate gate. No reviewer runtime was executed.

The final integrated source inherits current main and the shared pins. Preserve latest disposition artifacts in source custody before the fresh current-main exact-final all-targets gate for foundation, semantics, eval, analyze, replay, inspect, command and root. Any subsequent source/base change requires inspecting its actual delta; no assurance or merge claim is made by this artifact.

### Round 1 machine-readable disposition

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "    super::finite_domain::first_unbounded_component(\n        types,\n\
    \        value_type,\n        super::finite_domain::DomainContext::PlanParameter,\n\
    \    )"
```

## Accepted-main dependency cohort integration addendum

This extends disposition round 1 under the same reviewer and IDs; it is not a new initial review or a fix round. Frozen integrated head `b550caa02768b038dc002f63f0a745b323631dfe`, actual merge base `86f2af011032ad58f7d4483ef3f5a86815457129`, tree `63c2c2dc094ccfaae0b4c722856fe6203e93700a`. Compared every own-file blob with pre-cohort custody head `5a08f4deb920cac65211084ce9ba4faa2bdc2a0c`: only `fsl-replay/src/lib.rs` differs, inheriting the owner's `WorkloadFreeDelayRefusal` re-export alongside preserved finite exports. No own finite production/test behavior or fixed-finding code was changed by the rebase. Manifest and lock cohort inputs equal actual main, with no local pin override.

Examined actual owner diffs at the accepted revisions, rather than assuming dependency bumps are inert. `filament-exact` b26a8e4c..a58474cc adds opt-in WorkScope shared work entitlement, an optional scope on Meter, guarded counter transition helpers, and the appended ModelCandidate charge point. FR-359 and FR-368 document shared work only, local size/result counters, cancellation polling before the owner lock, refusal atomicity, and unchanged plain Meter::new/clone behavior. Finite ModelSystem still constructs the plain meter and shares that existing meter through its request session; no WorkScope adoption or fresh-meter policy was added here. The new owner tests explicitly assert unscoped clone independence and scoped cancellation/denial behavior. This inspection is API compatibility context, not independent qualification of the whole upstream owner.

`filament-semantic-value` fa5e78aa..31d807cc, `filament-eval`/checked a0e8e125..79b8c42b and `filament-contract-ir` 96554ea5..c46c29a5 change their owner pins, not reference/enum/value/domain implementation APIs. `filament-observation` a583c512..004c7c4f also adds the history-free completeness revision seam and adapts revise; local callers found are temporal evidence, not finite argument enumeration or retained invocation/certificate code. `filament-mltl` c21d0d0a..b3881775 aligns owner pins. FSpec remains f6b0480c. Incoming timed probability implementation is accepted-main context on separate analysis/certificate modules; it is not re-reviewed as new finite work.

The full 29 focused finite/compatibility/traversal/codec/inspection cases require refreshed actual evidence at this cohort because core meter code changed. Earlier-cohort passing receipts remain historical and do not qualify the new dependency head. At this static checkpoint the coherent compile and refreshed runtime results are pending. No new static finding. Original findings, round-1 fixed outcomes and prior evidence text remain unchanged.

## Cohort integration style successor

At exact new-cohort `b550caa02768b038dc002f63f0a745b323631dfe`, the lead reports all 29 refreshed focused cases passed, zero failed/ignored: analyze16, semantics integration2 plus shared traversal unit3, root3, replay codec4 and inspect1. This supersedes older-cohort evidence for dependency compatibility, while retaining its historical provenance above. The exact eight-package test-code compile also passed without API adaptation.

Static successor reviewed: `60fec7edd8e84ea258c1eeceffe360a637127b28`. Its entire delta is three fsl-eval files, correcting peer-reported clippy findings without suppression. ApplicationContext groups the same borrowed receiver row, application and selected clauses at both apply_argument call sites; direct Option consumption preserves the recommendations/transition early returns, accounting vectors and final enabled vector. No invocation/domain ordering or frame contract is changed.

EvaluationMeter::Local now publicly contains Box<Meter> rather than Meter. This is an actual Rust variant representation/API change, not a claim that every public type is unchanged. All in-repository constructor and match sites were inspected; Deref, DerefMut and EvaluationMeter::with_cancel continue to act on the same underlying Meter. The Shared MutexGuard variant remains unchanged. Walk moves the boxed local Meter into its consuming with_cancel operation and reboxes it, retaining its counters and caller cancellation; source planning still uses its existing shared meter. No WorkScope adoption, meter reset policy, accounting/pin/wire change or lint allowance is introduced. External callers that explicitly construct or pattern-match the public Local payload must account for the Box representation; the reviewed workspace compile and affected runtime evidence establish only this repository's compatibility.

No new static finding. Exact-successor compile, focused fsl-eval library clippy and affected analyze16 runtime evidence remain pending at this checkpoint. The other 13 b550 cases are not represented as re-executed at the successor; their retention is limited to unchanged semantic/root/codec/inspect code after inspecting the three-file delta.

Integration checkpoint after successor checks: the lead reports eval/analyze/root test-code compilation PASS at exact `60fec7edd8e84ea258c1eeceffe360a637127b28`, and focused `cargo clippy --locked -p fsl-eval --lib --no-deps -- -D warnings` PASS. The earlier broader clippy attempt failed in dependency semantics large-error diagnostics and is not represented as passing. No suppression or dependency diagnostic fix was added.

The affected analyze16 invocation did not execute tests: the runner exhausted disk space (OS error 28) while writing `full.rmeta`. It yielded zero measured runtime cases and supplies no source or assertion failure. Runtime qualification at the successor remains pending; b550's actual 29 passes remain recorded without promotion to an exact-successor runtime claim. Final integration/custody approval is withheld until the affected 16 execute successfully after runner recovery.

## Final cohort integration verdict

PASS for the bounded finite PR review at `60fec7edd8e84ea258c1eeceffe360a637127b28`, base `86f2af011032ad58f7d4483ef3f5a86815457129`. Both original findings remain fixed; no new finding was identified in the accepted-main dependency integration or three-file style successor. This supersedes the earlier pending integration checkpoints, without changing their historical text or the original finding/disposition rows.

Actual retry evidence supplied by the lead: Titan `simple-parser-60fe-retry1`, exact `60fec7edd8e84ea258c1eeceffe360a637127b28`, exit 0, sixteen passed, zero failed/ignored. Runner reclamation preceded the single successful retry; no source change occurred. The original OS28 attempt remains unmeasured, not a failed test or a runtime pass.

The sixteen refreshed native analyze cases include finite argument ordering, subtype/current-state behavior, retained payload accounting and independent replay/checker cancellation, plus the compatibility regressions. Thirteen unaffected cases retain actual passing evidence at new-cohort `b550caa02768b038dc002f63f0a745b323631dfe`: semantics integration2/unit3, root3, replay codec4, inspect1. This is 29 qualified unique cases across explicit provenance heads, not a 29-case execution at the style successor. All 29 had also passed at b550 before the inspected three-file change. Successor eval/analyze/root test-code compile and focused eval library `--no-deps` clippy passed; the broader dependency-diagnostic clippy failure remains visible.

The public Local(Box<Meter>) representation change and repository compatibility scope are explicitly recorded above. No new wire/accounting/cancellation policy or WorkScope adoption is claimed. No reviewer runtime/build/source/ref/index changes occurred. Original same-ID findings and disposition round 1 remain immutable. Existing Linear round-1 dispositions remain the durable finding outcomes; this integration adds no finding or new fix round and does not repost those dispositions.

Latest artifacts must enter source custody before the mandatory fresh-main exact-final eight-package all-targets gate. This source-review approval does not replace that gate or imply whole-ticket acceptance or whole-repository assurance. Inspect any actual source/base delta before merging. Plan completion: not assessed.

## Subsequent accepted-main emitter integration

Main advanced from `86f2af011032ad58f7d4483ef3f5a86815457129` to `be3ad51571bb5e7b9c0e86588cfa32611053ed3d`, adding protocol recommendation/correlation and requirement-claim readback work. No dependency pin, finite plan/native model, shared meter or imported locus implementation changed in that main delta. The generic package emitter now calls encode_checked_package_v2_with_protocol_correlation_occurrences and supplies Some(empty rows) for a graph with no protocols. This is an actual envelope-byte change: the old encoder wrapper supplied None and omitted the member; the new path writes a present empty protocol_correlation_occurrences sidecar.

Inspected the pinned c46c29a5 owner encoder and strict correlation admission. Empty rows form no groups; the coverage loop skips nonprotocol nodes before protocol work charging. Finite synthesis fixtures have no protocol controls, so no control join or missing correlation obligation is introduced. Incoming checker canonical-position handling likewise traverses checked protocols only and is empty for these fixtures. Protocol-specific analysis/run-call requirement changes do not alter the synthesis route. No new static finding.

A single targeted refresh is requested after the actual true rebase: root finite_arguments_render_typed_reference_and_enum_values_deterministically exercises real generic package emission/readback and finite output across the changed empty-sidecar boundary. The already passing analyze16 plus retained13 qualify their recorded heads; no reflexive full29 rerun is requested for this separate protocol emitter change. Final integrated-head inspection and this one smoke receipt remain pending at this checkpoint.

## Final-main composition identity integration checkpoint

The actual fetched main advanced again to `cab672d971d7d62adee85129f529a95129bf5e4e`. True-own integrated frozen head `6396a19281108ebdbe88cf308815927cd6b53899` has that actual merge base and expected/actual tree `eaf30713db3b742431eb43983e31a0cba58a56c0`. Comparing every own-file blob with `60fec7e`, only command.rs, outcome.rs and check/mod.rs differ, in inherited distinct hunks. Original fixed code, native frames/meter style correction, finite test code and review artifacts remain preserved.

Examined the additional accepted-main composition-item identity/provider changes. ComponentPackages is exported and threaded through composition item scopes, request resolution and held graph identity. Provider success behavior is preserved; unavailable emission now retains its typed cause. The command.rs own overlap is an inherited unit test for that error cause. outcome.rs changes empty-cause omission on ItemDiagnostic, a separate type from the finite OutcomeDiagnostic/locus contract. The protocol canonical-position handling is empty for a package without protocols. No new pin, finite plan, argument domain, retained invocation, native meter or imported-locus behavior changed.

No new finding or additional runtime selection. The same single root finite_arguments_render_typed_reference_and_enum_values_deterministically smoke remains the justified integration refresh, covering the earlier generic empty-correlation sidecar emitter change and successful package readback at the actual final base. Root test-code compile and that smoke are pending at this checkpoint; current-base final approval is not inferred from the prior head's receipts.

## Final integrated approval

APPROVED for bounded source review at `6396a19281108ebdbe88cf308815927cd6b53899`, actual base `cab672d971d7d62adee85129f529a95129bf5e4e`. No new finding; both original same-ID findings remain fixed, with their original rows and round-1 dispositions unchanged. This final checkpoint supersedes earlier pending integration checkpoints without rewriting their evidence history.

The lead supplies exact integrated root test-code compile PASS and actual Titan `simple-parser-6396-c1` receipt: exit0, one passed, zero failed/ignored. Its finite_arguments_render_typed_reference_and_enum_values_deterministically case exercises real source compile, generic checked-package emission/readback, typed reference/enum output, replay/certification and repeated deterministic documents across the present-empty correlation-sidecar envelope change.

Final unique focused qualification is 29 cases with explicit provenance: analyze16 at `60fec7edd8e84ea258c1eeceffe360a637127b28`; twelve unaffected cases at new-cohort `b550caa02768b038dc002f63f0a745b323631dfe` (semantics integration2/unit3, root imported-origin2, replay codec4, inspect1); the root typed-output case refreshed at `6396a19`. All29 had previously passed at b550. This is not a claim that all29 ran at the final head. The first affected16 attempt's OS28 compilation stop remains unmeasured, followed by the single successful exact-head retry. Successor compile and focused eval --no-deps clippy PASS remain distinct from the broader dependency-diagnostic clippy failure.

Public Local(Box<Meter>) compatibility scope and actual empty-correlation sidecar bytes are explicitly recorded above. No hidden accounting/pin/wire-policy workaround, whole outcome reader, new finding, fix-round reset, or whole-ticket completion is claimed. Plan completion: not assessed. Latest reviewer artifacts require source custody before the lead's mandatory fresh current-main exact-final all-targets check across all eight actually touched packages. This approval does not replace that gate; inspect any subsequent actual source/base delta before merge.

## Final selected-model union main integration

APPROVED by retained source proof at `9e93b8fd8f4ba5e4533dd0781bde93c9684acdbd`, actual main/base `1d2713e9a70538c81d1352caad44f56303ae3a52`. Expected and actual true-own rebase tree are `1a24266678950e0dc889b60c114f805e7b69c221`. This is the latest source-review approval, extending the prior integrated approval without a new initial review, fix round, finding or rewritten disposition.

Inspected the actual cab672d..1d2713e selected-model union producer/accessor diff and all four overlapping own-file blobs: lowering/model.rs, check/mod.rs, check/refusal.rs and model/intake.rs. The new lowering path is reached by open_composite only for selected MODEL declarations matching a model-owned structural key and a payload-bearing CompositeShape::Union. Finite generated receiver unions have source-generated keys and remain in the existing source composite path. Finite argument enums are nullary ValueType::Enum and do not enter that composite path. The new emitter model_union_rows iterates only the newly retained selected-model-union map; that map is empty for the reviewed finite fixtures. Its empty result is an internal Emission witness vector, not another envelope-sidecar change. No new pin, finite argument-domain, invocation, accounting, imported-parameter origin or locus path changed. Intake only retains each variant's authored position for that selected-model witness. Parameter-origin hunks and earlier source receiver/enum behavior remain intact.

No additional runtime selection is justified for this disjoint payload-bearing MODEL-union route. Retain the 29 unique qualified cases with their exact recorded provenance: sixteen analyze cases at60fec, twelve unaffected cases atb550, one generic empty-protocol-sidecar emit/read case at6396. No runtime is claimed at9e93; the earlier generic empty-sidecar byte change remains explicit and its existing smoke passed. Latest same-ID artifacts must enter custody before the mandatory fresh-main exact-final eight-package all-targets gate. No new tracker finding or disposition is posted for this retained-proof addendum; the previous integration record remains historical evidence.
