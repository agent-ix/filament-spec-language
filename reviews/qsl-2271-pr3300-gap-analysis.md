---
id: SR-1795263280874614189
title: gap-analysis of QSL-2271 PR3300 package untimed Await
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2271; PR3300; exact 949206cfecc781c9fd264d19cc087cc2f3895260..5e846bf37ee03312ca82edb0967d6afa5dbfea11;
  fsl-analyze/tests/it/analyze_package_protocol.rs; fsl-eval/src/simulation/protocol/package_program.rs;
  fsl-eval/src/simulation/protocol/program.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-206
  type: references
---

## Summary

Ticket QSL-2271, PR filament-spec-language#3300. Bounded planless gap analysis; full repository assurance is not assessed. No static findings in the frozen Await-only diff.

## Verdict

CONDITIONAL: static review is clean. Await runtime acceptance and the parent FIFO prerequisite remain pending. QSL-2271 stays In Progress/open.

Bounded reverse-gap inspection inventoried three added behavior obligations: admitted Await continuation recovery, shared runtime mapping/child shape, and the matched Attempt correlation guard. These map respectively to FR-383-AC-7/FR-940-AC-6, FR-206-AC-5, and the architecture ruling recorded on QSL-2271 with FR-731 context. Zero unowned behavior, source stubs, or test stubs were found in the scoped change. Tests exercise the real source compiler, emitted package reader, engine and replay, with no mocks or test-only bypass.

Computed matrix verification and whole-repository reverse-gap inventory were explicitly excluded by the dispatch. Trace tags were inspected directly as a provisional scoped index only; this is not a complete repository gap audit or an engine-backed coverage PASS. Optional full semantic review was not invoked. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined only the three changed Rust files and the callers needed to establish their behavior. The 3 commits change 380 insertions/14 deletions; base 949206cfecc781c9fd264d19cc087cc2f3895260, head 5e846bf37ee03312ca82edb0967d6afa5dbfea11. Parent #3294 is contextual and is not re-reviewed. No spec, plan, public enum, dependency, CI, wire, or transport delta occurs. Spec-review methods are not applicable. No applicable AssuranceProfile document was found under spec/.

The dispatch prohibits Cargo, runtime tests, matrix, aggregate gates, target operations, and source edits. None were performed. The lead reports the required locked-wrapper debug=0/incremental=0 `cargo check --locked -p fsl-analyze --test it -j4` passed in 2m23s; this reviewer did not independently execute it. Five Await integration tests, nine fixture cases including the separate replay-bound workspace, remain queued through Draco after the parent FIFO four integration plus one package witness test are GREEN. No runtime outcome is assumed.

Reviewed test cases: healthy Receive under exactly-once-premise and at-least-once; timeout deadlock; separate replay input-byte refusal; false matched Receive; matched Send; matched Attempt; related matched Attempt with false and true Owns self-edge. Full item/exit equality and refuted replay assertions use channel_send_parity, after source removal. Strong witnesses identify timeout or the actual Send/Attempt step and the blocked branch; state/edge counts distinguish healthy and false-match execution. Existing finite_channel_model, channel_request, channel_send, receive_fifo_receive, channel_related_premise and related_by::add_owns_relationship are reused. No copied model/snapshot/Owns setup was added.

Scope excerpts below are verbatim fragments capped at 600 characters; examined means the Await-specific obligation was inspected, not that the whole multi-behavior criterion was proven.

## Examined scope

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
```
