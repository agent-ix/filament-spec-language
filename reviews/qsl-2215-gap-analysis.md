---
id: SR-2026111612172400000038
title: "Gap analysis — filament-spec-language protocol remainder"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2215/QSL-2220 ticket criteria, FR-218, FR-464, FR-865, FR-880, FR-884; changed protocol source and test files"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-218", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-884", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-880", type: references}
---

## Summary

Scoped the traceability and reverse-gap check to QSL-2215 and QSL-2220 acceptance criteria and the changed protocol behavior. The requested matrix output binds the changed requirement criteria to tests; one message-case oracle gap is reported in the separate code-review artifact. This is planless; plan completion was not assessed.

## Verdict

**PASS** — no traceability or reverse-gap finding within the selected ticket scope; the test-oracle finding is recorded by code-review.

## Scope examined

- QSL-2215: read/guard/role refusal loci; choice/decision/duplicate-role loci; refusal messages; duplicate protocol names within one unit; same name across units; minimal accept/reject construct corpus.
- QSL-2215 corrected ticket comment: the typed cause carries the offending item's name and span, and rendered text shows name and location; source text is not required.
- QSL-2220: post-test repeat body-binder reads; repeat through repeat; scalar, record, Option and collection `over`; `terminal when` reading `over` under `on each`; evaluator exploration; no-progress repeat refusal.
- QSL-2220 declared exclusions: repeat invariant/variant well-founded typing (QSL-2222), attempt-result reads (QSL-2409), post-test repeat reader schema and non-object `over` package readback (QSL-2424 / pinned contract reader).
- Changed source and tests in `fsl-replay`, `fsl-semantics`, `fsl-package`, and `fsl-command`; context-only: owning FR-218, FR-464, FR-865, FR-880, FR-884 and the ignored `protocol_refusal_names` command test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found in the selected ticket-scope traceability and reverse-gap checks (placeholder) | - |

## Coverage

- Reconciliation: quoin matrix (`quoin 0.36.6`, `quire 0.52.0`, engine `0.62.2`); selected requirement criteria were checked against its computed bindings.
- Selected trace bindings: FR-884-AC-1 through AC-5, FR-880-AC-6/8, FR-218-AC-17 and FR-865-AC-5 are tagged by test functions. Quoin's evidence store reports no run evidence; the six focused local tests listed in code-review were run separately.
- Reverse-gap: changed source paths add no public API or user-visible surface without an owning requirement; changed behavior maps to the FRs listed above. Changed tests contain behavioral assertions rather than stubs.
- QSL-2220 remains partially tracked: post-test body-binder repeat compilation and evaluator exploration are not supplied by this PR; the issue's remaining reader and non-object cases are explicitly assigned to QSL-2424. The ticket remains In Progress.
- Semantic review: skipped as a whole-repository activity; changed test-oracle strength was reviewed in the separate code-review artifact.
- Plan completion: not assessed.
