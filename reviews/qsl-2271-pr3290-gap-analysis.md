---
id: SR-1795255306601230608
title: PR 3290 parallel Any package parity gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2271; fsl-eval/src/simulation/protocol/package_program.rs;
  fsl-eval/src/simulation/protocol/program.rs; fsl-analyze/tests/it/analyze_package_protocol.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-206
  type: references
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3290. Frozen head: d50e806f9b06a7a68b6e3035f8b0bd0133355b82; reviewed base: 333f12ccd5697dcfab1f021b1a4d02c63cde8cb3 (terminal parent #3289). Only the three-file child slice was reviewed. QSL-2271 remains In Progress for the remaining protocol families. No worktree source or branch was changed.

No specification or plan changes occur, so spec-review and its sub-analyses do not apply. No applicable AssuranceProfile was found. The exact model identifier is not exposed to this reviewer; tracker markers record `model=unknown` rather than inventing one.

## Verdict

CONDITIONAL: no slice-local defects found; matrix, full repository audit and runtime acceptance remain unassessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The admitted package join maps closed All/Any policies, preserves the admitted joined labels and optional cancel/continue policy, and feeds the same Program::Join representation used by source. Existing join_enabled, join, recursive cancellation, finish waiting and replay paths perform execution; no CheckedGraph, CheckedProtocol or CheckedBody is reconstructed. Quorum/Predicate remain explicit Unsupported, rather than being coerced to All/Any. The strict reader already validates label membership/order/uniqueness and nonempty Any labels. Unknown outstanding words produce a typed package refusal rather than a guessed valid policy.

The three new real source/package tests contain nine fixture cases: Any cancel/continue, a blocked selected label, a ready selected label, blocked/ready All joins, nested cancel/continue, and replay input-byte refusal. Successful, refuted and inconclusive records and exits are compared; refuted cases require reproduced replay. The package run follows removal of program.fsl in every case. The model fixture retains Level/Cell/Idle, all Cell operations and fields, and all populations needed by its snapshots. It excludes unrelated Fill variant/compensation types; it does not repair or claim coverage of general variant-reader admission. FR-206's broader Quorum/Predicate and direct transition assertions are outside this slice and are not claimed newly verified.

Reconciliation: computed matrix and coverage extraction explicitly excluded by dispatch; manual slice-local requirement/test/source inspection only. This is a bounded gap review, not a full gap audit.

Plan completion: not assessed

Inventoried changed behaviors: 3 (policy, selected labels, outstanding policy); slice-local unowned behaviors: 0; production stubs: 0; test stubs: 0. Optional semantic-review workflow: skipped; ordinary code-test alignment was inspected in the code review.

## Validation limits

Per explicit dispatch, no Cargo, runtime, full gate, computed matrix, coverage command, or whole-repository audit was run. The lead reports an earlier compile success before fixture corrections and scoped rustfmt/diff checks after them; this is not independent runtime evidence. Draco's three focused Any tests remain pending. Repository-wide criterion coverage, ignored-test census and full reverse-gap completeness are not assessed. No runtime success or full repository assurance is implied by this review.

## Examined scope

```yaml
clean: true
scope:
- id: FR-206-AC-3
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: "Three branches each with one always-enabled attempt: `join any \u2026\
    \ outstanding cancel` is enabled after the first branch completes, and its step\
    \ sets both other threads `cancelled`, which take no further step; `join any \u2026\
    \ outstanding continue` leaves them `running`, and each still takes its attempt;\
    \ `join quorum(2) outstanding cancel` is not enabled after one completion and\
    \ is after two; `join predicate(f) outstanding cancel` with `f` true exactly when\
    \ branch `a` is `completed` is enabled after `a` completes and not after only\
    \ `b` and `c`."
- id: FR-206-AC-6
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: "Two branches, `a` one always-enabled attempt and `b` a `sequence` of two\
    \ always-enabled attempts, joined `any \u2026 outstanding continue`, then `finish`\
    \ with constraint `true`: in a state where the root rests at `finish` and `b`\
    \ is still `running`, `finish` is not enabled; once `b` completes, `finish` is\
    \ enabled and its step marks the instance finished."
- id: FR-206-AC-9
  path: spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md
  role: examined
  excerpt: '`parallel Outer { branch a attempt Fast; branch b sequence { parallel
    Inner { branch x attempt X1; branch y attempt Y1 } join any [x, y] outstanding
    continue; attempt After } } join any [a] outstanding cancel`, with every attempt
    always enabled: in a state where `a` has completed, `b` rests at `Inner`''s join
    and `x` and `y` are both `running`, the `join(Outer)` step makes `b`, `x` and
    `y` `cancelled`, settles and removes `Inner`''s instance, and in no later state
    does any of them take a step; `Y1` and `After` are enabled in no state after that
    join.'
- id: FR-383-AC-6
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: 'In **W** and its variants, `sequence`, `choice`, `parallel` and `repeat`
    emit their layouts: a `sequence` its children in order and an empty `sequence`
    an empty list; a `choice` its owner, its set of visible expression-node references
    and at least two cases sorted by label; a `parallel` each of the join policies
    `all`, `any`, `quorum` (with `count`) and `predicate` (with the function node),
    the memory model and `outstanding` exactly when written (every non-`all` join
    is written with one, FR-218), and the joined labels in ascending order; a `repeat`
    with a maximum and an `exhausted` control, wi'
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
- id: FR-531
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: context_only
  excerpt: 'FSL owns evaluation (FR-107, FR-390, FR-516); Contract IR owns the reader
    and the

    package''s admission. This requirement adds no checked-package member and no Contract'
findings: []
```
