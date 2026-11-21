---
id: SR-1795253271046542315
title: "PR 3289 terminal package parity code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2271; fsl-eval/src/simulation/protocol.rs; fsl-eval/src/simulation/protocol/package_program.rs; fsl-eval/src/simulation/protocol/program.rs; fsl-eval/src/simulation/protocol/roles.rs; fsl-analyze/tests/it/analyze_package_protocol.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/filament-spec-language/FR-940, type: references}
  - {target: ix://agent-ix/filament-spec-language/FR-211, type: references}
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3289. Frozen head: 9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8; base: d9400e7640639fd74c9bebeaa89cc2d25ab862ce. Read-only source review. This is the terminal family of the parent inventory; the parent remains In Progress for remaining protocol controls and families. No source files or PR comments were changed.

No applicable AssuranceProfile was found. No spec or plan files change, so spec-review and its sub-analyses are inapplicable. Existing FR-531/FR-383/FR-211/FR-940/FR-850 own these behaviors; no new specification is needed for this bounded parity change.

## Verdict

PASS for the static code/Rust review: no defects found. Merge authorization remains conditional on actual focused runtime and required gate evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Inspected absent/any/when normalization, admitted terminal obligation mode and condition slot, capture rows in authored order, binder identity/type alignment, capture initialization against origin observation and earlier capture values, exact slot binder projection, and terminal evaluation against current observed memory. Package execution uses admitted nodes and ModelEvaluator slots; it does not reconstruct CheckedGraph, fabricate source, or use a compatibility fallback. The shared source branch retains its checked-body evaluation and terminal classification, including intended states with no live instance. No new public API, unsafe code, narrowing wire conversion, concurrency, gate reduction, or external copy was introduced.

Capture failure conversion matches source decide_body: Completed values initialize the capture; undefined/refused/incomplete outcomes become the same undetermined model finding. Terminal slot outcomes use the existing package_slot_verdict and shared classification for undefined and exhausted evaluation meters. Existing supported static-role validation, observation admission, graph admission/cycle handling and unsupported-family boundaries remain dependencies, rather than being reimplemented by this slice.

## Validation limits

No cargo, runtime, full gate, or matrix command was run, per dispatch. The lead reported a successful locked analyze test compile; that is not independent runtime evidence. The four focused runtime tests are delegated to Draco and their results were unavailable when this review was written. Undefined and evaluation-meter denial branches were inspected, not executed; the added bound test specifically checks replay input byte refusal, not those two branches. Required aggregate checks remain the lead's responsibility.

## Examined scope

```yaml
clean: true
scope:
- id: FR-531
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: protocol slot, binders, binder types and levels, locations, callees
- id: FR-383
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: '`captures`, the protocol captures in source order'
- id: FR-383-AC-5
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: '`finish` emits its closing-record binder node and constraint; `terminal
    when {...}` emits mode `when` with the condition and `terminal any` mode `any`
    with none; a protocol without `terminal` has no `terminal` argument, and `scheduling
    adversarial` emits the text `adversarial` in the clause, a protocol without it
    the absent optional.'
- id: FR-211
  path: spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md
  role: examined
  excerpt: a terminal state with no live instance is intended.
- id: FR-211-AC-2
  path: spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md
  role: examined
  excerpt: Over `Fill` with no `terminal` member, s6 is terminal and intended, and
    s2 is terminal and deadlocked. The deadlock-freedom item settles `refuted` with
    the prefix `fork(Both)`, `attempt(A)` and blocked threads `(0, Both, right, 0)`
    at `B` with `Precondition{TwoPre}` and `(0)` at `join Both` with `JoinWaiting{[(0,
    Both, right, 0)]}`. With `terminal when k.v = 1`, s2 is intended and the item
    settles `proved`; with `terminal any`, the request carries no deadlock-freedom
    item.
- id: FR-940
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: The package form SHALL return the value the source form returns for the
    package and the source it was compiled from.
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
- id: fsl-eval/src/simulation/protocol.rs
  path: fsl-eval/src/simulation/protocol.rs
  role: examined
  excerpt: Frozen diff d9400e7640639fd74c9bebeaa89cc2d25ab862ce..9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8.
- id: fsl-eval/src/simulation/protocol/package_program.rs
  path: fsl-eval/src/simulation/protocol/package_program.rs
  role: examined
  excerpt: Frozen diff d9400e7640639fd74c9bebeaa89cc2d25ab862ce..9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8.
- id: fsl-eval/src/simulation/protocol/program.rs
  path: fsl-eval/src/simulation/protocol/program.rs
  role: examined
  excerpt: Frozen diff d9400e7640639fd74c9bebeaa89cc2d25ab862ce..9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8.
- id: fsl-eval/src/simulation/protocol/roles.rs
  path: fsl-eval/src/simulation/protocol/roles.rs
  role: examined
  excerpt: Frozen diff d9400e7640639fd74c9bebeaa89cc2d25ab862ce..9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8.
- id: fsl-analyze/tests/it/analyze_package_protocol.rs
  path: fsl-analyze/tests/it/analyze_package_protocol.rs
  role: examined
  excerpt: Frozen diff d9400e7640639fd74c9bebeaa89cc2d25ab862ce..9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8.
findings: []
bindings: []
```

## Disposition round 1 scope

Reviewed fix delta from 9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8 to 5d3de7a37ee2986b05109eb3701851392e3460c0 (excluding the trailing review-document copies). Initial runtime evidence from Draco was 0 passed / 4 failed, exit 101: malformed fixture grammar and unsupported package declaration selection prevented the intended behavioral checks. The prior static PASS was not runtime acceptance.

The fix corrects fixture grammar (sequence root, nonreserved attempt binder, captures before role), uses the already source-supported explicit temporal-satisfaction/deadlock-freedom selection, and requires an item record before the existing label checks. Expected proved/refuted/inconclusive labels, full item/exit equality, deleted-source replay and named replay refusal checks remain. The package resolver adds only the matching named protocol-declaration deadlock route and classifies its extent bounded like source; race freedom retains its existing unbounded extent. Declaration-global-conformance and remaining control families stay on the parent.

Checked source namespace resolution and protocol declaration ownership against document.rs; the new path resolves the declaration via admitted protocol_execution_named and retains OccurrenceKey::declaration. No source behavior or terminal engine implementation changes in this round. FR-940 and FR-620 own route parity, and FR-206 owns the attempt/structural-step fixture behavior. No spec change is required. No cargo/runtime/matrix/full gate was run in this round. The lead reports successful locked compile and fmt/diff checks; rerun of the four runtime tests on 5d3de7a is pending.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The new explicit protocol-declaration deadlock route returns different malformed-subject records from source: absent temporal.subject.protocol produces invalid_runtime_input/missing-member, and a subject naming another protocol produces unknown-claim; source returns declined invalid_runtime_input/invalid-value naming temporal.subject.protocol in both cases (document.rs:2100-2119). | fsl-analyze/src/analyze/document/package_run.rs:1139 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | Newly exposed malformed-subject branches differ from source at frozen 5d3de7a. Align the new deadlock route with source while retaining race behavior, and verify both absent and mismatched protocol subjects. |

## Round 1 verdict

CONDITIONAL; not mergeable while FND-002 remains open. No other defect found in the bounded fix delta.
