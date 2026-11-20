---
id: SR-202611200234060
title: "code-review (rust-review lane) of PR3129 temporal and fairness export lowering, slice 1"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-command/src/command/export/lower/temporal.rs, fsl-command/src/command/export/lower.rs, fsl-command/src/command/export/map.rs, fsl-command/src/command/export.rs, tests/it/export_temporal_lowering.rs; PR3129 head f977c3a4d876162f4099033d659b2e278357b7e3"
review_set: subset
---

## Summary

Ticket: QSL-2647. Slice 1 lowers infinite-trace temporal clauses and fairness to TLA+ and Lean. Rulings 1-4 of the 2026-11-19 arch comment (direct `<>[]P`, constant identity universe, nondeterministic monitor, bounded past history encoding) are owed by the fix round and are not reported as missing. Protocol and timed subjects are slice 2. The review checked the emitted text against FSpec FR-813, FR-811, FR-091, FR-092 and FR-367.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Unbounded and `[a,*]` past operators read pre-origin positions in both targets, so `historically holds(p)` is constantly false and `once not holds(p)` constantly true; FR-092/FR-367 restrict them to positions 0..i-a | fsl-command/src/command/export/lower/temporal.rs:314-346, 1038-1060 |
| FND-002 | high | TLA+ step counter does not advance at FSL terminal stutter (Next is disabled at a terminal state) while Lean's Behaviour counts those positions; interval clauses then differ between targets and TLA+ can prove a false `always[a,b]` under a `preserved` record | fsl-command/src/command/export/lower/temporal.rs:813-819, 896-905 |
| FND-003 | medium | Ruling 2 must also cover the monitor, history and accept domains and the `each` fairness quantifier, which all range over the state-dependent `DOMAIN v` | fsl-command/src/command/export/lower/temporal.rs:756, 782, 853-857, 928, 936, 950 |
| FND-004 | medium | `always[0,3]` emits `n_T \in 0..4`; FR-813-AC-3 bounds the counter by 3, and the AC-3 stutter-insertion verdict has no test | fsl-command/src/command/export/lower/temporal.rs:739-746 |
| FND-005 | medium | Admission refusal is flattened into reason text of ExportMapper (runtime_invariant, exit 30); `resource_exhausted` and `missing_import` lose their catalogued code; no test | fsl-command/src/command/export.rs:427-430, 626-642 |
| FND-006 | medium | Non-origin activations and captures, window aggregates and the other step-counting profiles exit 30 and are in neither slice 1, the rulings nor slice 2 | fsl-command/src/command/export/lower/temporal.rs:158-163, 195-203 |
| FND-007 | medium | No golden for historically, since, triggered, bounded until or release, `[a,*]` intervals or non-each weak fairness; FND-001 is unguarded | tests/it/export_temporal_lowering.rs |
| FND-008 | low | once and release goldens are tagged FR-813-AC-1 and AC-2, whose text describes other clauses | tests/it/export_temporal_lowering.rs:271, 317 |
| FND-009 | low | `TypeOK_T` precedes the auxiliary assignments in `Init_T`, so TLC enumerates every monitor function before filtering | fsl-command/src/command/export/lower/temporal.rs:882-889 |
| FND-010 | low | Monitor dispatch by `&'static str` operator names, and a silent `Tf::Hist => False` arm in the Lean writer | fsl-command/src/command/export/lower/temporal.rs:455, 629-662, 997 |

## Verdict

Not mergeable as slice 1 until FND-001 and FND-002 are fixed. Verified correct: future operator meanings including the FR-091 `[a,d)` until convention in both targets; strong until monitor as `[]<>accept` (absorbing `acc`, `run` forever rejects); release and bounded always as `[]~rej` with the release check ordered reject-before-release; bounded counters saturate at the window end without truncating any bound; history update order (initial value from the origin state, `h'` from primed atoms, histories assigned before the monitor that reads them); since and triggered update recurrences; closed-interval pre-origin reading in Lean; WF/SF premise shapes in Lean and `SF_vars`/`WF_vars` with existential argument choice and `each` as `\A p_self`; Lean `holds_` is non-recursive and needs no termination argument; no `theorem` or `sorry` in Lean; no Python, no pin changes, no ignored tests; merges cleanly with origin/main.

## New findings (disposition pass 1)

Head reviewed: cac6a55f906c16dd8f2f697b071e40b6d64cc235, against filament-specification main 73a169ff (FR-813 AC-1..28, FR-814, FR-817) and filament-spec-language main (FR-1015).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | Nondeterministic monitor theorem is unsound. Mon_T enforces the tableau constraints, so a behaviour with no consistent guess run stops stepping and stutters forever. A clause with no eventuality gets `accept_T == TRUE`. `always (holds(c.value = 1) implies eventually[1, 1] holds(c.value = 3))` therefore exports a theorem TLC reports as holding, although FSL refutes it at position 1. | fsl-command/src/command/export/lower/temporal/tla.rs:1204-1277, 1013-1020 |
| FND-012 | high | The code never reads a protocol's activation relation. A clause that a protocol requires on an `origin` row is exported over the model subject and recorded `preserved`. FR-813 Binding, FR-817-AC-16 and FR-1015-AC-15 require the typed refusal that names the protocol clause node. | fsl-command/src/command/export/lower/temporal.rs:200-253; fsl-command/src/command/export/map.rs |
| FND-013 | high | FR-812 definedness conditions (for example the range check of `numeric.narrow`) are conjoined into each atom through `Low::clause()`, not into `ok_<clause>`. An undefined read under negation makes the clause true where FSL refutes with `undefined-evaluation`. | fsl-command/src/command/export/lower.rs:360-364, 1136-1142; fsl-command/src/command/export/lower/temporal/tla.rs:1028-1037; fsl-command/src/command/export/lower/temporal.rs:461-472 |
| FND-014 | medium | Profile and activation refusals name the clause node (`temporal/temporal_clause`), and the test asserts that. `defer()` labels such as `temporal/formula` and `temporal/fairness-target` are not a node's `node_tag`/`semantic_form`. FR-813 "Constructs not lowered" and FR-817 require the named sub-construct's node. | fsl-command/src/command/export/lower/temporal.rs:208-222; fsl-command/src/command/export/lower/temporal/tla.rs:252, 369, 463, 1325; tests/it/export_temporal_lowering.rs:1862 |
| FND-015 | medium | Refusals raised in the TLA+ stage carry no obligation, so map.rs gives them to the first lowerable obligation and its root node. In a request with several obligations the refusal names the wrong obligation and node. | fsl-command/src/command/export/lower/temporal/tla.rs:893-926; fsl-command/src/command/export/map.rs |
| FND-016 | medium | The `Init_T` of the nondeterministic monitor reads `m_T_g0[o_c]` before the `TypeOK_T` conjunct binds it, so TLC stops with an undefined-variable error on that text. | fsl-command/src/command/export/lower/temporal/tla.rs:1058-1065, 1214-1218 |
| FND-017 | medium | `historically` and `triggered` are emitted as duals (Init `~φ`, read `~h`). FR-813-AC-23 requires the native form `m_T_h0 = [o_c \in Ids_Counter \|-> A1(o_c)]`, and the AC-23 goldens assert the dual. | fsl-command/src/command/export/lower/temporal/tla.rs:50-73; tests/it/export_temporal_lowering.rs:954-985 |
| FND-018 | medium | Five spec-covered forms that no "Constructs not lowered" row lists are refused: an Int-range binder (FR-813-AC-7 `\A o_n \in 0..2`), a future operator inside a past operand (FR-813 sends it to the monitor rules), fairness on another object type's operation, nested intervals wider than 32, and more than 128 guesses. | fsl-command/src/command/export/lower.rs:633-640; fsl-command/src/command/export/lower/temporal/tla.rs:248-257, 365-373, 462-467, 1324-1329 |
| FND-019 | medium | Every parameter node spelled `self` becomes `r_self`, so a declared operation parameter spelled `self` collides with the receiver. FR-814-AC-8 requires `p_self`. | fsl-command/src/command/export/lower.rs:439-445 |
| FND-020 | medium | Lean `fair_` names `Ids_<T>` of the operation's object type, but `Ids_` is defined only for clause subjects. Fairness on another type's operation therefore gives an unbound Lean name under a `preserved` record; TLA+ refuses the same case. | fsl-command/src/command/export/lower/temporal.rs:427-431, 449-460 |
| FND-021 | medium | Untested: every typed refusal except the profile one; `since[a,b]`'s two registers (AC-10); AC-6's since example; AC-11; AC-23's `historically[1,*]` and `[1,1]` cases; the AC-3 stutter verdict, which is asserted on the text only. | tests/it/export_temporal_lowering.rs |
| FND-022 | low | `defer_at(graph.position(node.node).unwrap_or_default(), ...)` names node 0 when the position is missing. Each-fairness ranges over `Ids_<T>` where AC-22 says `Keys_<T>` when that is declared. Conditions are added `bounded-identity-universe` first. | fsl-command/src/command/export/lower/temporal.rs:136, 173; fsl-command/src/command/export/lower/temporal/tla.rs:905-915, 1338 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |
