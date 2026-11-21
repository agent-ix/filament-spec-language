---
id: SR-202611210022442313144108503
title: gap-analysis of QSL-2405/QSL-2406 package protocol parity
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; fsl-analyze/src/analyze/clause_item.rs, fsl-analyze/src/analyze/document/package_run.rs,
  fsl-analyze/src/analyze/protocol_item.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/model_check/check.rs,
  fsl-analyze/src/protocol_analysis/mod.rs, fsl-analyze/src/protocol_analysis/subject.rs,
  fsl-analyze/tests/it/analyze_package_protocol.rs, fsl-analyze/tests/it/analyze_protocol_subject.rs,
  fsl-analyze/tests/it/main.rs, fsl-analyze/tests/it/protocol_roles.rs, fsl-command/src/command/analyze.rs,
  fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/simulation/protocol.rs,
  fsl-eval/src/simulation/protocol/access.rs, fsl-eval/src/simulation/protocol/footprint.rs,
  fsl-eval/src/simulation/protocol/package_program.rs, fsl-eval/src/simulation/protocol/program.rs,
  fsl-eval/src/simulation/protocol/roles.rs, fsl-eval/src/simulation/protocol/selection.rs,
  fsl-eval/src/simulation/protocol/steps.rs, fsl-eval/src/simulation/protocol/wire.rs,
  fsl-package/Cargo.toml, fsl-package/src/analysis_view.rs, fsl-package/src/bare_slot.rs,
  fsl-package/src/bare_term.rs, fsl-package/src/checked_v2.rs, fsl-package/src/emit/tests/protocol_view.rs,
  fsl-package/src/model_input_form.rs, fsl-package/src/protocol_view.rs, fsl-replay/src/certificate/closure.rs,
  fsl-replay/src/lib.rs, fsl-replay/src/model_check/replay.rs, fsl-replay/src/model_check/settle.rs,
  fsl-replay/src/spine/protocol_structure.rs, fsl-eval/src/value/expression/mod.rs,
  fsl-eval/src/value/mod.rs, fsl-package/src/bare/observation.rs, fsl-semantics/src/model/observation.rs,
  fsl-semantics/src/model/observation/population_document.rs, fsl-semantics/src/model/state.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-940
  type: references
- target: ix://agent-ix/filament-spec-language/FR-850
  type: references
---

## Summary

Reviewed PR #3276 at frozen head 373f3ecdfd6630918de1fe947f81f91236ec7123 against origin/main 01572e0da58c6e3ef0e4e12768dc38d45da93208. Primary ticket QSL-2405; companion [QSL-2406](https://linear.app/agent-ix/issue/QSL-2406). This is the bounded litmus/memory/race SC/TSO/RA and input-threading/bytes-only replay slice, not whole FR-940 completion. The QSL-2271 architecture inventory comment d9959bab-f8ca-4e22-8b4e-51c436b1aa98 owns deferred controls, replication, non-All joins, other item kinds and reductions. The lead additionally confirms protocol deadlock name routing/default derivation remains QSL-2271; accepting DeadlockFreedom in an internal helper does not make that route complete.

## Verdict

PASS — disposition round 1: all original findings for this method are fixed, no new demonstrated regression, and authoritative exact-head focused checks pass. Mergeable within the approved bounded QSL-2405/QSL-2406 scope; QSL-2271 remains open for deferred protocol work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new the_execution_view_reads_attempt_facts_from_package_nodes test is tagged only FR-519 and TC-1314. Neither is an acceptance-criterion id, so the computed coverage report lists the test as an untracked symbol and no criterion records this new execution-view evidence. Add the criterion tag(s) whose mapped node/member behavior the assertions actually check; keep the existing artifact/case tags if useful. | fsl-package/src/emit/tests/protocol_view.rs:1172 |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2), with quire coverage — no run evidence read. The computed matrix is filtered to examined criteria; broader FR-940 gaps are outside this bounded PR. All five examined criterion rows (FR-940-AC-6, FR-850-AC-11, FR-940-AC-22, FR-1033-AC-4, FR-519-AC-5) are tagged, zero ignored-only. Existing tags do not establish that every new branch is tested. The package execution-view test appears in untracked_symbols with trace_id FR-519. Raw reports and stderr are retained beside this artifact; unrelated symbol collisions, NFR extraction and cross-repository link diagnostics are not attributed to this PR.

Changed behavior inventory: seven groups — package protocol views and identity/slot access, executable sequence/All-parallel/attempt/finish program, role/over binding, memory/access/race classification, analyze item/subject construction, byte replay and certificates, shared-node operand-row identity. All map to FR-940/FR-850 and existing FR-531/FR-390/FR-1033; zero unowned groups, source stubs or hollow tests. New APIs have real consumers; no copied upstream implementation, compatibility layer, suppression attribute, gate weakening or unsafe code was introduced.

Code-review ran its Rust lane with rust-style API, implementation, safety/concurrency, wire and testing references; schema and test oracles were inspected. No applicable AssuranceProfile was found. No spec/plan files changed relative to current main, so spec-review and its sub-analyses do not apply. Optional semantic intent/test/code inspection was authorized explicitly in the dispatch and is reflected in the concrete code findings. Scope is PR-only by instruction; no repository-wide PASS is claimed.

No Cargo command was run by this reviewer, per the dispatch constraint. Reused measured focused evidence: analyzer 21 and package view 28 passed at 106991ea00; protocol/subject 12 passed at ba39a0dc2f; integration analyze-lib check passed at 070c18e128. The red binder diagnostic at the final test-only head is recorded below. Existing bare_slots setup failures are MissingField(count) before slot evaluation, reproduced on main ab98ea12d8 by Draco; not introduced. The reserved-word where_undefined fixture is excluded on QSL-2678. Scoped Clippy had six introduced diagnostics fixed before freeze; remaining baseline diagnostics are disclosed, not lint-green. No full gates, whole-crate tests, all-target tests, workspace Clippy, cargo deny or independent final TEST-RUN were performed by this reviewer. Final exact-head TEST-RUN belongs to the lead after fixes.

## Binder diagnostic

The authorized single binder regression completed at frozen head 373f3ecdfd6630918de1fe947f81f91236ec7123: 0 passed, 1 failed, exit 101, in 1.15s. It failed at analyze_package_protocol.rs:169 on package/source item equality, after source proved with exit 0. Source exploration records 4 states and 3 transitions, confirming execution beyond A through B and finish; package returns failed/runtime_invariant/established-invariant-broken (internal-failure). This reproduces FND-001. Log: /tmp/qsl-2405-2406-binder-lifetime-red.log. No additional run was launched by the reviewer.

## Findings data

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
- id: FR-1033-AC-4
  path: spec/functional/FR-1033-select-a-protocol-subject-and-memory-models-in-an-analyze-request.md
  role: examined
  excerpt: A `memory` entry with `model` `pso` refuses the whole request `invalid-request`,
    exit 20, naming `/memory/0/model`; entries naming `SB::Both::left::L::Inner` (nested),
    `SB::Both::Sx` (an attempt) and `Nope::Both` (no such protocol), and two entries
    naming `SB::Both`, each refuse the whole request `invalid-request` with `invalid_runtime_input`/`invalid-value`
    naming the entry, or both entries, exit 20, with no item run.
- id: FR-519-AC-5
  path: spec/functional/FR-519-publish-the-protocol-read-surface-the-filament-protocol-link-layer-imports.md
  role: examined
  excerpt: For each record the map marks read by an FSL view function, the view function
    over **R** returns the node, argument or lock member the map names, equal to the
    value the checked clause holds in memory.
- id: FR-940
  path: spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md
  role: examined
  excerpt: "The package form SHALL return the value the source\n  form returns for\
    \ the package and the source it was compiled from."
- id: FR-1033
  path: spec/functional/FR-1033-select-a-protocol-subject-and-memory-models-in-an-analyze-request.md
  role: examined
  excerpt: "An entry naming an outermost `parallel` of a protocol no item is decided\n\
    \  over is admitted and changes no item's identity."
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-519-AC-5
  related_ids:
  - FR-519
  path: fsl-package/src/emit/tests/protocol_view.rs
  lines: '1172'
  excerpt: For each record the map marks read by an FSL view function, the view function
    over **R** returns the node, argument or lock member the map names, equal to the
    value the checked clause holds in memory.
  finding: The new the_execution_view_reads_attempt_facts_from_package_nodes test
    is tagged only FR-519 and TC-1314. Neither is an acceptance-criterion id, so the
    computed coverage report lists the test as an untracked symbol and no criterion
    records this new execution-view evidence. Add the criterion tag(s) whose mapped
    node/member behavior the assertions actually check; keep the existing artifact/case
    tags if useful.
bindings:
- test_id: the_tso_litmus_claim_has_source_and_package_route_parity
  ac_id: FR-940-AC-6
  trace: correct
- test_id: the_ra_litmus_claim_has_source_and_package_route_parity
  ac_id: FR-940-AC-6
  trace: correct
- test_id: the_tso_race_freedom_item_has_source_and_package_route_parity
  ac_id: FR-850-AC-11
  trace: correct
- test_id: the_default_sc_litmus_claim_has_source_and_package_route_parity
  ac_id: FR-850-AC-11
  trace: correct
- test_id: a_replay_over_the_input_bytes_bound_settles_replay_refused_on_both_routes
  ac_id: FR-940-AC-22
  trace: correct
- test_id: bad_memory_selections_refuse_the_request
  ac_id: FR-1033-AC-4
  trace: correct
```


Oracle inspection: ProtocolRun::decide (check.rs:1207) forwards to Run::decide (check.rs:2134), which invokes explore. Its nonboundary queue expansion (check.rs:2380) calls system.successors_bounded independent of the constant true monitor. No constant-claim success shortcut was found; a successful source search must expand the A/B sequence. The measured source exploration has 4 states and 3 transitions; the failing package result is reported above.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: The execution-view test now carries FR-519-AC-5. A fresh computed matrix at the disposition candidate binds its actual Rust test symbol at line1174 to that criterion; the test assertions are unchanged. |

## Disposition pass 1 evidence

Candidate d633b321c49449d7776d2b356160e4d9fde44afe. FINAL: all six original findings have latest outcome fixed; no open or new finding remains. Exact-head focused verification is green, and this bounded PR is mergeable. No Cargo or branch edits by reviewer. No new demonstrated regression found in the inspected fixes. Original Findings and Findings data are preserved byte-for-byte.

Rechecked code-review/Rust conventions and scoped gap traceability. The observation adapter reuses StateModel/observe_state and the canonical map-reference walker; it validates reference/relationship closure, uses the package evaluator authority and observed state, and does not introduce a source-checked graph. Source scalar evaluation and binder-meter paths are unchanged. Draco reported authoritative exact-head verification at d633b321c49449d7776d2b356160e4d9fde44afe: all three serial commands exited 0, each with zero failed and zero ignored tests. `cargo test -p fsl-analyze --test it -- analyze_package_protocol`: 10 passed; `cargo test -p fsl-semantics --lib -- model::observation::population_document::tests::map_references_follow_canonical_key_order`: 1 passed; `cargo test -p fsl-package --lib -- the_execution_view_reads_attempt_facts_from_package_nodes`: 1 passed. Logs are on Draco at `/private/tmp/qsl-3276-d633-1.log`, `/private/tmp/qsl-3276-d633-2.log`, `/private/tmp/qsl-3276-d633-3.log`; the reviewer received the lead's confirmed results and did not claim local access to those remote logs. This is the final focused verification for the candidate, not a whole-repository gate. No Cargo, full gates, workspace Clippy, whole-crate tests or unrelated baseline cleanup were run by this reviewer, per the dispatch constraints. No further test round is required by this review.

No spec/plan requirement changed in the fix round, so spec-review/sub-analyses do not apply. QSL-2271 exclusions remain in effect. Plan completion: not assessed.

## Disposition data

```yaml
round: 1
head: d633b321c49449d7776d2b356160e4d9fde44afe
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '#[trace("FR-519", "FR-519-AC-5", "TC-1314")]'
  path: fsl-package/src/emit/tests/protocol_view.rs
  lines: '1172'
  reason: The execution-view test now carries FR-519-AC-5. A fresh computed matrix
    at the disposition candidate binds its actual Rust test symbol at line1174 to
    that criterion; the test assertions are unchanged.
```


Mechanical correction within round 1: Draco first attempted the three focused commands at 4109a4e3b1592c393f9e049c98399fd4309305e8; all stopped before tests on E0308 (RelationshipExtents versus CheckedExtent slice). The replacement d633b321c49449d7776d2b356160e4d9fde44afe removes that invalid call. observe_state still admits/validates extents before any closure building (initial.rs:183-196); the adapter resolves each typed relationship endpoint in the current holder catalog and adds both population directions before transitive closure. No validation, cast or source-graph reconstruction was substituted. The replacement passed the authoritative exact-head focused checks recorded above.


Fix-round source/test inventory examined: fsl-analyze/src/analyze/clause_item.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/tests/it/analyze_package_protocol.rs, fsl-command/src/command/analyze.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/simulation/protocol.rs, fsl-eval/src/simulation/protocol/package_program.rs, fsl-eval/src/simulation/protocol/program.rs, fsl-eval/src/simulation/protocol/steps.rs, fsl-eval/src/value/expression/mod.rs, fsl-eval/src/value/mod.rs, fsl-package/src/bare/observation.rs, fsl-package/src/emit/tests/protocol_view.rs, fsl-semantics/src/model/observation.rs, fsl-semantics/src/model/observation/population_document.rs, fsl-semantics/src/model/state.rs. Observation/state additions are governed by FR-940 package/source evaluation equality and FR-106 recursive reference admission; no new standalone requirement or plan is claimed.
