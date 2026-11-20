---
id: SR-2026111922300000000001
title: "code-review of QSL-2562 case binder names (FSL #3115, filament-eval #42)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR #3115 at 57c24c69 (fsl-command/src/command/tl.rs, tl/canonical.rs, fsl-semantics/src/check/{check,check/typing,sum_case,facts,lowering,observation}.rs, fsl-package/src/reader_protocol_projection.rs, fsl-replay/src/spine/clause.rs, fsl-replay/src/hybrid_replay/expand.rs, fsl-eval pattern updates, tests/it/request_surface_s1_16.rs, Cargo.toml/lock) and agent-ix/filament-eval PR #42 at e88c319a (crates/filament-checked/src/node.rs and two tests); context: FR-931 Tokens/Parenthesisation/Binders, FR-931-AC-13/14, FR-400-AC-13, FR-397 Operand binders and FR-397-AC-29, FR-065 FR-347 widening, Contract IR FR-347"
review_set: subset
---

## Summary

Code review (rust-style and rust-review lane) of the two QSL-2562 code PRs, read from `gh pr diff` and `git show` on fetched refs. Nothing was built or run. Ticket: QSL-2562.

filament-eval #42 adds the declared names beside every checked binding slot (`CaseArm.names`, `Let.name`, `Query.name`, `Fold.accumulator_name`/`binder_name`, `Seq.name`) and carries them through `map_ref`. filament-spec-language #3115 fills those names in the checker and the package reader, names the checker's operand binder `#ieee_operand`, adds union member names to `TemporalValuation`, rewrites the canonical printer with scoped resolution, and makes union-member construction widen integer arguments.

## Examined scope

- Package identity: `NodeKind` and `CaseArm` derive only `Clone, Debug, PartialEq/Eq`, with no serde and no `Hash`, and no code compares `NodeKind` structurally. The checked-record names therefore never reach the package wire. The reader takes them from the parameter node `name` that the package already carried, as the W82 ruling says. The only identity change is `fresh_slot("#ieee_operand")`: parameter nodes for float-ordering operand binders change from `ieee_operand_<slot>` to `#ieee_operand`. FR-397-AC-29 (#3044, merged) rules this. No committed fixture or golden contains `ieee_operand`, so no golden moves. (examined)
- `fresh_slot`: it pushes the fixed name. The only other reader of `slot_names` by slot is a diagnostic (`EqualityRangeContext`). Nothing looks up a slot by name, so two `#ieee_operand` entries cannot collide. (examined)
- Printer scoping: each binding site pushes a fresh scope, and reads resolve innermost first through the parent chain. Memo and plan are keyed on (node, scope), so sibling arms that reuse a slot print their own names. Arms print in stored order. The live test shows `Empty; Rect; Circle` printing as `Circle; Rect; Empty`, so the checker stores arms in member order. (examined)
- IEEE composition: the matcher follows FR-931's zero-, one- and two-let shapes. It recognises by structure, so author intrinsics also print as a comparison. Two bound operands keep source orientation through scope order. A stray `#ieee_operand` let is refused as malformed. An author's `ieee_operand_0` prints as written. Four unit tests cover this. (examined)
- Unary spacing: it now matches FR-931 (`-x`, `- (a + b)`, `not x`). Comments are `//`, so `--x` re-lexes as two minus signs. (examined)
- `the_checker_names_its_ieee_operand_binders_with_a_hash` is un-ignored. It asserts levels [2,3], [2] and [] and that no name begins with `ieee_operand`, which meets the arch condition. (examined)
- filament-eval #42 against main: main gained only #43 (spec, review and TC files). The `filament-eval` crate does not construct or exhaustively match the changed variants, so there is no conflict. (examined)
- No Python. No compatibility shim: `CanonicalError` is deleted outright and nothing replaces it with an alias. (examined)
- Merge state: #3115 is CONFLICTING against main, which is 24 commits ahead. (examined)

## Verdict

filament-eval #42 is mergeable, with one low finding (FND-008). filament-spec-language #3115 is not mergeable. It conflicts with main (FND-001). After the rebase, its first ignore reason is stale because QSL-2597 (#3100) already removed the field-identity fault (FND-002). All three ignores cite QSL-2562, which is Done, not a live ticket. The union-widening change has no FSL spec owner (FND-003). The printer does not parenthesise a `let`, `case` or `if` under a field read (FND-004). The binder-name design itself is correct: package identity changes only where FR-397-AC-29 rules it, and no golden moves. Merge order: #42 first (squash), then re-pin #3115 to the filament-eval main merge commit, rebase, and merge. Coordinate with the open #3112 (QSL-2566), which touches the same IEEE composition and the same files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | #3115 is CONFLICTING with main (24 commits ahead). Overlapping files: canonical.rs, where #3100 deleted `field_text`/`CanonicalRows.fields`, which the PR still calls in the Field/Attribute arms; spine/clause.rs, where `canonical_fields` was removed; check/typing.rs; lowering.rs; request_surface_s1_16.rs. A rebase is required, and the printer's Field/Attribute arms must take the alias-free names from main | fsl-command/src/command/tl/canonical.rs:522-548, fsl-replay/src/spine/clause.rs:498-560 |
| FND-002 | medium | The three `#[ignore]`s cite QSL-2562, which is Done, not a live ticket. The field-identity reason (`field has a selected exact identity row`) is stale: QSL-2597 / #3100 is merged on main, so `case_binders_print_under_their_declared_names` should run after the rebase. The remaining gaps have no ticket: `tl_lowering` over a selected checked package is refused as not implemented, and a float literal in a temporal clause refuses `invalid_package` at compile. File them and cite them | tests/it/request_surface_s1_16.rs:1006, :1128, :1151 |
| FND-003 | medium | `Callee::Union` now returns the member parameter type from `call_parameter`, so union-member construction widens integer arguments. This is a checker behaviour change outside QSL-2562's scope. FSL FR-065 states the FR-347 widening only for declared function calls. Contract IR FR-347 owns only the reader's union_value payload exactness. No FSL requirement or AC owns the checker widening for union payloads, and no live test covers it (it is reached only through the ignored AC-13 leaf `Shape::Rect(c.versionNumber, 2)`). It changes package bytes for sources with bounded-integer union payloads. Needs a spec-owner ticket: an FR-065 or union FR statement and AC, plus a test | fsl-semantics/src/check/check/typing.rs:557, :567 |
| FND-004 | medium | The Field and Attribute arms print `child(operand).name` with no precedence check. FR-931 Parenthesisation says "a let, if or case that is an operand of any operator or postfix form is parenthesised". Now that `let` and `case` print at precedence 0, `(case x { .. }).f` prints as `case x { .. }.f`, which parses back to a different tree. No test covers it | fsl-command/src/command/tl/canonical.rs:522-548 |
| FND-005 | medium | No passing test covers the package route of FR-931-AC-13 or FR-400-AC-13. Both route stubs are ignored. The reader name assertion in `the_reader_projects_nested_let_binders_into_distinct_slots` is in a fixture the PR says is red on main. The case-arm reader path (`parameter_name` lookup, `reader-union-case-binder-has-name`) has no test. Byte-equality across the source and package routes is unverified | fsl-package/src/reader_protocol_projection.rs:2748-2758, fsl-package/src/emit/tests/protocol_locations.rs:968-1006 |
| FND-006 | medium | `count`/`sum`/`only` queries and `fold`/`reduce` still fall through to `Failure::Fault` ("no canonical printer for this form"). FR-931 "Binders" and "Operator spelling" list them as printed forms, and the ticket says "No feature cut". The PR body says the type-argument rules they need are missing from FR-931. There is no live ticket for the spec rule or the code | fsl-command/src/command/tl/canonical.rs:506-510, :729 |
| FND-007 | low | `Typer::slot_name` returns `unwrap_or_default()`, an empty string, for a slot with no recorded name. The printer would then silently print `let  = ..` or `Circle()`, which violates FR-931's "never generates" rule. A missing name is an internal invariant break and should fault | fsl-semantics/src/check/check.rs:3181-3184 |
| FND-008 | low | `CaseArm.names` is a `Vec<String>` parallel to `binders: Vec<Slot>`. Nothing in the type enforces equal length: the printer checks it at runtime, and the checker and reader build the two vectors in lockstep. A `Vec<(Slot, String)>` or a binder struct would make the invariant structural (rust-style) | filament-eval crates/filament-checked/src/node.rs:243-252 |
| FND-009 | low | #3115 pins filament-checked and filament-eval to the #42 head e88c319a. A squash merge of #42 leaves that commit off main, so #3115 must be re-pinned to the main merge commit before it merges. QSL-2562 is also already marked Done while both code PRs are open | Cargo.toml:25, :28 |
| FND-010 | low | `an_authors_let_prints_under_its_declared_name` is traced `#[trace("TC-3039", "FR-931-AC-13")]` but asserts the FR-931-AC-14 case (an author's `ieee_operand_0` let prints as written); retrace it to FR-931-AC-14 | fsl-command/src/command/tl/canonical.rs:1105-1126 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | Case arms print in stored order; the printer never checks `arm.member` ascending, and the package-route test sorts the arm names before comparing, so FR-931-AC-13's member-declaration order is unasserted on the reader route. Assert unsorted order in the package test (or fault in the printer on out-of-order arms) | fsl-command/src/command/tl/canonical.rs:456-467, fsl-eval/tests/it/package_binder_names.rs:110-111 |
| FND-012 | low | Unit-test comment says "`Z > -A` is the same tree with the operands read the other way", but the block builds the non-strict composition and asserts `<=`; the comment describes a different case than the code tests | fsl-command/src/command/tl/canonical.rs:1038-1049 |
| FND-013 | low | `a_case_or_let_under_a_field_read_is_parenthesised` names `let` but exercises only a `case` base; add a `let` (and `if`) field-base assertion | fsl-command/src/command/tl/canonical.rs:1095-1156 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Rebased; `git merge-tree --write-tree origin/main 7539972` exits 0 with no conflicts; Field/Attribute arms use alias-free checked names |
| FND-002 | still-open | Stale field-identity reason is gone and two ignores cite live Backlog tickets (QSL-2627, QSL-2628), but `case_binders_print_under_their_declared_names` cites QSL-2629, which is Done (spec ruling); the code follow-up is QSL-2632 (Specced). Retarget the ignore to QSL-2632 |
| FND-003 | fixed | `Callee::Union(_) => None` reverts the widening; ruled in QSL-2629 (FR-317-AC-7/8, #3117) with code follow-up QSL-2632 |
| FND-004 | fixed | `postfix_base` parenthesises any base below precedence 9; unit test asserts `(case ix://p/U::B { A: true; B: true; }).f` |
| FND-005 | fixed | New `fsl-eval/tests/it/package_binder_names.rs` drives emit→read→`project_function` and asserts arm, `_` and `let` names including a rename |
| FND-006 | deferred | QSL-2630 (Backlog): FR-931 type-argument rules and canonical printing for count/sum/only/fold |
| FND-007 | fixed | `slot_name` returns `Result` and refuses with internal fault `checked-binding-slot-has-a-declared-name` |
| FND-008 | deferred | Lives in filament-eval (merged as #42 / ad8e55d); not changeable in #3115 |
| FND-009 | fixed | Cargo.toml/lock pin filament-eval main merge commit ad8e55d (on origin/main) |
| FND-010 | fixed | `an_authors_let_prints_under_its_declared_name` now traced `("TC-3039", "FR-931-AC-14")` |
