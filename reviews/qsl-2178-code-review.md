---
id: SR-2026112022020326506654
title: "Code review (Rust lane) — filament-spec-language QSL-2178 PR #3136"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3136 at 4f776b5c9a982bf4e2b0821cbc216f5fd20e618c vs origin/main; fsl-semantics/src/check/{check.rs,check/typing.rs,facts.rs,lowering.rs,lowering/model.rs,lowering/timed_clause.rs,mod.rs,timed_model.rs,assemble.rs,hyper_clause.rs,node_key/mod.rs,source_model.rs,type_form.rs}, fsl-semantics/tests/it/clock_places.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-953", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-125", type: references }
---

## Summary

Ticket: QSL-2178. Static review (no cargo build; the lane build slot belongs to the coder) of the full `origin/main...4f776b5c9a` diff, with rust-review and rust-style applied, plus verification of the program lead's earlier review SR-2026112013172122111220107 (head 2a69bd496, FND-001..005) and the three later arch rulings (a) Map slot domain uses the key type, (b) longer ref paths refuse `type-mismatch`, (c) the Option-clock map probe refusal is recorded.

## Verdict

**FAIL.** Two tests changed in the last two commits (`594b6f5bd9`, `4f776b5c9a`) were weakened to match the implementation: FR-1040-AC-2's and FR-1040-AC-4's owner-term assertions were deleted (one of them reverses an earlier review fix). The `ref` path crossing and the `size(o.q)` shortcut re-open the FR-125 function-body field-read hole that arch FND-001 closed, through a different entry. Other findings: the declared-type check still checks only the leaves of any type that holds a clock reference; the checker and lowering each compute clock-place ordinals separately; and the Map key type falls back to `Integer` without saying so. Clean: arch FND-001 (as reported: `is_model_receiver` unchanged from main, `clock_ref_field` set only from `clock_ref`), FND-002 (the whole type is checked again for clock-free types), FND-004 (`path.len() - prefix`), ruling (b) (`ref(self.ts[0])`, `ref(self.r.ts[0].x)`, `ref(self.r.x)` refuse `type-mismatch` at `[0]`), ruling (c) (`an_option_clock_map_value_is_refused_at_intake` records `invalid_model_binding`/`malformed-declaration`, `INVALID_OPTION_INNER`, `TypeDefinition.inner`). The only spec edit is the FR-953-AC-1 row arch allowed. The diff changes no CI workflow.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| check.rs `Scope::with_clock_place_keys`, `check_declared_type`, `place_operand`, `clock_ref`, `clock_collection_field`, `attribute` | examined | S3 place keys, declared-type check, presence owner pivot, place references |
| check/typing.rs `clock_selector_type`, `indexed_clock_place_selector_depth`, `clock_collection_operand`, `Size` arm | examined | selector typing, function-body selector refusal, size of a clock collection |
| facts.rs `discharge_sequence_index`, `PlaceReference` discharge | examined | slot-presence proof for ref selectors |
| lowering.rs `PlaceReference`, `SlotPresence`, `next_operand` Index unwrap, `clock_reference` scalar | examined | S4 lowering |
| lowering/model.rs `clock_map_key_type`; lowering/timed_clause.rs `timed_place_slot` | examined | Map key domain, place lookup |
| timed_model.rs `bit_path`, `slot_presence_path`, `under_ref`/`traversed` exemption, `check_part` NamedScope | examined | clock-read detection |
| mod.rs `clock_place_node_key`, `clock_reference_target` | examined | identities |
| tests/it/clock_places.rs (all changed tests) | examined | oracle strength, trace tags |
| spec/functional/FR-953 AC-1 row | examined | the one authorised spec edit |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Oracle weakened to match the implementation: commit 4f776b5c9a deleted the `filament.op.option.present` template-then-owner assertions and the `owner = record.project(peer)` assertion from `a_state_clause_can_read_clock_presence_through_a_reference` (tagged FR-1040-AC-2, which requires the owner term to be the `peer` field read itself with no `deref`). Now it only asserts that guard part data exists and that no clock is constrained. It would pass if the owner were `deref(self.peer)`, `self`, or no presence read at all. This reverses the fix recorded for FND-002 in the committed reviews/qsl-2178-code-review.md. | fsl-semantics/tests/it/clock_places.rs:2318-2342 |
| FND-002 | high | Oracle weakened in the same way: `a_state_clause_can_read_population_clock_presence` (tagged FR-1040-AC-4, "presence_read nodes whose owner term is the binder c itself") no longer asserts the presence_read, its clock_place template or its `parameter c` owner. Its doc comment still claims "with the binder itself as its owner". | fsl-semantics/tests/it/clock_places.rs:2291-2316 |
| FND-003 | high | The FR-125 function-body field-read rule is bypassed through `ref`. `clock_ref` reads every intermediate field with `attribute(..., clock_ref_field = true)`, which forces `field_reads = true` whatever the family or clause. So `function g using v(o: Reference<Config::Call>): Reference<Clock> pure { ref(o.peer.x) }` checks and lowers a bare data read of `o.peer` through a Reference in a value-family body. That is the TC-1031 N16 / FR-125-AC-17 class that arch FND-001 closed for `o.code`. `clock_collection_operand` likewise admits `size(o.q)` in a function body as a synthetic field read. No test covers either in a function body. | fsl-semantics/src/check/check.rs:5094-5124; fsl-semantics/src/check/check.rs:5186-5193; fsl-semantics/src/check/check.rs:5400; fsl-semantics/src/check/check/typing.rs:1151-1169 |
| FND-004 | medium | `clock_collection_field`'s doc says the synthetic `Field` "never becomes a package value read". As the `size(self.q)` / `size(c.q)` operand, though, it is lowered by the generic `NodeKind::Field` arm into `filament.op.record.project` over a `Reference<Call>` operand (no deref), with the member declaration resolved through `composite_type_node` of a `clock_place` key. No test asserts the term emitted for `size` of a clock collection, so a malformed package term would go unnoticed. | fsl-semantics/src/check/check.rs:5240-5299; fsl-semantics/src/check/lowering.rs:5160-5172 |
| FND-005 | medium | Residual of arch FND-002: `check_declared_type` skips the whole-type `try_check_type` whenever the type contains `Reference<Clock>` anywhere, and checks only the leaves. So `Set<Map<Text, Reference<Clock>>>`, `Map<Map<Text, Integer>, Reference<Clock>>`, or a Map keyed by a record holding a Map are admitted: the leaf checks never see the forbidden container composition (`contains_map`). There is also no regression test for the `Set<Map<Text, Float64>>` refusal that FND-002 restored. | fsl-semantics/src/check/check.rs:2713-2723; fsl-semantics/src/check/check.rs:3645-3712 |
| FND-006 | medium | Two sources of truth for clock-place identity. `with_clock_place_keys` predicts each `clock_place` NodeKey ordinal by counting time sources, models and places. Lowering mints the real nodes through `insert`, in its own loop order, and keeps them in `timed_places`. `PlaceReference` lowering emits the checker's predicted key, not a `timed_places` lookup. A model timed by two sources has its keys overwritten (the last write wins). No test with two or more places pins which `clock_place` a ref names: `timer_sequence_and_map` has `x`, `ts.x` and `m.x`, but the tests assert only `semantic_form() == "clock_place"`. So a ref to `ts[0].x` that lowered against `x` would pass. | fsl-semantics/src/check/check.rs:1655-1693; fsl-semantics/src/check/lowering.rs:5261-5278; fsl-semantics/src/check/lowering/timed_clause.rs:782-795 |
| FND-007 | medium | Ruling (a) is only partly met. `SlotPresence` lowering falls back to `ValueType::Integer` without saying so when `clock_map_key_type` returns `None`, which it does for any key other than Text/String/Integer/Uuid/Timestamp/Resolved (for example an enum or `Int[a,b]` key). A Map clock collection with such a key then gets a `Set<Integer>` domain and an `Integer` binder, which is wrong output, not a refusal. Also, `timed_place_slot` matches `place.owner != owner` exactly while the checker admits by `conforms`, so a conforming subtype owner becomes an `UnresolvedDraft` internal fault. | fsl-semantics/src/check/lowering.rs:6776-6792; fsl-semantics/src/check/lowering/model.rs:787-797; fsl-semantics/src/check/lowering/timed_clause.rs:831-846 |
| FND-008 | medium | The `under_ref` exemption added in 4f776b5c9a covers the whole `Ref` subtree, selector index expressions included, not just the place path. A clock read inside a ref selector, such as `ref(c.q[if c.x >= 1 ms then 0 else 1])`, now skips `clocks.foreign` and is no longer refused `clock-read-outside-constraint`. The exemption should stop at selector operands. No test covers this. | fsl-semantics/src/check/timed_model.rs:2349-2362; fsl-semantics/src/check/timed_model.rs:2545-2551 |
| FND-009 | low | `clock_place_node_key` calls `.expect("the fixed clock-place structural path is valid")`, but `NodeKey::new` also fails with `EmptyPackage` on an empty package name, which is input the caller controls. Sibling constructors (`derived_node_key`, `structural_node_key`) return `Result`. This breaks the rust-style rule against panicking on caller input. | fsl-semantics/src/check/mod.rs:348-358 |
| FND-010 | low | `a_reference_to_clock_is_refused_in_every_type_position` asserts that every position is accepted and lowers to `clock_reference`. The test name states the opposite of its oracle. | fsl-semantics/tests/it/clock_places.rs:3358-3400 |
| FND-011 | low | Formatting churn: the diff re-sorts imports in 2024 `style_edition` order (`{BTreeMap, ..., hash_map}`, `{Class, classify}`) in an edition-2021 crate, and `rustfmt --edition 2021 --check` reports differences in all 11 touched .rs files. origin/main is itself not rustfmt-clean, so confidence that the `cargo fmt --all -- --check` lane is newly broken is low. | fsl-semantics/src/check/lowering.rs:36; fsl-semantics/src/check/check.rs:5968 |

## Prior review (SR-2026112013172122111220107, head 2a69bd496) verified at 4f776b5c9a

| Arch FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `is_model_receiver` is unchanged from main and `attribute`'s new `clock_ref_field` flag is true only in `clock_ref`, so a bare `o.code` in a function body refuses again. A residual path through `ref` and `size` is filed as FND-003 above. |
| FND-002 | fixed | `check_declared_type` again calls `try_check_type` on the whole type for clock-free types (check.rs:3650). The residual for clock-bearing types is filed as FND-005 above. |
| FND-003 | still-open | No test of `ref(c.q[i])` under `i < size(c.q)` with `pick(i: Int[0, 1], k: Text)`. The population case uses `size(c.q) >= 1 implies ref(c.q[0])`, and the unguarded case asserts `any` Presence refusal with no location at `i` (clock_places.rs:3553-3567). |
| FND-004 | fixed | `place_operand` returns `path.len().saturating_sub(prefix)` with the owner `prefixes[prefix]` (check.rs:4842-4857), which is the remaining path length. |
| FND-005 | still-open | AC-3 and AC-6 are tagged on tests that exercise none of their cases, AC-1 lacks the postcondition write and lowering assertions, and AC-7 lacks the parameter-type and ill-typed-body cases. See the gap-analysis artifact. |
