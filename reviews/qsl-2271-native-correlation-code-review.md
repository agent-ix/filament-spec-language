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
