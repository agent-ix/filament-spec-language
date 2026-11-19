---
id: SR-2026111919340478491681404
title: "Gap analysis of QSL-2440 reward clause kind and source-declared rewards (filament-spec-language PR 3082)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@012f72fe4d9d9ee112d0588e5ee52c8cc1091e18; git diff origin/main...HEAD (PR 3082) against FSpec agent-ix/filament-specification@08db8132 FR-322, FR-640, TC-1563 and FSL FR-185, FR-740"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2440. Diff-scoped gap analysis: each acceptance criterion the PR's tests claim with
`#[trace]`, or that the changed code implements, is checked against what the tagged tests
actually assert. Plan completion: not assessed. The full-repo `quoin matrix` pass was not run
(diff-scoped review on a thermally constrained host); the AC-to-test check below was done by hand
against the FSpec text at filament-specification main 08db8132.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FSpec-FR-322-AC-471 is newly tagged on `a_reward_is_emitted_with_its_clause_function` and `the_reader_admits_the_reward_clause_kind_and_refuses_any_other_for_a_reward`, but together they assert only the `clause` binding `"reward"`, a `model` owner kind, and that the reader admits the package and refuses a `precondition` rewrite somehow. Unasserted: the `operation_bodies` row stays `no_body`; the precondition functions read for the operation hold no `reward` function; the owner moved to the object type and to a field member each refuse `missing_declaration`/`missing-selection` at the function's `node_id`; the `precondition` rewrite refuses `invalid_package`/`invalid-value` at `/body/expression`; the `body` rewrite refuses at `/operation_bodies/{i}/body`. The tags claim an AC whose refusal halves have no test | fsl-package/src/emit/tests/probabilistic.rs:1569-1574; fsl-package/src/emit/tests/probabilistic.rs:2141-2193 |
| FND-002 | medium | FSpec-FR-640-AC-7 is tagged on the reader test and on `a_source_declared_reward_is_a_source_owned_declaration_at_ordinal_zero`. Covered: clause `"reward"`, reader admits, the source-object function's declaration `[O, step, reward, cost]` at ordinal 0. Unasserted: no precondition read and `no_body` row for the operation; the refusal code and locus of the `precondition` rewrite (only `!Verified`); a function owned by another operation member refusing rule 4; the source function's declaration rewritten to another reward name (with node path and expression target rewritten to match) refusing rule 4 at `/body/expression` | fsl-package/src/emit/tests/probabilistic.rs:2122-2193 |
| FND-003 | medium | FSpec-FR-640-AC-3 is tagged on `reward_functions_rank_by_reward_name_whatever_the_written_order`, which writes two rewards on one operation in two orders and compares sorted `(name, ordinal)` pairs. AC-3's reward clause states byte-identical packages over three declaration orders of `cost`, `load` (on one operation) and `stable` (on another), ranking by (operation key, name), and that changing `load`'s expression changes the `load` node and its claims but keeps its function's id. The test asserts none of the byte identity, the operation-key rank, the expression-change behaviour, or the FR-640 rule that rewards follow every other clause function (see SR-2026111919340478491681403 FND-002 for the feasible fixture) | fsl-package/src/emit/tests/probabilistic.rs:2095-2120 |

## Scope and evidence

Examined:
- FSpec FR-322-AC-471, FR-640-AC-3, FR-640-AC-7: findings above.
- FSpec FR-640-AC-1 on `a_reward_is_emitted_with_its_clause_function`: the clause assertion now
  reads `"reward"`; the rest of the test is unchanged and still matches AC-1's reward half. Clean.
- FSL FR-740-AC-1 on `a_reward_on_a_source_object_is_emitted` (un-ignored): the source-object
  reward emits and the strict reader admits it. Clean for what it claims (the source-object
  extension of the reward node).
- FSL FR-185-AC-12 on `a_package_stage_refusal_of_a_delay_carries_its_locus` (fsl-command) and on
  `probabilistic_refusals_have_locus` (three cases): the delay fixture still refuses at S4 with a
  locus over the declaration and a message naming `delay` and `step`. The AC's reward clause is
  conditional ("an S4 refusal concerning ... a reward"); no source unit now produces one, so
  dropping the reward case leaves no reachable behaviour untested. Clean.
- Production code against FR-640 "Reward identity" and FR-322 "Source-declared object types"
  item 5: implemented as specified (see the code-review SR).

## Verdict

The production code meets FR-640 and FR-322 for the reward clause kind and source-owned reward
functions. The trace tags overclaim: three FSpec ACs (FR-322-AC-471, FR-640-AC-3, FR-640-AC-7)
are tagged on tests that cover their positive halves only. Either add the missing assertions and
mutation cases (most use the existing emit-and-read harness and the `V2ReadRefusal::Envelope`
pattern), or narrow the tags to what is tested and leave the rest to a named ticket. Not
mergeable as-is on tag accuracy.
