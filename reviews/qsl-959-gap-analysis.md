---
id: SR-2026111911313600000006
title: "QSL-959 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3034; fsl-analyze/tests/it/model_check.rs, fsl-analyze/tests/it/model_check_input.rs, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/simulation/model/terminal.rs, fsl-eval/src/value/expression/bare.rs, fsl-eval/src/value/expression/mod.rs, fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/assemble/tests.rs, fsl-semantics/src/check/lowering/terminal.rs, fsl-semantics/src/check/terminal.rs, fsl-semantics/tests/it/terminal_declarations.rs; fsl-eval/src/smt/query.rs; fsl-package/src/bare_terminal.rs"
review_set: subset
---
# QSL-959 gap-analysis

## Summary

Ticket: QSL-959. Frozen head fad9bdbad5b41375bfc320676b1ba6939f2eacdf. No source edits, commits, new worktree, duplicate build or aggregate checks. Named semantics terminal_declarations and analyze terminal_when_ runs are owned by the lead and had no completed result at review time. All 11 changed files inspected; propagation also checked through fsl-package bare terminal reader and SMT query encoder. No applicable AssuranceProfile found. No dependency, workflow, coverage-threshold or warning-policy changes.

## Verdict

CONDITIONAL: changed-scope manual review finds a missing regression for heterogeneous population members; the code-review artifact records the resulting implementation defect.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The changed self-type tests exercise no population and multiple types in separate populations, but omit multiple declared member types in one population. The canonical-member implementation survives all these tests despite binding self incorrectly for that real model shape. Add a name-filtered regression using the existing tc465_document_with_tag_type_sharing_a_field_name fixture (model_operations.rs:367), and assert missing_declaration/missing-name at self. | fsl-semantics/tests/it/terminal_declarations.rs:178-209 | correct-requirement-no-evidence |

## Coverage

Plan completion: not assessed

Code-review runs the rust-review lane with rust-style API, implementation, safety, wire and testing rules. Manual changed-scope gap analysis is the explicitly authorized substitute for whole-repository matrix/semantic work; no whole-repository assurance claim is made. Spec-review is inapplicable because no specs or plans changed. Full fmt, clippy, aggregate suites and matrix are omitted under Peter's direct named-test-only instruction. Diff whitespace check passed. Test results remain pending. The per-object classifier correctly preserves false over Undefined and its quantifier tests exercise both source and emitted-package routes. SMT's pre-existing false/Undefined conjunction was also noted as a separate route limitation; the concrete new receiver regression is recorded above.

Examined criteria and test bindings:

```yaml
{
  "scope": [
    {
      "id": "FR-124-AC-5",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In a model with one object type `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1, c2}`), `terminal Done using v on M when { self.value = 3 };` checks to `When`, and a terminal state is intended exactly when both counters hold 3."
    },
    {
      "id": "FR-124-AC-6",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In a model with two object types `Counter` (`value: Int[0, 3]`, population `counters`, universe `{c1}`) and `Gate` (`open: Boolean`, population `gates`, universe `{g1}`), operations `inc` on `Counter` (as `Counter` in AC-1) and `release` on `Gate` (precondition `not self.open`, postcondition `self.open`), `terminal Done using v on M when { forall(c in M::counters : c.value = 3) and forall(g in M::gates : g.open) };` checks to `When`; the terminal state with `c1.value = 3` and `g1.open` is intended, and no terminal state is reachable otherwise."
    },
    {
      "id": "FR-124-AC-7",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In the AC-6 model, `terminal Done using v on M when { self.value = 3 };` refuses `missing_declaration`/`missing-name` at `self`; in a model whose populations cover no object type, `terminal Done using v on M when { self.value = 3 };` refuses the same way and `terminal Done using v on M when { true };` checks to `When`."
    },
    {
      "id": "FR-124-AC-8",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In the AC-5 model with universe `{}`, the single state is terminal and intended under `terminal Done using v on M when { self.value = 3 };`; in the AC-5 model with universe `{c1}`, `terminal Done using v on M when { exists(c in M::counters : c.value = 3) };` is evaluated once at each terminal state: with universe `{c1}` and `c1.value = 3` the terminal state is intended, and with universe `{}` it is false there, so the state is deadlocked."
    },
    {
      "id": "FR-124-AC-9",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In a model with one object type `Counter` covered by two populations, `counters` and `archive`, `terminal Done using v on M when { self.value = 3 };` refuses `ambiguous_declaration`/`ambiguous-name` at `self`, naming `archive` and `counters`; the same predicate written `forall(c in M::counters : c.value = 3)` checks to `When`. In a model with `Counter` in `counters` and `archive` and `Gate` in `gates` (several object types), `self.value = 3` refuses `missing_declaration`/`missing-name` at `self`."
    },
    {
      "id": "FR-124-AC-10",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "In the AC-5 model with no operations and `terminal Done using v on M when { 3 mod (2 - self.value) = 1 or self.value = 3 };` (true at 0 and 3, false at 1, undefined at 2), universe `{c1, c2}`: with `c1.value = 2` and `c2.value = 1` the terminal state is deadlocked (one object false, one `Undefined`); with `c1.value = 2` and `c2.value = 3` it is neither intended nor deadlocked and the deadlock-freedom item settles `refuted` with cause `UndefinedEvaluation`; with `c1.value = 0` and `c2.value = 3` it is intended."
    },
    {
      "id": "FR-391-AC-9",
      "path": "spec/functional/FR-391-name-a-population-in-a-state-clause.md",
      "role": "examined",
      "excerpt": "On the `Counter` unit, `terminal Done using v on M when { forall(c in M::counters : c.v = 2) };` checks to `When` with `counters` as a domain read at `current`, and is evaluated over the snapshot's population as the `allInstances` spelling is; `M::counters` in a record-value invariant still refuses as AC-8 states."
    },
    {
      "id": "FR-124",
      "path": "spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "Where the model's populations cover no object type or several, the checker SHALL refuse `self` in `P` with `missing_declaration`/`missing-name` at `self`."
    }
  ],
  "bindings": [
    {
      "test_id": "terminal_self_requires_one_covered_type_and_refuses_at_self",
      "ac_id": "FR-124-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "terminal_self_refuses_two_covering_populations_but_quantification_checks",
      "ac_id": "FR-124-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "terminal_population_quantifier_checks_with_several_covered_types",
      "ac_id": "FR-124-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "terminal_population_quantifier_checks_with_several_covered_types",
      "ac_id": "FR-391-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_self_ranges_over_every_object_and_is_vacuous_when_empty",
      "ac_id": "FR-124-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_self_ranges_over_every_object_and_is_vacuous_when_empty",
      "ac_id": "FR-124-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_population_predicates_are_read_once_even_without_objects",
      "ac_id": "FR-124-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_population_predicates_are_read_once_even_without_objects",
      "ac_id": "FR-391-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_quantifies_two_populations_at_the_reachable_terminal_state",
      "ac_id": "FR-124-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_false_outweighs_undefined_regardless_of_object_order",
      "ac_id": "FR-124-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_population_and_self_classify_equally_on_both_routes",
      "ac_id": "FR-124-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_population_and_self_classify_equally_on_both_routes",
      "ac_id": "FR-124-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "terminal_when_population_and_self_classify_equally_on_both_routes",
      "ac_id": "FR-391-AC-9",
      "trace": "correct"
    }
  ]
}
```
