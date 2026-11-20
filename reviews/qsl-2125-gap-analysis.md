---
id: SR-20261119000000000002
title: "Focused gap analysis of QSL-2125 PR #3026"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@8983ead8d161765c8b5fddbd2221022162ad5460; PR #3026 diff: fsl-analyze/src/analyze.rs, fsl-analyze/tests/it/analyze_reach_rulings.rs, fsl-analyze/tests/it/analyze_request_wire.rs, fsl-command/src/command.rs, fsl-command/src/command/wire.rs, fsl-semantics/tests/it/state_graph_claims.rs, tests/it/compile_package_output.rs, xtask/src/refinement/layering.rs; FR-620-AC-11; FR-628-AC-4/5; FR-810-AC-5; FR-165-AC-7; FR-489-AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-489
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2215
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2291
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-2297
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1545
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1272
    type: reviews
---

## Summary

Ticket: QSL-2125. Plan completion: not assessed. Quoin's `gap-analysis` command is unavailable in the installed `quoin 0.36.7` (`quoin gap-analysis --help` exits with `command gap-analysis not found`), so this is the review skill's manual acceptance-to-test fallback. `quoin matrix --repo . --json` completed; each changed criterion is tagged to its expected test. The PR makes the request-package route exercise the state-graph evidence and budget behavior, refuses the `hashed_bytes` normalization limit, activates the temporal-complete profile case, and enables the output/handoff test. The output/handoff containment implementation still misses normalized traversal paths, as recorded in FND-001 of the code-review artifact. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — changed behaviors have tagged tests, but the output/handoff criterion is not fully implemented for paths that lexically traverse outside and back into the handoff directory. Plan completion: not assessed.

## Scope

- `FR-620-AC-11`: examined the `hashed_bytes` rejection and unaffected `work_units` acceptance against the newly active test.
- `TC-2215`: examined the mapping to `hashed_bytes_is_not_a_normalization_limit`.
- `FR-628-AC-4`: examined the Kernel-checked package selection and assertions for proof instance identity, undefined witness, and locus.
- `FR-628-AC-5`: examined the budget stop outcomes and assertions.
- `TC-2291`: examined the mapped state-graph test and trace tags.
- `FR-810-AC-5`: examined the successful independent writes and refusal/no-write behavior, including the path-containment edge case.
- `TC-2297`: examined the mapped integration test.
- `FR-165-AC-7`: examined the new acceptance test for the `filament.temporal.complete/v1` value profile on a state-graph claim.
- `TC-1545`: examined the test mapping and trace tag.
- `FR-489-AC-5`: examined exact linked descriptor coverage for EN-1, EN-4, EN-5, EN-6, EN-7, value-checker, contract-checker, and protocol-analysis.
- `TC-1272`: examined the registration-test mapping.
- Each changed Rust file listed in the scope header: examined for behavior introduced without an owning criterion or tagged test.

## Coverage

- `FR-620-AC-11` → `hashed_bytes_is_not_a_normalization_limit`, tagged `TC-2215` and the AC; confirms both refused forms and the admitted `work_units` case.
- `FR-628-AC-4/5` → `state_graph_evidence_keys_loci_and_budget_stops`, tagged `TC-2291` and both ACs; the fixture compiles under Kernel mode, selects the produced package in analyze, and checks the instance/evidence locus and budget outcomes.
- `FR-810-AC-5` → `output_and_handoff_are_independent_writes`, tagged `TC-2297` and the AC; exercises success and the specified literal `handoff/pkg.json` refusal, but does not cover lexical traversal such as `existing/../handoff/pkg.json`.
- `FR-165-AC-7` → `a_claim_accepts_the_temporal_complete_header_layer_as_its_value_profile`, tagged `TC-1545` and the AC; checks the selected positive temporal-complete case.
- `FR-489-AC-5` → `linked_engine_registrations_expose_the_owned_descriptors`, tagged `TC-1272` and the AC; asserts the exact descriptor IDs and EN-7 schedulability advertisement.
- No changed production behavior was found without an owning requirement. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-810-AC-5 refusal is incomplete: lexical prefix comparison misses an output such as `existing/../handoff/pkg.json`, which can resolve inside the handoff directory and be written there after the handoff exists. | fsl-command/src/command.rs:2371 |

Plan completion: not assessed.
