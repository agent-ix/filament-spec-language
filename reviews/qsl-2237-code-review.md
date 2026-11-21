---
id: SR-2026112017124277723988805
title: "Code review of QSL-2237 in FSL PR 3127"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2237, PR 3127 at e39e97365f8ce2016587f3170fc30ed8fe44d7f9; fsl-command/src/command/model_based_test.rs, fsl-command/src/command.rs, fsl-command/src/command/{output,items,package_run,wire}.rs, fsl-eval/src/simulation/{model,sample,trace}.rs, fsl-eval/tests/it/{mbt_shrink,model_based_tests,model_operation_coverage}.rs, fsl-eval/tests/it/model_simulation/model_based_run.rs, tests/it/request_surface_s1_16.rs"
review_set: subset
---

## Summary

Ticket: QSL-2237 (FR-752, direct `model_based_test` run surface). Code review with the rust-review lane folded in, by reading only, on head e39e97365f. Re-check of arch's fix list from head efea71f8d: FND-001 (AC-5 refusals carry `invalid_runtime_input` plus catalog cause) fixed for the evaluator refusals and asserted for code and cause; FND-002 (drop AC-2 and FR-190-AC-5 tags from the rewritten request-surface test) fixed; FND-003 (dead `uniform_below`) fixed, gone from fsl-eval/src/simulation/choice.rs; FND-004/FND-005 (trace tags) only partly fixed, see FND-001 and FND-002 below.

## Verdict

Medium findings: the command layer maps every evaluator refusal to `invalid_runtime_input` even when the cause belongs to another code, decode-level member refusals still go out as `invalid-request`, the hand-written snapshot reader drops incomplete populations without refusing, and source parse/check failures are flattened into a Debug string. The fsl-eval stopped-state findings change is consistent across sample, rebuild and replay. Gates (fmt, clippy, tests) were not run, because no build lane was available. `cargo fmt --all -- --check` is red repo-wide at this head, including on files this PR does not touch, so its output does not separate this PR's formatting from drift already on main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Trace tags still name criteria the test does not assert. `a_failing_clause_retains_a_one_minimal_replayed_counterexample` keeps `FSpec-FR-190-AC-6` (four `inc` from 0 over `Int[0,5]`), but it asserts a zero-invocation counterexample for `false`. `model_based_test_value_clause_runs_and_shrinks_natively` claims `FSpec-FR-190-AC-8` (size-bounded enumeration recorded in provenance; refusal when the bound is missing) but asserts only the x=200 counterexample. `a_passing_run_records_tested_and_never_proved` claims FR-752-AC-3, a `verify` plan step whose record names the step, but it runs a direct request with no step. | tests/it/request_surface_s1_16.rs:121, tests/it/request_surface_s1_16.rs:1068, tests/it/request_surface_s1_16.rs:186 |
| FND-002 | medium | Three FR-752-AC-4 tests stay `#[ignore = "QSL-2237: ... not implemented yet"]` over a `generated_coverage` stub that always returns None. This PR edits one of them (`operation_coverage_is_deterministic_and_settles_no_verdict`) and leaves it ignored, so the new assertions never run. The ignore reasons become false when QSL-2237 lands, and the stubs stay bound to AC-4 in `quoin matrix` (binders ignored: true). | fsl-eval/tests/it/model_based_tests.rs:55-58, fsl-eval/tests/it/model_based_tests.rs:68, fsl-eval/tests/it/model_based_tests.rs:93, fsl-eval/tests/it/model_based_tests.rs:113-127 |
| FND-003 | medium | `model_test_error` maps every `ModelBasedTestRefusal` to `RunCause::ModelBasedTestRequest`, which is always `Code::InvalidRuntimeInput`. The evaluator also returns the causes `invalid_model_binding/malformed-declaration` (empty domain), `invalid_model_binding` (subject admission), `cancelled` and the catch-all `invalid_runtime_input`. Example: a value clause over an empty parameter domain comes out as code `invalid_runtime_input` with cause `invalid_model_binding/malformed-declaration`. That cause is not in the catalog, and FSpec FR-190 requires `invalid_model_binding`/`malformed-declaration` (FR-190-AC-9 is tagged on model_based_test_request_rules). The evaluator's catch-all also turns Shrink, Sample and OracleDisagreement errors into a refusal with exit 20, where FR-752 makes them a record settlement (`failed`, exit 30). That arm is on main, but this PR's direct route makes it reachable. | fsl-command/src/command/model_based_test.rs:134-139, fsl-eval/src/simulation/model/model_based_test.rs:497-500, fsl-eval/src/simulation/model/model_based_test.rs:521, fsl-eval/src/simulation/model/model_based_test.rs:541-544 |
| FND-004 | medium | Part of the original FND-001 is still open. `decode` runs in `prepare_run_bytes_with_sources` before the evaluator and refuses through `RunCause::ClauseRequest` (`invalid-request`, no catalog cause). So `max_value_bits` 65 gives `invalid_runtime_input`/`invalid-value`, but 256 or -1 gives `invalid-request`, for the same 1..=64 rule. `max_cases` 0, `max_trace_length` 0, a missing `clause` and a bad `seed` likewise never reach the evaluator's typed `invalid-value`/`missing-member` refusals. | fsl-command/src/command/model_based_test.rs:29-101, fsl-command/src/command/model_based_test.rs:73-82, fsl-command/src/command.rs:3764-3766 |
| FND-005 | medium | The state-clause snapshot is parsed a second time by a private `Snapshot` serde struct. Populations not marked `complete` (and `complete` defaults to false when absent) are silently left out of `universes`. FR-620 and the existing `initial_subject` path refuse `incomplete_population` through `SubjectSelection::check_universes`. A snapshot with an incomplete or unmarked population therefore runs with a missing universe instead of being refused. | fsl-command/src/command/model_based_test.rs:104-128, fsl-command/src/command/model_based_test.rs:483-500, fsl-command/src/command.rs:3872-3899 |
| FND-006 | medium | `route_error` turns any parse, select or check refusal of the program into `invalid-request` naming `model_based_test.clause`, with a `{error:?}` Debug dump as the reason. Example: a syntax or type error in unit.native under a model_based_test request comes out as `invalid-request` at member `model_based_test.clause`, not as the source diagnostic code (e.g. invalid_syntax) that the clause and tl_lowering routes return. The prepare path also hard-codes `fsl_cst::Limits::default()`, `SpineLimits::default()`, `default_accounting(10_000)`, `max_candidates: 1_000_000` and `position_limit: 10_000` instead of the request's limits, and an empty `DependencyInput`. | fsl-command/src/command/model_based_test.rs:130-132, fsl-command/src/command/model_based_test.rs:393-425, fsl-command/src/command/model_based_test.rs:530-541 |
| FND-007 | low | FR-752-AC-5 asserts are incomplete in the request-surface tests. Not asserted: that each of the three refusals (bits 0 and 65, and the `Int[0,5]` unknown-member case) names `max_value_bits`; that `Square` with bits 8 settles `tested` with `cases.generated` 20; that no case leaves [-255,255]; and that no partition kind ends in `-refusal`. | tests/it/request_surface_s1_16.rs:963-975, tests/it/request_surface_s1_16.rs:978-1003, tests/it/request_surface_s1_16.rs:1023-1040 |
| FND-008 | low | `_observation_override` has a leading underscore but is used (`.unwrap_or_default()`), which misleads the reader. `max_trace_length` round-trips u64 to usize to u64 for no reason. The same six-line stopped-state findings block is copied into sample.rs, trace.rs and model.rs and belongs in one TransitionSystem helper. | fsl-command/src/command/model_based_test.rs:364, fsl-command/src/command/model_based_test.rs:514, fsl-command/src/command/model_based_test.rs:521-523, fsl-eval/src/simulation/sample.rs:148-162, fsl-eval/src/simulation/trace.rs:172-181, fsl-eval/src/simulation/model.rs:2471-2486 |
| FND-009 | low | The doc comments of the de-tagged tests still open with `FR-752-AC-1:` and `FR-752-AC-2:`. `quoin matrix` reports both as `unmarked-trace-id`, so the prose claims the criteria that the tags were removed to stop claiming. | tests/it/request_surface_s1_16.rs:118, tests/it/request_surface_s1_16.rs:145 |

## New findings (disposition pass 1)

FND-010..FND-013 record arch's round-2 findings (draco:QSL:arch FND-006..FND-009) in this file's sequence, so that their dispositions sit beside this review's. FND-014 and FND-015 are new findings from this pass, at head a868ede0ed22a5896e79d0feaa70785330f49674.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | (arch FND-006) The entry and the direct request refused `max_trace_length` on a value clause (`unknown-member`). FR-752 Entry says it is accepted and unused. | fsl-eval/src/simulation/model/model_based_test.rs, fsl-command/src/command/model_based_test.rs |
| FND-011 | medium | (arch FND-007) The plan coordinator re-derived the catalog code from the refusal cause string (`refusal_catalog`) instead of using the refusal's own code and cause. | fsl-analyze/src/plan.rs:590-600 |
| FND-012 | medium | (arch FND-008) Meter and cancellation stops were refused as faults instead of settling `incomplete` (exit 22); `runtime_invariant` must be reserved for internal faults. | fsl-eval/src/simulation/model/model_based_test.rs:852-905 |
| FND-013 | medium | (arch FND-009) The plan path rendered model_based_test records with a second writer (`model_based_test_record`) instead of the direct route's `result_json`. | fsl-command/src/command/plan/outcome.rs:271-290 |
| FND-014 | medium | No test exercises the new state-path conversions in `run_model_based_test_with_system_and_admission`: an `Expansion(EvaluationMeter)` stop settling an `incomplete` record with `limit-reached`, and `Shrink(NotCounterexample)` settling `failed`/`tool-failure{invalid-shrink}`. The only meter-limit test (`text_domain_work_limits_settle_a_record_before_any_case`) runs the value-clause path, and the cancellation test runs through the plan. A regression that dropped either conversion would leave every listed test green. | fsl-eval/src/simulation/model/model_based_test.rs:865-905 |
| FND-015 | low | The plan step record now comes from `result_json`. That record has no `result` member, although FR-753's dispatch row says the record's `result` and `cause` carry the step's settlement. It writes `domain` as the checked package name: `result_json(record, package)` in the plan path and `package_name(source identity)` in the direct route, not the clause's domain (e.g. `Config::ConfigVersion`). It also omits the generator profile that FSpec FR-190 "Result" lists. | fsl-command/src/command/model_based_test.rs:241-330, fsl-command/src/command/plan/outcome.rs:288, spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md:129 |

## Dispositions

### Round 1

Checked at head a868ede0ed22a5896e79d0feaa70785330f49674 against fix commits c4719fa3dc (round 1; merged with main as 0aaa3bfecc), de5c91363c and 4a120c4559 (round 2). By reading only. The coder's logs show the focused tests passing: qsl-2237-fixround-0aaa3bfecc.log 38/38 and 36/36, qsl-2237-round2.log, and qsl-2237-round2b.log 1/1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | accepted-no-change | The three ignored AC-4 stubs were deleted, not wired up. Arch accepted this because live tests cover AC-4: passing_run_reports_native_operation_coverage, bounded_run_reports_unreachable_within_bound and failing_run_retains_replayable_counterexample_without_losing_coverage in model_based_run.rs, plus model_operation_coverage.rs. I confirmed those binders are live (not ignored). |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |
| FND-011 | fixed | - |
| FND-012 | fixed | - |
| FND-013 | fixed | - |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      /// A state clause false at every state is refuted by a 1-minimal
      /// counterexample of zero invocations, replayed natively.
      #[test]
      fn a_failing_clause_retains_a_one_minimal_replayed_counterexample() {
      (FSpec-FR-190-AC-6 removed; FR-752-AC-3 removed from a_passing_run_records_tested_and_never_proved; FSpec-FR-190-AC-8 removed from model_based_test_value_clause_runs_and_shrinks_natively)
  - fnd: FND-002
    outcome: accepted-no-change
    reason: "Stubs deleted (fsl-eval/tests/it/model_based_tests.rs now holds only the AC-3 plan-form test); AC-4 coverage is live in model_based_run.rs and model_operation_coverage.rs."
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      fn refuse_binding(member: &'static str) -> ModelBasedTestRefusal {
          ModelBasedTestRefusal { member, code: "invalid_model_binding", cause: "malformed-declaration" }
      }
      ...
      fn model_test_error(error: ModelBasedTestRefusal) -> RunCause {
          RunCause::ModelBasedTestRequest {
              code: Code::from_code(error.code).unwrap_or(Code::RuntimeInvariant),
  - fnd: FND-004
    outcome: fixed
    after_excerpt: |-
      fn refuse(member: &str, cause: &'static str) -> RunCause {
          RunCause::ModelBasedTestRequest { code: Code::InvalidRuntimeInput, path: format!("/model_based_test/{member}"), cause }
      }
      ("max_value_bits", json!(256), "invalid_runtime_input", "invalid-value"),
      ("max_trace_length", json!(0), "invalid_runtime_input", "invalid-value"),
  - fnd: FND-005
    outcome: fixed
    after_excerpt: |-
      universe_subject
          .check_universes(&snapshots)
          .map_err(RunCause::AnalyzeUniverse)?;
      (test model_based_test_refuses_an_incomplete_initial_population asserts incomplete_population, exit 22)
  - fnd: FND-006
    outcome: fixed
    after_excerpt: |-
      let failure = |failure| match fsl_replay::spine::refusal_or_fault(failure) {
          Ok(refusal) => super::spine_run_failure(&source, libraries.iter().map(|(_, source)| source), *refusal),
      ... parse_selecting_extensions(.., limits.source, ..) ... select(&parsed, &packages, limits.model, ..)
  - fnd: FND-007
    outcome: fixed
    after_excerpt: |-
      refusal_names(&refusal, "max_value_bits");
      assert_eq!(first["model_based_test"]["status"], "tested", "{first}");
      assert_eq!(first["model_based_test"]["cases"]["generated"], 20, "{first}");
      ... .is_some_and(|value| (-255..=255).contains(&value)) ... !...ends_with("-refusal")
  - fnd: FND-008
    outcome: fixed
    after_excerpt: |-
      fn findings_at_stop(&self, state: &Self::State) -> Vec<Self::Finding> {
          self.state_findings(state)
              .map(|evaluated| evaluated.unwrap_or_else(|stop| self.stopped_findings(state, stop)))
              .unwrap_or_default()
      }
  - fnd: FND-009
    outcome: fixed
    after_excerpt: |-
      /// A state clause false at every state is refuted by a 1-minimal
      /// counterexample of zero invocations, replayed natively.
      /// A false initial state remains a replayable 1-minimal counterexample for a
      /// fixed set of seeds.
  - fnd: FND-010
    outcome: fixed
    after_excerpt: |-
      let with_length = fsl_eval::simulation::model::run_model_based_test(&value_world.package, &value_request, &NativeAdmission, &cancel)
          .expect("a value clause accepts max_trace_length and does not use it");
  - fnd: FND-011
    outcome: fixed
    after_excerpt: |-
      let mut settled = not_run(refusal.code, refusal.cause, refusal.member);
      settled.status = if refusal.code == "cancelled" { PlanStatus::Incomplete } else { PlanStatus::Declined };
  - fnd: FND-012
    outcome: fixed
    after_excerpt: |-
      if let Err(ModelBasedTestError::Expansion(ExpansionStop { limit: Some(ExpansionLimit::EvaluationMeter(kind)), .. })) = &outcome {
          if let Some(name) = clause_name(system, identity) {
              return Ok(text_limit_record(..., *kind, scalar_limit(&system.meter_limits(), *kind)));
  - fnd: FND-013
    outcome: fixed
    after_excerpt: |-
      PlanResult::ModelBasedTest(record) => super::super::model_based_test::result_json(record, package),
```
