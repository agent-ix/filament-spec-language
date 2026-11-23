---
id: SR-202611230621511
title: "gap-analysis of PR3553 finite hyper body temporal_profile (IR-1586 emitter half)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3553 3c7b2d59b069e2f6b4fe7c72ba7e128df9071bff; fsl-package/src/emit/tests/hyper_clause.rs; fsl-semantics/src/check/lowering/hyper.rs; fsl-package/src/emit.rs"
review_set: subset
---

## Summary

Ticket: IR-1586. Gap analysis scoped to the PR diff. Plan completion: not assessed. I checked FSpec FR-322-AC-464 / TC-2028 (filament-specification PR 815 at acc7fd01) against the emitter tests and code. The emitter obligations under AC-464 are covered by tests that assert exact values: the finite body carries the clause's own profile as an exact DefinitionRef, the lock selects that definition, and a behaviours-domain hyper omits the member. No stub, and no production code without an owning requirement, was found in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new test is tagged `#[trace("TC-2028", "FR-322-AC-464")]` with bare ids. In this repo's own spec namespace, FR-322 is "evaluate union construction and case", which has no AC-464, and there is no TC-2028. The binding is dangling or wrong in the local matrix. The repo convention for FSpec ids is the `FSpec-` prefix (tests/it/clock_derivation.rs:560 traces `FSpec-TC-2028`, `FSpec-FR-322-AC-464`). | fsl-package/src/emit/tests/hyper_clause.rs:184 |

## Scope and evidence

- FSpec FR-322-AC-464 (examined): the finite hyperproperty body carries a typed `temporal_profile` DefinitionRef that matches exactly one temporal-role lock selection. A behaviours-domain hyperproperty keeps its law on the body wrapper.
- Emitter coverage: `a_finite_hyperproperty_names_its_own_profile_not_the_first_lock_selection` (exact lock `[INFINITE, EVENT]`, body EVENT), `a_finite_trace_set_hyperproperty_emits_its_finite_domain` (exact body profile, admitted by the I2 reader), and `the_wrapper_carries_the_profile_law_activation_and_formula` (behaviours hyper, member absent).
- TCB-15..24 are reader refusals owned by contract-ir PR 1136 and are out of the emitter's scope. TCB-14's base is emitter-generated, and the finite test above produces that shape.
- Only the event-position profile is exercised. The fixed-sample and timestamped profiles go through the same `profile_definition` mapping, so this is not a coverage gap.

## Verdict

The emitter half of AC-464 is implemented and tested with exact values. One low trace-tag defect.
