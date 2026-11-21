---
id: SR-1795263280874614189
title: gap-analysis of QSL-2271 PR3300 package untimed Await
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2271; PR3300; exact 949206cfecc781c9fd264d19cc087cc2f3895260..5e846bf37ee03312ca82edb0967d6afa5dbfea11;
  fsl-analyze/tests/it/analyze_package_protocol.rs; fsl-eval/src/simulation/protocol/package_program.rs;
  fsl-eval/src/simulation/protocol/program.rs; runtime update at frozen head 5e846;
  Source admission failures only
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-206
  type: references
---

## Summary

QSL-2271 PR3300: final scoped code/Rust and bounded gap review of package Await causal execution and exact configured clock transport. Frozen code90baba8619 is accepted with all eleven focused tests green and no open findings. Historical findings/failures stay unchanged below; final dispositions supersede earlier outcomes.

## Verdict

PASS for bounded PR scope at 90baba8619: all twelve historical findings fixed; all eleven scoped tests GREEN (6 library +4 integration +1 actual source-replay authority). No full repository matrix/gap, parent completion or actual Parts smoke claim. QSL-2271 remains In Progress/open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Await fixture names line-clock but supplies no clocks configuration, so all three matched-Receive tests stop at Source S4 invalid-request/missing-clock-selection before package execution. Configure the authored clock through the existing request clocks member, then rerun the focused cases without weakening their expected results. | fsl-analyze/tests/it/analyze_package_protocol.rs:78-90 |
| FND-002 | high | The Send/Attempt match tests and correlated matched Attempt test require source programs that FR-218-AC-35 and both S3 and the pinned native-IR law explicitly refuse. They stop at Source S3 unsupported_construct/declaration-form before package execution; revise this slice to the legal matched-kind scope under a corrected architecture ruling, preserving the checker and IR laws. | fsl-analyze/tests/it/analyze_package_protocol.rs:222-303 |

## Evidence

Draco log `/tmp/protocol-3300-5e84-full.log`: 0 passed, 5 failed, 0 ignored, 2529 filtered out. The three Receive tests reach S4 and refuse missing-clock-selection for line-clock under event-position.false-extension/v1. The supported-match test stops on its first Send case; the separate correlated Attempt test stops at its Attempt match. Both return empty items and unsupported_construct/declaration-form at S3. None reaches package admission or stepping. Remaining loop cases are unexecuted, not observed failures.

FR-218 local Await rules lines143-147 and AC-35 require Receive/Effect/Event matches and refusal of Send/Attempt. `fsl-semantics/src/check/protocol_clause/awaits.rs:68-81` implements that rule, with existing graph tests covering both refusals. Exact locked native IR revision 20a4dfbf29e979da4326ac81f85e8266e464519f, `crates/filament-contract-model/src/checked_package/v2/protocol_body.rs:2860-2918`, separately admits send/receive/attempt/effect/event/commit as AFTER anchors but restricts the matched event to receive/effect/event with AwaitedEvent refusal. No additional package consumer rejection for Send/Attempt is needed: strict package admission already rejects those forms. The new matched Attempt correlation guard is unreachable under this law. Parser collection through event_node_anchors is not S3 or IR admission. Do not weaken source checking or native IR to make the tests pass.

FR-640 owns explicit request clocks configuration; existing `finite_hyper_request.rs:24-26` uses the request clocks member with kind event_position and a sequence_authority. Configure the Await fixture with its exact authored line-clock name using the established request API. Runtime remains untimed once admitted; clock configuration does not add interval/timing execution semantics.

## Coverage

All three changed Rust files were inspected. Shared runtime causal edges, child ordering, and existing timeout/event-step reuse have no separately demonstrated defect. Existing fixture helpers are reused; no copied model/snapshot/Owns setup was added. Effect/Event remain typed Unsupported and broader correlation remains parent work. No source/target edits or Cargo/runtime/matrix/gates were performed by this reviewer. Reported scoped cargo check remains compile-only evidence and did not establish source fixture admission. Parent FIFO acceptance is independent and cannot establish Await fixture correctness.

Bounded reverse-gap inventory covers three new obligations (causal continuation recovery, runtime mapping/children, related matched Attempt guard); the last has no admitted path under current law. Five tagged tests exist but supply no executed package-runtime evidence at this head. Zero new source stubs or mocked engine paths were found. Direct tag inspection is provisional only; full repository assurance and optional full semantic review were not performed. Plan completion: not assessed.

No spec/plan/public enum/dependency/CI/wire/transport changes occur. No applicable AssuranceProfile exists in spec/. Tooling warnings remain disclosed in tooling-note.txt.

## Examined scope

Verbatim fragments capped at 600 characters; examined is scoped inspection, not whole-criterion proof.

```yaml
scope:
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
- id: FR-940-AC-22
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: AC-4's Counter request over source with the items `Bounded` and `AtMostTwo`,
    run with `replay.input_bytes` set below the length of the package's emission (the
    up-front read-back is bounded by `i2.input_bytes` and admits the package), settles
    `AtMostTwo` `inconclusive` with cause `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded`
    naming `replay.input_bytes`, never `refuted`, never `failed` with `analyze`/`package-emission`,
    and settles `Bounded` `proved` as without the setting; the document's category
    and exit follow its items (category inconclusive, never `internal-failure
- id: FR-206-AC-5
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: An `await` with a timeout control gives `timeout(a)` at every state where
    its thread waits, and the matching event node's step settles the owner into `then`.
- id: FR-383-AC-7
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: 'In **W** and its variants, `await`, `check`, `send`, `receive`, `attempt`,
    `effect`, `event`, `commit` and `fence` emit their layouts: an `await` anchored
    at an event node and at a compensation template with its clock text, closed interval,
    awaited event, `then` and `timeout`; a `send` and a `receive` with the channel,
    the payload binder, the received `send` and the `related` records; an `attempt`
    with its role, operation anchor, each memory ordering or none, `contracts` and
    binder; an `effect` of its `attempt`; an `event` with and without a compensation;
    a `commit`; a `fence` with each orderi'
- id: FR-731-AC-1
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: context_only
  excerpt: 'Over a model with orders `o1`, `o2` and payments `p1` settling `o1` and
    `p2` settling `o2`, a protocol over `o1` whose `event Paid by R as (paid: M::Payment)
    related by settles (paid, input) { true }` explores `Paid` steps binding `p1`
    only; the same node with no `related by` clause explores steps binding both.'
- id: FR-211-AC-5
  path: spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md
  role: examined
  excerpt: 'Every channel below has `delivery exactly-once-premise` unless stated,
    so neither `lose` nor `duplicate` is enabled, and each deadlock is reached with
    the channel in its qualified name in the cause. Capacity 1, `block`: `sequence
    { send P via ch; send Q via ch }` deadlocks with the sender at `Q` and cause `FullQueue`;
    with `send Q via ch as (q) { false }` the sender at `Q` has cause `Constraint`,
    although the queue is full. Capacity 2, `fifo`: `sequence { send P via ch; send
    Q via ch; receive RQ via ch of Q }` deadlocks with the queue `[P, Q]` and cause
    `HeadBlocked` at `RQ`. The same with `re'
- id: ADR-027/QS-2
  path: spec/decisions/ADR-027-protocol-transition-system-for-parallel.md
  role: context_only
  excerpt: The step kinds ST-1 to ST-15, their enabledness and effects, folding of
    structural moves (FO-1, FO-2), atomicity (AT-1 to AT-3), and the untimed reading
    of `await`
- id: package_program/Await
  path: fsl-eval/src/simulation/protocol/package_program.rs
  role: examined
  excerpt: Await continuations derived from the admitted causal relation.
- id: program/new_package
  path: fsl-eval/src/simulation/protocol/program.rs
  role: examined
  excerpt: Build the sequence/parallel/attempt/send/receive/await program from the
    admitted
- id: analyze_package_protocol/Await
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  role: examined
  excerpt: A supported event kind keeps its own step when it is an await's match.
- id: FR-218-AC-35
  path: spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md
  role: examined
  excerpt: The `await` matching a `send` `Out` refuses `unsupported_construct`/`declaration-form`
    at `Wait`, naming `Wait`, `Out`, form `send` and the selected protocol profile;
    matching an `attempt` `Try`, the same refusal naming `Try`, form `attempt` and
    the profile.
```

## Fix round 1 evidence

Disposition head: `626ac6ec4dce0eb0eec6a81934bf46cd323c1bd9`, rebased on actual merged FIFO `f1c43209be`; original stacked parent `949206` is dropped. This section supersedes initial runtime status above without rewriting the original findings. Both defects are corrected in the inspected source. Final runtime acceptance is CONDITIONAL: four package_await functions are queued through Draco, with no result assumed. Reported locked-wrapper debug=0/incremental=0 cargo check of the one fsl-analyze integration target passed in 1m02s; this reviewer ran no Cargo/tests/gates/matrix/target operations.

FND-001: Await request helper supplies line-clock as event_position with sequence_authority protocol-events. Shared `fsl-analyze/tests/it/analyze_cli_support.rs::package_selection` conditionally forwards request clocks into native compile, then removes clocks from the selected package request. Existing library forwarding and removal are retained. Requests with no clocks retain their former behavior. The three valid matched-Receive bodies, expected labels, full item/exit/replay equality, transition/state counts and refusal code/cause assertions are unchanged.

FND-002: both invalid positive Send/Attempt functions and the unreachable matched Attempt correlation helper/guard are removed. The existing Send/Receive correlation refusal remains unchanged. Replacement `package_await_ineligible_matches_refuse_in_both_routes` loops over send/attempt. It starts with valid clocked Receive source plus an ordinary Attempt template in then, requires source proved and baseline package/full-item/exit equality, then mutates only raw untrusted emitted wire. The Send edit removes the Receive-only of member and dependency. The Attempt edit reuses the emitted template body while retaining the original event binder, related group and constraint, replacing corresponding direct dependencies. Dependencies sort through the existing WireGraph key_of_value API; no IDs or sidecars are fabricated and no graph is asserted admitted. Strict checked_handoff::verify must produce ReaderRefused with exactly UnsupportedConstruct/DeclarationForm and protocol detail AwaitedEvent for the intended kind. Consequently a generic malformed identity/hash/dependency refusal cannot satisfy this test. Package analyze must then report source's exact unsupported_construct/declaration-form and empty items after source removal. The explicit strict-reader detail is stronger than code/cause alone. Source checker, native IR law, production clock semantics and peer transport are unchanged.

Additional examined unit: `fsl-analyze/tests/it/analyze_cli_support.rs:75-100`, package_selection, role examined; exact excerpt: `if let Some(clocks) = request.get("clocks") {`. New raw-wire negative test inspected through all body/dependency updates and the direct strict-reader detail assertion. No new findings were identified. Spec methods remain inapplicable.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed statically at 626ac6ec: Await fixtures supply line-clock; shared package_selection forwards clocks to compile and removes clocks from the package request. Existing library forwarding/removal survives the rebase. Runtime acceptance remains pending. |
| FND-002 | fixed | Fixed statically at 626ac6ec: invalid positive fixtures and unreachable matched Attempt guard are removed; the new negative test checks Source and strict native admission refuse Send/Attempt. Runtime acceptance remains pending. |
| FND-003 | still-open | Observed at 626ac6ec; fix round 2 and focused runtime acceptance remain pending. |
| FND-004 | still-open | Observed at 626ac6ec; fix round 2 and focused runtime acceptance remain pending. |
| FND-005 | still-open | Observed at 626ac6ec; fix round 2 and focused runtime acceptance remain pending. |
| FND-003 | deferred | Deferred by draco:QSL:arch on open QSL-2271: configured Await handoff row transport moves to a separate PR, with FR-517 contract clarification and one spec review, peer agreement, and code only after peer PR3302 merges. Current CLI configured-clock producer remains blocked before the native guard; no row/count check is dropped. |
| FND-004 | deferred | Deferred by draco:QSL:arch on open QSL-2271 with the separate configured Await clock-transport slice after the FR-517 contract/spec review and peer PR3302 gate. The observed configured-clock source failure remains real and its deep cause was not independently established. Current ordinary core fixtures explicitly select legal InfiniteTrace and retain all source verdict/witness/count/replay assertions. |
| FND-005 | fixed | Fixed by inspection at 0c6581fa: the ordinary touch template and invalid matched Attempt use true rather than reading a void result. Strict-reader code/cause/AwaitedEvent detail checks remain unchanged; runtime pending. |
| FND-006 | still-open | Four source fixtures select an explicitly forbidden profile; architecture scope correction required. |
| FND-003 | fixed | Exact checked Await rows are derived/reconstructed/compared, including global count, at a4e57d8a; runtime pending. |
| FND-004 | still-open | The new code retains acquisition selections in SourceProvision and passes them to source replay emission/recompilation, addressing the missing-selection route. The previous source S6c cause was erased, so disappearance of that actual failure remains unverified until the queued exact-head timeout test passes. |
| FND-006 | fixed | Fixture selects legal EventPosition and carries caller-events, replacing forbidden InfiniteTrace; runtime pending. |
| FND-007 | still-open | Add a real source replay emission oracle for retained caller-events authority and the FR-517-AC-33 trace tag; outcome parity alone cannot detect consistent authority substitution. |
| FND-003 | fixed | Relevant retained exact-head runtime now passes. |
| FND-004 | fixed | Relevant retained exact-head runtime now passes. |
| FND-006 | fixed | Relevant retained exact-head runtime now passes. |
| FND-007 | fixed | Relevant retained exact-head runtime now passes. |
| FND-008 | fixed | Fixture corrected by source inspection in d080640e/7187ffe4; targeted runtime pending. |
| FND-009 | fixed | Fixture corrected by source inspection in d080640e/7187ffe4; targeted runtime pending. |
| FND-010 | fixed | Fixture corrected by source inspection in d080640e/7187ffe4; targeted runtime pending. |
| FND-011 | still-open | Test-only fixture/oracle correction pending. |
| FND-012 | still-open | Test-only fixture/oracle correction pending. |
| FND-001 | fixed | Configured caller clock supplied and preserved; core tests pass. |
| FND-002 | fixed | Illegal positive matches removed; exact Source/native and distinct public artifact refusals pass. |
| FND-003 | fixed | Exact Await rows/counts/identity/configuration/reconstruction pass six library and core handoff tests. |
| FND-004 | fixed | Actual timeout refuted witness, replay and byte-refusal parity pass. |
| FND-005 | fixed | Void touch uses true; valid baseline and negative checks execute. |
| FND-006 | fixed | Legal EventPosition admission and core behavior pass. |
| FND-007 | fixed | Actual replay-wire emitted caller-events/node/clock oracle and paired timeout replay pass; AC33 tagged. |
| FND-008 | fixed | Legal Due model-steps/global-three/source-order/roundtrip pass. |
| FND-009 | fixed | Distinct unread right binder preserves WaitZ ambiguity; AmbiguousClause test passes. |
| FND-010 | fixed | Official operand conversion/exact roundtrip reaches native AwaitedEvent for both forms. |
| FND-011 | fixed | Read-valid wrong-node fixture still requires exact ClauseNode; final library filter passes. |
| FND-012 | fixed | Public invalid_package/native cause/pointer/exit20/no item or engine passes; Source/native semantic checks retained. |

## New findings (disposition pass 2)

Observed head 626ac6ec: `/tmp/protocol-3300-626-full.log`, 0 passed, 4 failed, 0 ignored, 2583 filtered out. FND-001 clock configuration remains statically fixed. FND-002 illegal positive matched forms remain removed; the replacement negative baseline has a distinct newly observed type error.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | Valid clocked Await source cannot compile its handoff: Selection counts temporal clauses, compensation templates and hyper traces but omits Await bindings that the emitter writes, so global binding count comparison refuses clause.binding (clause None). Account for Await rows using checked-source/request facts and preserve the integrity comparison. | fsl-command/src/checked_handoff/selection.rs:704-719 |
| FND-004 | high | The valid matched-Receive timeout-deadlock fixture fails on the SOURCE route at S6c with internal-failure/result failed after 5 states and 4 transitions, instead of the required refuted timeout witness. Its public cause is null; diagnose and repair the actual underlying source exploration/witness failure without weakening the expected verdict. No package-kernel fault is established. | fsl-analyze/tests/it/analyze_package_protocol.rs:133-199,793-802 |
| FND-005 | high | The new negative-test baseline reads the Boolean attempt binder attempted from touch, whose operation has no result. Source S3 refuses ill_typed/type-mismatch before the valid baseline or intended AwaitedEvent refusal checks run. Use a legal predicate such as true for the void touch template and invalid Attempt source fixture, preserving strict-reader refusal detail assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:235-253 |

Binding evidence: Selection fields/derive contain only clauses, compensations and hypers; check_binding per-node failures carry Some(name), while final global count failure at719 carries None, matching the observed message exactly. Emitter emit.rs:9314-9380 explicitly writes a binding at each Await declaration locus. Preserve the compare/count and admitted provenance; do not suppress Await emission or ignore excess rows. This is a latent handoff omission exposed by the now-valid Await fixture.

Timeout evidence: channel_send_parity analyzes source and asserts expected label before package_request. Its assertion at801 sees failed/refuted, so package compile/admission/execution were not reached. Neither the erased public cause nor static inspection identifies a unique underlying invariant. No causal claim about the package stepping map is justified. A scoped deeper diagnostic is required before selecting that fix.

Negative evidence: fill::attempt always emits a Boolean binder; fill::document defines touch as no-result. The new Template uses attempted as its constraint, so the source error is specifically a read of a void result, not the intended forbidden Await match. Both source baseline and new strict-reader mutation checks remain runtime-unproven.

No Cargo, runtime rerun, source edits, matrix or broad gates were performed by this reviewer. New findings were gathered in one bounded pass before fix round 2. Runtime acceptance remains deferred, not a conditional PASS.

Additional examined scope:

```yaml
scope:
- id: FR-517-AC-2
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: The package read from `checked-package.json` with `read_checked_package`
    and evidence assembled from the written files is admitted, and `Selection::check`
    passes; the sidecar's clause ids, names, requirement entries, activation forms,
    profile selections, definition authority and `TemporalBinding` rows, those of
    clauses and those of compensation templates, equal the values read from the checked
    unit held in memory by the writer.
```

## Fix round 2 evidence

Frozen head `0c6581fa086f5f280ffd6263652ea4c6069fa0ca`, base main FIFO `f1c43209`. Architecture explicitly split delivery: this PR enables ordinary untimed matched Receive under InfiniteTrace; configured clock handoff/replay transport and the actual timestamped parts-saga example remain incomplete on open QSL-2271. They require a separate PR with FR-517 clarification, one spec review and peer agreement; implementation follows peer PR3302 merge. This is an explicit scope deferral, not a claim that the configured example works.

Ordinary tests now explicitly select filament.temporal.infinite-trace/v1. Existing emitter code emit.rs:9322-9340 emits configured Await rows only for EventPositionFalseExtension, FixedSampleFalseExtension or TimestampedEventFiniteWindow; InfiniteTrace legitimately has no configured clock row. No binding is removed or fabricated. The semantic control body and all expected labels, states/transitions, full item/exit equality, timeout counterexample and replay/refusal assertions of the three valid tests are unchanged (the 131-line region of the three tests is byte-equal across the fix). Negative Send/Attempt source/strict-wire admission tests use the same legal InfiniteTrace baseline and true predicates for void touch. No checker or IR rule is widened.

New configured-clock guard test uses the shared model_check_input handoff fixture, minimally extended with actual TemporalClockSelection inputs. It emits a real EventPosition protocol-events selection joined by the checked Await span, strictly admits the exact emission bytes, obtains identity rows and equality witness from that same emission/admitted package, and constructs BarePackage/PackageModelInput through the ordinary supply API. It asserts exactly one retained temporal binding, clock_name line-clock, full ClockConfiguration equality, and typed AwaitClockSelection containing that binding's declaration and authored clock. This is a legitimate native-admitted consumer boundary; it is not a CLI handoff test and bypasses no strict native admission. The existing CLI Selection count omission still precedes this guard on the configured CLI path.

The production guard scans actual admitted Await bindings before channel/control normalization and returns AwaitClockSelection without changing the binding. Analyzer PackageProtocolWork maps it through existing UnsupportedWithDetail catalog unsupported_construct/not-yet-implemented, detail fact await-clock-selection-transport plus node and clock. ProtocolSystem maps it to Unsupported::PackageAwaitClockSelection; Unsupported::name and the closed cfg-test consumer in protocol_roles are updated. The generic Send/Receive correlation guard is untouched. No command/replay/FR-517 row transport implementation, AST reconstruction, wire/pin change, source alias fabrication or source oracle weakening occurs. The configured test establishes only native refusal coverage, not analyzer wire-detail execution; analyzer mapping was inspected statically.

Additional reviewed paths for this fix: fsl-analyze/src/analyze/clause_item.rs:1128, fsl-eval/src/simulation/protocol.rs (public refusal family/name/mapping), fsl-analyze/tests/it/model_check_input.rs:29-65 (shared emission/admission/supply fixture), and fsl-analyze/tests/it/protocol_roles.rs:2505 (closed consumer). No fixture copies or new findings were identified.

All five fresh package_await functions are queued through Draco; no current runtime pass is claimed. Reported required locked-wrapper debug=0/incremental=0 check of the one integration target passed in 1m04s, log /tmp/protocol-batch-2-await-fix2-check.log. Reviewer ran no Cargo/runtime/matrix/aggregate gate/target operations. The pending runtime must validate the legitimate core/negative/native-guard boundaries before acceptance.

## New findings (disposition pass 4)

This is an evidence update within the existing reviewer invocation, not authorization for another implementation round. Actual log `/tmp/protocol-3300-0c65-full.log`: 1 passed, 4 failed, 0 ignored. The configured native clock guard passes. All four core functions fail at using t0 with unknown_profile/wrong-selection-role on the Source route before package compile/admission/execution.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | Four core Await tests select InfiniteTrace, which FR-508-AC-3 and protocol coverage.rs explicitly forbid at Await. All four stop at Source S3 unknown_profile/wrong-selection-role; only the configured native guard test passes. InfiniteTrace is correctly spelled but is not a legal no-clock Await profile, so restore a contract-admitted scope or obtain architecture direction without widening the checker or inventing a profile. | fsl-analyze/tests/it/analyze_package_protocol.rs:85-93 |

Reviewer correction: the prior static characterization of InfiniteTrace as legal for Await was wrong. Its spelling is canonical and globally registered (profile.rs:78); the site-specific coverage.rs:283-285 deliberately excludes it. Existing protocol coverage tests require exactly this rejection, and FR-508 table49/prose60-66/AC3 state the restriction explicitly. Each legal Await profile has Some clock kind: EventPosition, FixedSample, Timestamped or Timed. Only InfiniteTrace has None, and it is forbidden here. This rules out a legal profile with NoClockSelection under current contracts. Timed has Some(timed); any special model-derived clock path is separate behavior and has not been proven to provide an ordinary fixture. Do not infer a new zero-clock route or weaken profile checking. Architecture needs these facts before another scope choice.

The lower configured guard PASS proves a legitimate admitted-native refusal with actual retained binding facts. It does not deliver configured CLI transport or ordinary package core behavior. No compiler/IR/profile change, rerun, Cargo, matrix or broad audit was performed. Earlier configured-clock failures remain deferred and are not treated as fixed by this inadmissible profile substitution.

Additional scope:

```yaml
scope:
- id: FR-508-AC-3
  path: spec/functional/FR-508-admit-a-profile-only-at-the-use-sites-of-its-kind.md
  role: examined
  excerpt: An `await` and a `compensate` `using` an alias that selects each of the
    four admitted temporal identities admit; `using pp` (protocol profile), `using
    v` (value layer) and `using i` (`filament.temporal.infinite-trace/v1`) each refuse
    `unknown_profile`/`wrong-selection-role` at the alias.
```

## Transport review and new findings (disposition pass 5)

Frozen a4e57d8a0febf5d7dd6bbd77d8bd25692c56f0d9 on peer3302 base 7d9dd0ee52e92712f9e74cc4922fcd73d4eb0d9f. Accepted contract commit882fd2eb80 and independent base review20feae5169 (SR-1795267647659877508) supply the contract/spec gate; this pass does not repeat that spec review or a full audit. The illegal InfiniteTrace scope and temporary configured-clock guard/enum are withdrawn. All four core functions now select legal EventPosition and explicitly retain caller-events; void touch predicates remain true. No source admission law is widened.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | FR-517-AC-33 has no test trace tag or oracle inspecting the source replay emission's actual supplied clock authority. Existing parity/replay assertions could survive consistent substitution of protocol-events for caller-events because stepping is untimed; library roundtrip tests cover handoff, not this source replay boundary. Add a narrow real-source replay-wire emission assertion for retained node/clock/caller-events plus replay outcome, and tag that test with FR-517-AC-33. | fsl-analyze/tests/it/analyze_package_protocol.rs:114-237; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:605 |

Exact AwaitRow uses required node/name/occurrence and required-but-nullable binding, closed serde shape; empty awaits omit bytes. Selection::read rejects duplicate node/occurrence pairs. Derivation joins checked protocol node keys and one real Declaration ordinal, copies actual named caller selections, and refuses ambiguous selectors without reading expected values from wire. Selector joins reject unknown/duplicate/ambiguous keys; command capture retains the exact acquisition selections. check_await checks declared Await kind/name/node, real Declaration occurrence, exactly one matching native row, matching occurrence, root-unit/nontrace binding and existing clock axes. Global count adds awaits without filtering actual package rows. Inputs and recompile reconstruct names/configs; SourceProvision carries selections into replay emission and existing selected replay entry points. Model-derived selections only fill absent same-source/span/trace keys; supplied values retain precedence. Peer3302 actual emission/identity/equality/dependency/budget paths are preserved, with no re-emission in selection derivation. Public SourceProvision methods/error variant and affected command/analyze/monitor/replay callers were inspected.

Six cfg library fixtures were inspected before runtime queueing: legal Event/Boolean/acme-orders inputs, two Awaits plus Due with global3 binding count and source order, old no-Await encoding, sidecar axes including wrong real second Await node with equal clocks, required nullable/closed members and duplicate occurrences, strictly admitted native mutations with Declaration/source-map consistency before exact axes, and unknown/duplicate/missing/ambiguous selectors using nested after Main::Started. Four integration tests retain strong semantic labels, exact states/transitions, complete item/exit/replay equality and actual native AwaitedEvent refusal before package mapping. Runtime must still validate fixture legality and mutation admission; no result is assumed.

No other new static finding was identified. FND-004 remains open solely because its prior source failure had a null erased cause; passing the exact timeout/replay test is required before reporting it fixed. Reported local integration check exit0 in27s and command production-only lib check exit0 in1m46s are compile-only; six cfg tests have not been locally compiled or executed. No Cargo, tests, matrix, full gate, source edit or target operation was performed by reviewer. Direct trace scan, not a computed matrix, establishes FND-007. Actual timestamped example remains a later owner smoke; matched Effect/Event remain typed Unsupported parent scope.

Additional examined criteria:

```yaml
scope:
- id: FR-517-AC-29
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: A checked unit containing an Await `Wait` in protocol `Flow`, with an event-position
    clock selection whose sequence authority is `caller-events`, writes an `awaits`
    row naming `await Flow::Wait`, its own native node, its actual Declaration occurrence
    and the exact authored clock name and supplied configuration. The strict reader
    admits the package, `Selection::check` passes, and `compile` reconstructs the
    same selection and package bytes. The authority is never replaced with `protocol-events`.
- id: FR-517-AC-30
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: A unit with two Await declarations writes their rows in source order and
    counts both configured bindings alongside temporal-clause, compensation and hyper-trace
    bindings. Equal writes give byte-identical sidecars. A unit with no Await omits
    `awaits` and retains its prior sidecar bytes.
- id: FR-517-AC-31
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: For an admitted package with an Await binding, changing its clock name,
    kind or a configuration value to another reader-admitted value makes `Selection::check`
    refuse on the corresponding existing `clock.*` axis with `clause = Some` of the
    Await selector. A missing binding, a binding at another reader-admitted declaration
    occurrence, or an extra binding is refused on `clause.binding`; a missing or non-Await
    node is refused on `clause.node`. No mutation is accepted by matching another
    occurrence's row.
- id: FR-517-AC-32
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '`Selection::read` refuses an Await row with a missing, repeated or unknown
    member, a malformed occurrence, or a repeated `(node, occurrence)` pair. A supplied
    Await selection key that matches two declarations is refused as ambiguous and
    writes no directory; an unknown key and a repeated supplied key retain `UnknownClause`
    and `DuplicateClause`, respectively. A missing clock configuration required by
    the Await profile retains its existing refusal and writes no directory.'
- id: FR-517-AC-33
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: An analyze request whose checked acquisition receives a caller-supplied
    Await clock selection preserves that exact selection in its source replay and
    handoff emission. Source recompilation and the package path use the same clock
    configuration and produce the same replay outcome; neither path silently substitutes
    empty selections or model-derived clocks for the supplied Await selection.
```

Source replay evidence limit: timeout integration observes outcomes/replay reproduction, not the replay-wire package binding. Consistently fabricating protocol-events in replay emission and recompilation can preserve those untimed outcomes. Existing handoff roundtrip assertions inspect caller-events but cannot establish this separate source replay boundary. The private replay_wire function is available to its existing cfg tests and returns ReplayRequestWire.package for an actual emitted-binding assertion; a copied SourceProvision input assertion alone would be tautological. FND-007 therefore needs a bounded authority-preservation oracle as well as its trace tag.

## Runtime receipt and new fixture findings (disposition pass 6)

Root supplied actual a4 result receipt: library6 has 2 passed/4 failed, 0 ignored, exit101; integration4 has 3 passed/1 failed, 0 ignored, exit101. Healthy matched Receive, false match and timeout/refusal core tests pass. Only the negative matched-kind wire setup fails, before its strict refusal assertions. Retain those passes. Exact c846 new authority unit passes 1/0/0, exit0, 184 filtered; remote raw log /private/tmp/qsl-3300-clock-authority.log is named by the runner. Combined actual authority artifact inspection and retained timeout behavioral replay close FND-007. Prior source timeout failure FND-004 no longer reproduces in its actual core test. These do not establish all six library tests or negative strict mutation are green.

Full a4 raw stderr was requested by root but is not yet present locally; precise source stage of the reported library missing-clock refusals is not inferred from check_selections. The following invalid fixture facts are independently established by inspected source/owner rules regardless of their runtime diagnostic stage.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | The two-Await library fixture declares temporal Due over a model with free clock line-clock, which the model-subject clause_clock rule rejects. Use the existing model-steps role while retaining both Await caller-events bindings, Due binding, global-three count and exact roundtrip assertions. | fsl-command/src/checked_handoff/selection/await_tests.rs:31-33 |
| FND-009 | high | The ambiguous-selector fixture duplicates the matched event binder seenWaitZ across both nested controls, which is a protocol-wide duplicate binder before selector ambiguity can be checked. Give only the right unread event binder a distinct name while preserving both WaitZ names and the intended AmbiguousClause assertion. | fsl-command/src/checked_handoff/selection/await_tests.rs:425-431 |
| FND-010 | high | The negative Attempt wire fixture copies holder-relative operand slots from the Template node into the Receive node and resolves dependencies without the original holder row. Setup fails on a missing operand slot before the intended AwaitedEvent refusal. Use the existing resolve_operand_slots/write_operand_rows conversion, prove exact unedited roundtrip and preserve the strict native refusal checks. | fsl-analyze/tests/it/analyze_package_protocol.rs:279-360 |

Static correction batch: d080640e changes only Due clock role to model-steps and resolves/re-encodes the raw-wire fixture through existing owner APIs, asserting exact untouched wire roundtrip first. 7187ffe4 renames only the right unread event binder to seenOther; both WaitZ declaration selectors and after Main::Started remain. No production, source-oracle, target, budget or strict-refusal expectation changes occur. Library names/config/count/roundtrip and global-three assertions remain intact. Negative fixture continues demanding strict UnsupportedConstruct/DeclarationForm/AwaitedEvent for each kind; malformed-slot failure cannot satisfy it. No custom ID resolver or new framework is introduced. Root authorized only the four failed library plus one failed integration reruns, retaining the 2+3+1 passes. Reviewer did no Cargo/runtime/matrix/source edit.

## Native and public admission evidence (disposition pass 7)

Actual 7187 failed-filter rerun: 3 passed, 2 failed. Retain prior passes: total nine of eleven expected tests now green; only one library wrong-node case and one negative integration public-code assertion remain. Raw logs are local under raw-a4/ and raw-7187/. The prior three library acquisition failures are confirmed Source Check Temporal(ModelClockName line-clock), not an inferred omission in pre-S3 check_selections. The corrected model-steps fixtures now reach the intended later checks. Negative wire setup now reaches strict native UnsupportedConstruct/DeclarationForm/AwaitedEvent, proving FND-010 correction; the next public assertion fails invalid_package versus unsupported_construct.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | The wrong-node sidecar mutation copies the second real Await node into row zero but retains row one with the same node/occurrence. Selection::read correctly refuses a duplicate pair before ClauseNode comparison. Remove only the other expected row in this one mutation case, retaining the wrong real node, WaitZ name, equal clock/configuration and exact ClauseNode oracle. | fsl-command/src/checked_handoff/selection/await_tests.rs:204-222 |
| FND-012 | high | The negative public analyze assertion requires a reader-refused package artifact to have Source S3 unsupported_construct code. FR-850 Admission instead requires invalid_package with the native cause/pointer, exit20 before item/engine. Keep exact Source and native UnsupportedConstruct/DeclarationForm/AwaitedEvent assertions, then assert public invalid_package/declaration-form/native pointer and no item/engine; do not change production mapping. | fsl-analyze/tests/it/analyze_package_protocol.rs:410-419 |

FR-850 Admission311-317 explicitly mandates invalid_package for bytes the reader refuses, with typed native cause and JSON pointer, exit20 before item/engine. fsl-package/src/package_admission.rs:206-239 preserves cause and locus while mapping this artifact rejection to InvalidPackage; command/handoff.rs:152-167 forwards that projection. Source S3 and raw native verifier are distinct earlier boundaries and correctly keep unsupported_construct. The public observed cause declaration-form and pointer /semantic_graph/nodes/32 are retained, not erased. This is a test overassertion, not permission to broaden or normalize production errors. No architecture ruling is needed to apply the existing contract; changing the contract would be separate authority.

The library failure is SelectionReadError::Malformed duplicate Await declaration occurrence. Both node and ordinal match the unmodified second row, so parser refusal is correct under FR-517-AC-32. The single wrong-node case must remain parse-valid before testing ClauseNode, and wrong-node identity should be compared before the global count. All other mutation checks remain unchanged. Reviewer performed no Cargo/runtime/source edits/matrix/new whole review.

Additional examined obligation:

```yaml
scope:
- id: FR-850-AC-3
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: AC-1's handoff directory whose `checked-package.json` holds `{}`, one holding
    a `filament.checked-package/v1` document, one holding AC-1's bytes with one source-map
    entry removed, and one that is not JSON each refuse the whole request at the request
    stage with code `invalid_package`, the reader's cause and pointer, exit 20, `items`
    empty, and no engine runs.
```

## Final scoped acceptance

Reviewed code head 90baba8619f8806d8bb2dbef0f56d5473be34bd2. Final two runner receipts: command library binding-identity test 1 passed/0 failed/0 ignored/105 filtered, exit0; analyze integration ineligible test 1 passed/0 failed/0 ignored/2586 filtered, exit0. Raw logs /private/tmp/qsl-3300-90baba8-binding-identity.log and /private/tmp/qsl-3300-90baba8-package-ineligible.log. Joined with nine retained relevant passes, all eleven scoped tests are GREEN: six command library, four Await integration functions and one actual source-replay authority unit. Earlier production bodies/passing tests were retained; final changes are test-only oracle corrections.

All FND-001 through FND-012 have final outcome fixed. Historical failed runs, temporary architecture deferrals, withdrawn InfiniteTrace interpretation and earlier inaccurate assumptions remain unchanged below; actual configured transport and runtime receipts supersede them. No finding remains open, deferred or accepted without change. Strong authority/node/occurrence/count/roundtrip, full semantic source/package/replay, native AwaitedEvent and distinct public invalid_package assertions survive.

Acceptance is bounded to this PR delta and these eleven FSL tests. QSL-2271 remains In Progress/open; matched Effect/Event and broader protocol/correlation families remain parent scope. Architecture smoke ruling comment6b4902a6 assigns actual Parts external-driver smoke to the next consuming pin owned by codex-codex, outside this merge gate; no timestamped Parts smoke is claimed. Independently accepted base contract review SR-1795267647659877508 supplies the spec gate; no extra review method/artifact. A subsequent pure rebase retains receipts only when code-equivalent; actual API adapters require bounded check/review.

Worktree was concurrently rebasing while final artifacts were authored, so exact after-excerpts were read from frozen git object 90baba8619 rather than transient files. Reviewer did no Cargo/runtime/matrix/full gate/source edit/target operation. Plan completion: not assessed. Computed matrix/full repository assurance: not assessed by explicit scope.
