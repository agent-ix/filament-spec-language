---
id: SR-967175026302197768900474181941935084850
title: "Code review: FSL PR #3885 cfail, ctimeout and cexpire refinement rows"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@7508fadd789835811f58886d1a769e587321c5a0; PR #3885 against origin/main (merge base 631b95ec3c); fsl-semantics/src/check/refinement.rs, fsl-semantics/src/check/refinement/protocol.rs, fsl-semantics/src/check/refinement/internal_list.rs, fsl-semantics/src/check/lowering/protocol.rs, fsl-replay/src/refinement/step.rs, fsl-eval/src/simulation/protocol/fairness.rs and their tests; context fsl-replay/src/refinement/liveness.rs, fsl-analyze/src/model_check/refine.rs, fsl-replay/src/model_check/refinement_replay.rs, fsl-eval/src/simulation/protocol/compensation.rs"
review_set: subset
---

## Summary

Ticket: QSL-3019. Code review with the rust-review lane, at head `7508fadd789835811f58886d1a769e587321c5a0`.

What I examined:

- S3 entries. `cfail`, `ctimeout` and `cexpire` each need a template row of their own. An operation right side on any of them refuses as `cend` does, through `applied_operation` returning `None`.
- The internal-list kinds, the class catalog and the emission fix. The template declaration is inserted into `nodes`. Lookups in `protocol_selection.rs` check `compensations` before `nodes`, so the extra entries are benign.
- The fsl-eval fairness filters. They match FSpec FR-430's table: an operation's class is `attempt` and `cattempt`; a template's class is `cattempt` and `cend`.
- The replay `check_protocol_step` filter. It stops an operation row from taking a `cfail` or `ctimeout`.
- The `protocol_class` spelling fix. `ProtocolTransition::template()` returns the qualified `<protocol>::<template>` (program.rs:474), so the old `{protocol}::{template}` produced `P::P::T` for all five compensation kinds. The fix makes the class equal the catalog spelling. It changes the class text on every path that calls `protocol_transition_class`: internal-list matching in step.rs, refine.rs and refinement_replay.rs, and FR-379 observation labels (step.rs:1717). It is a correction. The old text matched no catalog or internal entry, and no test or fixture pinned it.

## Verdict

**CONDITIONAL: mergeable at this head only if FND-001 is accepted as a tracked follow-up.** The row checks, the catalog, emission, the product mapping and the fsl-eval fairness classes are correct, and their tests fail at base. FND-001 is a real inconsistency: three liveness sites still count a `cfail` or `ctimeout` as taking an operation's fairness class. That contradicts the FR-430 classes this PR implements in fsl-eval. The PR body describes these sites as hidden-field-only, but `AbstractClass::is_taken_by` is on the main liveness path. FND-002 is low.

Focused runs at head: see `## Coverage`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three refinement-liveness sites still treat a `cfail` or `ctimeout` as taking its operation's fairness class, because they read `ProtocolTransition::application()` of any step. `AbstractClass::is_taken_by` (Taken::Protocol and Taken::ProtocolTraces), `protocol_path_label` in fsl-analyze, and the replay hidden-liveness graph all do this. FR-430, as this PR now implements it in fsl-eval `fairness.rs`, keeps an operation's class to `attempt` and `cattempt`. Over an abstract protocol with an `on each` trigger, a loop of failed attempts could discharge the abstract operation's required fairness and hide a liveness counterexample. The PR body names only the two hidden-field sites and omits `liveness.rs`. No test covers an `on each` abstract protocol. | fsl-replay/src/refinement/liveness.rs:314-327; fsl-analyze/src/model_check/refine.rs:3105-3108; fsl-replay/src/model_check/refinement_replay.rs:2367,2388,2404 |
| FND-002 | low | The spelling fix also changes the `cattempt` and `cend` classes and every compensation observation label, but only the `cfail`, `ctimeout` and `cexpire` internal entries have product tests. No test asserts that an internal `Spec::P::T` (cattempt) or `cend Spec::P::T` entry is closed over, or checks the label text of a compensation step. A regression in those two arms would go unnoticed. | fsl-replay/src/refinement/step.rs:1280-1285; fsl-analyze/tests/it/refinement_protocol.rs:3093-3112 |

## Review Methods

- Code review and rust-review lane: diff `origin/main...HEAD` read in full. Callers of `protocol_class` and `protocol_transition_class` traced. Every `ProtocolTransition::application()` reader in fsl-analyze and fsl-replay checked against the new FR-430 filters. The compensation successor generator read to test the PR's claim that `cfail` and `ctimeout` are emitted only beside a `cattempt` of the same application. That holds per applied record, apart from a `Written::Disabled` or `Cut` local write, which cannot occur for `Access::local`. New tests read for tautology; they assert step kinds on real decision traces. No `unwrap` or `panic` in production code, no new integer conversions, and no `unsafe`.
- Gap analysis: in `qsl-3019-gap-analysis.md`. Spec review: in `qsl-3019-spec-review.md`.

## Coverage

- Re-run at head, name-filtered (log `sr-3885/tests.log`): fsl-analyze 17 selected, 14 pass, 3 fail (the three known failures; their fixtures declare no compensation template); fsl-semantics lib `check::refinement::protocol::tests` 36/36.
- Not run: workspace or whole-crate gates (the lead's slot-B check owns these), fsl-package and fsl-eval filters (the coder reports 13/13 and 4/4), and no `on each` abstract-protocol liveness case.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | `the_replay_candidate_graph_applies_no_failed_compensation_attempt` never asserts that a rebuilt row holds a `cfail` step. It asserts that `add` is applied, that `inc` is never applied, and that `inc` is enabled at some hop. If `reaches` dropped every failing path, the `inc` assertion would pass vacuously. Only the coder's out-of-tree red run shows the test can fail. The model-check twin guards this with `failed > 0` over paths that contain a `CFail`. | fsl-replay/src/model_check/refinement_replay.rs:2606-2624 |
| FND-004 | low | `each_compensation_step_is_labelled_with_its_class` is tagged `FR-148-AC-11`, which states how a refusal violation and a terminal-success mismatch are reported. The test builds no refusal or mismatch; it asserts only the class text that `protocol_observation_label` gives each compensation step. That evidences FR-379 label construction, so the binding is loose. | fsl-analyze/tests/it/refinement_protocol.rs:3202-3205 |

## Dispositions

Round 1 at `52ab15215f39a4552abc526dfb3444ce44d787b0`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 4dfd9a865c adds `ProtocolTransition::applied_application` (Attempt and CAttempt only). `AbstractClass::is_taken_by` (both protocol arms, through `is_taken_along`), `protocol_path_label`, all three replay hidden-graph reads, the replay operation row and the fsl-eval operation class now read it. Each reader has a one-step `on each` test that fails if `inc` is counted taken. |
| FND-002 | fixed | 4dfd9a865c adds `an_internal_template_entry_closes_over_compensation_attempts_only`, `an_internal_cend_entry_closes_over_the_end_of_a_failed_thread` and `each_compensation_step_is_labelled_with_its_class`. The last asserts all five label classes spelled with the protocol once. |
