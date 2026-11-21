---
id: SR-1795260655457219542
title: PR 3294 Receive and FIFO code-review
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language; QSL-2271; fsl-analyze/tests/it/analyze_package_protocol.rs;
  fsl-eval/src/simulation/protocol.rs; fsl-eval/src/simulation/protocol/channel.rs;
  fsl-eval/src/simulation/protocol/package_program.rs; fsl-eval/src/simulation/protocol/program.rs;
  runtime fix: fsl-package/src/reader_protocol_projection.rs; fsl-package/src/emit/tests.rs'
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-206
  type: references
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3294. Frozen head: f2f1bb1503eb2288c3f283697d0faab791899583; base: f8686b7d5bfb152604ee917340b238beb8966382. Code-review and Rust lane inspected only this five-file slice. Reviewer-only: the worktree and target were untouched. Exact model id is not exposed; markers record model=unknown.

## Verdict

PASS — all three findings are closed; all five integration-family tests and the reference-equality witness library unit are green. The slice is mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new receive_fifo_request copies the existing finite-channel model and request setup from channel_send_request, and receive_fifo_send independently formats Send syntax already supplied by channel_send. Extract/reuse the common setup and existing formatter, retaining FIFO-only payload extensions and exact fixture data/assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:80-151,155-160 |

## Coverage

The five-file change was inspected against its frozen parent squash. Receive maps the admitted `of` control into the shared message engine; FIFO ordering keeps source CheckedBody and package ProtocolSlot paths explicit. Actual package key evaluation supplies the separate level-zero payload binder through the shared reference-population conversion, filters unused binders for literal keys, evaluates the admitted key term, and reuses existing value encoding and queue partitioning. The extracted frame adapter preserves its previous conversion logic. The source constructor and admitted constructor map FIFO channels to the same ordering behavior. Package constraints and binder liveness use the existing parent machinery. No source graph/body is fabricated.

The five integration tests cover ten fixture cases: reference/scalar keys and complete send/receive/finish shape; equal-key head blocking versus distinct-key delivery; false receive constraint, emptied queue and displaced payload; explicit replay input-byte refusal code/cause; and nonempty correlation refusal naming its admitted node. Full item/exit comparison follows source removal and refuted cases require reproduced replay. The correlated Receive guard shares the admitted Group check and typed reason with Send; the existing global Parallel+Send guard remains intact. The full correlation port, symbolic capacities, compensation and mixed message-identity/weak-footprint work remain named on open QSL-2271, not claimed delivered here.

Searches for existing test setup in this module located channel_send_request and channel_send. The new helper repeats the model type set, Attempt-to-Level field rewrite, used construct set, initial snapshot and temporal subject construction before adding its FIFO-specific fields. This is the one duplication finding. Shared Owns relationship construction correctly uses related_by::add_owns_relationship, and no copied upstream artifacts or duplicate production evaluator/queue algorithm were found.

No new unsafe, concurrency, wire-payload schema or public enum variant is introduced. Existing malformed/absent admitted reads return typed package refusals; package FIFO evaluation retains common undecided handling. Scoped source/test stub search found no new stub. No applicable AssuranceProfile was found. Spec/plan files are unchanged, so spec-review methods are inapplicable.

Validation limits: no Cargo, runtime, computed matrix, full gate or whole-repository reverse-gap audit was run, as explicitly required by the dispatch. The lead reports the scoped integration-target compile passed. Subsequent Draco evidence below shows four ordinary runtime failures. Manual slice-local requirement/test/source alignment was performed, not an engine-backed repository assurance PASS. Optional semantic-review workflow was not invoked. Slice-local unowned behaviors: 0; production/test stubs: 0. Full-tree coverage and unrelated inventory remain unassessed.

Plan completion: not assessed

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
- id: FR-731-AC-1
  path: spec/functional/FR-731-take-an-event-step-only-where-its-related-by-clauses-hold.md
  role: context_only
  excerpt: 'Over a model with orders `o1`, `o2` and payments `p1` settling `o1` and
    `p2` settling `o2`, a protocol over `o1` whose `event Paid by R as (paid: M::Payment)
    related by settles (paid, input) { true }` explores `Paid` steps binding `p1`
    only; the same node with no `related by` clause explores steps binding both.'
- id: code-review/Duplication and Vendoring
  path: /home/peter/.codex/plugins/cache/agent-ix/dev-tools/0.3.2/skills/code-review/SKILL.md
  role: examined
  excerpt: '- **fail**: copy-pasted test setup that an existing fixture already provides'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'S6a SHALL evaluate one expression of one protocol node of a `CheckedPackage`:

    an activation guard, a capture initializer, a retry relation, a recovery

    predicate, a trigger guard, a control''s constraint, guard, visible entry or

    `related` end, a finish or terminal condition, a role''s lifetime or a channel''s

    key. The expression is the term the package holds in the node''s body (FSpec

    FR-464, FR-465, FR-466). The caller supplies the value of every binder and

    every captured value the expression reads, and an observation when a supplied

    value names a model object. Evaluation reads the package and the'
- id: FR-531/What the entry derives from the bytes
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: '| protocol slot, binders, binder types and levels, locations, callees
    | the admitted protocol nodes and `protocol_locations` rows (FR-390, FR-464) |
    the source `CheckedBody` and checked package |'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: other
  artifact_id: code-review/Duplication and Vendoring
  related_ids:
  - FR-940-AC-6
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  lines: 80-151,155-160
  excerpt: '- **fail**: copy-pasted test setup that an existing fixture already provides'
  finding: The new receive_fifo_request copies the existing finite-channel model and
    request setup from channel_send_request, and receive_fifo_send independently formats
    Send syntax already supplied by channel_send. Extract/reuse the common setup and
    existing formatter, retaining FIFO-only payload extensions and exact fixture data/assertions.
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-940-AC-6
  related_ids:
  - FR-390
  path: fsl-eval/src/simulation/protocol/channel.rs
  lines: 152-183
  excerpt: For each item family whose evaluator FR-531's inventory lists as taking
    a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant
    `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`),
    a `program` request and a `package` request over the handoff directory of the
    same compiled unit give equal item records, counterexamples and replays, and no
    package-route record is `unsupported`; a handoff directory whose `checked-package.json`
    the strict reader refuses, or that holds no `identities.json`, settles no item
    and compiles no source (FR-850-AC-
  finding: All four ordinary Receive/FIFO package tests fail with runtime_invariant/established-invariant-broken
    while source yields the required outcomes; the correlated guard alone passes.
    The public result drops the underlying invariant, so diagnose and fix the admitted
    key/supply/receive boundary without weakening full-record, wait-cause, or replay
    assertions.
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: FR-940-AC-6
  related_ids:
  - FR-531-AC-16
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  lines: 181-185; fsl-package/src/emit/tests.rs:3396
  excerpt: For each item family whose evaluator FR-531's inventory lists as taking
    a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant
    `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`),
    a `program` request and a `package` request over the handoff directory of the
    same compiled unit give equal item records, counterexamples and replays, and no
    package-route record is `unsupported`; a handoff directory whose `checked-package.json`
    the strict reader refuses, or that holds no `identities.json`, settles no item
    and compiles no source (FR-850-AC-
  finding: The new NE integration cases and reference-witness unit use the unsupported
    source token <> instead of !=, so both fail during source parsing before testing
    the intended inequality and witness behavior. Correct only the fixture lexeme,
    preserving operators, expected outcomes, authorities, and assertions.
bindings:
- test_id: package_receive_fifo_runs_the_complete_example_body
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_receive_fifo_runs_the_complete_example_body
  ac_id: FR-383-AC-4
  trace: correct
- test_id: package_receive_fifo_runs_the_complete_example_body
  ac_id: FR-383-AC-7
  trace: correct
- test_id: package_receive_fifo_preserves_key_partition_and_head_blocking
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_receive_fifo_preserves_key_partition_and_head_blocking
  ac_id: FR-206-AC-4
  trace: correct
- test_id: package_receive_fifo_preserves_key_partition_and_head_blocking
  ac_id: FR-211-AC-5
  trace: correct
- test_id: package_receive_fifo_preserves_receive_wait_causes
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_receive_fifo_preserves_receive_wait_causes
  ac_id: FR-206-AC-7
  trace: correct
- test_id: package_receive_fifo_preserves_receive_wait_causes
  ac_id: FR-211-AC-5
  trace: correct
- test_id: package_receive_fifo_preserves_replay_refusal_code_and_cause
  ac_id: FR-940-AC-22
  trace: correct
- test_id: package_receive_fifo_preserves_replay_refusal_code_and_cause
  ac_id: FR-206-AC-4
  trace: correct
```

## New findings (runtime evidence)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | All four ordinary Receive/FIFO package tests fail with runtime_invariant/established-invariant-broken while source yields the required outcomes; the correlated guard alone passes. The public result drops the underlying invariant, so diagnose and fix the admitted key/supply/receive boundary without weakening full-record, wait-cause, or replay assertions. | fsl-eval/src/simulation/protocol/channel.rs:152-183 |

## Runtime evidence

Draco at exact f2f1bb1503eb2288c3f283697d0faab791899583: 1 passed, 4 failed, 0 ignored, 2524 filtered, exit 101; raw log /tmp/protocol-3294-f2f-full.log. The correlated Receive guard passes. All four ordinary tests reach valid source results but package results are internal-failure with runtime_invariant/established-invariant-broken. The public record drops the underlying fault: ProtocolRefusal to ExpansionStop conversion and package-key map_err paths collapse it to BROKEN. No diagnostic rerun was performed.

Correction: the initial literal-root hypothesis was withdrawn after tracing the complete emitter. ordering() calls expression; lowering.rs:4660 routes a literal through literal_as:7195 and value_node:4530, which creates a keyed value/literal node and returns its reference. emit.rs:2379 preserves that reference. Therefore bare_slot.rs requiring a referenced root does not establish this failure cause. The four observed runtime failures remain real; their exact invariant is unresolved, and bounded error diagnostics are needed before a production correction. No literal-root API port should be justified by the withdrawn hypothesis. Assertions and accepted outcomes must stay intact.

## Established runtime cause

Diagnostic run a064 (four previously failing filters; markers /tmp/protocol-3294-a06-markers.txt) establishes the same failure before FIFO in every case: control-evaluate node 1 refuses unknown_required_feature/unsupported-feature with reason missing-operation-projection:filament.op.reference.eq, in the Send constraint comparing the payload target reference with k. This supersedes the unresolved-phase diagnosis; no inline-literal, frame, or provenance-conversion defect has been established.

The concrete local missing dispatch is reader_protocol_projection.rs:691-712 is_equality_identity: its closed list omits reference.eq and reference.ne. project_application:2867 already routes listed identities to equality:2910-2957, which reads the retained admitted equality decision and constructs the shared NodeKind::Equality. Source lowering:5158 chooses the reference operation, next_operand:6200 retains the equality producer data/identity for that checked node, emit:3270/3847 serializes all producer rows, and the pinned IR equality_projection:148 recognizes catalogued eq/ne suffixes. The shared value library selects Plan for compatible Reference types. A bounded real projection should reuse those admitted decisions, not re-run check_equality from types or create a checked graph. The ReferenceLiteral projection arm is separate and remains untouched. FND-002 stays open until the actual correction and focused runtime pass, with temporary diagnostics removed.

## Disposition pass 1

Inspected frozen 949206cfecc781c9fd264d19cc087cc2f3895260 against the original f2f1bb1503 head. The actual runtime correction is exactly two recognized operation identities; the existing retained-decision equality builder and shared kernel remain authoritative. No reader API, wire schema, runtime rechecking or ReferenceLiteral arm changed. The new package library regression uses the existing authenticated model-unit/identity/equality fixture and checks both EQ and NE projection plus typed MissingEqualityWitness. New integration parameter cases exercise same-reference NE false, distinct-reference NE true, and distinct-reference EQ false with explicit second-object snapshots. Original cases and full-record/replay assertions remain. No diagnostic marker remains; protocol.rs/channel.rs equal pre-diagnostic 13a9f9c90d. No source edits or Cargo by reviewer.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The common fixture now lives in finite_channel_model, channel_request, channel_snapshots and channel_send_body; FIFO adds only its extra fields. Default model JSON, source whitespace, snapshots, and assertions are preserved. |
| FND-002 | still-open | The reference.eq/ne dispatch omission is fixed statically and diagnostics are removed. The four failed integration filters and new package witness unit are queued; runtime equality is not yet accepted. |
| FND-002 | still-open | Round 2: statically repaired; the two remaining runtime checks are pending. |
| FND-003 | fixed | Round 2: fixture-only <> to != substitutions preserve NE semantics and all assertions. |
| FND-002 | fixed | Final round-2 acceptance: key-partition and witness unit passed at 9342b787; retained three ordinary 949 passes and original correlated-guard pass give all five family tests plus the witness unit green. |

## New findings (disposition pass 1 runtime)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new NE integration cases and reference-witness unit use the unsupported source token <> instead of !=, so both fail during source parsing before testing the intended inequality and witness behavior. Correct only the fixture lexeme, preserving operators, expected outcomes, authorities, and assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:181-185; fsl-package/src/emit/tests.rs:3396 |

## Runtime after disposition pass 1

At 949206cfecc, three ordinary integration tests pass; key partitioning and the package witness unit stop on the new NE source text with expected-expression syntax diagnostics. The parser mapping fsl-forms/src/value.rs:1738 recognizes !=. The original same-key/different-key and EQ witness loop iterations occur before the failing NE cases; no new equality-kernel or witness defect is established by these parse failures. Retain these three runtime passes and the original correlated Receive guard pass.

## Disposition pass 2

Inspected frozen 9342b787ac27d5f5b94b8cd78ee733acea52ca2b. Its complete diff is four source-fixture token substitutions across two test files, <> to !=. Production code, intended NE semantics, model/snapshot data, expected outcomes and witness assertions are unchanged; diagnostics remain removed. The root queued only the failed key-partition integration function and the package witness library function. No reviewer Cargo/runtime execution.

The initial static round-2 check awaited runtime for FND-002 and confirmed the fixture-only correction for FND-003. Final runtime acceptance follows.

## Final runtime acceptance

Draco at exact 9342b787ac27d5f5b94b8cd78ee733acea52ca2b reports key partitioning 1 passed, 0 failed, 0 ignored, 2528 filtered, and package witness unit 1 passed, 0 failed, 0 ignored, 1053 filtered; logs /private/tmp/qsl-3294-9342-1.log and /private/tmp/qsl-3294-9342-2.log. Retained evidence is the three other ordinary integration passes at 949206cfecc and the unchanged correlated Receive guard pass at f2f1bb1503. All five family tests plus the one witness unit are green. FND-001 fixture duplication, FND-002 missing reference projection, and FND-003 fixture syntax are closed. No new finding or source/oracle weakening remains. Temporary diagnostics are removed. QSL-2271 remains open for its named correlation/provenance and other remaining families; those do not hold this slice. No reviewer Cargo or runtime rerun was performed.
