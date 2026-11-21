---
id: SR-202611212119190339300000005
title: "gap-analysis \u2014 Source correlation producer and BarePackage getters"
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2271; fsl-package/src/emit.rs; fsl-package/src/emit/monitor_plan.rs;
  fsl-package/src/emit/tests/protocol_controls.rs; fsl-semantics/src/check/lowering.rs;
  fsl-semantics/src/check/lowering/protocol/controls.rs; fsl-semantics/src/check/mod.rs;
  fsl-semantics/src/check/protocol_slots.rs
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language
  type: references
---

## Summary

The bounded gap-analysis pass examined the seven-file Source correlation producer change against the pinned FR-831 contract and native public API. The initial shared model-fixture defect and the subsequent multi-region test-fixture defect are fixed, with all seven unique focused filters qualified by actual receipts. Static inspection found no separate production-code defect.

Ticket: QSL-2271. PR: agent-ix/filament-spec-language#3400.
Initial frozen head: `f02abe23b4d095f506523ed0d2f7e1dfbe78a115`.
Final reviewed head: `c8e7661f3b6abf0c0bf3c10f4536ac2dd5228a8a`.
Accepted base: `21edb2ead1c8b69768beedaab003e077ce4ed042`.
Contract: filament-specification `f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0`.
Native API: filament-contract-ir `96554ea506a19b16ff9863ad3d30066578bc609c`.

All seven changed files were read. This review is restricted to Source producer and BarePackage getter parity. Runtime correlation, Program/POR/replay/guard removal, Recommend execution, and completion of the parent ticket are outside this review. No applicable AssuranceProfile was found among declared profiles. No dependency, workflow, lint or gate was changed.

## Verdict

**PASS (bounded Source producer / BarePackage getters only)** — FND-001 and FND-002 are fixed; all seven unique focused filters have actual passing receipts. The computed matrix, whole-repository assurance, dynamic runtime correlation/Program/POR/replay/guard parity, parent-ticket completion and final current-main all-targets merge gate are not certified by this verdict. The final merge gate remains lead-owned and pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The shared correlation model fixture omits the required sourceEnd role on both relationships, so all six new producer/getter tests fail during model binding before source checking, emission or their assertions. | fsl-package/src/emit/tests/protocol_controls.rs:682 |

## Examined scope

The AC entries are evaluated only for the producer and getter facets identified above. AC-8 is context for genuine Source variants and canonical identity: this review does not certify equality of SourceRef identity, authored byte coordinates or all occurrence provenance across different source texts. AC-12 is context for the native getter refusal test; dynamic consumer execution remains separate. AC-3 and AC-9 reader rejection/dependency isolation portions remain owned by IR. AC-11 compares the same admitted package with/without the member, avoiding claims about equal source bytes across distinct programs.

| Unit | Role | Excerpt |
| --- | --- | --- |
| FSpec-FR-831-AC-1 | examined | A package with no member is admitted and its typed read is `unavailable_correlation_provenance`, whether its controls' `related` arrays are empty or not; the same package carrying `[]` is admitted as a complete empty provenance when every eligible control occurrence has an empty `related`, and refuses at the member (FR-831-AC-5) when any carries a record. |
| FSpec-FR-831-AC-2 | examined | Rows for zero, one and many clauses of one control are admitted: two or more identical authored clauses give two or more rows of one `canonical_index` over one canonical record; repeated aliases, two aliases of one canonical record and an alias unlike the relationship's model name are admitted; reversing the source order of two clauses is admitted with the `source_ordinal` mapping reversed and the same canonical graph. |
| FSpec-FR-831-AC-3 | examined | Every eligible form is admitted: `send`, `receive`, `attempt`, `effect`, `event` and `recommend`; a row naming any other control operator or form refuses `ill_typed`/`operator-ineligible` at `control`. |
| FSpec-FR-831-AC-8 | examined | A producer that lowers one source with an authored clause written once, the same source with the clause repeated, and the same source with the clauses in the opposite order emits the same nodes, canonical `related` triples, node ids, `identityRef`, `source_map` entries and `protocol_locations` rows, and the three packages differ only in the member's rows. |
| FSpec-FR-831-AC-9 | examined | The typed read of a group borrows the whole admitted `source_map` entry of the full key `(control, declaration, occurrence.ordinal)` with every region; a dependency control is read from its own package's member and entry and never from the root's; a row naming a second declaration occurrence of a control that has one (FR-464) refuses `invalid_package`/`invalid-value` at `occurrence`. |
| FSpec-FR-831-AC-11 | examined | The member is a separate top-level member: a package that carries it equals the same package without it, or with other rows, in every other member. |
| FSpec-FR-831-AC-12 | context_only | A consumer that needs the clause order of a control whose canonical `related` is nonempty and holds only an absent member refuses with `unavailable_correlation_provenance` and uses no empty or guessed premises; a control whose `related` is empty needs no clause order and is not refused; a complete table supplies the clauses in source order. |
| FSpec-FR-831-producer-1 | examined | When a producer publishes the member, it SHALL record every authored   correlation clause before deduplication. |
| FSpec-FR-831-producer-2 | examined | When a producer publishes the member, it SHALL record each clause in source   order with the checked `Identifier` spelling of its alias and its actual   canonical position. |

Changed paths:

- `fsl-package/src/emit.rs`
- `fsl-package/src/emit/monitor_plan.rs`
- `fsl-package/src/emit/tests/protocol_controls.rs`
- `fsl-semantics/src/check/lowering.rs`
- `fsl-semantics/src/check/lowering/protocol/controls.rs`
- `fsl-semantics/src/check/mod.rs`
- `fsl-semantics/src/check/protocol_slots.rs`

## Evidence

`CheckedRelated` keeps one row for each checked authored clause, with the exact case-sensitive relationship lookup already performed by `protocol_slots::check`. The shared `related_arguments` path enumerates clauses per control before sorting, retains each original ordinal, and assigns canonical positions while traversing the exact existing sort result before deduplication. Thus aliases resolving to one relationship and trailing duplicates retain separate authored rows without changing canonical triples. `check` attaches positions using protocol plus control plus authored ordinal and refuses missing/excess mappings or inconsistent repeated lowering with typed internal facts.

`protocol_correlation_rows` uses the graph's actual protocol control key and the final producer ID mapping after timed rekey and index writing. It joins to the unique recorded declaration at `ProtocolSite::Control`, converts that recorded origin through `checked_occurrence`, and checks the whole source-map entry under `(control, role, ordinal)` for uniqueness and nonempty regions. It never picks a region or infers an occurrence from source bytes. Rows sort by the required full key and source ordinals increment per control with checked arithmetic. The public IR writer receives `Some(rows)`, including `Some([])` for complete emptiness; native reader coverage, validation and budget authority are reused.

The timed re-encoder preserves complete rows versus unavailable absence. Protocol control IDs are declaration identities, unchanged by the timed-only rewrite. The monitor re-encoder applies the existing ordered `Rekey(changes)` to each row's control then re-sorts by the full row key. Both retain existing sidecar arguments. The accepted base used `encode_checked_package_v2`, whose native implementation passes `None` for requirement_items; the new explicit `None` preserves this actual base behavior. No accepted requirement-items input was removed. No composition QName, declaration, SourceOwner, source-map-coordinate, claim-row or unrelated sidecar hunk changes.

The six new test functions are designed to compile real Source using the existing model fixture, emit and strictly admit it, then read the native capability through BarePackage. At the initial freeze the shared fixture prevented all six paths from reaching those operations (FND-001); the final receipts below confirm the repaired paths execute. Assertions cover all six control forms, exact alias case, all four trailing/alias duplicate rows over one canonical record, repeated aliases selecting distinct canonical records, reversed authored order with equal CheckedNodeProjectionV2 identities, complete empty presence, whole-entry borrowing including two regions, and named unavailable errors only for nonempty related. The existing FR-218 regression now checks the row against the actual emitted declaration occurrence. Three re-encoder presence cases (populated, present empty, absent) exercise both production writers. Their monitor changes list is empty; a nonempty rekey runtime test is not claimed. The same-package with/without-member comparison checks graph, source_map and checked_term_data. No fake AST, internal-logic mock, test skip, placeholder implementation, copied schema or alternate reader was introduced.

## Validation evidence and limits

The lead reports focused `cargo check --locked -p fsl-package --tests -j4` exit 0 at this frozen head, with debug/incremental disabled and an owned dynamic locked target; seven-file rustfmt and diff checks exit 0. This reviewer ran no Cargo, local runtime, no-run build, aggregate gate or remote test dispatch. The lead supplied the actual TitanCode receipt for `simple3400-f02a-c1` at this frozen head: rc 101; 1 passed, 6 failed, 0 ignored. The existing `a_related_clause_carries_its_relationship_and_both_ends_into_the_package` passed. Every new `protocol_correlation_*` function fails through the shared `correlation_document` fixture at reachability.rs:176:10 during binding of alias M to example/config-version: `InvalidModelBinding` / `IntakeMalformedDeclaration`, required `role` absent at `/ir/types/1/relationships/0/sourceEnd` (owns), likewise relationship 1 (other). The fixture visibly omits the required sourceEnd role at lines 682-683 for both owns and other. The accepted intake contract at fsl-semantics/src/model/intake.rs:5030 and the existing relationship fixture at reachability.rs:39-43 confirm sourceEnd requires role; targetEnd role is optional. A one-line sourceEnd role addition can preserve endpoints, multiplicity, direction, origin, aliases and all oracles. None of the six failed cases reaches source checking, emission or its carrier assertions; no producer implementation failure is inferred from this receipt. No final merge compile is asserted. Final current-main validation and matched-head merge remain lead-owned.

Installed tools resolved to `/home/peter/.npm-global/bin/quire` (`quire 0.52.0 (engine 0.62.2)`) and `/home/peter/.npm-global/bin/quoin` (`0.36.8`). The loaded artifact module is `spec-artifacts-process@v0.36.0`; no old fallback was used. The newer installed versions are recorded as measured rather than assuming the older release notice.

## Coverage

Plan completion: not assessed

Reconciliation: computed matrix and whole-repository source inventory not assessed under the explicit bounded dispatch; no engine-backed matrix PASS, full criterion count or whole-repository reverse-gap claim is made. No matrix command failed and no grep fallback was substituted. Historical external gaps were not re-audited. The current review inventories four bounded behaviors: authored ordinal/canonical-position capture, checked-position attachment, final occurrence/carrier emission, and re-encoder carrier retention. All four map to FR-831 producer/carrier obligations. No unowned added behavior, source stub or test placeholder was found in those paths. FND-001 records the initial nonexecutable shared fixture. The final receipts below qualify all seven focused filters after the bounded fixture repairs; trace tags alone were never counted as executed producer proof.

Optional whole-repository semantic gap analysis was not requested and was skipped. The ordinary code-review requirement/test/code inspection above was performed for this bounded change. No spec/plan diff exists, so spec-review and its sub-analyses do not apply. The Rust lane is folded into code-review, with no separate Rust artifact.

## Disposition pass 1 evidence

Current reviewed head: `bd1bd5b53793fb0e9310b8839bb14f491cbacbca`. Read-only diff against the original frozen f02abe23 confirms exactly one changed JSON field: sourceEnd gains `"role": name` for the existing owns/other iteration. The production diff, Source aliases, endpoints, multiplicity 0..2, direction, origin, assertions and re-encoders are unchanged; targetEnd role stays absent as the accepted schema permits. The lead reports focused `cargo check --locked -p fsl-package --tests -j4` exit 0 in 19.50s and rustfmt/diff checks exit 0. No Cargo or runtime was executed by this reviewer. Only the six originally failed `protocol_correlation_` filters are queued for rerun; the original control filter's 1-pass receipt is retained. Runtime results and the final current-main all-targets merge check remain pending.

## Dispositions
| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | D1 static inspection confirms bd1bd5b53793fb0e9310b8839bb14f491cbacbca adds only sourceEnd role=name in the shared owns/other fixture; targetEnd role remains optional. The six failed runtime filters are queued but no result has yet been supplied, so the required runtime proof remains open. |
| FND-001 | still-open | D1 sourceEnd role correction is present at bd1bd5b53793fb0e9310b8839bb14f491cbacbca, but actual simple3400-bd1b-c1 exits 101 with 0 passed, 6 failed, 0 ignored. All six still fail before emission because relationship[0].identity ix://example/config-version/owns does not match the required canonical ix://example/config-version/relationship/<Name>-<verb>-<TargetName> form. The shared fixture remains invalid; complete accepted fixture identities/references must be grounded and corrected, then all six filters must pass. |
| FND-001 | still-open | D2 static inspection at ed165f4362c44096d56e509925d9064a4a7a2602 confirms the complete fixture repair now includes sourceEnd role=name and the accepted package-owned relationship namespace. No production/assertion change exists. Actual six-filter rerun is pending, so FND-001 remains open. |
| FND-001 | fixed | D2 actual simple3400-ed16-c1 at ed165f4362c44096d56e509925d9064a4a7a2602 exits 101 with 5 passed, 1 failed, 0 ignored. All six model intakes now succeed and the other five tests pass, proving the shared fixture role/namespace repair. The sole remaining failure is the distinct multi-region fixture overlap recorded as FND-002. |
| FND-002 | still-open | D2 simple3400-ed16-c1 fails only protocol_correlation_group_retains_whole_source_map_entry: InvalidSourceMap at /source_map/22/regions/1 before the multi-region oracle. The variant appends [start+1,start+2) while retaining original [start,end), violating the accepted same-source nonoverlap invariant. Correct the fixture to two valid disjoint regions, retain the whole-entry assertions, and pass the remaining test. |
| FND-002 | still-open | D3 static inspection at c8e7661f3b6abf0c0bf3c10f4536ac2dd5228a8a verifies the exact original span is partitioned into adjacent [start,start+1) and [start+1,oldend) with the same source and full occurrence key. All strict-admission/pointer/length/all-regions assertions remain. The one remaining actual runtime receipt is pending, so FND-002 remains open. |
| FND-002 | fixed | D3 c8e7661f3b6abf0c0bf3c10f4536ac2dd5228a8a partitions the original range into adjacent nonempty ranges under the same locked source and full declaration occurrence key. Actual TitanCode simple3400-c8e7-c1 returns rc0, 1 passed, 0 failed, 0 ignored for the exact whole-entry test, exercising strict admission, pointer equality, count 2 and all-region equality. FND-002 is fixed. |

## Disposition pass 1 runtime receipt

The subsequent actual lead-supplied receipt `simple3400-bd1b-c1` at `bd1bd5b53793fb0e9310b8839bb14f491cbacbca` exits 101: 0 passed, 6 failed, 0 ignored. Every failed test still stops in the shared fixture before emission. SourceEnd role is fixed; the next first refusal is relationship[0].identity `ix://example/config-version/owns`, which fails the accepted canonical identity form `ix://example/config-version/relationship/<Name>-<verb>-<TargetName>`. This is further evidence that the original shared-fixture finding is unresolved, not a producer implementation failure. The coder must ground the complete fixture identities and corresponding references against accepted constructors/schema/data before the next fixture-only batch. The original related-clause control's independent 1-pass receipt remains retained. No producer/getter runtime green or final gate is asserted.


## Disposition pass 2 evidence

Current reviewed head: `ed165f4362c44096d56e509925d9064a4a7a2602`. The exact diff from bd1bd5b changes only the fixture identity format to `ix://example/config-version/relationship/{name}`, retaining the previous required sourceEnd `role: name`. The cumulative correction against original f02abe23 is exactly these two JSON fields in one shared model helper. All production, Source aliases, endpoints, cardinality 0..2, targetEnd optional role absence, category, direction, origin, assertions and re-encoders are unchanged.

Authoritative clarification of D1's diagnostic wording: accepted `read_relationship` at `fsl-semantics/src/model/intake.rs:4974` requires the package-owned `/relationship/` prefix followed by one nonempty artifact segment with no slash. The `<Name>-<verb>-<TargetName>` text was the diagnostic's descriptive template, not a literal required suffix grammar. D2's `relationship/owns` and `relationship/other` meet the actual rule. The accepted valid fixture in `fsl-analyze/tests/it/related_by.rs` uses `relationship/owns`; existing `lowering/model.rs::export` resolves `M::owns` and `M::other` to these canonical relationship slots without changing Source declarations. D1's historical record above preserves the observed refusal and diagnostic wording; this clarification supersedes any reading that hyphenated suffix structure itself is required.

The lead reports warm focused `cargo check --locked -p fsl-package --tests` exit 0 in 20.79s, plus formatter/diff checks exit 0. The six originally failed `protocol_correlation_` filters are queued at ed165f4; their runtime result is pending. The original related-clause control 1-pass receipt remains retained. No local Cargo/runtime, tracker operation, source edit or final current-main gate was executed by this reviewer.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The multi-region getter test appends a region nested inside the unchanged original region, so strict admission rejects overlapping source-map ranges before the two-region borrowing assertions execute. | fsl-package/src/emit/tests/protocol_controls.rs:1027 |

## Disposition pass 2 runtime receipt

The lead supplied actual `simple3400-ed16-c1` at `ed165f4362c44096d56e509925d9064a4a7a2602`: rc101, 5 passed, 1 failed, 0 ignored. All six now pass shared model intake. Complete passes are all-six-source-forms, aliases/trailing-duplicates, reversed-source-order, complete-empty-source, and missing-member typed unavailable including their integrated re-encoder cases. The original related-clause control's prior 1-pass receipt remains retained, totaling six qualified filters and one remaining. The single-region borrowing assertions run before the remaining failure.

Only `protocol_correlation_group_retains_whole_source_map_entry` fails: strict I2 admission returns `InvalidSourceMap` at `/source_map/22/regions/1`, before the multi-region assertions. The variant clones region 0, leaves its original extent unchanged, then appends a nested same-source interval `[start+1,start+2)`. Accepted native `source_map.rs::validate_source_map_entries` requires locked, nonempty half-open ranges and internally sorts by source/start/end before rejecting overlap when the prior end exceeds the next start. Wire array sorting is not the issue: the unchanged original and inserted nested range necessarily overlap. Both regions must remain within accepted source provenance and disjoint; the existing full-key entry and whole-entry borrowing assertions must remain. FR-831 controls have no SourceOwner, so this fix must not invent one.

FND-001 is fixed by the retained sourceEnd role plus canonical relationship namespace corrections; its original finding text remains untouched. FND-002 records the distinct invalid test variant. No production mapping, provenance getter, admission, runtime engine or guard defect is established by this test fixture failure. No final current-main compile is claimed.

## Disposition pass 3 evidence

Current reviewed head: `c8e7661f3b6abf0c0bf3c10f4536ac2dd5228a8a`. The exact diff against ed165f4 replaces the nested-range construction with a partition of the original range: the first region's end becomes `start + 1`, and the cloned second region's start becomes `start + 1` while its old end is retained. The existing `end >= start + 2` assertion proves both half-open regions are nonempty, their boundary equality proves no overlap, and their union is the exact original span. Cloning retains the locked SourceRef; control, actual declaration occurrence and full source-map entry key remain unchanged. No SourceOwner is fabricated. All strict I2 admission, borrowed pointer equality, two-region count and whole-regions equality assertions remain intact. Only this test fixture changed; production and the other six previously qualified filters are unchanged.

The lead reports warm focused `cargo check --locked -p fsl-package --tests` exit 0 in 23.49s, plus rustfmt/diff checks exit 0. Only the exact remaining whole-entry test is queued at c8e7661. Its runtime receipt is pending. The five new complete passes from ed165f4 and the original related-clause control pass remain retained. This reviewer ran no Cargo/runtime, tracker operation, code edit or final current-main gate.

## Disposition pass 3 final receipt and bounded acceptance

The lead supplied actual TitanCode `simple3400-c8e7-c1` at full head `c8e7661f3b6abf0c0bf3c10f4536ac2dd5228a8a`: rc0, 1 passed, 0 failed, 0 ignored for the exact `protocol_correlation_group_retains_whole_source_map_entry` filter. The repaired variant therefore reaches strict I2 admission and the unchanged borrowed-entry pointer, region-count 2 and full-region-vector assertions. FND-002 is fixed; FND-001 remains fixed by the shared sourceEnd role and relationship namespace corrections. Original finding rows and all earlier disposition history remain preserved.

Actual receipt chain (distinct frozen heads, retained without reinterpreting failures as passes):

| Receipt | Head | Passed | Failed | Ignored | Exit |
| --- | --- | --- | --- | --- | --- |
| simple3400-f02a-c1 | f02abe23 | 1 | 6 | 0 | 101 |
| simple3400-bd1b-c1 | bd1bd5b | 0 | 6 | 0 | 101 |
| simple3400-ed16-c1 | ed165f4 | 5 | 1 | 0 | 101 |
| simple3400-c8e7-c1 | c8e7661 | 1 | 0 | 0 | 0 |

Together, the five passing new filters from ed165f4, the original related-clause control pass from f02abe23, and the remaining whole-entry pass from c8e7661 qualify all seven unique filters. The intervening changes are the reviewed fixture-only corrections, so no unaffected filter was rerun or counted twice. This bounded evidence covers real Source production and strict BarePackage/native getters, not dynamic engine correlation or guard removal. No production defect was inferred from the fixture failures and no such production change was made.

The final SRs were explicitly validated at their actual cache paths using `quire validate --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-correlation-provenance <absolute-SR-path>`: exit 0 for each artifact, with quire 0.52.0 (engine 0.62.2), quoin 0.36.8 and spec-artifacts-process@v0.36.0. Local machine-comment YAML and finding/disposition consistency checks pass. Tracker posting and read-back remain delegated to the lead; this reviewer posted no comments, queried no tracker, and ran no Cargo/runtime.

The lead must copy these exact final SRs into the PR before its final current-main all-targets checks and matched-head merge. That final compile/merge has not run as part of this review and is not claimed complete.
