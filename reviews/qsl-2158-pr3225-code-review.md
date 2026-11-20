---
id: SR-2026112022314402245831200
title: "Code review — QSL-2158 / QSL-2551 PR #3225 probability items through analyze and delay weighting"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3225 diff at cb17156278d9482160fc2cbf58b504e550129bb5: fsl-analyze, fsl-package, fsl-replay, fsl-semantics, tools/fsl-probability-check"
review_set: subset
evaluated_revision: "cb17156278d9482160fc2cbf58b504e550129bb5"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-254
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: references
---

## Summary

Tickets: QSL-2158 (with QSL-1826 item 1) and QSL-2551. Reviewed PR #3225 at head `cb1715627` (base `fa6efb9fa`) by reading the diff (`gh pr diff 3225`). Context came from `git show` against FSL `origin/pr-3225` and `origin/main` in `/Users/peter/dev/filament-spec-language`, and FSpec `origin/main` `cd89fcb31`. The brief's context checkout `/Users/peter/dev/qsl-2631-code` does not exist, so the main FSL clone was used read-only (fetch only). Nothing was built and no test was run. Neither ticket has an earlier reviewer comment on the two ported branches, so there are no open findings to disposition.

The routing is sound where it runs. The evidence kind selects the backend, and a negotiated candidate other than that backend settles `unsupported` (`request.rs:1251-1266`). An exact `certificate` is written only from `proved_by_exact_certificate`, which takes only an `Accepted` check (`proof_result.rs:1651-1683`). A refuted exact item comes only from a witness that `replay_probabilistic_witness` reproduced (`exact.rs:1546`). Replay parity settles `inconclusive`. A `measured` result is never a gap, a proof or a certificate. `limits` reach both engines, and the echo writes every default.

Four kinds of defect remain. A probabilistic item with no `probabilistic` member is not routed. The exact `certificate` member does not have the FR-621 shape. The FR-410 verdict wire drops members and invents values. EN-4 refuses delay weighting with the wrong cause.

## Verdict

**FAIL — fix round needed.** 1 high, 5 medium, 1 low.

Clean, examined: evidence default (exact-only claims give `exact`, all others `statistical`). Method defaults: a long-run fraction gives `regenerative{min_cycles: 100}`, every other form gives `okamoto`, and `min_cycles` on a non-regenerative method is refused. An exact item that carries `method`, `seed` or `timed_sampling` is refused. Exact and statistical limits are read with `deny_unknown_fields` and positive counts. `precision_bits <= max_precision_bits` is enforced. `mean of` with `sprt` is refused through `form.admits`. `max_samples` 10 settles `incomplete`. QSL-2551 S3: `delay` is resolved before any reward lookup, which FR-186:193-203 requires (the spelling shadows a reward named `delay`). An untimed `delay` is refused `TimeTypeMismatch{MissingTimeBinding}` at the `delay` word, giving `ill_typed`/`type-mismatch` with the FR-186-AC-15 message. The normal form has a distinct `delay` arm. FR-498 emits weighting kind `delay`. EN-5 (`TimedFormShape`), exact long-run (`not_yet`) and the long-run certificate document (`Unsupported`) refuse with a typed cause. No engine computes an unweighted value for a delay weighting. There is no Python and no logic-bearing shell.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | An item with no `probabilistic` member settles `unsupported` (FND-001). The certificate shape breaks FR-621/FR-741 (FND-002). The verdict wire breaks FR-410 (FND-003, FND-004). EN-4's delay refusal has the wrong category (FND-005). |
| Oracle integrity | `probability_refuted_from_source` moved its assertion from `counterexample.replay` to item-level `replay`. That is right by FR-621:122 and is not a bent test. The new `#[ignore = "QSL-2156: ..."]` on `pta_from_source` is honest: #3204 (QSL-2156, timed digital-clock certificates) is open and owns that dispatch. The 7 stubs still keyed to QSL-2158 are FND-006. |
| Test fidelity | The trace tag sits on a helper, one test is duplicated, and the bounded `always` and every-scheduler tests do not cover AC-2 as written (FND-007). |
| Safety/concurrency/wire | No unsafe code and no concurrency. `workers` is echoed but not passed to EN-4, which FR-191 allows (results do not depend on it). In `statistical_verdict_wire`, `let _ = method_name;` is dead code. |

## Coverage (manual AC-to-test check)

`quoin gap-analysis` / `quoin matrix` could not run at the PR head: the brief allows no worktree and no build. This manual check covers the changed tests.

- FR-741-AC-1: the okamoto, sprt, clopper-pearson and regenerative tests run over `Link` `Delivered` and the health `LongRun`, not over FR-185-AC-1's `Service` `P95`. They assert `verdict`, which is right (spec answer 1). No witness is checked, and the wire drops witnesses (FND-003).
- FR-741-AC-2: active for exact unbounded reachability, unbounded until and expected reward, each with an accepted `exact` certificate. `exact_finite_horizon_from_source`'s `Always` is `always[0,2]`, which is bounded, so unbounded `always` is not exercised. The every-scheduler test asserts no certificate. The fair every-scheduler test is bounded. Still undecided through analyze: `P95` `ExactValue{24233/25000}` with an independently checked certificate; unbounded fair eventual; the `Link` §15.3 long-run (kind `long-run`); §15.1 `lower`/`upper` at `max_rational_bits` 1,024; and checking an analyze certificate later over the emitted package.
- FR-741-AC-3: refuted with item-level `replay.reproduced` is active. The timed half (`pta_from_source`) is ignored for QSL-2156/#3204.
- FR-741-AC-4: all three clauses are active.
- FR-186-AC-15 and FR-186-AC-16 (TC-2168/TC-2169) and FR-498-AC-6 (delay part, TC-1288): covered.

## Spec questions

1. **`measurement` or `verdict`?** `verdict`. FSpec FR-331 (`spec/objects/interfaces/FR-331-backend-provider-envelope.md`) states it at :139 ("`measured` … carries the FR-409 decision and the FR-410 `StatisticalVerdict`"), :667 ("`verdict` (FR-410 for `measured`)") and in the table at :723 ("`measured` | `verdict`"). FR-410:23-26 says the provider records the `StatisticalVerdict` "in the FR-331 result of a `measured` item". FSL FR-621:119 makes `record` "the FSpec FR-331 terminal record, in its wire spelling", and FR-621:258 says FSpec FR-331 owns the spellings. FR-741:86 (`measurement`) and FR-741-AC-1 (:151, "a `measurement` naming the method and seed") contradict the wire. FR-741:90 also lists `witnesses` as a separate member, but FR-410:38/67 puts them inside the verdict. This is a spec gap: FR-741 should be reworded to `verdict`.
2. **Delay weight at the window end and on an idle tail?** The spec does not clip. FSpec FR-407:147-151 weights each position in `[0, h]` "by the delay of the step that leaves it". FSL FR-254:61-63 says the same, and FR-254:73-74 ends a sample when "its next position's time stamp passes the horizon, or at an idle tail". So the last position before the horizon carries the full delay of its leaving step, not `h − stamp`. For an idle tail, FR-407:153 continues a terminal window on its stutter steps but defines no delay for them, so a position with no leaving step has no defined weight. That is a real FSpec gap; FR-407 needs a ruling. The PR refuses delay weighting in EN-4 and computes no value, so no wrong verdict follows from the gap today.
3. **Where does `replay` go?** At the item level. FR-621:82 lists `replay` as an item member beside `counterexample`, and FR-621:122 defines `replay` as "Present with `counterexample`: `{target, reproduced}`". FSpec FR-413 gives the counterexample "exactly the members below" with no `replay`. FR-741:100-102 ("`counterexample` … with `replay` present only after FR-202 replay reproduced it") and FR-741-AC-3:153 ("a `counterexample` whose `replay` is present") should be read as the item-level member, and they need rewording. The code and `probability_refuted_from_source` are right. The disagreeing test is `every_scheduler_without_random_is_refuted` (`probability_from_source.rs:703`, ignored, QSL-2159, outside this diff). It asserts `counterexample.replay` and has to be corrected when QSL-2159 un-ignores it.

## Unattributed failures

- `provision_contract::source_completeness_is_checked_even_when_the_package_cannot_emit`: not this PR. The test expects `emit_checked` to refuse `prob_world::service("")`. The diff changes `provision_contract.rs` only by `workers: None`, and nothing in the diff changes S4 for an unweighted service. main's FR-740 emission makes the oracle stale.
- `statistical_model::an_undecided_contract_conjunction_leaves_its_test_undecided` and `branching_timed_formula_stops_at_the_longest_path_horizon`: not this PR as far as the diff shows. Neither claim has a `weighted by` clause. The S3 changes only add a `reason` to existing `TimeTypeMismatch` sites and a `delay` arm, and the `model.rs` changes branch only on `Weighting::Delay`. A check refusal at world construction does not come from this diff.
- `cyclic_mdp::separate_retry_actions_roundtrip…` and `cyclic_reward::cyclic_reward_roundtrip_stops_before_post_target_cycles` (`conflicting_identity`): not this PR. The tests swap the source (a different threshold or target) under an unchanged certificate, and the checker binds the claim identity. The diff does not touch the checker or identity bytes for unweighted or reward-weighted claims. The new dev-dependencies (`fsl-command`, `fsl-route`) bring no feature on a shared crate except `serde_json/unbounded_depth`, which does not affect identity.

All five need a control run on main to confirm. main moved to `82e13e97f` after the base, and QSL-2678 (#3212) refreshed stale oracles.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `probabilistic-satisfaction` item with no `probabilistic` member (`{"claim": "P"}`) builds no `ItemForm::Probabilistic` and settles `unsupported` through `AnalyzeItem::unadapted`. FR-741:52-71 defaults the kind and the evidence and has FSL write every left-out member, so this refuses valid input. Fix: when the kind is `ProbabilisticSatisfaction`, resolve `probabilistic_item(selection.probabilistic.unwrap_or(json!({})), …)` and add a no-member test. | fsl-analyze/src/analyze/document.rs:1825-1850; fsl-analyze/src/analyze/request.rs:1005-1055 |
| FND-002 | medium | The exact `certificate` is written as `{accepted, kind: "exact", document}`, without FR-621:120's `identity` and `checker`. `kind` is hard-coded, so an exact long-run proof (which also goes through `checked_report` → `proved_by_exact_certificate`) is labelled `exact`, not `long-run` (FR-741:93-99, FR-741-AC-2). An unparsable document drops the member silently. Fix: build the member in `item_document` with the item's identity and the EN-5 checker, and take `kind` from the document's objective. | fsl-analyze/src/analyze/document.rs:3725-3733, 4015-4016 |
| FND-003 | medium | `statistical_verdict_wire` drops FR-410 members. `witnesses` is always `[]` although `StatisticalVerdict.witnesses` holds the kept and undefined witnesses (FR-410:67, FR-741:90, FR-741-AC-1). Every provenance test writes `binding: null` and no `instance` (FR-410:65, FR-741:111-118), and the test rows have no binding. A `rejected` decision drops its `undefined-evaluation{where, cause}` (FR-410:61). Fix: spell the witnesses, the bindings and instances, and the rejection cause from `TestResult.undefined`, and remove `let _ = method_name`. | fsl-replay/src/outcome.rs:1615-1627, 1698-1717, 1754-1766 |
| FND-004 | medium | `statistical_estimate_wire` invents values. A `None` estimate (no sample counted, or no regenerative cycle of positive weight) is written as `0/1`. A `Censored` quantile is written as the probability estimate, so a censored measure reads as a finite quantity. Fix: spell `censored` per the backend-provider `Estimate` schema, and do not write a number for an absent estimate. If FR-410 has no spelling for an absent estimate, raise that as a spec gap. | fsl-replay/src/outcome.rs:1629-1650 |
| FND-005 | medium | EN-4 refuses a delay-weighted fraction or long-run fraction with `StatisticalRefusal::NotStatistical`, which settles `invalid-request` `not-statistical` ("a method the form does not admit", FR-741:142-143). FR-254:61-63 makes the form valid, so the caller is told their method choice is wrong. Fix: refuse with the engine's `unsupported` not-yet-implemented cause, the one timed fractions settle today. | fsl-analyze/src/statistical/model.rs:136-146 |
| FND-006 | medium | Seven ignored stubs still carry `ignore = "QSL-2158: …"` (exact_probabilistic_item.rs:603, 632; certificate_kinds.rs:90, 104, 122, 139, 161). Five of them are FR-741-AC-2 classes, so closing QSL-2158 on this PR leaves ignores that point at a Done ticket. Fix: re-key each ignore reason to the ticket that lands it (QSL-2156/#3204, which the PR body says rewrites `analyze/exact.rs` and `probability_certificate`), or keep QSL-2158 open. | fsl-analyze/tests/it/exact_probabilistic_item.rs:603; tools/fsl-probability-check/tests/it/certificate_kinds.rs:90-161 |
| FND-007 | low | The FR-741-AC-1 trace is on the helper `statistical_method_from_source`, not on the four `#[test]` functions. `exact_unbounded_from_source` only re-runs `an_exact_item_settles_proved_with_its_certificate`. `exact_every_scheduler_from_source` asserts no accepted certificate, which AC-2 requires. Fix: move the trace to the tests, delete the duplicate wrapper, and assert `certificate.accepted` and `kind` on the every-scheduler item. | fsl-analyze/tests/it/probability_from_source.rs:173-175, 320-322, 713-719 |
