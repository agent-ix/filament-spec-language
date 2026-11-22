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

## Dispositions

### Pass 1

Reviewed fixed head 8fed2271bed34117e1158e26468f745d112a4fb9 against e88d82fdf90dc0a4712139658ef9c155ecce7b2f. Original finding rows above are unchanged. Production code is unchanged; this pass covers fixture repairs and test oracles. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Inspected differing canonical-index and repeated-row assertions, earlier true owns/later false Other fixture, disabled full-graph Source/Bare parity, exact later Other refusal through BOTH replay routes, swapped-clause disabled graph, and retained original repeat-only case. This repairs the static oracle gap; runtime execution is separately pending. |
| FND-002 | fixed | Removed the unmatched whole-requirement FR-216 trace; retained FR-731-AC-1 and FR-940-AC-6. New mixed-premise test additionally carries FR-731-AC-2. |

Fixed-head compile evidence supplied by the lead: cargo check --locked -p fsl-package -p fsl-eval -p fsl-analyze --tests -j4, same wrapper-owned target and DEBUG=0/INCREMENTAL=0 environment, exit 0 in 23.50s; log /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix1-check.log. Reviewer ran no Cargo or runtime. git diff --check exit 0. Fixed-head runtime: PENDING/UNASSESSED. The root reports a relay Redis os11 request failure; it is not a queued test run or product failure. Initial e88 runtime remains historical evidence only (exit 101, 0 passed/7 failed); no new runtime PASS inferred.

Bounded verdict: Both original gap findings fixed by static inspection; execution of the new assertions is pending. NOT MERGEABLE while code-review FND-001 and required runtime qualification remain open.

After excerpt for FND-001:

```rust
assert_ne!(clauses[0].canonical_index, clauses[1].canonical_index);
assert_eq!(clauses[0].canonical_index, clauses[2].canonical_index);
```

After excerpt for FND-002:

```rust
#[trace("FR-731-AC-1", "FR-940-AC-6")]
```

### Pass 2

Actual FIX1 receipt simple3407-8fed-c1 for head 8fed2271bed34117e1158e26468f745d112a4fb9: exit 101, 4 passed / 3 failed / 0 ignored, 2791 filtered out. Raw log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix1-runtime.log. This supersedes Pass 1 pending status; the initial e88 receipt remains historical (0 passed / 7 failed).

Both gap findings retain their Pass 1 fixed outcomes. FND-001 now additionally has actual passing runtime evidence: replay_reports_first_authored_alias_including_repeated_canonical_premises passes the differing canonical-index, earlier true/later false exact Other replay, repeated-row and swapped-order assertions. FND-002 remains fixed by the unchanged valid criterion tags and removal of whole-requirement FR-216 from the marker. No finding was open in this artifact before Pass 2, so no duplicate disposition row is appended. No new findings.

Actual passing tests: sc_send_receive_uses_real_extents_and_preserves_queue_and_binder_values; missing_authored_carrier_is_typed_unavailable_but_empty_legacy_group_executes; unported_attempt_and_recommend_correlations_remain_named_unsupported; replay_reports_first_authored_alias_including_repeated_canonical_premises. The last test executes the new differing canonical-index, earlier-true/later-false, exact Other replay, swapped-order and original repeated-alias assertions. The failing false_send_and_receive_premises_disable_only_their_candidate test executes its genuine Owns/Other negative graph, waiting queue and disabled Receive assertions before failing in the separate undefined capture constructor.

Bounded verdict: gap findings fixed; NOT MERGEABLE while code-review FND-001 remains open and the exact required filter reports three fixture failures. No whole-program, weak correlation, Effect/Event or full FR-731 qualification is asserted.

### Pass 3

Actual FIX2 receipt simple3407-99eb-c1 at 99eb9f0da39ac1f5bd13ec8cf83ae780f4fe104d: exit 101, 1 passed / 2 failed / 0 ignored, 2795 filtered out; the request selected the three previously failing tests. Raw log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix2-runtime.log.

Both gap findings retain their Pass 1 fixed outcomes; the Pass 2 actual passing mixed-premise replay evidence remains historical proof for that unchanged test. No prior-open gap findings require new rows. No new findings.

weak_package_profile_refuses_correlation_outside_parallel_and_sc_override_executes passed at 99eb: whole-protocol Tso/Ra correlation refusal and the SC override assertions executed. Combined historical receipts now cover five unique passing test functions: this weak-profile test plus the four retained tests passing at 8fed (positive SC graph/queues/binders; missing-carrier/empty-legacy; Attempt/Recommend guards; mixed-canonical/repeated-alias/replay). This historical union is not a seven-test exact-head PASS. The two remaining test functions are unqualified. Production code is unchanged.

Original finding rows and Pass 1/2 history are preserved. Reviewer ran no Cargo, tests, services, relay or tracker writes, and made no branch changes.

Bounded verdict: NOT MERGEABLE — Code FND-001 remains open for the two fixture preconditions and complete required runtime qualification is red. No full FR-731, weak correlation, Effect/Event or POR claim is made.

### Pass 4

Actual FIX3 authorized Terra-local receipt at unchanged frozen head 6fc61f3830c0e1a404e2ac43312e60a71a6f8679: exit 0, 2 passed / 0 failed / 0 ignored, 2796 filtered out, test runtime 0.28s; normal cold code generation finished in 8m 20s. Log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix3-local-runtime.log. Both previously outstanding tests passed: correlation_only_capture_and_prior_binder_reads_remain_live_after_an_earlier_step and false_send_and_receive_premises_disable_only_their_candidate. The reviewer read the actual result and frozen head; the reviewer ran no Cargo or runtime.

Both original gap findings remain fixed from Pass 1. The mixed-canonical/repeated-alias/replay runtime evidence passed in FIX1 and its test body remains unchanged. The invalid FR-216 trace stays removed and valid criterion tags remain. No prior-open gap finding requires another disposition row; no new findings.

| Receipt / head | Selected result | Qualification established |
| --- | --- | --- |
| initial e88d82f | 0 passed / 7 failed, exit 101 | Initial fixture failures only; retained history. |
| FIX1 8fed227 | 4 passed / 3 failed, exit 101 | Positive SC graph/queue/binder parity; missing-carrier and empty-legacy behavior; Attempt/Recommend guards; mixed-canonical/repeated-alias/replay test. |
| FIX2 99eb9f0 | 1 passed / 2 failed, exit 101 | Whole-protocol Tso/Ra correlation refusal and SC override. |
| FIX3 6fc61f3 | 2 passed / 0 failed, exit 0 | Prior-binder/capture liveness and footprint; genuine false-premise/queued-message cases and undefined-end retained findings. |

These receipts cover all seven unique bounded SC/correlation test functions across successive heads, not one seven-test suite on the final head. The five previously passing test bodies, model and production implementation are retained; FIX3 only changes the two outstanding test bodies plus fixture-helper plumbing. The helper retains exactly the Linked default, original declarations/input/linking path, identity and bytes construction, check limits and S6aRewardEvaluator. Empty prelude is the default. Only the undefined subcase selects the existing Kernel remainder fixture pattern. The targeted static review found no new defect.

Fixed-head focused compile supplied by the lead: exit 0 in 25.67s; /home/peter/.cache/handoff/protocol-batch-2-native-correlation-fix3-check.log, same wrapper and DEBUG=0/INCREMENTAL=0 environment. git diff --check exit 0 during targeted static review. Production code and admitted protocol profile are unchanged in this fix round. Branch remains read-only to this reviewer.

#### Additional examined scope in Pass 4

```yaml
scope:
- id: fsl-analyze/tests/it/fill.rs::package_over_with_mode
  path: fsl-analyze/tests/it/fill.rs
  role: examined
  excerpt: package_over_with_mode(document, package_name, profiles, body, CheckMode::Linked)
- id: fsl-analyze/tests/it/native_correlation.rs::with_prelude
  path: fsl-analyze/tests/it/native_correlation.rs
  role: examined
  excerpt: 'function q using v(x: Integer): Integer pure { 6 mod x }'
```

Current bounded verdict: PASS — all three original findings now have fixed latest outcomes. Bounded review is mergeable subject to the lead committing these exact final SRs and completing its mandatory current-main exact-head final compile/gate before the matched-head merge. This review does not assert a single full-suite final-head run, repository-wide assurance, general weak correlation, relationship-aware POR, Effect/Event support, full FR-731 support, or QSL-2271 parent completion.

Original findings and all previous dispositions, receipts and diagnosis clarifications remain unchanged.

### Pass 5

Targeted current-main integration review at frozen head a32ce9b7a11f39e71e9f9fdb7509d9264d2503c7 over current base ee381b0e6e23e4410bff577b337b04c6e85e1dfd. Scope is the mechanical replay-field integration, import context and preservation of the previously reviewed behavior; no full review, broad matrix or runtime rerun. Original findings and Pass 1–4 history are unchanged.

The first final current-main four-crate compile actually FAILED, exit 101: E0063 at fsl-analyze/tests/it/native_correlation.rs:581, missing stutter_closure in ModelTraceSubject. Log: /home/peter/.cache/handoff/protocol-batch-2-native-correlation-final-current-main-check.log. The candidate adds the sole missing initializer stutter_closure: false. This is a reviewed source fix, not a successful compile receipt. A fresh mandatory exact-head/current-main final compile is PENDING and must run after the final D5 SR commit. No earlier failed check is represented as green.

The one-line fixture adjustment preserves the prior no-closure protocol replay semantics. At 6fc the ModelTraceSubject had no stutter_closure member. Current ModelTraceSubject documentation at fsl-replay/src/model_check/replay.rs:109–114 says the admitted model-liveness item alone can select closure, while every other subject leaves false. Native correlation supplies an untimed protocol deadlock/replay subject and its genuine traces have no closure-stutter steps. Both Source and Package protocol replay still settle through ProtocolReplay::settle, which calls refuse_closure_alphabet(payload); this protocol path does not enable the new model-liveness closure route. stutter_closure: false also matches incoming related_by.rs and analyze_package_protocol.rs fixtures. Thus the field is an explicit declaration of the existing semantics, not a change to correlation evaluation or a new runtime profile.

Compared with current ee381, fsl-package/src/lib.rs retains incoming SelectedModelUnionPayload and SelectedModelUnionRow while adding only this PR's CheckedNodeOwner, ProtocolCorrelationReadError, ProtocolCorrelationRow and UnavailableCorrelationProvenance exports. Incoming ReferenceCatalog exports remain present. These are import/re-export context unions, with no catalog duplication or removed incoming API. Direct comparison against 6fc shows no change in the correlation production files or fill helper and no Cargo.toml/Cargo.lock change. The only native_correlation.rs content change is stutter_closure: false. The prior five retained passing test bodies and two final local passing tests therefore retain their reviewed bounded evidence; no additional runtime broadening is needed for this semantic-equivalent initializer adjustment.

All original findings were fixed before this pass; no duplicate disposition rows are appended. The already repaired compile integration is recorded above without inventing an outstanding correlation product defect. Reviewer ran no Cargo, runtime, service, relay, tracker or branch mutation. git diff --check exit 0; worktree clean at reviewed head.

#### Additional examined scope in Pass 5

```yaml
scope:
- id: fsl-analyze/tests/it/native_correlation.rs::replay_both
  path: fsl-analyze/tests/it/native_correlation.rs
  role: examined
  excerpt: 'stutter_closure: false,'
- id: fsl-package/src/lib.rs::filament_contract_model
  path: fsl-package/src/lib.rs
  role: examined
  excerpt: SelectedModelUnionPayload, SelectedModelUnionRow, TemporalBinding,
- id: fsl-replay/src/model_check/replay.rs::ModelTraceSubject
  path: fsl-replay/src/model_check/replay.rs
  role: context_only
  excerpt: 'pub stutter_closure: bool,'
- id: fsl-replay/src/model_check/replay.rs::ProtocolReplay::settle
  path: fsl-replay/src/model_check/replay.rs
  role: examined
  excerpt: refuse_closure_alphabet(payload)?;
```

Current bounded verdict: PASS for this targeted integration review. All original code/gap findings retain fixed latest outcomes; no new or unresolved finding. Bounded review is mergeable CONDITIONED ON the lead committing the exact final D5 SRs and obtaining the mandatory fresh CURRENT-main, exact-head four-crate compile/gate before a matched-head merge. The final compile is PENDING. All seven unique bounded tests passed across successive pre-integration heads (initial 0/7 → FIX1 4/3 → FIX2 1/2 → FIX3 2/0); this is not a single final-head seven-test suite. No full weak correlation, POR, Effect/Event, full FR-731 or QSL-2271 parent-completion claim.
