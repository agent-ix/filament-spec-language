---
id: SR-20261126014317002
title: "gap analysis of PR 4017 admitted-type instance bindings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #4017 head 859955ed3db61f5784b4ef741e30c3d39a992dcb; FR-125-AC-8, FR-125-AC-9, FR-125-AC-10, FR-741 Binding"
review_set: subset
---

## Summary

Ticket: QSL-3126. Planless gap audit scoped to the criteria the changed code implements. `quire matrix --scope . --format json` ran. FR-125-AC-8, FR-125-AC-9 and FR-125-AC-10 each have tagged test binders. Seven new tests tag FR-125-AC-9, and eleven new `implements` markers resolve. The reverse-gap and stub checks covered the changed production functions. Semantic review was not requested and was skipped. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-125-AC-9's refuted binding on the digital probabilistic route (`exact_probabilistic/digital.rs`) has no test with an admitted type other than the declared one. A writable harness exists (`settle_over_base`) | fsl-analyze/src/exact_probabilistic/digital.rs:103-108; fsl-analyze/tests/it/timed_state_graph.rs:441 |
| FND-002 | medium | No test covers a protocol replicated role over a subtype object together with the `over` binding of the same object, which is the paired side of FR-125-AC-9's protocol binding (see the code-review SR, FND-001) | fsl-eval/src/simulation/protocol.rs:1945-1966 |

## Verdict

CONDITIONAL. The exact, statistical, `on each`, protocol source and package routes, and the cycle identity have tagged tests that pass on this head. Each asserts literal expected types. No stubs or tautological assertions were found in the added tests. The cycle identity test compares two identities instead of literal text; for a key-alone property that is adequate.

## Coverage

- FR-125-AC-9: tagged by ten tests, seven of them new. The exact, statistical, `on each`, protocol source and package, zone temporal, and cycle identity routes are covered. The digital probabilistic route (FND-001) and protocol role pairing (FND-002) are not.
- FR-125-AC-8 and FR-125-AC-10: tagged on main. This PR does not change them.
- Plan completion: not assessed
