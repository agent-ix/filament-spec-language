---
id: SR-202611210022442313144108503
title: "gap-analysis of QSL-2737 model-unit inheritance and member narrowing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-semantics/src/model/intake.rs, fsl-semantics/src/model/intake/model_unit.rs, fsl-semantics/src/model/normalize.rs, fsl-semantics/tests/it/model_unit_inheritance.rs; FR-710, FR-081, FR-082"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: references
---

## Summary

Ticket: QSL-2737. This is a planless, PR-scoped gap analysis of frozen PR #3277 at a12a7f6e1adc4fa4757a8fecec99a35cfffc8d20 against FR-710, FR-081 and FR-082. Every touched criterion is tagged. FR-710-AC-3/AC-4 and FR-082-AC-1/AC-2 gain the 11 new inheritance integration tests and the 2 new table unit tests as binders. The bound tests make real assertions over the compiled document, the read records, typed refusal payloads and source spans. Three requirement branches the change introduces have no test: same-owner operation redefinition, a wrong-meaning supertype written with the new `extends` syntax, and the inheritance clauses on non-object declarations.

## Verdict

**CONDITIONAL** — medium findings only; no touched criterion is untagged. This is a PR-scoped claim, not a repository-wide PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers FR-082's proper-ancestor rule for operation redefinition through the new suppression row. The table negatives cover a missing, field-kind or record-meaning target but not an existing same-owner or self target, and no integration test redefines `Child::x` from `Child`. This is why code-review FND-001 is undetected. | fsl-semantics/src/model/intake.rs:7715 |
| FND-002 | medium | FR-710 Refusals ("a supertypes entry of another meaning: the code and cause FR-154 gives ... at the declaration's span") has no test for the new `extends` syntax: no object extending a record, machine or process. The only `extends` refusals tested are missing names. | fsl-semantics/tests/it/model_unit_inheritance.rs:220 |
| FND-003 | medium | No test exercises `extends`, `subsets` or `redefines` on record, event, interface or store declarations, all of which reach the changed `read_object`. That untested branch is where code-review FND-002 silently drops or leaves unchecked the clauses. | fsl-semantics/src/model/intake/model_unit.rs:1237 |

## Coverage

Plan completion: not assessed

Reconciliation used `quire matrix --scope . --format json` (/home/peter/.npm-global/bin/quire, quire 0.52.0, engine 0.62.2), static with no run evidence. It completed and emitted repository-wide symbol-id-collision diagnostics outside this diff. FR-710 AC-1..AC-10: all tagged. FR-082: AC-12 and AC-13 are untagged; both are pre-existing environment-limit criteria this diff does not touch, so they are not counted as defects of this PR. FR-081: AC-11 (retired) is untagged and CON-1..3 are without symbols, also pre-existing and untouched.

The resolved CLIs (quoin 0.36.8, quire 0.52.0) are newer than the 2026-11-10 release notice (quoin 0.30.0, quire 0.40.0). They are the installed binaries; no older fallback was used. The quoin 0.30.0 gap-analysis skill was loaded. Module: spec-artifacts-process@v0.36.0.

Reverse gap: every changed behavior has an owner. `link_inheritance`/`declaring` belong to FR-710 and FR-081, the normalize unreachable-operation refusal and the table rows to FR-082 and FR-056-AC-33, and inherited process identity to FR-056/FSpec FR-154. There are no stubs, placeholder returns, ignored tests or mocked production seams in the diff. The optional semantic review was not run (the brief did not opt in), although manual intent/test/code inspection of the changed paths informed the findings. No Cargo/test gate was run by this reviewer; focused runtime evidence is from draco:QSL:code. No AssuranceProfile applies. No spec files changed, so spec-review was not applicable.

## Dispositions

Disposition round 1. Fix head: a756b1f685fcd4e406a8d89c07463c9383f47b5d. The test bodies were inspected at that head; Draco's focused run of them is queued, not green, so they are not claimed as passing.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `same_owner_and_self_operation_redefinitions_refuse_the_original_owner` (model_unit_inheritance.rs:302, FR-082-AC-2) asserts one typed RedefinitionTarget, with exact target and redefiner keys, for self, same-owner and mixed invalid-local cases, and that the proper-ancestor SHOP form checks clean. |
| FND-002 | fixed | `an_object_extending_another_meaning_refuses_at_its_declaration` (model_unit_inheritance.rs:485, FR-710-AC-4) asserts exactly one invalid_model_binding/malformed-declaration for an object extending a record, a machine or a process, with the span equal to the whole authored declaration. |
| FND-003 | fixed | Six new tests (model_unit_inheritance.rs:523-723) cover extends on record, event, interface and store; record subsets/redefines; event and interface local subsets that survive intake; unresolved field and operation links on event, interface and store; non-ancestor redefinitions projected to the declaration span; and disallowed event and store members. |
