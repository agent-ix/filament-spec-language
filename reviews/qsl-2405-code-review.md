---
id: SR-202611210022442313144108502
title: code-review of QSL-2405/QSL-2406 package protocol parity
type: SpecReview
analysis: code-review
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
| FND-001 | medium | The package program installs two empty ReaderPositions instead of the real binder readers. Consequently reads_from is false for every attempt binder and normalize removes its value immediately after the attempt. In the supported sequence A=Core::look followed by B=Core::copy constrained by not a, A returns false on the existing litmus snapshot, source retains a and proceeds, but package B receives Unavailable and stops with BROKEN. Compute reader positions for every admitted binder, including readers in later attempts and finish. | fsl-eval/src/simulation/protocol/program.rs:1188 |
| FND-002 | medium | Supported package attempt constraints receive supply_for_slot(..., None), and package_binder_values gives reference values no population provenance. For an SB attempt constraint k.r1 >= 0, the over binder k is an object reference: admit_slot_supply rejects its missing ProtocolReferenceOccurrence before evaluating the expression, and the call is mapped to BROKEN. Source evaluates the same constraint against its current observed state. Thread the admitted observation and reference provenance into the package slot supply; finish uses the same defective supply path at lines 2850-2856. | fsl-eval/src/simulation/protocol.rs:2829-2835 |
| FND-003 | medium | Package protocol replay reads the package and replays with ReplayLimits::default(), discarding self.limits.replay. On the existing refuted TSO NoBothZero request, set analyze_under_replay_bytes to package_bytes.len()/2 (or len()-1 if needed to keep the witness envelope below the bound): source read_package refuses the package and settles inconclusive/replay-refused/stage_limit_exceeded, whereas package protocol replay can reproduce and settle refuted under its larger default. Reuse read_package and forward self.limits.replay to the replay entry point. | fsl-analyze/src/analyze/clause_item.rs:1287-1307 |
| FND-004 | medium | The newly enabled package memory route copies memory into subject and starts settlement without request-level memory admission. With the existing SB request and memory [{parallel: Nope::Both, model: tso}], source check_memory refuses the whole request as invalid-request with empty items before any engine runs; package reaches PackageProtocolWork and converts NotAParallel to an item-level declined invalid-value. Duplicate entries and nonparallel targets follow the same missing check. Validate every package memory selection before item resolution or execution, preserving the request-stage pointer and empty items contract. | fsl-command/src/command/analyze.rs:254-275 |
| FND-005 | medium | Package protocol_binding resolves and stores all subject.memory entries, whereas source ProtocolSelection::binding filters entries to the selected protocol. In a valid unit containing SB and Other, with subject SB and memory [{parallel: Other::Both, model: tso}], source admits Other and leaves the SB item identity/verdict unchanged; package resolution passes Other::Both to SB and declines the item as NotAParallel. Filter the selected protocol entries after independently admitting all request entries; test both the verdict and effective subject identity. | fsl-analyze/src/analyze/document/package_run.rs:852-886 |

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
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-940
  related_ids:
  - FR-850
  path: fsl-eval/src/simulation/protocol/program.rs
  lines: '1188'
  excerpt: "The package form SHALL return the value the source\n  form returns for\
    \ the package and the source it was compiled from."
  finding: The package program installs two empty ReaderPositions instead of the real
    binder readers. Consequently reads_from is false for every attempt binder and
    normalize removes its value immediately after the attempt. In the supported sequence
    A=Core::look followed by B=Core::copy constrained by not a, A returns false on
    the existing litmus snapshot, source retains a and proceeds, but package B receives
    Unavailable and stops with BROKEN. Compute reader positions for every admitted
    binder, including readers in later attempts and finish.
- fnd: FND-002
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-940
  related_ids:
  - FR-850
  path: fsl-eval/src/simulation/protocol.rs
  lines: 2829-2835
  excerpt: "The package form SHALL return the value the source\n  form returns for\
    \ the package and the source it was compiled from."
  finding: 'Supported package attempt constraints receive supply_for_slot(..., None),
    and package_binder_values gives reference values no population provenance. For
    an SB attempt constraint k.r1 >= 0, the over binder k is an object reference:
    admit_slot_supply rejects its missing ProtocolReferenceOccurrence before evaluating
    the expression, and the call is mapped to BROKEN. Source evaluates the same constraint
    against its current observed state. Thread the admitted observation and reference
    provenance into the package slot supply; finish uses the same defective supply
    path at lines 2850-2856.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-940-AC-22
  related_ids:
  - FR-850
  path: fsl-analyze/src/analyze/clause_item.rs
  lines: 1287-1307
  excerpt: AC-4's Counter request over source with the items `Bounded` and `AtMostTwo`,
    run with `replay.input_bytes` set below the length of the package's emission (the
    up-front read-back is bounded by `i2.input_bytes` and admits the package), settles
    `AtMostTwo` `inconclusive` with cause `replay_refused` carrying `stage_limit_exceeded`/`input-bytes-exceeded`
    naming `replay.input_bytes`, never `refuted`, never `failed` with `analyze`/`package-emission`,
    and settles `Bounded` `proved` as without the setting; the document's category
    and exit follow its items (category inconclusive, never `internal-failure
  finding: 'Package protocol replay reads the package and replays with ReplayLimits::default(),
    discarding self.limits.replay. On the existing refuted TSO NoBothZero request,
    set analyze_under_replay_bytes to package_bytes.len()/2 (or len()-1 if needed
    to keep the witness envelope below the bound): source read_package refuses the
    package and settles inconclusive/replay-refused/stage_limit_exceeded, whereas
    package protocol replay can reproduce and settle refuted under its larger default.
    Reuse read_package and forward self.limits.replay to the replay entry point.'
- fnd: FND-004
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-1033-AC-4
  related_ids:
  - FR-850
  path: fsl-command/src/command/analyze.rs
  lines: 254-275
  excerpt: A `memory` entry with `model` `pso` refuses the whole request `invalid-request`,
    exit 20, naming `/memory/0/model`; entries naming `SB::Both::left::L::Inner` (nested),
    `SB::Both::Sx` (an attempt) and `Nope::Both` (no such protocol), and two entries
    naming `SB::Both`, each refuse the whole request `invalid-request` with `invalid_runtime_input`/`invalid-value`
    naming the entry, or both entries, exit 20, with no item run.
  finding: 'The newly enabled package memory route copies memory into subject and
    starts settlement without request-level memory admission. With the existing SB
    request and memory [{parallel: Nope::Both, model: tso}], source check_memory refuses
    the whole request as invalid-request with empty items before any engine runs;
    package reaches PackageProtocolWork and converts NotAParallel to an item-level
    declined invalid-value. Duplicate entries and nonparallel targets follow the same
    missing check. Validate every package memory selection before item resolution
    or execution, preserving the request-stage pointer and empty items contract.'
- fnd: FND-005
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-1033
  related_ids:
  - FR-850
  path: fsl-analyze/src/analyze/document/package_run.rs
  lines: 852-886
  excerpt: "An entry naming an outermost `parallel` of a protocol no item is decided\n\
    \  over is admitted and changes no item's identity."
  finding: 'Package protocol_binding resolves and stores all subject.memory entries,
    whereas source ProtocolSelection::binding filters entries to the selected protocol.
    In a valid unit containing SB and Other, with subject SB and memory [{parallel:
    Other::Both, model: tso}], source admits Other and leaves the SB item identity/verdict
    unchanged; package resolution passes Other::Both to SB and declines the item as
    NotAParallel. Filter the selected protocol entries after independently admitting
    all request entries; test both the verdict and effective subject identity.'
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
| FND-001 | fixed | Round 1: Package readers are now built from each admitted slot binder inventory, converted to span positions and finish reachability for every slot. The prior A/B regression remains intact; the new finish regression also reads a previous attempt result. |
| FND-002 | fixed | Round 1: Attempt paths pass the observed pre-state and finish passes state.model. Package-owned state admission uses selected models/types and retains the exact BarePackage owner. The selected binders carry per-reference population provenance; expressions still use the existing meter and byte evaluator, with no CheckedGraph reconstruction. New attempt/finish regressions read current k.r1 after A updates it. |
| FND-003 | fixed | Round 1: Package protocol replay now uses read_package with the caller limits and forwards the same limits into replay. The new byte-bound regression asserts source replay-refused code/cause/setting and complete package item equality. |
| FND-004 | fixed | Round 1: Both routes share check_memory_using before item settlement. The package closure resolves the admitted protocol execution view; invalid/duplicate/nonparallel entries preserve the source request-stage refusal pointers. The new regression asserts empty items, exit20 and identical diagnostics/category. |
| FND-005 | fixed | Round 1: After request-level validation, protocol_binding filters entries by the selected protocol before resolving/storing effective memory. The two-protocol regression compares source baseline, source with Other selection, and package full item identities/results. |

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
  after_excerpt: "        program.reader_positions = program\n            .readers\n\
    \            .iter()\n            .map(|readers| ReaderPositions {\n         \
    \       controls: readers\n                    .iter()\n                    .filter_map(|node|\
    \ program.spans.get(node).map(|span| span.first))\n                    .collect(),\n\
    \                finish: readers.contains(&program.finish.node),\n           \
    \ })\n            .collect();"
  path: fsl-eval/src/simulation/protocol/program.rs
  lines: 1199-1209
  reason: Package readers are now built from each admitted slot binder inventory,
    converted to span positions and finish reachability for every slot. The prior
    A/B regression remains intact; the new finish regression also reads a previous
    attempt result.
- fnd: FND-002
  outcome: fixed
  after_excerpt: "        let observation = self.package_observation(state)?;\n  \
    \      let binders = self.package_binder_values(frame, &observation, slot)?;\n\
    \        let supply = package\n            .supply_for_slot(slot, binders, Some(&observation))\n\
    \            .map_err(|_| broken())?;"
  path: fsl-eval/src/simulation/protocol.rs
  lines: 2834-2841
  reason: Attempt paths pass the observed pre-state and finish passes state.model.
    Package-owned state admission uses selected models/types and retains the exact
    BarePackage owner. The selected binders carry per-reference population provenance;
    expressions still use the existing meter and byte evaluator, with no CheckedGraph
    reconstruction. New attempt/finish regressions read current k.r1 after A updates
    it.
- fnd: FND-003
  outcome: fixed
  after_excerpt: '        let package = read_package(self.package_bytes, &self.limits)?;'
  path: fsl-analyze/src/analyze/clause_item.rs
  lines: 1287-1306
  reason: Package protocol replay now uses read_package with the caller limits and
    forwards the same limits into replay. The new byte-bound regression asserts source
    replay-refused code/cause/setting and complete package item equality.
- fnd: FND-004
  outcome: fixed
  after_excerpt: "    check_memory_using(&memory, |protocol, entries| {\n        use\
    \ fsl_package::AnalysisView;\n        package\n            .protocol_execution_named(protocol)\n\
    \            .ok()\n            .flatten()\n            .map(|form| ResolvedMemory::resolve_package(&form,\
    \ entries))\n    })?;"
  path: fsl-command/src/command/analyze.rs
  lines: 260-267
  reason: Both routes share check_memory_using before item settlement. The package
    closure resolves the admitted protocol execution view; invalid/duplicate/nonparallel
    entries preserve the source request-stage refusal pointers. The new regression
    asserts empty items, exit20 and identical diagnostics/category.
- fnd: FND-005
  outcome: fixed
  after_excerpt: "    let memory: Vec<_> = subject\n        .memory\n        .iter()\n\
    \        .filter(|entry| entry.parallel.split(\"::\").next() == Some(selection.protocol.as_str()))\n\
    \        .cloned()\n        .collect();"
  path: fsl-analyze/src/analyze/document/package_run.rs
  lines: 852-857
  reason: After request-level validation, protocol_binding filters entries by the
    selected protocol before resolving/storing effective memory. The two-protocol
    regression compares source baseline, source with Other selection, and package
    full item identities/results.
```


Mechanical correction within round 1: Draco first attempted the three focused commands at 4109a4e3b1592c393f9e049c98399fd4309305e8; all stopped before tests on E0308 (RelationshipExtents versus CheckedExtent slice). The replacement d633b321c49449d7776d2b356160e4d9fde44afe removes that invalid call. observe_state still admits/validates extents before any closure building (initial.rs:183-196); the adapter resolves each typed relationship endpoint in the current holder catalog and adds both population directions before transitive closure. No validation, cast or source-graph reconstruction was substituted. The replacement passed the authoritative exact-head focused checks recorded above.


Fix-round source/test inventory examined: fsl-analyze/src/analyze/clause_item.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/tests/it/analyze_package_protocol.rs, fsl-command/src/command/analyze.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/simulation/protocol.rs, fsl-eval/src/simulation/protocol/package_program.rs, fsl-eval/src/simulation/protocol/program.rs, fsl-eval/src/simulation/protocol/steps.rs, fsl-eval/src/value/expression/mod.rs, fsl-eval/src/value/mod.rs, fsl-package/src/bare/observation.rs, fsl-package/src/emit/tests/protocol_view.rs, fsl-semantics/src/model/observation.rs, fsl-semantics/src/model/observation/population_document.rs, fsl-semantics/src/model/state.rs. Observation/state additions are governed by FR-940 package/source evaluation equality and FR-106 recursive reference admission; no new standalone requirement or plan is claimed.
