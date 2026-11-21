---
id: SR-1795255306593490537
title: PR 3290 parallel Any package parity code-review
type: SpecReview
analysis: code-review
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

PASS — no open findings; focused Any runtime family is green under the lane’s scoped-check policy.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The admitted package join maps closed All/Any policies, preserves the admitted joined labels and optional cancel/continue policy, and feeds the same Program::Join representation used by source. Existing join_enabled, join, recursive cancellation, finish waiting and replay paths perform execution; no CheckedGraph, CheckedProtocol or CheckedBody is reconstructed. Quorum/Predicate remain explicit Unsupported, rather than being coerced to All/Any. The strict reader already validates label membership/order/uniqueness and nonempty Any labels. Unknown outstanding words produce a typed package refusal rather than a guessed valid policy.

The three new real source/package tests contain nine fixture cases: Any cancel/continue, a blocked selected label, a ready selected label, blocked/ready All joins, nested cancel/continue, and replay input-byte refusal. Successful, refuted and inconclusive records and exits are compared; refuted cases require reproduced replay. The package run follows removal of program.fsl in every case. The model fixture retains Level/Cell/Idle, all Cell operations and fields, and all populations needed by its snapshots. It excludes unrelated Fill variant/compensation types; it does not repair or claim coverage of general variant-reader admission. FR-206's broader Quorum/Predicate and direct transition assertions are outside this slice and are not claimed newly verified.

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

## Mechanical range check

Current head: 2394498f7556566e29cd15c8726c399a20983fe7; current parent: 5cb84ef9e24d5ef94d32619dd447b87f406190d4. Git range-diff marks all three original child commits equivalent. The two production files are byte-identical to reviewed d50e806f9b. The only further executable change is nine fixture lines retaining construct metadata for value_type, object_type and population after the existing Level/Cell/Idle type pruning. These are exactly the construct kinds the remaining declarations and populations use; unused record/variant construct declarations are removed. All oracles, replay assertions and runtime implementation are unchanged. No defect remains in this mechanical delta.

No Cargo or runtime command was run in this follow-up. The three focused Any tests remain conditional on Draco; no runtime acceptance is inferred from the parent's static review or this range check.

## New findings (runtime disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | Package finish uses the synthetic label Flow::finish instead of authored Done, changing blocked-thread locations and counterexample state keys on the package route. | fsl-eval/src/simulation/protocol/program.rs:1162; fsl-analyze/tests/it/analyze_package_protocol.rs:158; fsl-analyze/tests/it/analyze_package_protocol.rs:188 |

Runtime at 2394498f7556566e29cd15c8726c399a20983fe7: 1 passed, 2 failed, 0 ignored, 2513 filtered, exit 101. Replay-refusal passes. The two package/source equality tests fail after the expected source outcome assertions pass. Parsing the full item records and decoding their state keys shows only the finish name differs: package `Flow::finish`, source `Done`, at blocked[0].rest.node and the root thread's rest in prefix post-states and canonical state keys. Nested case has five differing paths including decoded keys; ordinary joined-label case has three. All other record fields compare equal for those two failing cases. The source builder uses protocol.finish's authored name; the package builder hardcodes protocol-name::finish. The admitted finish node already carries declaration identity. Preserve that native authored identity; do not normalize the assertions or change the fixture name to the synthetic value.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | Runtime establishes an actual FR-940-AC-6 counterexample/state-key mismatch; no fix has been reviewed. |
| FND-002 | fixed | At edb954558b, package finish text preserves the authored declaration name, removing the sole decoded record mismatch. Static fix verified; the two failed runtime tests remain pending Draco. |

## Fix disposition pass 2

Head: edb954558b346829c6d0ff317cd4a117e717b536. The only executable delta from 2394498f75 is nine additions and one deletion in program.rs. The package finish text now comes from the admitted finish node's declaration qualified-name component 2. The emission contract is corroborated by fsl-package/src/emit/tests/protocol_view.rs:385 (`[protocol.name, "finish", "Closed"]`) and protocol_reference.rs:337 (`["Fill", "finish", "End"]`); source uses the same authored finish name. Missing declaration/name returns the existing typed invariant fault instead of panicking or guessing a name. No test fixture or assertion changed. No new static finding was found.


The unchanged replay-refusal result at 2394498f75 remains evidence for that test only. No Cargo/runtime/matrix command ran during this disposition.

## Final focused runtime acceptance

The lead supplied Draco's exact-head result for edb954558b346829c6d0ff317cd4a117e717b536: both previously failed exact filters passed, 2 passed, 0 failed, 0 ignored, 2514 filtered, exit 0; raw runner log `/private/tmp/qsl-3290-edb9-fix1.log`. These are `package_parallel_any_cancels_nested_running_branches` and `package_parallel_any_preserves_outstanding_and_joined_labels`. The unchanged `package_parallel_any_preserves_replay_refusal_code_and_cause` passed at 2394498f7556566e29cd15c8726c399a20983fe7 and is retained rather than rerun. The entire three-test focused family is therefore green. The sole executable fix since 2394498f75 preserves the authored finish name and changes no assertion or replay-limit setting. FND-002 is closed with runtime evidence.

No additional Cargo/runtime command was executed by this reviewer. Under the explicitly scoped lane policy, this slice is mergeable; QSL-2271 remains In Progress for its remaining families. Pure equivalent rebases need range checking, not a new full runtime cycle. Repository-wide assurance remains outside this review.

## Final equivalent rebase

Landing head f0f99c6cd2c096ce0be9fef624abc7fdd67457a7 on main32afa435df retains the same production/test patch as 5cb84ef9..edb954558b. The lead measured equal patch-id 06474a6ad65b1fe2b61df32c57963a7f7af099c3. Reviewer inspection of the resulting production diff shows only upstream CheckedTerm::Binding pattern `..` additions outside the child patch; Any join projection, authored finish-name fix and assertions are unchanged. Existing focused runtime evidence remains applicable; no new test cycle was run.
