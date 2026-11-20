---
id: SR-20261120100922
title: "spec review of completion spec batch 5, FSL PR3172 package-route lowering input, named type arguments, source workloads and body-location names"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; FR-931 The lowering input and Type arguments, FR-931-AC-16, FR-931-AC-18, FR-1039 tl_lowering bullet and AC-6/AC-7, FR-185 workloads and AC-14, FR-531 body-location names and AC-40, TC-2601, TC-3019, TC-620, TC-3133; PR3172 0ea505b45fdfad392853ae3ddcb5f2309048b0d1"
review_set: subset
---

## Summary

Ticket: QSL-2627 (batch with QSL-2630, QSL-2463, QSL-2701). One spec-review pass over `git diff origin/main...HEAD`, checked against origin/main code for rulings (c) and (d). There are no high findings. Four medium findings and two low findings need text fixes. None of them lets a label change checking, contradicts merged text or cuts a feature.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-931 "The lowering input", its Dependencies and the FR-1039 tl_lowering bullet cite "FR-940's temporal subject view" and "FR-940's clause-term projection". FR-940 defines neither term. The temporal subject view and `formula_graph()` are FR-515's, and FR-940 calls a term's package read its "reader projection". | spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:140 |
| FND-002 | medium | Ruling (a) asks FR-931 to say that the `alias` label is what the producer wrote and is not evidence of the declaration. The text does not say so. It also says the alias "changes the atom text and nothing else", but FSpec FR-322 refuses a malformed alias at admission. | spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:524 |
| FND-003 | medium | Ruling (d) case not named or tested: a hyper clause, whose name only `claim_names` carries, is covered only as "a claim", and FR-931-AC-18 tests `Inv` only. | spec/functional/FR-931-select-the-tl-correspondence-through-the-tl-lowering-request-member.md:592 |
| FND-004 | medium | FR-185 requires that an operation declared by several unrelated nearest ancestors be "counted once per declaration". FR-185-AC-14 does not test this, and the text never says how `T::op` spells either weight when two ancestors declare `op`. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:165 |
| FND-005 | low | FR-1039-AC-7 gives the unknown-profile locus as "at that law under .../laws" and not as the exact FSpec FR-370 pointer. | spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md:275 |
| FND-006 | low | FR-531 reads like a change log: "It is the source route that changes". | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:278 |

## Scope and evidence

- Ruling (c): the refusal for a trace over `{file}` is not a feature cut. On the source route (`fsl-command/src/command/tl.rs` `selection`), a lowering is an FR-312 `temporal` clause run over the request's trace. `run_temporal` authenticates and admits the trace samples against the admitted package (`authenticate_trace` with `admitted_package`), and on the package route that admission needs the identity witness, which only the handoff directory carries (FR-850 "Decision: the selection names the handoff directory"). Merged FR-1039 already refuses a `clause` run over `{file}` with `requires: directory`, and a lowering's trace run is that clause run. The text's "requires directory" for both forms is parity, and the member runs over `{directory}`.
- Ruling (d): on origin/main the source route finds the clause only among `graph.temporal_clauses()` (`fsl-replay/src/spine/clause/temporal.rs` `run_temporal`). A hyper clause is in `hyper_clauses()`, so the lookup returns `MissingName`, which becomes `RunCause::TlClause`: `missing_declaration`/`missing-name`, stage `request`, exit 20 ("tl_lowering.clause ... names no temporal clause"). The package text refuses the same way, so the code and cause match. FND-003 covers only the naming and the test.
- Ruling (b): confirmed. FR-931-AC-17 keeps structural printing for population forms.
- Ruling (e): confirmed. FR-531 makes the source route adopt `ix://I/<path>`, and FR-531-AC-40 tests it on both routes.
- FR-370: `formula` is the sixth argument, and the profile check runs first with an `unknown_profile`/`unsupported-selection` locus that agrees with FR-1039-AC-7.
- The text agrees with merged FR-1039 (`{file}` refusal for clause runs), FR-850, FR-322 "Temporal clause names" (hyper names live in `claim_names`; body wrappers have no row) and FR-569 (`extends` admits several supertypes).
- Open PR3148 (FR-931-AC-16/17 retags): `git merge-tree` against it is clean, and its `count<ix://I/Small>` test matches AC-16. There is no conflict with origin/main.
- No collisions for FR-931-AC-18, FR-1039-AC-7, FR-185-AC-14 and FR-531-AC-40 on origin/main or in open PRs.
- `quire validate` on the changed files exits 0. FR-1039 lines 108 and 110 carry EARS warnings only.

## Verdict

Mergeable once FND-001 to FND-004 are fixed. None blocks under the batch's high-only rule, but each is a one-line text fix. Replacement text:

FND-001, in the FR-931 table: "formula graph | ... | the `formula_graph()` entries of FR-515's temporal subject view over that node, read through FR-940's `AnalysisView` package form: each `filament.op.temporal.*` application with its operator identity and `temporal_interval` member, each connective and constant, and each leaf". And: "atom texts | ... | printed from each leaf's term read through its reader projection (FR-940 'Package forms of the source-checked evaluators'), by the same printer". Make the same substitution in FR-1039 ("the formula graph from that node's `formula` argument read as FR-515's `formula_graph()`, and the atom texts from the reader projections of its leaves (FR-940)") and in the FR-931 Dependencies entry ("FR-515 (the temporal subject view), FR-940 (the reader projection the package route reads)").

FND-002, replacing "The alias is a label only: ... nothing else.": "The alias is a label only. It is the key the producer wrote for the alias the source named, and it is not evidence that the alias is declared or that it resolves to `declaration`: the reader checks its shape, its owner, its spelling and, for a domain-package alias, its resolution, but nothing else. The package checks and evaluates the application from `declaration` alone, so an admitted alias changes the atom text and nothing else."

FND-003, in "The lowering input": "A name that only a `claim_names` row carries (a state clause, a refinement, a hyper clause or another claim) names no temporal clause and refuses the same way, with the code, cause and exit the source route gives for that name." In FR-931-AC-18, after `Inv`: "and, for a unit that also declares FR-940-AC-16's hyper clause `Equal`, `{\"clause\": \"Equal\"}`".

FND-004, append to FR-185-AC-14: "With `object L { n: Integer; operation a() modifies self.n; }`, `object R { n: Integer; operation a() modifies self.n; }` and `object J extends L, R { }`, `workload W on J` checks with exactly the two weights `L::a` and `R::a`; omitting `R::a` is refused `incomplete-workload` naming `R::a`; writing `J::a` is refused `ambiguous_declaration` naming `J::a`, as the effective view refuses an ambiguous operation (ADR-016 SC-1)." Add matching text to the bullet: "such an operation is weighed under its declaring ancestor's name, and `T::op` naming it is ambiguous".

FND-005: "`unknown_profile`/`unsupported-selection` at `/request/package/semantic_graph/nodes/{n}/body/operation/laws/0/definition`".

FND-006: replace "It is the source route that changes, because" with "The source route takes the bytes' spelling because".
