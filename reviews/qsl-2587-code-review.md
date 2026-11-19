---
id: SR-2026111920260700000001
title: "Code review of QSL-2587 PR #3084 export state-clause lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@48774e5d8bfcd8925f14b8b3871ea9b595b2fe39; git diff origin/main...HEAD (PR 3084); fsl-command/src/command/export/lower.rs, map.rs, export.rs; fsl-inspect/src/render.rs; tests/it/export_state_lowering.rs, export_rulings.rs, export_surface.rs, plan_export_surface.rs; QSL-2587"
review_set: subset
date: 2026-11-19
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1016
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1017
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1035
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: reviews
---

## Summary

Ticket: QSL-2587. This reviews PR #3084 at `48774e5d`. The PR adds `export/lower.rs`, which lowers FSL invariant state clauses over a single-member object population into TLA+ and Lean 4 text. It also adds the FR-865-AC-7 export lines to the text renderer, un-ignores FR-1015-AC-5/6/7 and the four-line render test, and replaces `an_unlowered_invariant_is_a_mapper_failure` with a spine refusal test. The Rust lane (rust-review) is folded into this file.

Evidence:
- A throwaway probe test, added in the review worktree and never pushed, dumped the emitted TLA+ and Lean text for four cases: the counter, the counter with `bound B on Counter::counters [3]`, the counter with its frame moved to `modifies_receiver`, and the precondition-as-obligation case.
- On the PR merged with current `origin/main`, the name-filtered `export_state_lowering`, `export_rulings`, `export_surface` and `plan_export_surface` tests ran with 31 passed, 0 failed and 34 ignored.
- `cargo clippy -p fsl-command -p fsl-inspect` (cap-lints warn) gave no new lints in lower.rs. The only hits are the repo-wide `result_large_err` on the shared `Result` type.

## Verdict

REQUEST CHANGES.

What holds up:
- The TLA+ text is plausibly valid as written. The `VARIABLES`, `TypeOK`, `Init` (a `CASE` function over the snapshot keys), `op_inc` with primed reads, `DOMAIN v' = DOMAIN v`, the per-field frames, `UNCHANGED` for other variables, `TypeOK'`, `Next` with `\E` binders, `Spec == Init /\ [][Next]_vars /\ WF_vars(Next)`, `c_` and `THEOREM Th_` all use only the FSpec FR-816 subset. Bulleted conjunction lists keep each `\A` inside its own bullet.
- The definedness lowering matches FR-812 and FR-006. It conjoins conditions in evaluation order before the value, `narrow` adds `lo <= x /\ x <= hi`, and `and`/`implies` (and `or` with the left side negated) guard the right operand's conditions. The probe shows `(v < 5) => (0 <= v+1 /\ v+1 <= 9)`.
- The Lean helpers are plausibly accepted:
  - `F_key_lt` uses a two-discriminant match with structural recursion.
  - `F_get_`, `F_has_` and `F_ok_` use structural recursion.
  - `F_sorted_` recurses on `rest` from the outer match, which is fine.
  - `s'` is a legal identifier.
  - The output is ASCII.
- Refusals stay honest for supertypes, nullable fields, non-int/bool types, create/delete frames, Boolean parameters, unknown operations and non-invariant clauses. Each is exit 30 and names the construct.
- No `#[allow]`, no panics in production code, and FSpec- trace prefixes are used consistently.
- The PR merges cleanly with main: `git merge-tree` reports no conflict, and #3081 only touched render.rs tests.

What blocks:
- The lowering claims `preserved` while dropping semantics it does not read: population `bound` declarations (FND-001) and domain-package operation requires/ensures (FND-003).
- The Lean receiver-frame conjunct swallows `WF s'` and the remaining frame lines (FND-002).

Not machine-checked:
- No SANY, TLC or Lean run. The one residual Lean doubt is elaboration of `s.v_Counter = [([99], { f_value := 0 })]`, an anonymous structure instance under `=`. Lean's binop elaborator should postpone it and resolve it from the left-hand type, but this is unverified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The population `bound` is silently dropped. `lower.rs` never reads FR-1035/FR-502 bounds, so the counter unit with `bound B on Counter::counters [3]` exports exit 0 in both targets with no `Cardinality`/`length` conjunct and records `Bounded` as `preserved` (probe B). FR-812 says the bound "is part of the population's meaning". For today's per-object subset the invariant's truth does not depend on cardinality, but the text breaks FR-812-AC-6 and FR-1035-AC-1 while claiming exact fidelity. The fix is to refuse (exit 30, naming `bound`) whenever a `bound` names an exported population, until `CheckedGraph::population_bound` lands. | fsl-command/src/command/export/lower.rs:240-257,1102-1126,1320-1383; spec FR-1035; FSpec FR-812 |
| FND-002 | high | In the Lean `op_`, the receiver-only frame line `forall k, F_has k s.v -> Not (k = p_self) -> (get k s').f = (get k s).f /\` is joined to the following lines with ` /\\\n  `. `forall` and `->` extend to the end of the definition, so `WF s'`, every `s'.w = s.w` and every later frame line sit under `k /= p_self`. With a singleton population (the counter snapshot) they become vacuous. The Lean step then admits ill-typed and unframed post-states that TLA+ and FSL exclude, so `Ob_` can be false where FSL holds (probe C). Unguarded frame lines nest the same way and only stay equivalent because `F_has p_self` holds. The fix is to parenthesize each frame conjunct. | fsl-command/src/command/export/lower.rs:1445-1464 |
| FND-003 | high | Domain-package operation `pre`/`post` (the kept `requires`/`ensures` model-operation clauses of operation_clause.rs, FR-103) are never read. `read_models` builds `Operation` from params and frame only, and `op_` conjoins only FSL-unit clauses. An operation with a nonempty `requires` or `ensures` therefore exports without it and the invariant is still recorded `preserved`. The fix is to lower these clauses or refuse when they are nonempty. | fsl-command/src/command/export/lower.rs:195-226,916-944 |
| FND-004 | medium | The name map and `fsl:` comments point at the wrong nodes, and the name map is incomplete. `op_inc` maps to, and is commented with, the object-type node (`OpLow.node = object.node`), whose region is the unit's `model Counter = ...` line (124-164), not the operation. `v_Counter` (both targets), the Lean `F_` helpers and `State` fields have no name-map entry (the TLA+ map is exactly `op_inc`, `c_Bounded`, `Th_Bounded`), and `VARIABLES` has no comment. FSpec FR-814 requires a name map "from each generated name", and FR-814-AC-5 requires `v_Account`/`op_deposit` to read back to the type and operation nodes. | fsl-command/src/command/export/lower.rs:920-924,1184-1189,1543-1575; tests/it/export_state_lowering.rs:84 |
| FND-005 | medium | `a_lowered_text_over_the_byte_limit_is_incomplete` is un-ignored and tagged `FR-1015-AC-7`. AC-7 is "`SlowEnough` under the default floor with output_bytes 1 refuses below-fidelity-floor, exit 21", which the neighbouring `the_floor_is_decided_before_the_text_is_charged` already asserts. This test asserts the counter stopping `incomplete` with exit 22, which is FR-1015-AC-2 / FSpec FR-817-AC-10 (second sentence). | tests/it/export_rulings.rs:182-191 |
| FND-006 | medium | `an_unlowered_state_construct_is_a_mapper_failure` is tagged `FR-1015-AC-5`. AC-5 says `Bounded` is `preserved` and describes the `SlowEnough` causes; it says nothing about a spine mapper failure. The test asserts a different behaviour from the AC it claims. | tests/it/export_state_lowering.rs:183-195 |
| FND-007 | medium | Large parts of the lowering are untested. The tests use only the parameterless counter, so these paths have no coverage: operation parameters (`p_` binders, `lo..hi` domains, Lean `Int` params with range conjuncts), Boolean fields (`= true`, `<->`), `or`/`implies` guard placement, `not`/`negate`, multiple exported objects (`UNCHANGED`, `s'.w = s.w`), `modifies_receiver` frames (where FND-002 hides) and refusal attribution. The counter test is tagged FSpec FR-812-AC-1, but that AC's `deposit(a, n: Int[1, 10])` parameter case is not exercised. | tests/it/export_state_lowering.rs:41-127 |
| FND-008 | medium | The export meter deviates from FR-1015 now that regions exist. "Writing the text charges one `export.work_units` unit for each output region" is not implemented (`assembled` charges only output bytes). The lowering also charges every node it reads, including the clause nodes of every operation and `narrow` targets outside the obligations' dependency closures, which FR-1015 does not count. That changes the AC-11/AC-12 `n(S)` arithmetic. | fsl-command/src/command/export/map.rs:294-308; fsl-command/src/command/export.rs:479-482 |
| FND-009 | low | The deleted `an_unlowered_invariant_is_a_mapper_failure` asserted exit 30 under all three floors, no `export` member, `last_stage` S4 and stage `emit`. Its replacement checks only the default floor, exit 30, `Low`, `model/field` and no file. The floor-independence and FR-286 stage assertions for a mapper failure are lost. | tests/it/export_surface.rs (deleted block); tests/it/export_state_lowering.rs:160-195 |
| FND-010 | low | Refusals are attributed to the wrong place in two ways. A lowering refusal always names the first lowerable obligation (`lowerable.first()`), not the one whose construct failed. `read_models` refuses on any object type in any selected model, including types no obligation reads; for example the spine refuses on an unrelated nullable field. The exit is honest but the attribution can mislead. | fsl-command/src/command/export/map.rs:294-321; fsl-command/src/command/export/lower.rs:166-238 |
| FND-011 | low | Naming deviates from FSpec FR-814. Operation parameters use the prefix `p_`, which FR-814 does not list. Record fields use `f_`, which FR-814 reserves for functions. The Lean text uses `Prod.fst`, which is outside FR-816's Lean subset (`.1`/`.2`). | fsl-command/src/command/export/lower.rs:394,612,1070-1076,1442-1444 |
| FND-012 | low | Rust smells: `int_text(value, target)` ignores `target` (`let _ = target;`); `let binary = \|op\| Some(op)` is a no-op closure; `Show::expr` returns `String::new()` for an unknown object or field index instead of failing, which would silently write empty text. | fsl-command/src/command/export/lower.rs:428,582-589,605-610 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | The fix commit committed `reviews/qsl-2587-code-review-comment.md` and `reviews/qsl-2587-gap-analysis-comment.md`. These are Linear comment bodies with no SpecReview frontmatter, and `quire validate` exits 3 on both. Only the two SR files belong under `reviews/`. | reviews/qsl-2587-code-review-comment.md; reviews/qsl-2587-gap-analysis-comment.md |
| FND-014 | low | `writing_the_regions_charges_work_units` asserts only that the least admitting `export.work_units` is at least the number of `fsl:` comments. The node charges alone may already exceed that count, so the test may pass without the per-definition charge. A stronger assertion is that the least `export.work_units` equals the least `export.nodes` plus the number of definitions written. | tests/it/export_state_lowering.rs:570-588 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | map.rs refuses a `bounded` state clause as `model/bound`, exit 30. The probe and `a_bounded_population_is_a_mapper_failure` confirm this in both targets. |
| FND-002 | fixed | Each Lean `op_` conjunct is parenthesized, and the probe shows `(forall k, ... -> Not (k = p_self) -> ...) /\ (WF s')`. |
| FND-003 | fixed | `read_object` refuses a nonempty `pre`/`post` as `model/operation`. The probe with an ensures clause exits 30. |
| FND-004 | fixed | `v_`/`T_` are in the name map, `op_inc` maps to its `state/operation_anchor` node (region 211-225), and `VARIABLES` has a comment. An operation with no FSL clause still maps to the type node, which is acceptable. |
| FND-005 | fixed | The test is retagged `TC-2865`/`FR-1015-AC-2`. |
| FND-006 | fixed | The FR-1015-AC-5 tag was removed from the spine refusal test. |
| FND-007 | fixed | Tests were added for parameters with a receiver frame, Booleans with not/or, the or-guard, two exported types, bound, contract, lazy type reads and the Lean receiver frame. |
| FND-008 | fixed | The lowering no longer charges nodes outside the closures, and one work unit is charged per written definition. Whether a definition counts as FR-1015's "region" is an open architecture question. |
| FND-009 | fixed | The spine refusal test again covers all three floors, both targets, no `export` member, stage `emit` and `last_stage` S4. |
| FND-010 | fixed | `Refusal.obligation` names the owning obligation, and type bodies are read lazily (`only_the_exported_types_are_read`). |
| FND-011 | deferred | `Prod.fst` is replaced by `.1`. The `p_`/`f_` prefixes remain, because FR-814 lists no prefix for parameters or record fields. That gap needs an FSpec FR-814 decision, not a code change in this PR. |
| FND-012 | fixed | The unused `target` parameter and the `binary` closure are removed, and `Show::expr` returns `Lower<String>`, refusing on a bad index. |
| FND-013 | still-open | New this round. The two `*-comment.md` files under `reviews/` must be deleted before merge (the coordinator has said they will do this). |
| FND-014 | still-open | New this round. The work-units test should pin `least(export.work_units) == least(export.nodes) + definitions`, or something equally exact. |
