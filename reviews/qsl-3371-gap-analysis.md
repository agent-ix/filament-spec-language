---
id: SR-2026112517540002
title: "Gap analysis — filament-spec-language QSL-3371 value bounds over an acyclic eventual base"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/partial.rs, fsl-replay/src/probability_certificate/document/mdp.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, tools/fsl-probability-check/tests/it/value_bounds.rs, spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, spec/test-cases/TC-3151-certify-value-bounds.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1042", type: references }
---

## Summary

Ticket: QSL-3371. PR agent-ix/filament-spec-language#3950 at head `bc3b621fc52f64ca1e81828b1af5f47cf767abfd`. A planless audit scoped to the diff, using the computed matrix (`quire matrix --scope . --format json`, quire 0.36.10). The new checker behaviour (value bounds over the unbounded, acyclic `reachability/v1` base under a workload) is owned by FR-1042 "Bases", which admits "both subsets", but no FR-1042 acceptance criterion states it. The two new tests carry only `#[trace("TC-3151")]`, and TC-3151 requires `FR-1042-AC-n` tags, so the matrix binds them to no criterion.

Plan completion: not assessed

## Verdict

**CONDITIONAL** — one medium trace/coverage finding. There are no stubs or coverage inflation: both new tests assert exact values, rules and loci, and fail on main, where the base is refused `Unsupported`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No FR-1042 AC covers value bounds over the unbounded acyclic `reachability/v1` base. The new tests `an_acyclic_eventual_base_takes_checked_bounds` and `a_partial_lower_over_an_acyclic_eventual_names_only_the_stopped_graph` are tagged `TC-3151` alone, against TC-3151's tagging rule, so the computed matrix shows no criterion for the newly admitted checker behaviour. | tools/fsl-probability-check/tests/it/value_bounds.rs:916, tools/fsl-probability-check/tests/it/value_bounds.rs:971 |

## Coverage

- FR-1042-AC-1, AC-2, AC-16, AC-18 and FR-201-AC-51 are tagged in the computed matrix. FR-1042-AC-11 is untagged (pre-existing; its `shifted` clause is the remaining red case at `certificate_kinds.rs:353`, QSL-3352).
- The removed measure case in `value_bounds_certificates_are_accepted` was bound to no AC that requires it. FR-201-AC-51's list does not include a partial measure proof, and FR-1042-AC-11 lists the chain, `shifted`, reward and timed cases. The replacement adds checker coverage (accepted 40/81 exact, `inequality-fails`, `threshold-missed` on a prefix `lower`).
- Reverse gap: `name_lower_rows` (mdp.rs) and the `acyclic_eventual` admission are owned by FR-1042 "Document" and "Bases".
- Semantic review: not run (opt-in, not requested).
