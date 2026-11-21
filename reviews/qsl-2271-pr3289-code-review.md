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
| FND-002 | fixed | The DeadlockFreedom-only guard returns shared invalid_value for missing and mismatched protocol subjects before the old race-path branches. Verified statically at 49f66b3. |
| FND-003 | still-open | Valid package requests fail S4 admission; the round-2 delta does not change that path. Runtime parity and the new invalid-subject test remain unverified. |
| FND-003 | still-open | Round 3 selects the required terminal-fixture closure and removes the traced unrelated variant trigger; actual admission and all five focused runtime tests remain pending. The general variant-reader gap remains architecture-owned outside this slice. |
| FND-003 | fixed | Required type and construct closure selected at 5cb84ef; Draco reports all five exact-head terminal tests passed, exit 0. General variant-reader limitation remains architecture-owned outside this slice. |

## Round 1 verdict

CONDITIONAL; not mergeable while FND-002 remains open. No other defect found in the bounded fix delta.

## Disposition round 2 scope

Reviewed only the executable delta 5d3de7a37ee2986b05109eb3701851392e3460c0..49f66b37fbcc62139af3e520f8e7b5c1eb08faed. The added guard recognizes only DeadlockFreedom and returns the shared invalid_value helper for absent or mismatched protocol subjects. The race route retains its prior branches. The added three-case malformed-subject test checks category refusal, exact invalid_runtime_input/invalid-value diagnostic with member, absence of exploration, exit 20, and full source/package item parity after deleting the source file.

FND-002 is fixed by static inspection: the new guard precedes both differing old branches and matches source document.rs:2100-2119. No runtime pass is claimed. The new invalid-subject test also constructs the package through package_request/package_selection and analyzes that package; it crosses S4 admission before reaching the resolver. Thus its assertions cannot verify the guard until admission succeeds. The capacity case is supplementary existing-helper coverage, not evidence of execution here.

New runtime evidence supplied by the lead: all four valid terminal tests at 5d3de7a failed on the package route before items, last_stage S4, code unknown_required_feature, cause unsupported-feature, details.reason missing-type-representation. Source records were valid. Evidence: /tmp/protocol-3289-fix1-inbox.txt. This round changes only resolver guarding and a test, so it does not repair that earlier admission failure. Root cause is being traced by the coder; this review does not infer its location or waive the failure. No Cargo, runtime, matrix or full gate was run in this review round. QSL-2271 remains In Progress.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The valid terminal parity fixtures still refuse on package S4 admission with unknown_required_feature/unsupported-feature, reason missing-type-representation, and empty items, while the same source requests settle proved/refuted/inconclusive. Draco reports 0 of 4 focused tests passed at 5d3de7a; the round-2 selector guard does not change admission or package materialization. Root cause remains under investigation. | fsl-analyze/tests/it/analyze_package_protocol.rs:105 |

## Round 2 verdict

FAIL; not mergeable. FND-002 is statically fixed; FND-003 blocks runtime acceptance.

## Disposition round 3 scope

Reviewed executable delta 49f66b37fbcc62139af3e520f8e7b5c1eb08faed..333f12ccd5697dcfab1f021b1a4d02c63cde8cb3: only terminal_request's domain fixture selection changes. It retains the existing shared model's Level, Cell and Idle declarations, with their full fields and operations; populations and snapshots remain unchanged. This is a selection from the existing fixture, not a replacement semantic implementation or copied model definition.

Checked the required closure in fill::document: Cell.v and Cell.w require Level; Cell operations take Cell peers and return native Boolean; Cell has no supertypes or relationships. Idle is required because the unchanged idles population and initialization snapshot name it. The removed record and variant declarations are unused by the source unit, captures, static role, attempt contracts, finish Boolean, selected populations and snapshots. The valid blocked attempt still has false TwoPre, and the completed program remains an empty sequence. All explicit result, diagnostic, byte-refusal, deleted-source replay, full item and exit assertions are preserved. No next deterministic failure was identified by this static inspection.

The lead reports the original S4 refusal traced to the shared model's complete variant projection in pinned Contract IR 7b76c8b (runtime_type_projection:891), with an existing bare_slots reproduction at 2094. Architecture accepts this bounded terminal-fixture closure and owns placement of that general reader gap. This review does not treat the fixture selection as fixing the general reader limitation; no reader edit or dependency pin changes here.

The five focused terminal tests (including invalid-subject parity) on 333f12c remain pending with Draco. No Cargo, runtime or matrix command ran here, and the prior S4 failures are not replaced by an assumed pass. The invalid-subject test uses the same narrowed fixture and must still cross S4 before its resolver assertions. QSL-2271 remains In Progress for remaining families.

## Round 3 verdict

CONDITIONAL static fixture review; not mergeable until the five focused runtime tests succeed and required gates are satisfied. FND-003 remains open pending that evidence. FND-002 remains statically fixed. No new finding in this delta.

## Disposition round 4 scope

Reviewed fixture correction 333f12ccd5697dcfab1f021b1a4d02c63cde8cb3..5cb84ef9e24d5ef94d32619dd447b87f406190d4: terminal_request now filters model.constructs to value_type, object_type and population alongside the prior type selection. These are exactly the kinds instantiated by Level, Cell/Idle and the retained cells/idles populations. record_value_type and variant_type have no remaining instance. The previous static closure assessment missed the parser rule forbidding those unused construct declarations: Draco's five tests on 333f failed source I1 for that reason before terminal evaluation.

All semantic and refusal assertions remain unchanged. No further deterministic fixture failure was identified by static inspection of construct/type/member/operation/population/snapshot references. No reader, source evaluator, dependency or admission workaround changes. FND-003 remains open pending actual five-test evidence on 5cb84ef. No Cargo/runtime/matrix checks ran here. QSL-2271 remains In Progress.

## Round 4 runtime evidence and final verdict

Draco reports the exact frozen head 5cb84ef9e24d5ef94d32619dd447b87f406190d4 passed all five package_terminal_ test functions: 5 passed, 0 failed, 0 ignored, 2508 filtered, exit 0. Evidence reported by the lead: draco private /tmp/qsl-3289-5cb8.log. This is delegated runtime evidence, not a local rerun or direct access to Draco's filesystem. The fifth test includes missing/mismatched protocol-subject and capacity refusal assertions, so FND-002 now also has runtime confirmation.

FND-003 is fixed for this bounded terminal-family slice: the shared model is selected to its required type and construct closure, avoiding unrelated complete-variant projection, and all source/package terminal, ordered-capture, completed-instance, replay and invalid-subject oracles now pass. The general variant-reader limitation remains architecture-owned and is not claimed fixed by this fixture selection.

The bounded PR review is mergeable, with no open review finding. QSL-2271 remains In Progress for remaining protocol families. No Cargo/runtime/matrix rerun was performed by this reviewer. Repository-wide matrix/full-tree gap assurance is still not assessed; no repository-wide gap PASS is claimed.
