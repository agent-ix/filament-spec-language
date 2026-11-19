---
id: SR-2026111918484900000002
title: "spec-review of QSL-2464 FR-230 place binders"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3072 at a23d0279; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md (guard/invariant binders, Writing selected and family constraints, Family parts, FR-230-AC-55, FR-230-AC-56, FR-230-AC-70), spec/functional/FR-930-scope-an-element-presence-quantifier-in-the-predicate-of-its-timed-clause.md, spec/test-cases/TC-3046-a-family-clock-bound-is-written-with-a-created-binder.md"
review_set: subset
---

## Summary

Spec review of FSL PR #3072. The PR renames timed quantifier binders from "slot parameters" to "place binders" and states that a place binder's name is never in FSpec FR-402's reserved slot form. Ticket: QSL-2464. The review compared the PR with FSpec PR #751 (head ad252f61) and with the FSL checker sources. `quire validate` passes on the changed files with pre-existing warnings only.

## Examined scope

- FR-230 binder prose, AC-55, AC-56 and AC-70 (examined). The rename is consistent within the diff and matches FSpec #751's FR-322 wording ("place binders, the quantified `value`/`parameter` nodes").
- FR-930 and TC-3046 (examined). Consistent.
- Created `<field>#<depth>` names (examined). These are correctly outside the reserved form, because `#` is not an identifier character.
- Ids (examined). No new ids.
- Merge order (context_only). FR-230 cites FR-402's "reserved slot form", which exists only once FSpec #751 merges, so #751 should merge first.

## Verdict

Mergeable after FSpec #751, with one medium finding that may be deferred to a ticket. The rename in the diff is complete and consistent. The new sentence "never in FSpec FR-402's reserved slot form" holds only once the FSL checker refuses reserved source identifiers. No FSL requirement or test covers that refusal, and the checker has no such check today.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230 says a place binder's name, the source binder's, is "never in FSpec FR-402's reserved slot form". That holds only through FSpec FR-402's new checker refusal (`ambiguous_declaration`/`ambiguous-name`), and FSL has no FR, AC or TC for it. fsl-semantics has no reserved-name check, so `forall(slot_sx_sy in self.us: ...)` today emits a place binder that a reader classifies as a slot parameter. Add an FSL requirement and test, or file a ticket | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:648 |
| FND-002 | low | Rename leftovers outside the diff. FR-400:300 says "The slot binders a timed model's producer creates", in the paragraph that separates them from checker slots. TC-685 step 33 and its expected result say "typed slot `binders`". The Rust doc comments in fsl-semantics/src/check/timed_model.rs (881, 1026, 1216) say "slot parameters"; they are not normative and can be fixed when that code is next changed | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:300 |

## Dispositions

Round 1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-488 "Reserved slot names" states the checker refusal of a source identifier in FSpec FR-402's reserved slot form (`ambiguous_declaration`/`ambiguous-name` at the declaring source region, naming the identifier and its slot tag), with FR-488-AC-29 and TC-3082. The code is ticket QSL-2581; no stub test is added. FR-488's note that a temporal clause could declare a slot name is corrected |
| FND-002 | fixed | FR-400 and FR-931 say place binders for the producer-created timed binders, and TC-685 step 33 and its expected result say typed place binders. The Rust doc comments in fsl-semantics are left for QSL-2581 or the next change to that code, as the finding allows |
