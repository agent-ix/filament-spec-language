---
id: SR-1485
title: "QSL-2084 code-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-2084; PR 3166; e6d7dfdfc70a0e75f3459b4c96229527a460b17f; all twelve changed files listed in Coverage"
relationships:
  - {type: references, target: "ix://agent-ix/filament-spec-language/FR-406"}
---

## Summary

Reviewed the twelve-file test-only change on the frozen head. Code-review, including the Rust lane and repository conventions, found no defect in the new tests or the registration/helper changes.

## Verdict

**PASS** for the submitted std regression feature. Independent portable checked-term evaluation and std retirement remain pending existing IR-590 and IR-593; this review does not close QSL-2084 or those facets. One final affected-component gate remains with the original coder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The complete examined file scope and exact examined/context-only contract excerpts follow.

```yaml
scope:
  - {"id":".agent/receipts/qsl-2084/scope.md","path":".agent/receipts/qsl-2084/scope.md","role":"examined","excerpt":"The current FSL expression and clause entries still execute the std Machine."}
  - {"id":".agent/receipts/qsl-2084/validation.md","path":".agent/receipts/qsl-2084/validation.md","role":"examined","excerpt":"Twelve new active tests each passed focused execution."}
  - {"id":"fsl-eval/tests/it/evaluator_closure_cases.rs","path":"fsl-eval/tests/it/evaluator_closure_cases.rs","role":"examined","excerpt":"assert_eq!(Record::new(&completed(closed), &closed_meter), expected);"}
  - {"id":"fsl-eval/tests/it/main.rs","path":"fsl-eval/tests/it/main.rs","role":"examined","excerpt":"mod evaluator_closure_cases;"}
  - {"id":"fsl-eval/tests/it/model_simulation.rs","path":"fsl-eval/tests/it/model_simulation.rs","role":"examined","excerpt":"mod evaluator_closure_cases;"}
  - {"id":"fsl-eval/tests/it/model_simulation/evaluator_closure_cases.rs","path":"fsl-eval/tests/it/model_simulation/evaluator_closure_cases.rs","role":"examined","excerpt":"assert_eq!(actual, value == 1, \"current counter value {value}\");"}
  - {"id":"fsl-eval/tests/it/named_population.rs","path":"fsl-eval/tests/it/named_population.rs","role":"examined","excerpt":"holds_atom_metered(unit, objects, &mut Meter::new(UNLIMITED))"}
  - {"id":"fsl-eval/tests/it/named_population/evaluator_closure_cases.rs","path":"fsl-eval/tests/it/named_population/evaluator_closure_cases.rs","role":"examined","excerpt":"Separation::Unmet(SeparationStep::Element)"}
  - {"id":"fsl-eval/tests/it/refinement_populations.rs","path":"fsl-eval/tests/it/refinement_populations.rs","role":"examined","excerpt":"mod evaluator_closure_cases;"}
  - {"id":"fsl-eval/tests/it/refinement_populations/evaluator_closure_cases.rs","path":"fsl-eval/tests/it/refinement_populations/evaluator_closure_cases.rs","role":"examined","excerpt":"(\"missing_declaration\", \"missing-name\")"}
  - {"id":"fsl-eval/tests/it/type_invariants.rs","path":"fsl-eval/tests/it/type_invariants.rs","role":"examined","excerpt":"mod evaluator_closure_cases;"}
  - {"id":"fsl-eval/tests/it/type_invariants/evaluator_closure_cases.rs","path":"fsl-eval/tests/it/type_invariants/evaluator_closure_cases.rs","role":"examined","excerpt":"ValueInvariantOutcome::False"}
  - {"id":"FR-049-AC-2","path":"spec/functional/FR-049-admit-composed-evaluation-inputs.md","role":"examined","excerpt":"Runtime binders and model/observation values are admitted only through the owning checked environment and authority contracts; duplicate, surplus, foreign and wrong-type inputs refuse at admission, and concrete population/window/clock roles remain distinct."}
  - {"id":"FR-366-AC-4","path":"spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md","role":"examined","excerpt":"TC-235 M13 run 1: with `Money/NonNegative` (`self.amount_minor >= 0`) and `Order` given `quoted: Money`, an ordered `history: Money {0, 3}`, optional `discount: Money` and operation `requote(amount: Money)`, `evaluate_value_invariants` over current observation `s1`"}
  - {"id":"FR-406-AC-6","path":"spec/functional/FR-406-evaluate-clause-families-through-a-filament-eval-clause-entry.md","role":"context_only","excerpt":"A history field with no value at the read's observation returns `FamilyResult::Refused` with `missing_declaration`/`missing-name` and field `history` naming the field. A `PopulationObjects` read returns its members in ascending reference-key order, and the same result and charges on each of ten runs."}
  - {"id":"NFR-015-AC-4","path":"spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md","role":"context_only","excerpt":"`fsl-package` declares a `std` feature with `default = [\"std\"]` that enables `dep:filament-contract-model` and `fsl-foundation/std`; `cargo check --locked -p fsl-package --no-default-features` succeeds on the host with the crate root `no_std`;"}
  - {"id":"NFR-015-AC-5","path":"spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md","role":"context_only","excerpt":"With `std` disabled, every export in the \"With `std` disabled\" column of the `fsl-package` surface table resolves, and every export in its \"Behind `std`\" column does not."}
```

Checked arithmetic results against explicit sums and direct kernel charge records; typed argument admission and unchanged meters; terminal/state/clause values against actual observations and field trails; reward arithmetic; temporal population traversal and complete-empty/missing/incomplete distinctions; captured-witness replay with a changed-index refusal; refinement values and visit counts; pre/post and stored-history binding with missing-history refusal; key-ordered PopulationObjects and ten-run charge stability; reachable Money invariant outcomes, paths and six comparison charges. The record negative controls intentionally exercise the comparator, not a second evaluator.

Generic FR-406 tags claim the current supported behavior only. No post-switch E entry, recorder architecture, CallAdmissionFailure architecture, no_std execution or evaluator retirement is asserted. FR-049-AC-2 and FR-366-AC-4 are supplementary tests of their admission/value-invariant portions, not sole discharge of every clause. FR-406-AC-6 is context only because portable clause-entry implementation remains pending. No new production surface, copied interpreter, source scanner, ignored test, warning suppression, config or gate change.

Reconciliation: manual exact diff/criterion binding inspection; no matrix rerun, repository-wide counts or repository assurance claim, per the assignment and user prohibition on repeated measurement.
Plan completion: not assessed
Author evidence accepted: twelve new focused std cases passed; foundation thumbv7em-none-eabi no-default build and package host no-default check each passed once. These build results are manual evidence, not permanent mode tests or portable clause execution.
Scoped lint limitation: existing EvaluationEnv::charged dead_code and reference_adapter LiveContext unused import; no green lint claim, retry or suppression.
No tests, builds, lint, product probes, mode checks, or Done audits rerun by this reviewer. Final complete affected-component gate is pending.

