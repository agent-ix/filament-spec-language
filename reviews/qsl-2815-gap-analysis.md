---
id: SR-967175026302197768900474827
title: "Gap analysis of QSL-2815 AC-4 performance follow-up"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2815; PR #3816 at 5d267c34b1f45bc3e6871e506e1c2870f8fdd99f; changed Rust implementation/tests; FR-101-AC-2, FR-120-AC-32, FR-851-AC-4"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec
    type: reviews
date: 2026-11-24
---

## Summary

Audited the changed behavior against the computed static requirement/test matrix, trace bindings, changed production code, and stubs/reverse gaps. The three relevant requirements have live test binders, the changed production behavior is owned by FR-851-AC-4 and the existing canonical-state-key requirement, and no new traceability or implementation gap was found in this PR slice.

## Verdict

**PASS** — the changed criteria are tagged to live tests and no new unspecified behavior or stub was found in the reviewed slice.

## Coverage

Plan completion: not assessed

Matrix evidence: `quire matrix --scope . --format json` exited 0 and produced the static matrix. `FR-101-AC-2`, `FR-120-AC-32`, and `FR-851-AC-4` are tagged. The whole-repository static matrix also reports 819 untagged and 24 tagged-by-ignored non-retired criteria; these are existing repository-wide records outside this PR's changed criterion set, so this subset review makes no repository-wide coverage claim. The preferred `timeout 60 quoin matrix --repo . --json` produced no output and timed out at 60 seconds, so no engine run evidence was available; the documented static `quire matrix` fallback was used.

Reverse-gap and stub review: the new product/state-key caches and observation fact reuse are scoped to existing FR-101, FR-120, and FR-851 behavior. The changed production files contain no TODO/FIXME/XXX markers, `todo!()`/`unimplemented!()` placeholders, or stub-only modules. The new regression test executes real model state keying and cell mutation and asserts the changed behavior. No plan was supplied.

Semantic intent review was skipped; it is optional and was not requested. No CI workflow files changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Examined scope

```yaml
- {id: FR-101-AC-2, path: spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-fspec-sampler.md, role: examined, excerpt: "The state key is the JCS encoding of FR-181's typed canonical form, and exploration coalesces two states exactly when their full key bytes are equal. The state whose semantic member is float64 bits 0000000000000000 and every map empty has its JCS key bytes as its key; with bits 8000000000000000 (negative zero) it has its own, different, key bytes, and the two remain two states. Two NaNs with different payloads are two states. Each key is a StateKey under filament.simulation.state-key/v1."}
- {id: FR-120-AC-32, path: spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, role: examined, excerpt: "Over AC-29's Cell, in a model where Cell also declares the query operation succ(): Int[0, 100], whose body is self.a + 1 and whose precondition is self.b != 5, and whose one other operation has the equation clause post A { self.a = pre(self.succ()) }, from the state with a 3 and b 5, where the dispatched call's precondition is false and so e is Undefined with reason precondition-false (FR-301-AC-2; the succ application itself is not enabled and charges its one argument vector in both forms), and in one whose equation evaluation is Incomplete under a meter budget whose work_units is 0, the application is not solved. Each gives the result of its enumerated reference form"}
- {id: FR-851-AC-4, path: spec/functional/FR-851-enumerate-a-bounded-integer-field-from-its-postcondition-bounds.md, role: examined, excerpt: "The rover fixture analyzed through native-analyze/1 for the temporal clause always (holds(not r.lidarOk) implies eventually holds(r.stopped)) under fair weak brake and fair weak halt, with no temporal.limits, settles proved with basis exhaustive, reaches no limit at EN-1's published defaults, and completes within 60 seconds in the test profile."}
```
