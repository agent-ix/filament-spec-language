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
