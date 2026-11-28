---
id: SR-967181
title: "Gap analysis — QSL-3301 keep only an ordinary identifier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs, fsl-cst/tests/it/reserved_declaration_names.rs; PR #3914 at 13127f026a9639c2b406a8337c41685394ef4dfb; QSL-3301; governing criterion FSpec FR-376-AC-8 (agent-ix/filament-specification)"
review_set: subset
---

## Summary

Ticket: QSL-3301. Planless, diff-bounded gap analysis of PR #3914. The governing criterion is FSpec FR-376-AC-8 with test case FSpec TC-311, both in agent-ix/filament-specification. They are not in this repository's `spec/`, so the local computed matrix cannot bind the `FSpec-` tag. The changed production line has an owning requirement (FR-376-AC-8 and the shared grammar's contextual-word rule), and no stub was introduced.

## Verdict

**CONDITIONAL**: low findings only, both outside this PR's diff. The code change is owned and tested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FSpec TC-311 has no procedure step for FR-376-AC-8's identifier clause ("in a function body `only` and `seq` are ordinary identifiers"). Its step 8 expected result says the state-clause `only` and `seq` spellings refuse `invalid_syntax`/`unexpected-token`, while AC-8 now reads them as the forms. The test case the new test cites does not verify what the test asserts. | filament-specification spec/test-cases/TC-311-population-valued-mapping-expressions.md; FR-376-AC-8 |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (Quire/Quoin 0.36.10) ran. FR-376-AC-8 is not a local criterion. quire reports the new `#[trace("FSpec-TC-311", "FSpec-FR-376-AC-8")]` as "not read by the declared trace grammar". That warning is systemic for every `FSpec-` tag in the repository (7455 such warnings) and is not specific to this PR.
- Scope: FR-376-AC-8 (examined), shared-grammar refinement population forms paragraph (context), TC-311 (examined), the changed grammar line and the new test.
- Reverse-gap/stub inspection: the changed line is owned by FR-376-AC-8 and by FR-1040-AC-11 (collection `only`). No stub was found.
- Semantic review: skipped as a separate pass. Intent, test and code were inspected for the changed lines.
- Plan completion: not assessed
- Evidence: no run evidence read; no tests run (memory gate refused).
