---
id: SR-1795253271066498708
title: "PR 3289 terminal package parity gap-analysis"
type: SpecReview
analysis: gap-analysis
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

CONDITIONAL: no bounded reverse-gap, stub, or test-assertion defect found. This is not an engine-backed repository gap PASS because the matrix and full-tree audit were excluded by dispatch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Bounded review only: inspected the five changed files and owning requirement sections. The repository-wide computed matrix, whole source inventory and engine-backed criterion completeness were not run. Manual trace inspection is provisional and cannot establish repository assurance PASS. No optional semantic gap-analysis was run.

Inventoried four changed behaviors: terminal decoding, capture extraction, capture initialization, and stopped-state classification. All four have requirement ownership (FR-383, FR-211, FR-940 and FR-531); zero unowned changed behaviors, source stubs or hollow test functions were found. No coverage thresholds or test gates changed.

Four real integration test functions exercise nine cases: blocked live absent/any/true/false; finished any with adversarial scheduling and false predicate; two ordered-capture predicates; replay byte refusal. They compare explicit expected labels and full source/package item records and exits. The blocked/capture tests remove program.fsl before package analysis, and refutations assert reproduced replay. Existing helper functions supply real compile/analyze calls rather than mocking the behavior under review. Tags on the added tests are supplementary coverage; they do not establish all scenarios of the broad cited ACs.

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
bindings:
- test_id: package_terminal_classifies_blocked_live_instances
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_terminal_classifies_blocked_live_instances
  ac_id: FR-850-AC-11
  trace: correct
- test_id: package_terminal_classifies_blocked_live_instances
  ac_id: FR-211-AC-2
  trace: correct
- test_id: package_terminal_preserves_finished_instances
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_terminal_preserves_finished_instances
  ac_id: FR-211-AC-2
  trace: correct
- test_id: package_terminal_reads_origin_captures_in_authored_order
  ac_id: FR-940-AC-6
  trace: correct
- test_id: package_terminal_reads_origin_captures_in_authored_order
  ac_id: FR-211-AC-2
  trace: correct
- test_id: package_terminal_reads_origin_captures_in_authored_order
  ac_id: FR-383-AC-5
  trace: correct
- test_id: package_terminal_preserves_replay_refusal_code_and_cause
  ac_id: FR-940-AC-22
  trace: correct
- test_id: package_terminal_preserves_replay_refusal_code_and_cause
  ac_id: FR-211-AC-2
  trace: correct
```

## Disposition round 1 scope

Reviewed fix delta from 9b5dc648ea9521f5ff9a7b22cbadbcdc486247e8 to 5d3de7a37ee2986b05109eb3701851392e3460c0 (excluding the trailing review-document copies). Initial runtime evidence from Draco was 0 passed / 4 failed, exit 101: malformed fixture grammar and unsupported package declaration selection prevented the intended behavioral checks. The prior static PASS was not runtime acceptance.

The fix corrects fixture grammar (sequence root, nonreserved attempt binder, captures before role), uses the already source-supported explicit temporal-satisfaction/deadlock-freedom selection, and requires an item record before the existing label checks. Expected proved/refuted/inconclusive labels, full item/exit equality, deleted-source replay and named replay refusal checks remain. The package resolver adds only the matching named protocol-declaration deadlock route and classifies its extent bounded like source; race freedom retains its existing unbounded extent. Declaration-global-conformance and remaining control families stay on the parent.

Checked source namespace resolution and protocol declaration ownership against document.rs; the new path resolves the declaration via admitted protocol_execution_named and retains OccurrenceKey::declaration. No source behavior or terminal engine implementation changes in this round. FR-940 and FR-620 own route parity, and FR-206 owns the attempt/structural-step fixture behavior. No spec change is required. No cargo/runtime/matrix/full gate was run in this round. The lead reports successful locked compile and fmt/diff checks; rerun of the four runtime tests on 5d3de7a is pending.

## Round 1 verdict

CONDITIONAL: no new bounded reverse-gap or hollow-assertion defect found. Matrix/full-tree assurance remains unassessed, and runtime evidence remains pending. Code-review SR-1795253271046542315 FND-002 records the malformed-subject record discrepancy; it is not duplicated here.

## Disposition round 2 scope

Reviewed only the executable delta 5d3de7a37ee2986b05109eb3701851392e3460c0..49f66b37fbcc62139af3e520f8e7b5c1eb08faed. The added guard recognizes only DeadlockFreedom and returns the shared invalid_value helper for absent or mismatched protocol subjects. The race route retains its prior branches. The added three-case malformed-subject test checks category refusal, exact invalid_runtime_input/invalid-value diagnostic with member, absence of exploration, exit 20, and full source/package item parity after deleting the source file.

FND-002 is fixed by static inspection: the new guard precedes both differing old branches and matches source document.rs:2100-2119. No runtime pass is claimed. The new invalid-subject test also constructs the package through package_request/package_selection and analyzes that package; it crosses S4 admission before reaching the resolver. Thus its assertions cannot verify the guard until admission succeeds. The capacity case is supplementary existing-helper coverage, not evidence of execution here.

New runtime evidence supplied by the lead: all four valid terminal tests at 5d3de7a failed on the package route before items, last_stage S4, code unknown_required_feature, cause unsupported-feature, details.reason missing-type-representation. Source records were valid. Evidence: /tmp/protocol-3289-fix1-inbox.txt. This round changes only resolver guarding and a test, so it does not repair that earlier admission failure. Root cause is being traced by the coder; this review does not infer its location or waive the failure. No Cargo, runtime, matrix or full gate was run in this review round. QSL-2271 remains In Progress.

## Round 2 verdict

CONDITIONAL: no additional bounded reverse-gap or assertion defect found. The package admission/runtime failure is recorded once as code-review SR-1795253271046542315 FND-003; repository matrix/full-tree assurance remains unassessed. No test runtime success is claimed.
