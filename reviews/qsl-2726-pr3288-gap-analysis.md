---
id: SR-20261121003288002
title: gap-analysis of package temporal selections PR 3288
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2726; fsl-command/src/checked_handoff/selection.rs,
  fsl-command/src/command/package_run.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/input.rs,
  fsl-eval/src/simulation/model/letters.rs, fsl-eval/src/simulation/model/reference_bindings.rs,
  fsl-eval/src/temporal/evidence/binding.rs, fsl-eval/src/temporal/mod.rs, fsl-eval/src/temporal/observed.rs,
  fsl-eval/src/value/expression/bare.rs, fsl-eval/src/value/expression/evaluate.rs,
  fsl-eval/src/value/expression/mod.rs, fsl-package/src/bare_term.rs, fsl-package/src/model_input_form.rs,
  fsl-package/src/reader_protocol_projection.rs, fsl-replay/src/model_check/replay.rs,
  fsl-replay/src/spine.rs, fsl-replay/src/spine/clause.rs, fsl-replay/src/spine/clause/model_trace.rs,
  fsl-replay/src/spine/clause/temporal.rs, fsl-replay/src/spine/clause/temporal/package_selection.rs,
  fsl-replay/src/spine/clause/tests/temporal/wire.rs, fsl-replay/src/spine/clause/trace_refusal.rs,
  fsl-replay/src/spine/native_result.rs, fsl-replay/src/spine/native_result/temporal.rs,
  fsl-semantics/src/check/temporal_clause.rs, tests/it/request_surface_s1_16.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-1039
  type: references
---

## Summary

Frozen head 777e1c271262c76556a97c6d1c070302e8de9e90, compared to origin/main through d9400e7640. Ticket QSL-2726 remains open for partial delivery. Read README, AGENTS, PR body, architecture comments including 956ac915, and all 27 changed files. No applicable AssuranceProfile found. No production, spec or plan edits. Progressed/timestamped/ledger, source atom spelling/locus rendering and decisive-walk plumbing are accepted reader-pin boundaries; no finding treats them as missing delivery.

## Verdict

CONDITIONAL — No additional scoped implementation defect found; computed-matrix extraction warnings and existing unmatched criteria prevent a repository assurance PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

No Cargo, aggregate gates, clippy, fmt, whole-crate tests or reviewer reproduction batches ran, as directed. Measured evidence supplied in the brief: five package_clause_selections tests passed at 0f17629bc0, and the existing timed_no_row regression passed once. Frozen-head rerun results were not independently observed and are not claimed. Required final premerge focused TEST-RUN remains with the lead/Draco.

Rust checks included API ownership, typed failures, read accounting, exact timed interval/authentication paths, shared observed evaluator regressions, reference literal admission and universe/current-state separation, missing-object undefined, equality witness usage, stage mapping, strict result parsing, test oracles and source removal. No dependency or CI gate changes; no duplicate vendor tree or hollow changed implementation found.

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2), static only; no execution evidence store read. The matrix JSON completed, with many existing symbol-id-collision/unmarked-trace-id/section-matches-nothing diagnostics. Existing full requirement rollups: FR-1039 tagged 6/untagged 1, FR-267 tagged 21/untagged 8, FR-330 tagged 17/untagged 1, FR-522 tagged 26, FR-642 tagged 6, FR-931 tagged 16/untagged 1/method-without-symbol 1. These full requirement counts include unchanged and reader-dependent work; no global coverage or completion claim follows. New five tests carry TC-3019/FR-1039 parent tags; these are behavioral evidence, not an assertion of new criterion-level coverage. Reverse-gap inventory: 27 changed files and their new public APIs map to FR-1039/FR-330/FR-522/FR-642/FR-267; zero new unowned behaviors or source/test stubs found in this subset. Semantic intent review ran over these scoped behaviors under the explicit dispatch.

Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: fsl-command/src/checked_handoff/selection.rs
  path: fsl-command/src/checked_handoff/selection.rs
  role: examined
  excerpt: //! The selection sidecar, `filament.checked-handoff.selection/1` (FR-517),
    and
- id: fsl-command/src/command/package_run.rs
  path: fsl-command/src/command/package_run.rs
  role: examined
  excerpt: '//! FR-100, FR-945, FR-1039: a `native-run/1` request that selects an
    existing'
- id: fsl-eval/src/simulation/model.rs
  path: fsl-eval/src/simulation/model.rs
  role: examined
  excerpt: '//! FR-120: [`ModelSystem`], the FR-101 [`TransitionSystem`] of a checked'
- id: fsl-eval/src/simulation/model/input.rs
  path: fsl-eval/src/simulation/model/input.rs
  role: examined
  excerpt: '//! FR-531, FR-940: [`ModelCheckInput`], what a model check reads of a'
- id: fsl-eval/src/simulation/model/letters.rs
  path: fsl-eval/src/simulation/model/letters.rs
  role: examined
  excerpt: '//! FR-125 Instancing, FR-107: the production [`ModelLetters`] of a checked'
- id: fsl-eval/src/simulation/model/reference_bindings.rs
  path: fsl-eval/src/simulation/model/reference_bindings.rs
  role: examined
  excerpt: '//! FR-1000: resolve subject-independent reference literals at FR-120
    subject'
- id: fsl-eval/src/temporal/evidence/binding.rs
  path: fsl-eval/src/temporal/evidence/binding.rs
  role: examined
  excerpt: '//! FR-522: the profile, clock binding name and clock parameters a'
- id: fsl-eval/src/temporal/mod.rs
  path: fsl-eval/src/temporal/mod.rs
  role: examined
  excerpt: '//! The layer-5 TemporalTrace evaluator (FR-327, FR-328, FR-329, FR-125):'
- id: fsl-eval/src/temporal/observed.rs
  path: fsl-eval/src/temporal/observed.rs
  role: examined
  excerpt: '//! FR-330 (FR-327, FR-328, FR-329): a temporal clause evaluated over'
- id: fsl-eval/src/value/expression/bare.rs
  path: fsl-eval/src/value/expression/bare.rs
  role: examined
  excerpt: '//! FR-531, FR-533, FR-945: evaluation of calls from a package read from
    its'
- id: fsl-eval/src/value/expression/evaluate.rs
  path: fsl-eval/src/value/expression/evaluate.rs
  role: examined
  excerpt: //! The metered evaluator of checked expressions and function calls.
- id: fsl-eval/src/value/expression/mod.rs
  path: fsl-eval/src/value/expression/mod.rs
  role: examined
  excerpt: //! Complete-V1 value expressions and total pure functions (FR-145, FR-146).
- id: fsl-package/src/bare_term.rs
  path: fsl-package/src/bare_term.rs
  role: examined
  excerpt: '//! FR-940 "Package forms of the source-checked evaluators": the `holds`'
- id: fsl-package/src/model_input_form.rs
  path: fsl-package/src/model_input_form.rs
  role: examined
  excerpt: '//! FR-940: the forms of a package the package route''s model-check input
    does'
- id: fsl-package/src/reader_protocol_projection.rs
  path: fsl-package/src/reader_protocol_projection.rs
  role: examined
  excerpt: //! Reader-owned protocol expression nodes projected from admitted I2 terms
- id: fsl-replay/src/model_check/replay.rs
  path: fsl-replay/src/model_check/replay.rs
  role: examined
  excerpt: '//! FR-128 (ADR-018 CX-3, DL-5, UE-5, UE-6): [`replay_model_trace`], the'
- id: fsl-replay/src/spine.rs
  path: fsl-replay/src/spine.rs
  role: examined
  excerpt: "//! FR-027, FR-098 and FR-278: the spine (ADR-011 \xA75, \xA77.3 M-6a;\
    \ ADR-013"
- id: fsl-replay/src/spine/clause.rs
  path: fsl-replay/src/spine/clause.rs
  role: examined
  excerpt: '//! FR-109: `fsl_replay::spine::run_clause`, the spine''s clause-run'
- id: fsl-replay/src/spine/clause/model_trace.rs
  path: fsl-replay/src/spine/clause/model_trace.rs
  role: examined
  excerpt: '//! FR-330 `TraceInput::Model`: a clause run over a model-arm behaviour.'
- id: fsl-replay/src/spine/clause/temporal.rs
  path: fsl-replay/src/spine/clause/temporal.rs
  role: examined
  excerpt: '//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves
    one'
- id: fsl-replay/src/spine/clause/temporal/package_selection.rs
  path: fsl-replay/src/spine/clause/temporal/package_selection.rs
  role: examined
  excerpt: '//! FR-1039, FR-330: a temporal clause a checked package carries, run
    over a'
- id: fsl-replay/src/spine/clause/tests/temporal/wire.rs
  path: fsl-replay/src/spine/clause/tests/temporal/wire.rs
  role: examined
  excerpt: '//! TC-1160 (FR-267 "Temporal disposition wire", FR-266-AC-4, FR-285-AC-7):'
- id: fsl-replay/src/spine/clause/trace_refusal.rs
  path: fsl-replay/src/spine/clause/trace_refusal.rs
  role: examined
  excerpt: '//! FR-645: a malformed or mismatched trace refused with a reason, a message'
- id: fsl-replay/src/spine/native_result.rs
  path: fsl-replay/src/spine/native_result.rs
  role: examined
  excerpt: '//! FR-267: canonical clause-run result production from the FR-109 report.'
- id: fsl-replay/src/spine/native_result/temporal.rs
  path: fsl-replay/src/spine/native_result/temporal.rs
  role: examined
  excerpt: '//! FR-267 "Temporal disposition wire": the closed `disposition.temporal`'
- id: fsl-semantics/src/check/temporal_clause.rs
  path: fsl-semantics/src/check/temporal_clause.rs
  role: examined
  excerpt: '//! FR-326: the `TemporalTrace` family''s S3 check of one `temporal` clause'
- id: tests/it/request_surface_s1_16.rs
  path: tests/it/request_surface_s1_16.rs
  role: examined
  excerpt: //! Request-surface stubs for model-based testing (FR-752), the planner
- id: FR-1039
  path: spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md
  role: examined
  excerpt: A `native-run/1` request MAY select an existing `filament.checked-package/v2`
    artifact through its `package` member in place of a `program`. FSL SHALL then
    admit the package with the strict checked-package reader, take every selection
    a source run reads from its header and bindings from the package's own `lock`
    and name rows, run the request's member over the
- id: FR-931
  path: spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md
  role: context_only
  excerpt: 'A `native-run/1` request asks for FR-757''s lowering through its `tl_lowering`
    member. The member names the clause, may name the versioned TL correspondence
    the lowering is to use, and may ask for cross-evaluation. The correspondence is
    part of the claim: `mltl.closed-trace/v1` and `mltl.online-prefix/v1` read the
    same formula under a complete and an open surrounding execution, and'
- id: FR-267
  path: spec/functional/FR-267-write-run-results-as-native-run-result-2.md
  role: examined
  excerpt: The clause-run result document of the spine check route and the `run` command's
    command-error envelope SHALL be `native-run-result/2` (FSpec FR-352), as ADR-031
    SW-10 and SW-11 and ADR-013 OQ-1 decide. The library serializes `fsl_replay::spine::run_clause`'s
    `ClauseRunReport` (FR-109) as the `/2` clause-run document; this requirement owns
    that producer. A
- id: FR-330
  path: spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md
  role: examined
  excerpt: '`fsl_replay::spine::run_clause` (FR-109) SHALL accept a `Temporal` selection
    that names one temporal clause, a supplied trace and the clause''s `over` binding.
    It compiles the unit through the spine, admits each observation of the trace (FR-106),
    evaluates the clause at S6a through the TemporalTrace evaluator (FR-327, FR-328,
    FR-329), and returns one `ClauseRunReport`. With'
- id: FR-522
  path: spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md
  role: examined
  excerpt: The spine temporal evaluator SHALL compare the profile, the clock binding
    name and the clock parameters that a clock-bearing request asserts with the checked
    clause's own selection before it visits any position. For a request that agrees,
    the evaluator evaluates it and the report retains the authenticated binding the
    request agreed with. For a request that differs, the evaluator
- id: FR-642
  path: spec/functional/FR-642-run-a-timed-clause-over-a-recorded-timed-trace.md
  role: examined
  excerpt: 'A clause under `filament.temporal.timed/v1` SHALL be checkable and runnable
    over a recorded timed trace: a sequence of positions each with an exact rational
    time stamp. Over a timed model subject the clause binds `model-time` (FR-233)
    and the trace supplies the stamps the snapshots do not hold. Over a declared record
    trace subject the clause''s clock name is a role that the request''s `clocks`
    member'
```

## Disposition round 1 verification

No gap findings required disposition. Fix-only inspection at 9b91fdd454bf140f1ff40e3cdbe9f34815885d51 found no additional gap in typed-locus retention or the new focused regression. The original scoped matrix limitations and partial-delivery boundaries remain; no matrix rerun or global assurance PASS is claimed. No Cargo or gates ran in this reviewer.

## Final test-oracle verification

Verified 9b91fdd..b6015b50b4388fe80ddb20b99b0b72d318189ea2: only the focused regression changed. Code/cause/fields/locus assertions now read the actual clause-report path `disposition.refusal`; typed-failure equality, stage and strict-reader assertions remain. Production behavior is unchanged, so FND-001 remains fixed in disposition round 1. The initial run failed because the test looked for root-level `refusal`; the corrected regression passed 1 test, 0 failed, 0 ignored, as observed in `/tmp/qsl-2726-selections-admission-locus-fix.log`. No reviewer Cargo/gates ran. Lead-owned final focused TEST-RUN remains pending.
