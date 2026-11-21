---
id: SR-2026112022020329042696
title: "Gap analysis — filament-spec-language QSL-2178 PR #3136"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3136 at 4f776b5c9a982bf4e2b0821cbc216f5fd20e618c; FR-1040-AC-1..7, FR-953-AC-1..2 against fsl-semantics/tests/it/clock_places.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-953", type: references }
---

## Summary

Ticket: QSL-2178. This is a manual check of acceptance criteria against tests for the criteria this PR implements. It reads each tagged test's body against each AC's text. The `quoin`/`quire matrix` commands were not run because the dispatch brief excluded them, so the repository-wide matrix, reverse-gap inventory and stub sweep are incomplete. This is not a full gap audit. Plan completion: not assessed.

## Verdict

**FAIL.** Two FR-1040 criteria (AC-3, AC-6) carry trace tags on tests that exercise none of their cases, which inflates coverage. AC-5's population half is still untested in the form the AC states. AC-1, AC-2, AC-4 and AC-7 are covered only in part. FR-953-AC-1 is covered for the Map form, the Option-clock refusal and, through TC-685 tests, the Sequence form. FR-953-AC-2 has no test.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-1040-AC-1 | examined | ref(self.x)/ref(self.q[0]) in a postcondition write and precondition check as PlaceReference typed Reference<Clock>, lower to clock_reference.ref with result_type clock_reference, no unsupported-feature |
| FR-1040-AC-2 | examined | ref(self.peer.x), ref(self.peer.ts[0].x) owner term is the peer field read, no deref; Option<Reference<Call>> refuses type-mismatch; ref(value(self.peer).x) under present checks |
| FR-1040-AC-3 | examined | refinement row only(... ref(c.x)) and count(... ref(c.x) = self.r) check, lower with binder owner, evaluate per FSpec FR-829-AC-2; same count in a state invariant |
| FR-1040-AC-4 | examined | count/sum/all/exists presence forms lower to presence_read with binder owner and FR-772 slot domain at c; evaluate per FSpec FR-829-AC-3 |
| FR-1040-AC-5 | examined | ref(self.v) checks; ref(c.q[i]) under i < size(c.q) checks; without the guard refuses unproved-presence at i |
| FR-1040-AC-6 | examined | sum/only c.x clock-read-outside-constraint; sum ref(c.x) and sum M::calls type-mismatch; ref(c.m[k].x) type-mismatch; ref(c.x) < ref(c.y) operator-ineligible; late(c.x) clock-read-outside-constraint |
| FR-1040-AC-7 | examined | same(a, b) parameters typed clock_reference; call lowers to that node; ill-typed body refuses once at body not call |
| FR-953-AC-1 | examined | option.slots domain node, binder type, Map<Text, Clock> Set<Text> domain with Text binder, Option<Clock> map value refused at TypeDefinition.inner |
| FR-953-AC-2 | examined | forall/exists evaluation at size 0 and size 2 with slot 1 absent; two quantifiers over self.q and one over r hold exactly two slot-domain nodes |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1040-AC-3 is tagged on `a_ref_sequence_selector_requires_a_presence_proof`, which contains no refinement row, no `only(... ref(c.x))`, no `count(... where ref(c.x) = self.r)`, no evaluation and no state-invariant form. The tag is wrong and the AC has no test. | fsl-semantics/tests/it/clock_places.rs:3527-3600 |
| FND-002 | high | FR-1040-AC-6 is tagged on `a_selector_outside_a_ref_is_not_checked_yet`, which tests unchecked selectors. None of AC-6's six negative cases (`sum`/`only` over `c.x`, `sum` of `ref(c.x)` or `M::calls`, `ref(c.m[k].x)`, `ref(c.x) < ref(c.y)`, `late(c.x)`) appears in any test. The tag is wrong and the AC has no test. | fsl-semantics/tests/it/clock_places.rs:3735-3906 |
| FND-003 | high | FR-1040-AC-5's population half is still untested as stated (arch FND-003). There is no `pick(i: Int[0, 1], k: Text)` fixture and no `count(c in M::calls where i < size(c.q) and ref(c.q[i]) = self.r) >= 0`. The test substitutes a constant index under `size(c.q) >= 1`, and its unguarded negative asserts only that some Presence refusal exists, never the location `i`. The positive relational case (`i < size(c.q)` proving slot `i`) is the one most likely to fail under the interval discharge in facts.rs. | fsl-semantics/tests/it/clock_places.rs:3553-3567; fsl-semantics/src/check/facts.rs:1525-1548 |
| FND-004 | medium | FR-1040-AC-1, -2 and -7 are covered only in part. AC-1 lacks the postcondition write `self.r = ref(self.x)` and any check that the `clock_reference.ref` application's `result_type` is the `clock_reference` scalar. AC-2 lacks `ref(self.peer.ts[0].x)`, the `Option<Reference<Call>>` `type-mismatch` refusal and `ref(value(self.peer).x)` under `present(self.peer)`. AC-7 lacks the assertion that the function node's parameters are typed by the clock_reference node, the call lowering to that node, and the refuse-once-at-the-body case. | fsl-semantics/tests/it/clock_places.rs:3239-3265; fsl-semantics/tests/it/clock_places.rs:3269-3356 |
| FND-005 | medium | FR-1040-AC-4 is covered for `exists(c in calls : present(c.v))` with the binder owner (time invariant) and for `all(c ... exists(e in c.q ...))`. The `count` form's owner assertion was removed (code-review FND-002), and no test covers the `sum(... if present(c.v) then 1 else 0)` form, the `exists(c ... forall(e in c.q ...))` form, or the FSpec FR-829-AC-3 evaluation. | fsl-semantics/tests/it/clock_places.rs:2226-2316 |
| FND-006 | medium | FR-953-AC-2 (snapshot evaluation of `forall`/`exists` at size 0 and at size 2 with slot 1 absent, and exactly two slot-domain nodes for two quantifiers over `self.q` and one over `r`) has no test in the PR. | spec/functional/FR-953-emit-a-presence-quantifier-over-the-slot-domain-of-its-clock-collection.md:68 |

## Coverage

- Reconciliation: manual. Each AC's text was compared with the body of every test whose `#[trace]` names it. `quoin`/`quire matrix` were not run (excluded by the dispatch brief).
- Trace-tag form: every new tag uses bare FSL ids or the `FSpec-` prefix as AGENTS.md "Trace tag ids" requires. No malformed tag was found.
- Bindings checked: `a_ref_crosses_an_object_reference_field` to FR-1040-AC-2 is correct but partial. `a_state_clause_can_read_clock_presence_through_a_reference` to FR-1040-AC-2 is correct but its oracle was weakened. `a_population_clock_presence_read_uses_the_visited_reference` to FR-1040-AC-4 is correct. `a_state_clause_can_read_population_clock_presence` to FR-1040-AC-4 is correct but its oracle was weakened. `a_ref_sequence_selector_requires_a_presence_proof` is correct for AC-5 (partial) and AC-1 (partial) and wrong for AC-3. `a_selector_outside_a_ref_is_not_checked_yet` to FR-1040-AC-6 is wrong. `clock_references_are_function_values` to FR-1040-AC-7 is correct but partial. `quantified_presence_over_a_clock_map_uses_its_key_domain` and `an_option_clock_map_value_is_refused_at_intake` to FR-953-AC-1 are correct.
- Untraced behaviours / stubs: not inventoried repository-wide. No `todo!()`/`unimplemented!()` was added in the changed paths.
- Semantic review: skipped.
- Plan completion: not assessed.
