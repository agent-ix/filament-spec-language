---
id: SR-20261120232202000001
title: "code-review of PR 3227 contract-ir pin 3a4bc36d and emitter-versus-reader shape fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3227 head e4987be348d15213e89416fa8a59cf7f39b50934; Cargo.toml, Cargo.lock, fsl-package/src/{bare.rs,bare/carried.rs,bare_world.rs,emit.rs}, fsl-package/src/emit/{identity_rows,monitor_plan}.rs, fsl-package/src/emit/tests/{identity_budget,protocol_compensation,protocol_observe,source_record_rows,source_tuple_rows}.rs, tests/it/{bare_package,request_surface_s1_16}.rs"
review_set: subset
---

## Summary

Ticket: QSL-1880 (PR title also names QSL-2678). Rust code-review with the rust-review lane folded in. The PR moves the filament-contract-model pin from af45a983 to 3a4bc36d, adapts the row structs that became optional, adds hybrid claim name rows, keys the await and compensation binding rows, and corrects three tests. No build or test was run. The diff was read against contract-ir at 3a4bc36d, FSpec origin/main and FSL main 754aece9.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The identity file reader now admits a record member entry whose `type_node` and `value_type` disagree (a type node with a null `value_type`, or no type node with a `value_type`). Neither carried.rs nor contract-ir's `clock_entries_are_well_formed` checks the pair. `BarePackage` then silently drops a data field with a null `value_type` from the record's `FieldDeclaration` list, and keeps a field for a clock entry that carries a stray `value_type`. FR-531 line 350 requires refusing "a `value_type` present without `type_node` or absent with it" and forbids silently omitting a row. | fsl-package/src/bare/carried.rs:264-277; fsl-package/src/bare_world.rs:212-216 |
| FND-002 | medium | The `ids` closure in protocol_observe is no longer used after the structural rewrite. That raises an `unused_variables` warning, which turns `make` lint (`cargo clippy --workspace --all-targets -- -D warnings`) red. | fsl-package/src/emit/tests/protocol_observe.rs:162-167 |

## Verdict

Fix round needed: one high and one medium. Both fixes are small.

FND-001 fix: in `SourceRecordJson::value`, refuse `Malformed` unless the member is exactly a data entry (`type_node` and `value_type` both present, `member_type` null, `collections` empty) or a clock entry (`type_node` and `value_type` both null, `member_type` present). In bare_world.rs, skip only members whose `member_type` is set, and do not use `value_type.clone()?` to drop any member without a value type.

FND-002 fix: delete the `ids` closure.

What holds:

- Pin: only `filament-contract-model` moves, in Cargo.toml:30 and in the one Cargo.lock source line. Cargo.lock has one copy of the package, and its dependency list is unchanged. af45a983 is an ancestor of 3a4bc36d, which is contract-ir origin/main. The range includes b81e4eb2 (the seq binder level), so un-ignoring TC-2601 is legitimate.
- `from_stage_rows` with empty tuple rows fails closed. In contract-ir source_clock_join.rs `join_places`, a `tuple` step whose declared tuple has no row in `SelectedSourceClockRows.tuples` returns `MalformedClockPlace` (`tuple_row(..) else return Err(bad())`). The same happens for a union with no clock payload (`positions.is_empty()`) and a record member with no entry (`clock_entry(..) else return Err(bad())`). So a package with a source tuple clock place cannot be read without its rows. Instead its identity file read is refused, which is the identities/5 `source_tuples` work QSL-2347 already tracks (ignored TC-1331 tests).
- Identity format: the format did not move in this PR. `IDENTITIES_FORMAT` is `identities/4` on main (bare.rs:35), and `tests/it/bare_package.rs` held a stale `/3` expectation. Other formats are still refused by `format == IDENTITIES_FORMAT` (bare.rs:1150). fsl-command consumers pass the bytes to `ReadIdentityRows::read` and need no change.
- Emitter: await and compensation rows go into `keyed.rows` once each, at the node key the emitter resolves. No row is written twice: each selection is consumed by `selected.remove`, node ids are unique, and `rows` is rebuilt from `keyed.rows` after a stable sort on (NodeKey, occurrence). A package with no await, compensation or hybrid claim takes no new branch, so its rows, names and identity are unchanged. The hybrid name rows come from `hybrid_claim_sites`, a list separate from `temporal_clauses`, so no clause gets two names.
- Tests: the compensation locus test still pins the refusal to a compensation node. The observe test compares structural keys and structural bodies, so a wrong binder or level target still changes the compared key and fails. FSpec FR-322-AC-462 ("leaves every `node_id` unchanged") reads as structural identity under FR-322's operand index segments (FR-322 lines 1705-1709 and AC-95).

Outside the diff: map clock places are still spelled `{parent}[key]` with a `ix://filament/native/String` key at fsl-semantics/src/check/lowering/model.rs:302-305 and 229-236. FSpec FR-789 line 79 and contract-ir timed_model.rs:1235 require `<C>[slot]`, and FR-789 lines 67-68 a bare `Text` key. place_rows.rs:122 pins the wrong spelling, and no test sends a map clock place through the strict read. This was already true on main. The PR does not change it, so it is not a finding here.

Not verifiable without a build: that the protocol_observe `structural_value` calls (with no holder row) resolve every node-held id; that the seven remaining failures fail for the reasons given; and that clippy shows no warning beyond FND-002.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At e418f820, `SourceRecordJson::value` (fsl-package/src/bare/carried.rs:259-270) refuses `Malformed` unless a member is exactly a data entry (`type_node` and `value_type`, no `member_type`, empty `collections`) or exactly a clock entry (neither, `member_type` set). bare_world.rs:212-214 filters on `member_type.is_none()`. The remaining `value_type.clone()?` cannot drop a member that was read from a file. tests/it/bare_package.rs:392-417 forges both directions (`value_type` null, `type_node` null) and expects `Malformed{SourceRecords}`. Confirmed: the identity file carries no tuple rows (no `source_tuples` in bare.rs or bare/), and every `UnionClockPayloadJson` field is required with `deny_unknown_fields`, so a union entry cannot be half data and half clock. |
| FND-002 | fixed | The unused `ids` closure is gone from fsl-package/src/emit/tests/protocol_observe.rs. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The map-key fix renders only a `String` key as bare `Text`. An `Integer`, `Boolean`, `UUID` or `Timestamp` key still renders `ix://filament/native/<Name>`. The reader's `key_rendering` (contract-ir c353c3f3 source_rendering.rs:129-138) gives every native scalar key its bare `native_name`, and FSpec FR-322 line 5403 says "`K` a native key by its name". So an `Integer`-keyed map clock place, such as place_rows.rs's `Timers`, is spelled differently from what the reader expects. | fsl-semantics/src/check/lowering/model.rs:237-243 |

FND-003 fix: render every native scalar map key by its bare name (`Text` for `String`, `Integer`, `Boolean`, `UUID`, `Timestamp`), and keep the full rendering only for a non-scalar key such as a native enum.

Map clock places have no strict-read test. That is accepted: no route on main emits a map clock place (map-of-Clock lowering belongs to PR #3136), and the strict read checks every `clock_place` row, so a wrong spelling is refused, never admitted. Nothing can emit one unchecked.

Rulings applied in this round:

- FR-093 line 938 now states ascending node-id order. This matches FR-880-AC-13 and FSpec FR-322 lines 871-874 and 1692.
- The FR-093-AC-42 fixture `P = Decimal[0, 100000; 2, 2; exact]` gives P = [0, 1000.00], which contains R = [0, 100.0] at a higher scale, so the conversion is provable in principle. The test still fails because of a checker defect: `discharge_conversion` (fsl-semantics/src/check/facts/conversions.rs:29-127) tries only the integer interval and a literal against the target. It never tries the operand's declared type R, so a decimal operand typed R into P is refused `Unproved(Range)`. This is outside the PR.

Hybrid interval path: the argument that "the two formulas merge" does not hold under FSpec. FR-322 line 1692 says the ordinal "tells apart nodes that share one `package` and `path`". Line 1696 says "A producer disambiguates other nodes sharing a package and path by distinct ordinals of its choosing; body content is not part of the path". So `always[0,2] p` and `always[0,3] p` would be two nodes, at ordinals 0 and 1. FSpec is inconsistent, though:

- FR-322 line 1691 limits string operands to "a literal's text, an operator or member name", and hybrid-node-vectors.json gives `once [0,2]` the path `["temporal","formula",{operand},"filament.op.temporal.once","temporal_formula",{operand}]` with no interval.
- FR-640 line 340 says "FR-322 keys an expression or `temporal`/`formula` node by its content", and line 346 says a claim's id changes with "a formula and its intervals".
- With an interval-free path, a lone claim's formula stays at {path, 0} whatever its interval, so FR-640 line 346 (and FSL FR-498-AC-7) cannot hold.

The FSL emitter follows FR-640 and FR-498. The change belongs in FSpec: either FR-322 line 1691 admits the canonical interval text as a formula operand and the hybrid vectors are regenerated, or FR-640 drops that claim.
