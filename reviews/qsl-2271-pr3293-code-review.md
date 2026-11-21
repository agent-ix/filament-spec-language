---
id: SR-1795258741788456652
title: PR 3293 unordered channel Send code-review
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; QSL-2271; fsl-eval/src/simulation/protocol/package_program.rs;
  fsl-eval/src/simulation/protocol/program.rs; fsl-eval/src/simulation/protocol/steps.rs;
  fsl-analyze/tests/it/analyze_package_protocol.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-206
  type: references
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3293. Frozen head: b160c7c8fb923d6892b4994fd15a23f01f83e20d; reviewed base: b47224c03fdabf7c9e7eb985b989bf9f6ec85560. Only the four-file PR slice was inspected. No source files, branch, target or Cargo process were changed. QSL-2271 remains In Progress for the remaining families. No applicable AssuranceProfile was found; no spec or plan changed, so spec-review methods are inapplicable. Exact model identity is not exposed; markers record model=unknown.

## Verdict

PASS — all seven findings are fixed, all seven family tests plus the focused library unit are green, and no new findings remain. The bounded slice is mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The invalid-capacity test is tagged FR-850-AC-11, whose SB litmus, memory selection and racing-counterexample behavior it never exercises. Remove that unrelated binding; retain the existing FR-940-AC-6 parity binding. | fsl-analyze/tests/it/analyze_package_protocol.rs:294 |
| FND-002 | medium | SC settlement reads channel presence through the source-only evaluator.protocol method, so package channel records omit the max_messages memory_bound that the source record states. Read admitted Source/Package protocol facts; preserve the complete-record parity assertion. | fsl-replay/src/model_check/settle.rs:347-354 |
| FND-003 | medium | Three new parity tests use the reserved binder name first, so source parsing fails before queue, constraint or replay behavior executes. Rename the binder and its reads without changing the tested outcomes or assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:183-185,243-244,265-266 |
| FND-004 | medium | A refused capacity binding is mapped to Refusal::Failed, producing an internal-failure for unknown capacity input. FR-1033 requires declined invalid-input with the typed cause and named member. Use the existing declined-input mapping on both routes; preserve the invalid-capacity refusal oracle. | fsl-analyze/src/analyze/items.rs:1161-1163 |
| FND-005 | high | New package Send execution admits nonempty related-by clauses but never populates Program.related, so related_admits returns true and sends payloads the source correlation excludes. Implement admitted correlation evaluation or explicitly refuse correlated Send with a typed correlation-naming reason and a regression; preserve the named remaining full port on open QSL-2271. | fsl-eval/src/simulation/protocol/program.rs:1267 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The exhaustive cfg(test) diagnostic helper omits the new Refusal::UnsupportedWithDetail variant, so fsl-analyze library tests cannot compile (E0004). Add its diagnostic-extraction arm; the integration-target check does not compile this helper. | fsl-analyze/src/analyze/items.rs:1261-1275 |
| FND-007 | high | The new correlation regression duplicates the existing Cell-to-Cell Owns fixture definition in related_by.rs:40-60. Reuse/extract that test-only relationship setup instead of maintaining two definitions; preserve the finite closure and relation-present/absent cases. | fsl-analyze/tests/it/analyze_package_protocol.rs:411-428 |

## Coverage

Native channel/queue facts are read through the admitted protocol view and mapped exhaustively to existing delivery/overflow enums. Unordered head/tail queues preserve send order. Program message entries retain the admitted payload type/binder and channel identity; Send constraints use the existing package slot evaluator, including earlier binder reads. The source path continues to evaluate its CheckedBody. Capacity conversion follows the source route's existing address-space saturation policy. Existing shared channel execution, blocked-cause and replay code applies overflow, delivery, message ordinals and counterexamples; none of those semantics is copied.

The architecture-authorized global Parallel+Send guard catches Send inside or after a parallel before program execution, covering the known numeric message-node identity and weak-Send footprint gap. It uses the existing typed Unsupported(Send) variant. Tests check source support and package unsupported_construct/not-yet-implemented without exploration or counterexample for SC/Tso variants. This is a named remaining obligation on open QSL-2271, not a claim of full route parity. FIFO keys, symbolic capacities, Receive, compensation and replicated roles remain outside this slice.

Six real integration tests contain fifteen fixture cases and compare full item records and exits after deleting program.fsl. Overflow tests use distinct payloads and inspect three send records and displacement; false constraints are distinguished from full queues. Replay refuses with explicit code/cause. Fixture types and construct kinds form a matching finite closure. No production/test stubs, mocks of the implementation, copied upstream artifacts, new public boundary contracts or unsafe/concurrency changes were found. No runtime pass is inferred from static checks.

Draco runtime evidence received during the initial review: exact b160c7c8fb923d6892b4994fd15a23f01f83e20d yielded 1 passed, 5 failed, 0 ignored, 2517 filtered, exit 101. The Parallel/Send guard passed. Full raw log is retained at /tmp/protocol-3293-b160-full.log. The ordinary complete-body test reaches equal proved/exploration results but differs in max_messages bound metadata; three tests fail S1 on the reserved binder first; the unknown-capacity case returns an internal-failure instead of a typed input refusal. The negative-capacity case passed before that second case failed. No local rerun was made. These failures are not waived or normalized away.

Reconciliation: computed matrix, coverage extraction and whole-repository reverse-gap audit explicitly excluded by dispatch. Manual slice-local criteria/test/source inspection only; this is not an engine-backed repository gap PASS. Inventoried changed behaviors: 4 (channel premises, Send program mapping, package constraint admission, guarded mixed-family refusal); slice-local unowned behaviors: 0; production stubs: 0; test stubs: 0. Optional semantic-review workflow: skipped; ordinary code-test alignment was inspected in the Rust/code review.

Plan completion: not assessed

## Correlation boundary finding

Additional boundary evidence before fix round 1 is pushed: fsl-eval/tests/it/protocol_duplicate_effect.rs:390 applies `related by Owns (sent.source, view)` to Send and source execution sends only node-2. The admitted package provides Send `related.from`/`related.to` slots (fsl-package/src/protocol_selection.rs:347), but ControlBody::Send exposes only channel/binder; package program construction leaves `related` empty. channel.rs:277 invokes related_admits, whose steps.rs:897 absent-entry branch returns true. FR-731 requires every relation to hold. Architecture authorized a correlation-naming typed guard for nonempty admitted Send related groups and a dedicated test; full native correlation evaluation remains named on open QSL-2271. Existing Attempt correlation omission is preexisting and is not expanded into this fix.

## Validation limits

This reviewer ran no Cargo, runtime, computed matrix or whole-repository gate. Final Draco acceptance is green: six focused integration tests passed at 9638b9046038eabafccc207aad9e00496d1dd317, the unchanged Parallel/Send guard passed at b160c7c8fb923d6892b4994fd15a23f01f83e20d, and the existing library unit passed at dc915bdbb2c70897fae9e4ed96abf6d920dedd7b (1 passed, 0 failed, 0 ignored, 172 filtered, exit 0; /private/tmp/qsl-3293-dc91-libunit.log). Thus all seven family tests plus the one library unit are green. The final test-only fixture extraction is equivalent; no production assertions were weakened. All seven code-review findings and all three gap findings are closed. QSL-2271 stays open for the named remaining families and full native correlation port. Full-tree criterion coverage and remaining FIFO/Receive/mixed-Parallel/correlation execution are not claimed.

## Examined scope

```yaml
clean: false
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
- id: FR-206-AC-4
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: 'A `fifo` channel of capacity 1 under `block`: a second `send` is not enabled
    while the queue holds one entry. Under `reject` the second send is enabled, leaves
    the queue unchanged and its step records a send record with outcome `rejected`.
    Under delivery `at-least-once`, `duplicate` is enabled when capacity admits it
    and `lose` never is; under `at-most-once`, `lose` is enabled and `duplicate` never
    is. An `unordered` channel holding two distinct payloads gives two `receive` steps;
    a `fifo` channel gives one, for the head.'
- id: FR-206-AC-7
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: Over a `fifo` channel `ch` of capacity 2, each `send` binds its binder
    to the payload and records a send record `{channel, payload, outcome, displaced}`
    with `channel` in the qualified name. On an empty queue the outcome is `accepted`
    and `displaced` is none. With the queue holding `P1` then `P2`, `overflow reject`
    gives `rejected` and `overflow loss new` gives `lost`, each leaving the queue
    `[P1, P2]`; `overflow loss head` gives `accepted` with `displaced` `P1` and the
    queue `[P2, new]`; `overflow loss tail` gives `accepted` with `displaced` `P2`
    and the queue `[P1, new]`.
- id: FR-206-AC-12
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: Over an `unordered` channel `ch` of capacity 2 holding `P1` sent before
    `P2`, a third `send` of `P3` under `overflow loss head` is enabled, gives outcome
    `accepted` with `displaced` `P1`, and leaves `P2` and `P3` queued; under `overflow
    loss tail` it gives `accepted` with `displaced` `P2` and leaves `P1` and `P3`
    queued. In both, the protocol is explored rather than refused as unsupported,
    and each queued entry is still deliverable to a matching `receive`. The same sends
    made in the order `P2` then `P1` displace `P2` under `loss head` and `P1` under
    `loss tail`. With capacity 3 under `loss hea
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
- id: FR-383-AC-4
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: In **W** and its variants, each channel emits `from`, `to`, `carries`,
    an `unordered` ordering with no binder or key or a `fifo` ordering with the message
    binder node and the key term, and the delivery text for each of `unknown`, `at-most-once`,
    `at-least-once` and `exactly-once-premise`; there is one `queue` for each channel,
    with a `literal` capacity value or a `symbolic` capacity name, overflow `reject`,
    `block` or `loss`, and a fourth `loss` argument exactly for `loss`, with `head`,
    `tail` and `new` giving three different queue keys; a queue has no `declaration`
    and one `generated` occurre
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
- id: FR-383-AC-11
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: Every guard, constraint, condition, key, initializer and `until` term is
    a literal or a reference to its FR-093 `expression` node, and a walk of the emitted
    JSON classifies every protocol body position as a Leaf, Group, Tuple, Member or
    Body of FSpec FR-322's stratified grammar, with no nested application.
- id: FR-850-AC-11
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'FR-1033-AC-1''s `SB` unit is compiled by a `native-compile/1` request
    carrying the `handoff` member into its handoff directory, and FR-1033-AC-2''s
    racing variant into its own. A `package` request over each directory, with `models`
    the `test/litmus` domain package document the package selects (FR-850 "Inputs")
    and the same `subject.protocol`, `memory` `[{"parallel": "SB::Both", "model":
    "tso"}]` and items as the source request, gives item records equal to the source
    route''s: `NoBothZero` `refuted` with the same counterexample (both pending stores),
    replay target and `replay.reproduced` `true`, a'
- id: FR-531
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: context_only
  excerpt: '# FR-531: Evaluate from a checked package read from its bytes alone'
- id: FR-894
  path: spec/functional/FR-894-keep-message-delivery-and-business-effect-identities-apart.md
  role: context_only
  excerpt: '# FR-894: Keep message, delivery and business-effect identities apart'
- id: FR-205
  path: spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md
  role: context_only
  excerpt: '# FR-205: Define the protocol subject and its canonical state key'
- id: FR-225-AC-6
  path: spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md
  role: examined
  excerpt: 'FR-636-AC-4''s `Burst`, a parallel join whose two branches each send on
    the `unordered` channel `Order` of capacity 1 with `overflow reject` and which
    has no RA access, checked with `max_messages` 1 leaves the second send unexpanded
    in the state where `Order` holds one entry, so the overflow is never reached and
    the item settles as FR-636-AC-4 states, its cause naming `max_messages`, its record
    stating `max_messages` 1 reached; with `max_messages` 2, and unset (4, stated
    not reached), `Order` never holds the bound, the second send is rejected and the
    item settles `refuted` with the overflow as '
- id: FR-1033
  path: spec/functional/FR-1033-select-a-protocol-subject-and-memory-models-in-an-analyze-request.md
  role: examined
  excerpt: "- A protocol binding the subject builder refuses (FR-205: a protocol the\n\
    \  package does not declare, `invalid_runtime_input`/`invalid-value`; a role\n\
    \  binding missing, extra or naming a model the role does not admit,\n  `wrong-role-mapping`;\
    \ an `over` object outside its population universe,\n  `invalid-value`; a capacity\
    \ entry naming no capacity, `unknown-member`, or\n  repeated, `duplicate-member`)\
    \ SHALL settle each item over that subject\n  `declined` `invalid-input` with\
    \ that code, naming the member, as FR-625\n  settles a refused subject admission;\
    \ the request's other items run."
- id: FR-731
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: examined
  excerpt: "- A candidate step SHALL be a step of the node only when the node's\n\
    \  `constraint` and every one of its `related by` clauses hold. Clauses are\n\
    \  conjoined; their order does not change which steps exist. A node none of\n\
    \  whose candidates satisfies its clauses has no step, and its thread waits,\n\
    \  as for an unsatisfied constraint (FR-206)."
- id: fsl-analyze/src/analyze/items.rs::tests::code
  path: fsl-analyze/src/analyze/items.rs
  role: examined
  excerpt: "fn code(refusal: &Refusal) -> Option<String> {\n        match refusal\
    \ {\n            Refusal::Declined(_, _, code)\n            | Refusal::DeclinedWithDetail(_,\
    \ _, code, _)\n            | Refusal::Unsupported(code)\n            | Refusal::FailedWithDetail(code,\
    \ _)\n            | Refusal::Failed(code) => Some(code.to_string()),"
- id: fsl-analyze/tests/it/analyze_package_protocol.rs::package_channel_send_related_premises_refuse_before_execution
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  role: examined
  excerpt: "cell[\"relationships\"] = json!([{\n            \"identity\": \"ix://test/fill/relationship/owns\"\
    , \"category\": \"structural\",\n            \"composite\": false, \"direction\"\
    : \"source-to-target\","
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-850-AC-11
  related_ids:
  - FR-940-AC-6
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  lines: '294'
  excerpt: 'FR-1033-AC-1''s `SB` unit is compiled by a `native-compile/1` request
    carrying the `handoff` member into its handoff directory, and FR-1033-AC-2''s
    racing variant into its own. A `package` request over each directory, with `models`
    the `test/litmus` domain package document the package selects (FR-850 "Inputs")
    and the same `subject.protocol`, `memory` `[{"parallel": "SB::Both", "model":
    "tso"}]` and items as the source request, gives item records equal to the source
    route''s: `NoBothZero` `refuted` with the same counterexample (both pending stores),
    replay target and `replay.reproduced` `true`, a'
  finding: The invalid-capacity test is tagged FR-850-AC-11, whose SB litmus, memory
    selection and racing-counterexample behavior it never exercises. Remove that unrelated
    binding; retain the existing FR-940-AC-6 parity binding.
  test_id: package_channel_send_invalid_capacity_records_match_source
  claimed_ac: FR-850-AC-11
  correct_ac: FR-940-AC-6
- confidence: high
  method: code-review
  check: code-bug
  fnd: FND-002
  severity: medium
  artifact_id: FR-940-AC-6
  related_ids:
  - FR-225-AC-6
  path: fsl-replay/src/model_check/settle.rs
  lines: 347-354
  excerpt: For each item family whose evaluator FR-531's inventory lists as taking
    a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant
    `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`),
    a `program` request and a `package` request over the handoff directory of the
    same compiled unit give equal item records, counterexamples and replays, and no
    package-route record is `unsupported`; a handoff directory whose `checked-package.json`
    the strict reader refuses, or that holds no `identities.json`, settles no item
    and compiles no source (FR-850-AC-
  finding: SC settlement reads channel presence through the source-only evaluator.protocol
    method, so package channel records omit the max_messages memory_bound that the
    source record states. Read admitted Source/Package protocol facts; preserve the
    complete-record parity assertion.
- confidence: high
  method: code-review
  check: code-bug
  fnd: FND-003
  severity: medium
  artifact_id: FR-940-AC-6
  related_ids:
  - FR-206-AC-7
  - FR-206-AC-12
  - FR-211-AC-5
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  lines: 183-185,243-244,265-266
  excerpt: For each item family whose evaluator FR-531's inventory lists as taking
    a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant
    `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`),
    a `program` request and a `package` request over the handoff directory of the
    same compiled unit give equal item records, counterexamples and replays, and no
    package-route record is `unsupported`; a handoff directory whose `checked-package.json`
    the strict reader refuses, or that holds no `identities.json`, settles no item
    and compiles no source (FR-850-AC-
  finding: Three new parity tests use the reserved binder name first, so source parsing
    fails before queue, constraint or replay behavior executes. Rename the binder
    and its reads without changing the tested outcomes or assertions.
- confidence: high
  method: code-review
  check: code-bug
  fnd: FND-004
  severity: medium
  artifact_id: FR-1033
  related_ids:
  - FR-205
  - FR-940-AC-6
  path: fsl-analyze/src/analyze/items.rs
  lines: 1161-1163
  excerpt: "- A protocol binding the subject builder refuses (FR-205: a protocol the\n\
    \  package does not declare, `invalid_runtime_input`/`invalid-value`; a role\n\
    \  binding missing, extra or naming a model the role does not admit,\n  `wrong-role-mapping`;\
    \ an `over` object outside its population universe,\n  `invalid-value`; a capacity\
    \ entry naming no capacity, `unknown-member`, or\n  repeated, `duplicate-member`)\
    \ SHALL settle each item over that subject\n  `declined` `invalid-input` with\
    \ that code, naming the member, as FR-625\n  settles a refused subject admission;\
    \ the request's other items run."
  finding: A refused capacity binding is mapped to Refusal::Failed, producing an internal-failure
    for unknown capacity input. FR-1033 requires declined invalid-input with the typed
    cause and named member. Use the existing declined-input mapping on both routes;
    preserve the invalid-capacity refusal oracle.
- fnd: FND-005
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-731
  related_ids:
  - FR-940-AC-6
  - FR-383-AC-7
  path: fsl-eval/src/simulation/protocol/program.rs
  lines: '1267'
  excerpt: "- A candidate step SHALL be a step of the node only when the node's\n\
    \  `constraint` and every one of its `related by` clauses hold. Clauses are\n\
    \  conjoined; their order does not change which steps exist. A node none of\n\
    \  whose candidates satisfies its clauses has no step, and its thread waits,\n\
    \  as for an unsatisfied constraint (FR-206)."
  finding: New package Send execution admits nonempty related-by clauses but never
    populates Program.related, so related_admits returns true and sends payloads the
    source correlation excludes. Implement admitted correlation evaluation or explicitly
    refuse correlated Send with a typed correlation-naming reason and a regression;
    preserve the named remaining full port on open QSL-2271.
bindings:
- test_id: package_channel_send_runs_the_complete_loss_head_body
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_channel_send_preserves_unordered_overflow_records
  ac_id: FR-206-AC-12
  trace: correct
- test_id: package_channel_send_preserves_capacity_and_constraint_causes
  ac_id: FR-211-AC-5
  trace: correct
- test_id: package_channel_send_preserves_replay_refusal_code_and_cause
  ac_id: FR-940-AC-22
  trace: correct
- test_id: package_channel_send_invalid_capacity_records_match_source
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_channel_send_invalid_capacity_records_match_source
  ac_id: FR-850-AC-11
  trace: wrong
```

## Dispositions

Round 1; inspected frozen 9638b9046038eabafccc207aad9e00496d1dd317. Static fixes do not imply runtime acceptance.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed only the unrelated FR-850-AC-11 trace binding. |
| FND-002 | fixed | Settlement now derives channel presence from the admitted Source/Package protocol definition; no record normalization. |
| FND-003 | fixed | Renamed reserved fixture binders and their reads while retaining all outcome/counterexample assertions. |
| FND-004 | fixed | All CapacityRefusal variants preserve their cause and named capacity through the existing declined-input path. |
| FND-005 | fixed | Admitted nonempty Send correlations explicitly refuse. The existing unsupported detail map names correlation and the admitted node; dedicated relation-present/absent tests preserve source expectations. Full native correlation remains named on QSL-2271. |
| FND-006 | still-open | The exhaustive cfg(test) diagnostic helper omits the new Refusal::UnsupportedWithDetail variant, so fsl-analyze library tests cannot compile (E0004). Add its diagnostic-extraction arm; the integration-target check does not compile this helper. |
| FND-007 | still-open | The new correlation regression duplicates the existing Cell-to-Cell Owns fixture definition in related_by.rs:40-60. Reuse/extract that test-only relationship setup instead of maintaining two definitions; preserve the finite closure and relation-present/absent cases. |

## Runtime evidence after round 2

Draco exact 9638b9046038eabafccc207aad9e00496d1dd317 passed all six requested integration tests (0 failed, 0 ignored, 2518 filtered; /private/tmp/qsl-3293-9638-fix1.log). The unchanged Parallel/Send guard passed at b160c7c8fb923d6892b4994fd15a23f01f83e20d, making all seven family tests green. The final three-file mechanical correction preserves production and integration fixture meaning; only the explicitly requested existing fsl-analyze library unit at dc915bdbb2c70897fae9e4ed96abf6d920dedd7b remains pending to compile the previously missed cfg(test) target.

## Round 2 scope

Inspected dc915bdbb2c70897fae9e4ed96abf6d920dedd7b against 9638b9046038eabafccc207aad9e00496d1dd317: one missing exhaustive cfg(test) arm and extraction of the exact existing Owns relationship fixture into test-only add_owns_relationship. Both original related_document and the new minimal channel fixture call it. fill::PACKAGE is test/fill, so the emitted relationship identity, origin, roles, end types and multiplicities match the former inline data. Snapshot self-edge present/absent cases and every assertion are unchanged. Production runtime code is unchanged. No Cargo, target operation or test rerun was performed by this reviewer.

## Dispositions round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The exhaustive cfg(test) diagnostic helper extracts UnsupportedWithDetail; all enum variants are now covered. Focused lib-unit execution is pending. |
| FND-007 | fixed | Both correlation fixtures use the one existing relationship definition through test-only add_owns_relationship; no relationship data or assertions changed. |

## Final acceptance

Final Draco acceptance is green: six focused integration tests passed at 9638b9046038eabafccc207aad9e00496d1dd317, the unchanged Parallel/Send guard passed at b160c7c8fb923d6892b4994fd15a23f01f83e20d, and the existing library unit passed at dc915bdbb2c70897fae9e4ed96abf6d920dedd7b (1 passed, 0 failed, 0 ignored, 172 filtered, exit 0; /private/tmp/qsl-3293-dc91-libunit.log). Thus all seven family tests plus the one library unit are green. The final test-only fixture extraction is equivalent; no production assertions were weakened. All seven code-review findings and all three gap findings are closed. QSL-2271 stays open for the named remaining families and full native correlation port.
