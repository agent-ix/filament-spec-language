---
id: SR-2026112005262802
title: "gap-analysis of PR 3145 (QSL-2664, QSL-2559, QSL-2596)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3145 head 86d922cf71c1a7fb476fd269c963e5bb3ef9943b; ticket Done-when vs tests; FR-124-AC-11, FR-104-AC-9, FR-365-AC-7"
review_set: subset
---

## Summary

Ticket: QSL-2664 (with QSL-2559, QSL-2596). Manual acceptance-criteria-to-tests check over the diff; Plan completion: not assessed. quoin matrix was not run (read-only pass, no builds).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSL-2664 Done-when asks for a model-check-route test; the tests cover only the refusal's outcome document and of_import_view, so nothing exercises the analyze_bound wiring | fsl-command/src/command.rs:4098 |
| FND-002 | medium | FR-124-AC-11 classification cases (one identity under two aliases: undefined Done with true/false Stop, swapped placements, Halt any writes no deadlock item, refused PM read) are not tested; QSL-2559 scope beyond the classifier (source-route ordinal from terminal node keys, not alias order) is not in this PR | fsl-analyze/tests/it/terminal_two_models.rs:434 |
| FND-003 | low | New tests carry no trace tags | fsl-command/tests/it/reaches_optional_reference.rs:61 |

## Verdict

The behaviour changes are each covered by at least one focused test, but QSL-2664's model-check-route test and FR-124-AC-11's classification cases are missing. Keep QSL-2559 open (or split) after merge: TerminalModel::with_ordinal still orders source-route declarations by alias (fsl-eval/src/simulation/model/terminal.rs:55-57), against FR-124's terminal-node-key order.

## Scope and evidence

- FR-124-AC-11 (spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md): On the source route and on the package route, with equal results, one model identity under two aliases M and N; Done undefined with Stop true settles proved; with Stop false settles refuted naming Done's terminal key; Halt any writes no deadlock-freedom item; swapping lines leaves every report equal.
- FR-104-AC-9 (spec/functional/FR-104-check-state-clauses.md): reaches(self, target, parent) over the inherited parent: Option<Reference<ConfigVersion>> field checks.
- FR-365-AC-7 (spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md): Reachability is one or more edges over the extent: a chain reaches its far end and not its start.

## Recommendations

- FND-001: drive provider::run (or analyze) over a unit that imports a library package while the SubjectBinding's `packages` omits it, so the strict reader refuses missing_import on the model-check route; assert code, pointer and exit 20. Confirm ModelSystem::new and check_provision do not refuse first.
- FND-002: extend two_aliases_of_one_identity_keep_their_terminals_apart with the AC-11 classification cases on both routes.
- FND-003: reaches_over_an_optional_reference_target_evaluates -> FR-104-AC-9, FR-365-AC-7; two_models_read_their_terminals_in_selection_identity_order_on_both_routes -> add FR-124-AC-11 (TC-519); a_refused_analyze_package_keeps_its_own_code_cause_pointer_and_exit and a_refused_read_carries_the_readers_code_cause_and_pointer -> FR-286 outcome mapping with FR-850-AC-3 by analogy; no AC states the source-route rule, a spec gap for the spec steward.
