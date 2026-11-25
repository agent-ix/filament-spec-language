---
id: SR-919264714
title: "code-review of PR 3911 (QSL-3302) hyper red-test fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3911 head 2f5031cd8fdf6c96fd6d345edbd43f1a6946e2be on 22551131a6; fsl-eval/src/simulation/model.rs, fsl-eval/src/smt/hyper.rs, fsl-analyze/src/analyze/{document.rs,document/package_run.rs,hyper_route.rs,package_hyper.rs}, fsl-analyze/src/model_check/hyper.rs, fsl-replay/src/{certificate/product.rs,model_check/existential_settle.rs,model_check/graph_hyper.rs,model_check/hyper_smt_query.rs}, fsl-analyze/tests/it/{hyper_reductions,hyper_replay,hyper_single,hyper_wire,package_hyper_analysis}.rs"
review_set: subset
---

## Summary

Ticket: QSL-3302. Code review with the rust-review lane folded in. The PR claims
seven causes (A to G) for 22 red fsl-analyze hyper tests. Each changed test
expectation was checked against its owning requirement in the FSL spec
(`spec/functional`), each production change against its requirement and against
the other call paths, and the nine other red tests were compared with and
without the production change.

## Verdict

CONDITIONAL. One medium finding (FND-001). The test edits are not weakened
assertions: A (FR-158 `constraint` keyword), B (FR-183 "Undefined evaluation":
one path per trace variable; `variables` are (trace variable, model alias)
pairs, so `V`; FR-176-AC-5 tuple `a = (0, 0)`, `b = (1, 0)`, `G` naming `b`
only), C (FR-169 V-7 "a certificate check a limit stopped", basis
`unavailable`; FR-169-AC-13), E (FR-173 "The request writer SHALL ... add the
deadlock-freedom item of each distinct subject"; the Counter is deadlocked at
`value = 3` without a terminal declaration) and the changed expectation under
G (FR-182 V-8 row and FR-281: `unsupported_projection`/
`unsupported-requested-capability`, no engine run) each state what the
requirement says, with literal values. D: every hyper call path (analyze
`hyper_route`, `package_hyper`, `model_check::hyper` incl. HP-5 single, SMT
`encode_hyper_script`, replay certificate `complete_hyper_scope` and `check`,
`graph_hyper`, `hyper_smt_query`, `existential_settle`) now reads
`hyper_instances`; `hyper_settle` binds the counterexample key directly and
lists nothing, so analyze and replay agree on one instance set. No non-hyper
caller changed (`instances()` is unchanged; state-graph `graph_replay::graph_over`
and temporal callers still use it). FR-172 "Object parameters" says one
instance per key of the parameter's universe, and FR-183-AC-8 refuses only a
key outside the universe, so dropping the held-key rule for hyper is what the
requirement says. F: the package route's derived items mirror
`append_hyper_subject_items` (TemporalSatisfaction hyper claims only, alias
order, skip when a deadlock claim for the model is already written, skip on
`terminal any`, run subject unchanged), so it cannot write a duplicate claim
item for one identity named by two aliases (`deadlock_item` with an empty
options map always returns a claim, so the `written` check catches the second
alias). G: the override applies only to `HyperForm::Other` (HP-4), which was
never decided (it reached an engine refusal `unsupported_construct`), so no
capability that worked is removed. Merge with origin/main b539d7e296 is clean
(git merge-tree) and main touched none of the PR's files since 22551131a6.
Review test run (name-filtered: hyper model_instances derived_request terminal
deadlock, both QSL-3249 skips): 427 passed, 2 failed; both failures are among
the coder's nine pre-existing reds with identical panic text and location.
The nine reds: no file of theirs and no shared helper was edited by the PR,
and their panic messages and locations are byte-identical between the run
with the production change (QSL-3302-analyze-filters-2.log) and without it
(QSL-3302-base-prod-other9.log).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `hyper_instances` lists every universe key of every population that overlaps the parameter type (supertype populations included) with no member record required, so a key of a supertype population that no initial state holds becomes an instance bound as `T` with no evidence it is a `T`; FR-172 resolves `v: V::T` to one population whose element type is `T` and ranges over that parameter's universe | fsl-eval/src/simulation/model.rs:2570-2592 |

## Coverage

- Methods: code-review, rust-review lane (same file). Spec review not run: no spec file changed.
- Examined: FR-158 Inputs; FR-183 "Undefined evaluation"; FR-176-AC-5; FR-169 V-7 row, FR-169-AC-13; FR-172 "Object parameters"; FR-125 "Instancing"; FR-173 request-writer bullet, FR-173-AC-2/3/4/5; FR-124-AC-11/12; FR-182 V-8 row; FR-281 unsupported disposition; FR-183-AC-8; FR-181-AC-7.
- Gates seen: workspace check exit 0 on 2f5031cd8f (coder log); 24 named tests pass (coder log); reviewer run QSL-3302-review-analyze-hyper-2f5031cd8f.log, 427 passed, 2 pre-existing failures.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (739841f338): `hyper_instances` takes the parameter's `PopulationDomain` and lists only the universe keys of the one population `StateModel::resolved_population` finds at that ordinal and canonical member (the same `population_domains` enumeration S3's `populations_overlapping` and the package form use); every hyper path (S3 `model_check::hyper`, `hyper_route`, `package_hyper`, SMT `encode_hyper_script`, certificate `product.rs`, `graph_hyper`, `hyper_smt_query`, `existential_settle`, `hyper_settle`, `check_match_witness`) passes `&parameter.population`. |
