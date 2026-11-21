---
id: SR-197644590097040779572473300621804150376
title: QSL-2271 native correlation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; frozen PR3407 changed files; bounded native
  SC Send/Receive
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-731
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
---

# QSL-2271 native correlation gap-analysis

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3407. Frozen head: e88d82fdf90dc0a4712139658ef9c155ecce7b2f; base: be3ad51571bb5e7b9c0e86588cfa32611053ed3d. Exactly one independent reviewer; read-only branch review. This is the approved bounded native SC Send/Receive slice, not full FR-731, weak correlation, POR or Effect/Event qualification.

## Verdict

CONDITIONAL — two medium findings require same-PR fixes. The code-review artifact records the actual seven failing fixtures as high FND-001; no merge until all findings are resolved and actual required runtime proof passes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Distinct canonical record selection and later authored failures lack an oracle: the only multi-clause group repeats one canonical_index, so selecting members[0] for every row or inspecting only the first premise would survive these tests. Add a genuine group with differing canonical indexes, an earlier true premise and a later false premise; assert disabled exploration and exact later alias in both replay routes, retaining repeated-row coverage. | fsl-analyze/tests/it/native_correlation.rs:598-651 | correct-requirement-no-evidence |
| FND-002 | medium | The correlation-only binder test adds the unmatched trace id FR-216, which names a requirement rather than a criterion. quire coverage lists this exact test/tag in untracked_symbols. Move whole-requirement context outside the trace marker; retain the existing valid criterion tags and add an FR-216 criterion only if the test actually asserts its stated acceptance scenario. | fsl-analyze/tests/it/native_correlation.rs:340 | correct-requirement-no-evidence |

## Evidence

Exact-head compile evidence supplied by the lead: cargo check --locked -p fsl-package -p fsl-eval -p fsl-analyze --tests -j4, wrapper-owned warm target, DEV/TEST DEBUG=0 and INCREMENTAL=0; exit 0, log /home/peter/.cache/handoff/protocol-batch-2-native-correlation-check-exact.log (Finished dev profile in 0.27s; existing sample_fingerprint dead_code warning). Reviewer ran no Cargo, tests, services or gates. Actual initial runtime receipt supplied by the root: simple3407-e88d-c1, cargo test --locked -j4 -p fsl-analyze --test it native_correlation:: -- --nocapture, exit 101; 0 passed, 7 failed, 0 ignored, 2791 filtered out. Log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-initial-runtime.log. All seven fail in fixture construction or parsing; production correlation runtime is UNQUALIFIED, not PASS. git diff --check: exit 0.

## Coverage

Plan completion: not assessed

Whole-repository computed static census: quoin matrix --repo /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-native-correlation --json exited 1 with full stderr `Error: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. Documented fallback quire matrix --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-native-correlation --format json exited 0; matrix-static.json has 6263 criteria: tagged 4940, untagged 1137, tagged-by-ignored-test 65, method-without-symbol 106, retired 15. These inherited repository-wide counts are not a PASS or adjudicated PR defects. quire coverage --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-native-correlation --json exited 0; Rust census candidates 13996, tagged 13420, bound 12880; symbol-collision and trace diagnostics reported. Native new tags bind FR-731-AC-1/3 and FR-940-AC-6; existing FR-731-AC-2 is tagged by other tests, not a new native mixed-premise oracle. New FR-216 is unmatched as recorded in FND-002. Three FSpec-FR-831 context tags are external-repository references and do not establish local AC coverage.

Reverse/stub inspection over this authorized frozen diff: 8 substantive behavior surfaces (premise construction, semantic identity, native values, binder inventory, replay diagnostics, weak profile refusal, unported-control refusal, item unavailable mapping), 0 unowned surfaces, 0 source stubs, 0 test stubs; FR-731/FR-940 own them, with admitted FSpec-FR-831 provenance context. Tests use real Source emission, strict Bare admission and actual native systems. Optional whole-repository semantic review skipped; targeted code/test oracle inspection is part of the explicitly requested bounded PR review. Full-repository reverse inventory and inherited matrix defect remediation are outside this dispatch; no repository-wide gap PASS is asserted.

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
