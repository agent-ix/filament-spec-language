---
id: SR-17526088197672665835421904791776023501
title: QSL-2271 native correlation code-review
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; frozen PR3407 changed files; bounded native
  SC Send/Receive
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-731
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
---

# QSL-2271 native correlation code-review

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3407. Frozen head: e88d82fdf90dc0a4712139658ef9c155ecce7b2f; base: be3ad51571bb5e7b9c0e86588cfa32611053ed3d. Exactly one independent reviewer; read-only branch review. This is the approved bounded native SC Send/Receive slice, not full FR-731, weak correlation, POR or Effect/Event qualification.

## Verdict

FAIL — all seven required tests fail in fixture setup before runtime. No production defect established by static inspection; fix FND-001 and the separate gap-analysis findings in this PR. Not mergeable.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | All seven native-correlation tests fail before reaching correlation runtime: four abort in PackageModelInput::of at line 163 because the reused fill model includes unsupported variant type test/fill/Mode; the weak-profile fixture writes parallel memory sc instead of the required using syntax; the binder fixture puts a channel before the required role section; and Attempt uses the reserved binder name result. Repair the genuine fixtures within the bounded slice and rerun the exact seven-test filter; the current suite provides no runtime qualification. | fsl-analyze/tests/it/native_correlation.rs:43-48,159-163,343-347,422-423,436 | correct-requirement-no-evidence |

## Evidence

Exact-head compile evidence supplied by the lead: cargo check --locked -p fsl-package -p fsl-eval -p fsl-analyze --tests -j4, wrapper-owned warm target, DEV/TEST DEBUG=0 and INCREMENTAL=0; exit 0, log /home/peter/.cache/handoff/protocol-batch-2-native-correlation-check-exact.log (Finished dev profile in 0.27s; existing sample_fingerprint dead_code warning). Reviewer ran no Cargo, tests, services or gates. Actual initial runtime receipt supplied by the root: simple3407-e88d-c1, cargo test --locked -j4 -p fsl-analyze --test it native_correlation:: -- --nocapture, exit 101; 0 passed, 7 failed, 0 ignored, 2791 filtered out. Log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-initial-runtime.log. All seven fail in fixture construction or parsing; production correlation runtime is UNQUALIFIED, not PASS. git diff --check: exit 0.

## Coverage

Plan completion: not assessed

Methods: code-review with Rust-review lane and Rust-style API, implementation, safety, boundary and testing references. All ten changed files examined; no spec/plan edits, so spec-review and its sub-analyses do not apply. No applicable AssuranceProfile found under spec/. No dependency, gate, workflow, manifest or lockfile edits. Checked borrowed admitted rows, canonical-index-selected record and end slots, Model owner DeclarationKey, same-state value outcomes, declared-binder supply, BOTH end readers before normalization, authored alias replay and false-constraint/control-flow precedence, whole-protocol weak refusal, Attempt/Recommend guards, and additive item-level unavailable provenance mapping. Source weak behavior and unsupported Effect/Event paths remain unchanged. Existing FIFO outcome conversion now reuses value_outcome; no duplicated evaluator or wire catalog introduced. Strong state/transition, queue, binder liveness and footprint assertions inspected; distinct-premise oracle and trace issue belong to the separate gap artifact. No broad program assurance claim.

## Examined scope

```yaml
scope:
- id: fsl-analyze/src/analyze/clause_item.rs
  path: fsl-analyze/src/analyze/clause_item.rs
  role: examined
  excerpt: CorrelationProvenance(cause)
- id: fsl-analyze/tests/it/main.rs
  path: fsl-analyze/tests/it/main.rs
  role: examined
  excerpt: mod native_correlation;
- id: fsl-analyze/tests/it/native_correlation.rs
  path: fsl-analyze/tests/it/native_correlation.rs
  role: examined
  excerpt: fn replay_reports_first_authored_alias_including_repeated_canonical_premises()
    {
- id: fsl-analyze/tests/it/protocol_roles.rs
  path: fsl-analyze/tests/it/protocol_roles.rs
  role: examined
  excerpt: '| Unsupported::PackageCorrelationProvenance(_)'
- id: fsl-eval/src/simulation/protocol.rs
  path: fsl-eval/src/simulation/protocol.rs
  role: examined
  excerpt: 'fn value_outcome(outcome: filament_eval::FamilyOutcome<Value>) -> Result<Value,
    ClauseOutcome> {'
- id: fsl-eval/src/simulation/protocol/channel.rs
  path: fsl-eval/src/simulation/protocol/channel.rs
  role: examined
  excerpt: match super::value_outcome(outcome) {
- id: fsl-eval/src/simulation/protocol/package_program.rs
  path: fsl-eval/src/simulation/protocol/package_program.rs
  role: examined
  excerpt: pub(super) struct PackageRelated<'a> {
- id: fsl-eval/src/simulation/protocol/program.rs
  path: fsl-eval/src/simulation/protocol/program.rs
  role: examined
  excerpt: pub(super) enum RelatedEnds<'a> {
- id: fsl-eval/src/simulation/protocol/steps.rs
  path: fsl-eval/src/simulation/protocol/steps.rs
  role: examined
  excerpt: pub fn related_by_failure(
- id: fsl-package/src/lib.rs
  path: fsl-package/src/lib.rs
  role: examined
  excerpt: ProtocolCorrelationReadError, ProtocolCorrelationRow, SelectedExactIdentities,
    TemporalBinding,
- id: FR-731-AC-1
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: examined
  excerpt: 'Over a model with orders `o1`, `o2` and payments `p1` settling `o1` and
    `p2` settling `o2`, a protocol over `o1` whose `event Paid by R as (paid: M::Payment)
    related by settles (paid, input) { true }` explores `Paid` steps binding `p1`
    only; the same node with no `related by` clause explores steps binding both.'
- id: FR-731-AC-2
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: examined
  excerpt: The AC-1 node with a second clause that no payment satisfies has no step,
    and its thread waits; swapping the order of the two clauses gives the same states.
- id: FR-731-AC-3
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: examined
  excerpt: Replaying a trace whose `Paid` step binds `p2` refuses `invalid_runtime_input`/`invalid-value`
    at that step, naming `Paid` and the clause `settles`; the trace binding `p1` replays.
- id: FR-940-AC-6
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: For each item family whose evaluator FR-531's inventory lists as taking
    a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant
    `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`),
    a `program` request and a `package` request over the handoff directory of the
    same compiled unit give equal item records, counterexamples and replays, and no
    package-route record is `unsupported`; a handoff directory whose `checked-package.json`
    the strict reader refuses, or that holds no `identities.json`, settles no item
    and compiles no source (FR-850-AC-
- id: FR-216
  path: spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md
  role: context_only
  excerpt: Give every protocol step a static footprint, enabling footprint and visibility
```

## Tool provenance

/home/peter/.npm-global/bin/quire: 0.52.0 (engine 0.62.2); /home/peter/.npm-global/bin/quoin: 0.36.8; spec-artifacts-process@v0.36.0. `quire schema --module /home/peter/.ix/filament/modules/spec-artifacts-process SpecReview` exit 0; stderr UnknownEdgeType ADR.realizes_requirement, TestMatrix.covers, TestMatrixIndex.covers and UnknownRole ADR.FR/NFR when loaded alone. Actual repository-scoped scratch validation is recorded separately in validation logs.

## Dispositions

### Pass 1

Reviewed fixed head 8fed2271bed34117e1158e26468f745d112a4fb9 against e88d82fdf90dc0a4712139658ef9c155ecce7b2f. Original finding rows above are unchanged. Production code is unchanged; this pass covers fixture repairs and test oracles. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Fixture repairs are present at 8fed2271bed34117e1158e26468f745d112a4fb9: minimal Level/Cell/Idle model and touch operation, valid declaration order, checks OK/Between/Stop now use using v, and Attempt binder is worked. The initial runtime remains 0 passed/7 failed; no fixed-head seven-test receipt is available. Qualification stays open until those genuine tests execute successfully. Clarification of the original finding: the missing using was on nested check OK (and checks Between/Stop), not on parallel Background memory sc; that parallel syntax was not defective. |

Fixed-head compile evidence supplied by the lead: cargo check --locked -p fsl-package -p fsl-eval -p fsl-analyze --tests -j4, same wrapper-owned target and DEBUG=0/INCREMENTAL=0 environment, exit 0 in 23.50s; log /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix1-check.log. Reviewer ran no Cargo or runtime. git diff --check exit 0. Fixed-head runtime: PENDING/UNASSESSED. The root reports a relay Redis os11 request failure; it is not a queued test run or product failure. Initial e88 runtime remains historical evidence only (exit 101, 0 passed/7 failed); no new runtime PASS inferred.

Bounded verdict: NOT MERGEABLE — code FND-001 remains open pending actual fixed-head seven-test qualification.

Historical diagnosis clarification: the initial panic said expected using at check OK within parallel Background. The fix correctly adds using v to check OK, Between and Stop. The original finding row is retained for audit history, and its suggestion that parallel memory sc itself lacked using is corrected here.

### Pass 2

Actual FIX1 receipt simple3407-8fed-c1 for head 8fed2271bed34117e1158e26468f745d112a4fb9: exit 101, 4 passed / 3 failed / 0 ignored, 2791 filtered out. Raw log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix1-runtime.log. This supersedes Pass 1 pending status; the initial e88 receipt remains historical (0 passed / 7 failed).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | FIX1 now passes four required tests, but three fixture preconditions still prevent complete qualification: binder-liveness setup at native_correlation.rs:371 uses reserved word first; weak-profile setup at :463 declares only one branch and parser refuses expected branch; negative-premise test reaches its native undefined subcase at :338-350 then constructor refuses Option<Reference<Cell>> capture as PackageParameterType / MissingTypeRepresentation. Keep the existing high fixture finding open; use valid admitted fixture forms within the approved slice and obtain the exact seven-test passing receipt. No distinct correlation production defect is established. |

Actual passing tests: sc_send_receive_uses_real_extents_and_preserves_queue_and_binder_values; missing_authored_carrier_is_typed_unavailable_but_empty_legacy_group_executes; unported_attempt_and_recommend_correlations_remain_named_unsupported; replay_reports_first_authored_alias_including_repeated_canonical_premises. The last test executes the new differing canonical-index, earlier-true/later-false, exact Other replay, swapped-order and original repeated-alias assertions. The failing false_send_and_receive_premises_disable_only_their_candidate test executes its genuine Owns/Other negative graph, waiting queue and disabled Receive assertions before failing in the separate undefined capture constructor.

The third failure is a typed parameter-representation precondition at native ProtocolSystem construction, before related-value evaluation. It does not establish a correlation semantic defect, and this review does not authorize or require a general native type port. Production code is unchanged. No new finding is allocated: these are remaining defects of the same genuine qualification fixtures. Reviewer ran no Cargo, tests, services, relay or tracker writes.

Bounded verdict: NOT MERGEABLE. Code FND-001 remains open; the full seven-test filter is red. Existing gap findings remain fixed. Initial findings, Pass 1 rows and diagnosis clarification remain unchanged.

### Pass 3

Actual FIX2 receipt simple3407-99eb-c1 at 99eb9f0da39ac1f5bd13ec8cf83ae780f4fe104d: exit 101, 1 passed / 2 failed / 0 ignored, 2795 filtered out; the request selected the three previously failing tests. Raw log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix2-runtime.log.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Two genuine qualification fixtures still fail at Source checking before their target correlation runtime: the binder-liveness fixture declares spare carrying Reference<M::Cell>, refused ProtocolAnchor::ChannelCarriesObject at native_correlation.rs:377; the undefined-end subcase uses IntegerMod in a RelatedEnd slot, refused SlotClause::ProfileProhibitedOperator(IntegerMod) at :338-345. These are invalid channel/slot-profile preconditions, not observed correlation production defects. Keep Code FND-001 still-open until admitted fixture forms execute the intended binder/undefined oracles and pass. Do not widen channel reference support, the operator profile, or general native types to repair these fixtures. |

weak_package_profile_refuses_correlation_outside_parallel_and_sc_override_executes passed at 99eb: whole-protocol Tso/Ra correlation refusal and the SC override assertions executed. Combined historical receipts now cover five unique passing test functions: this weak-profile test plus the four retained tests passing at 8fed (positive SC graph/queues/binders; missing-carrier/empty-legacy; Attempt/Recommend guards; mixed-canonical/repeated-alias/replay). This historical union is not a seven-test exact-head PASS. The two remaining test functions are unqualified. Production code is unchanged.

No new finding is allocated: the actual failures remain within the existing high fixture-qualification finding. Original finding rows and Pass 1/2 history are preserved. Reviewer ran no Cargo, tests, services, relay or tracker writes, and made no branch changes.

Bounded verdict: NOT MERGEABLE — Code FND-001 remains open for the two fixture preconditions and complete required runtime qualification is red. No full FR-731, weak correlation, Effect/Event or POR claim is made.
