---
id: SR-202611210022442313144108429
title: "Code and Rust review of QSL-2726 PR #3246 (package behaviours, cross_evaluate and clause over a directory)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@53e0a81714 on main d84ed5685b; PR #3246 diff: fsl-command/src/command/{package_run.rs,tl.rs,tl/package_observed.rs}, fsl-eval/src/simulation/model/input.rs, fsl-eval/src/temporal/{mod.rs,observed.rs}, fsl-package/src/bare_term.rs, fsl-replay/src/spine.rs, fsl-replay/src/spine/clause.rs, fsl-replay/src/spine/clause/{model_trace.rs,temporal.rs}, fsl-replay/src/spine/request.rs, fsl-semantics/src/check/temporal_clause.rs, tests/it/request_surface_s1_16.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: reviews
---

## Summary

Ticket: QSL-2726. Code review with the Rust lane of PR #3246 at head 53e0a81714 (merge base d84ed5685b). The PR adds `cross_evaluate` over a `{directory}` package selection, both the `trace` form (record samples valued through the package's own evaluator) and the `initial` form (behaviour enumeration over the package's model system). It also adds a `clause` run over record samples.

The `initial` form follows the arch ruling. It builds a `ModelSystem` from `ModelCheckInput` of the package and uses the same `behaviour_valuations` enumerator as the source route, so the enumerator is shared, not forked. The bounded-profile horizon and step intervals are now read into the package clause form, so `Low` (`G[0,3]`) enumerates. The trace form does not go through `EvidencePackage::evaluate_temporal_evidence`. It values atoms through `BarePackage`'s `ModelEvaluator::evaluate_holds` and `evaluate_guard` in a new `PackageLetters` letter source. That evaluator is the same one the model system uses, and the result is the same `ObservedValuation` that `cross_evaluate` reads.

The three new tests compare the whole result document between routes, `serde_json::Value` equality plus the exit code. serde_json is built without `preserve_order`, so this is byte equality of the serialized document. Each positive arm fails on the old "not implemented yet" refusal (exit 20 against 0). The `{file}` and `{file, equality}` negatives assert code 20, `invalid-request` and `details` `{"member": ..., "requires": "directory"}`.

Several error paths differ from the source route (FND-001 to FND-003). The tests cannot catch a slot-mapping defect in the lasso letter source (FND-004).

Note: while this review ran, the worktree had uncommitted edits by another session (fsl-replay/src/spine/clause/temporal.rs `run_package_sampled` refused-condition arm, and a new test `a_refused_condition_in_a_clause_run_has_the_same_code_and_cause_on_both_routes`). Those edits were not part of head 53e0a81714. The test and clippy runs below compiled the tree with those edits present.

## Verdict

**CONDITIONAL**: there are no high findings. The main paths match the source route byte for byte. FND-001 to FND-004 are medium: refusal paths that give a different document from the source route, and a lasso oracle that cannot fail on a slot-mapping bug. The coder-reported fsl-analyze failure is pre-existing on main and is not caused by this PR.

## Checks

- `locked-build.sh <target> cargo test -p filament-spec-language -p fsl-analyze --test it -- a_lasso_cross_evaluation_over_a_handoff_directory a_clause_run_over_a_handoff_directory an_initial_cross_evaluation_over_a_handoff_directory a_fairness_constraint_over_a_lasso a_tl_lowering_over_a_package package_holds_equals_source_holds_where_undefined`: exit 101. In the root `it`, 5 passed and 0 failed (the 3 new tests, the refactored fairness test, and the `{file}` lowering test). In fsl-analyze `it`, 1 failed: `package_holds_equals_source_holds_where_undefined` panics at fsl-analyze/tests/it/counter.rs:223 with the S1 diagnostic "`part` is a reserved word". The `part` keyword comes from fsl-cst/src/grammar.rs (`P::PartDeclaration`, added by 8f71444d8a / #3202, an ancestor of d84ed5685b). The PR diff touches neither fsl-cst nor fsl-analyze, so this failure is pre-existing on main and not caused by this PR.
- `locked-build.sh <target> cargo clippy -p fsl-eval -p fsl-package -p fsl-semantics -p fsl-replay -p fsl-command -- -D warnings`: exit 101. fsl-semantics fails with about 600 errors, mostly `result_large_err`, all in lines this PR does not change (for example temporal_clause.rs:1150, 1267, 1367 and 2413, and check/typing.rs). The `-D warnings` gate is red on main before this PR, and the dependent crates were not linted in that run. A `--cap-lints warn` run filtered to the PR's changed lines is recorded under Gate follow-up.
- The three `too_many_arguments` warnings the coder reported at fsl-eval/src/simulation/model/input.rs:1149, 1166 and 2200 are `selected_footprint`, `attempt_footprint_fallback` and `package_footprint`. Those signatures are unchanged and sit at d84ed5685b lines 1110, 1127 and 2161: they are pre-existing, not changed code.
- `rustfmt --edition 2021 --check` on the touched files gives the same diff count at base and at head (main's own import ordering drift). The PR adds no new formatting drift.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `cross_evaluate.initial` over a package maps every non-admission behaviour disposition (an expansion bound reached, a broken invariant, any `replay_refusal`) to `RunCause::TlFault` with Debug-formatted text, an internal failure. For the same disposition the source route's `execute_lowering` returns `output::clause_report(report, &selection)`, so the two routes give different documents and exit codes, against FR-931 "over a `{directory}` selection ... return the source route's record". | fsl-command/src/command/tl/package_observed.rs:247-254; fsl-command/src/command.rs:4128-4141 |
| FND-002 | medium | The `cross_evaluate` trace form over a package refuses a record sample the type admission rejects (for example `level` 12 for `Int[0, 9]`) as `RunCause::TlSubject`, a request-stage refusal. The source route, whose `TlSubject` arm applies only when `behaviours.is_some()`, reports the `Admit` disposition through `output::clause_report`. A valuation stop (a refused or failed atom) becomes `TlFault` "the valuation reads every atom and activation", where the source reports the clause disposition. | fsl-command/src/command/tl/package_observed.rs:162-187; fsl-command/src/command.rs:4135-4141 |
| FND-003 | medium | At head 53e0a81714, `run_package_sampled` turns a refused `holds` condition into `EvaluateFault("package-condition-refused")`, an internal failure, and its doc comment says "the source route reports its record". FR-1039 Outputs require the source route's outcome document. (An uncommitted working-tree edit seen during review replaces this arm; it is not on the PR head.) | fsl-replay/src/spine/clause/temporal.rs:1356-1359, 1430-1437 |
| FND-004 | medium | No lasso in the new tests can tell slots apart: `loop: [L0]`; `prefix: [s1], loop: [s1]`. Every unrolled position therefore maps to an equal sample, and a defect in `PackageLetters::subject`'s lasso wrap (for example `prefix.checked_add(offset)` changed to `Some(offset)`) or in prefix/loop concatenation in `package_observed::trace` would pass all three tests. | fsl-eval/src/temporal/observed.rs:1607-1636; fsl-command/src/command/tl/package_observed.rs:137-152; tests/it/request_surface_s1_16.rs:1369-1370, 1417-1418 |
| FND-005 | low | `PackageLetters` copies `ObservedLetters`' `subject` lasso-wrap and `note` logic, and `PackagePositions` copies `ObservedPositions`, instead of making `ObservedLetters` generic over `&dyn ModelEvaluator` and the atom naming. Two letter sources now drift independently (silent samples, absent valuations, captures). `PackagePositions::Objects` is never constructed: it is dead public API. `evaluate_package_positions` and `package_observed_valuation` build the same struct twice. | fsl-eval/src/temporal/observed.rs:1548-1748 |
| FND-006 | low | `package_observed::behaviours` copies `command.rs` `initial_subject`: the JSON round trip into `SubjectSelection`, `check_universes`, and the `max_steps` conversion. Because the package route runs it at execute time, after the clause is resolved, a request naming a missing clause together with an incomplete-population snapshot refuses `missing-name` on the package route and `incomplete_population` on the source route. | fsl-command/src/command/tl/package_observed.rs:205-234; fsl-command/src/command.rs:3889-3915 |
| FND-007 | low | `package_horizon` re-implements `horizon` over `FormulaNode`, and maps a `u64` overflow of the interval sum to `PackageClauseRefusal::Arena`. The source route reports that overflow as a work-budget refusal (`formula_horizon`), so the same clause refuses differently on the two routes. | fsl-semantics/src/check/temporal_clause.rs:2622-2667, 2408-2420 |
| FND-008 | low | The PR's new functions add 9 `clippy::result_large_err` violations, which the repo's `[lints.clippy] all = "deny"` / `-D warnings` gate rejects: package_run.rs:225, 300, 311; tl/package_observed.rs:71, 108, 168, 193, 205; spine/clause/model_trace.rs:434. They are the only clippy hits on changed lines. Main is already red under the same gate, with about 600 instances of this lint in fsl-semantics, so the PR does not turn a green gate red, but it adds to the debt. | fsl-command/src/command/package_run.rs:225; fsl-command/src/command/tl/package_observed.rs:71; fsl-replay/src/spine/clause/model_trace.rs:434 |

## Gate follow-up

`locked-build.sh <target> cargo clippy -p fsl-eval -p fsl-package -p fsl-semantics -p fsl-replay -p fsl-command -- --cap-lints warn`: exit 0, 881 warnings in total. Filtered to lines changed against d84ed5685b, the only hits are the 9 `result_large_err` warnings in FND-008. No `too_many_arguments`, `len_zero` or other lint class falls on a changed line.
