---
id: SR-202611211854273
title: "integrity review of FSL PR3373 QSL-2739 provider contract alignment"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d; FR-1044, FR-530, FR-507, TC-1330 against FSpec #792 5423fcf482"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739. This review checks the four changed files for completeness, consistency and atomicity against each other and against FSpec #792.

Consistent:
- **Route and family.** FR-507's row and family bullet agree with FR-1044 Decisions and FSpec FR-290 rows 154–156 and 255–258.
- **Cycle answer.** FR-1044 Behavior and Outputs, FR-530 Cycles, FR-530-AC-4 and TC-1330 step 4 agree with FSpec FR-489 "Cycles". The answer is given under all four rules, before any premise read, planner or provider run, or request write, whatever the registration. No `request`, no `candidates` and no premise item are written, and a malformed declaration keeps its own refusal.
- **Slot key.** FR-530's slot-key `assumptions` and FR-530-AC-10 agree with FSpec FR-492 "Request" and admission step 5.

Defects: one admission rule with no input that can reach it, and two stale or unordered statements in FR-530.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FSL-side `assumptions` admission has no input that can reach it. FR-1044 Decisions (lines 101–106) and the second sentence of AC-11 say FSL "admits a composition request item only when its `assumptions` is exactly the derived array", and otherwise refuses at `/request/items/<i>/assumptions` with "no provider is called". Yet FSL is the only writer of that member, and it writes the derived array. A caller cannot supply one: AC-4(f) refuses it as an unknown member. FSL has no reader of FR-331 provider requests (`/request/items` appears nowhere in the code), and author note "Wire assumptions admission" concedes the function would have to be built. The refusal's record, stage and exit are also unstated. A test that feeds mutated arrays to a function no production input reaches tests nothing. It is the "new unowned transport/admission" the brief rules out. FSpec FR-331-AC-64 is owned by the reader holding the checked package, here the provider (FSpec TC-986). Fix: remove the FSL-side refusal sentence from Decisions and the mutation sentence from AC-11. Keep the requirement that FSL writes exactly the derived array (AC-1, AC-2, AC-5, AC-11 first sentence), the different-identity statement and AC-4(f). If architecture wants a pre-send self-check over FSL's own bytes, in the FR-940 read-back style, state it as an invariant that settles `failed` `runtime_invariant` and is not an `invalid_runtime_input` admission. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:101-106,404 |
| FND-002 | low | FR-530 "Settlement and outcomes" still lists `fixed-point` as a discharge-record relation ("`identity`, `fairness-implication`, `assumption` or `fixed-point`"). That is stale now that FR-530 Cycles derives no `fixed-point` slot and no cycle is ever discharged. Fix: drop `fixed-point` from the relation list. | spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:157-158 |
| FND-003 | low | FR-530 never orders the cycle answer against the interface mismatch. Cycles says the planner answers "before it reads any slot", and Settlement says "Before the slots, the item settles by FSpec FR-492 'Interface mismatch'". A cyclic declaration that also has an `absent` rely (an `H` outside the cycle that is not declared) could settle either way. FSpec FR-489 gives the cycle answer "before the planner's … settlement runs", and the mismatch is part of that settlement (FR-492 "Decision order"). Fix: state in FR-530 Settlement that the cycle answer comes first and the mismatch runs only on an acyclic declaration. FR-1044 is already ordered, because its cycle answer precedes any provider answer. | spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:128-146 |

## Verdict

**Changes required:** FND-001; FND-002 and FND-003 are low.

Examined and clean:
- **Atomicity.** Each new AC (FR-1044-AC-11..14, FR-530-AC-9/10) is tied to one Behavior block.
- **Steps and tags.** TC-1330 steps 7 and 8 carry the FR-530-AC-9 and AC-10 tags.
- **Ids.** AC ids were kept on rewrite (FR-1044-AC-6/7, FR-530-AC-4/5).
- **Removed text.** The FR-1044 "never refutes" and "assumptions wire open" statements are gone, with no orphan references to them.

## Scope and evidence

Spec-only. `quire validate` exited 0 on the four files. The `/request/items` absence was found with a grep over all `*.rs`, which returned no hit.

- FR-1044 Decisions, Outputs, Behavior, Registered provider, AC-1..AC-14: examined.
- FR-530 Behavior S3, reuse table, Discharge, Cycles, Settlement, AC-1..AC-10: examined.
- FR-507 lines 105 and 132-138; TC-1330 steps 3-8: examined.
- FSpec FR-331 Items 604-630, FR-492 Request/Admission/Slot outcome, FR-489 Cycles: context_only.

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FSL admission function and the AC-11 mutation sentence are removed. FSL is the derived writer only. A caller assumptions is an unknown member (AC-4f, AC-11), and refusal of a non-derived wire array belongs to the provider/reader (FSpec FR-331-AC-64). No /request/items pointer is left in the changed files. |
| FND-002 | fixed | fixed-point is removed from FR-530's discharge-relation list. |
| FND-003 | fixed | FR-530 Settlement opens 'After the cycle answer above, which comes first'. FR-1044 Behavior says the cycle answer comes before any interface-mismatch decision, and the complete slot list stays in slot order. |
