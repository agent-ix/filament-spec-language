---
id: SR-17952655000003
title: "QSL-2178 spec-review/base of frozen PR 3303"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2178 QSL-2717; frozen PR 3303"
review_set: subset
---

## Summary

Ticket: QSL-2178; linked QSL-2717. Base checklist for the two changed specification artifacts. The declared ClockRef admission exception and positive/negative procedures are specific, observable, and consistent with ordinary reference closure.

## Verdict

PASS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Subset: base, integrity, ears-conformance. Scope is the changed FR-106 statement blocks, AC72/73 and TC-464 steps 7/8 and expected results; other 71 FR-106 criteria are context only, not re-certified. New criteria have distinct IDs and matching test tags, and TC-464 names real positive/negative instances, wrappers, order and exact refusal records. No added relationships require a dependency analysis and no new domain object definition warrants an object review. No applicable AssuranceProfile document exists: the two textual search hits merely mention that type in previous review prose. Both changed spec documents pass quire validation, 2/2 grammar-clean, 0 grammar findings. Specification readiness PASS does not override the separate implementation and evidence FAIL verdicts.

## Examined Scope

```yaml
scope:
  - id: FR-106-AC-72
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 7's dense-time `Call` fixture, a raw snapshot storing the actual
      per-instance place identities of `c1.x` and `c2.x` in `r:
      Reference<Clock>`, `maybe: Option<Reference<Clock>>` and `refs:
      Sequence<Reference<Clock>>` admits, retaining those identities as data,
      the optional presence and the sequence order. Neither place identity is an
      object key, and neither target is resolved as a population member. Reading
      the admitted fields distinguishes `c1.x` from `c2.x`; the required
      evaluation of the stored `self.r` under FR-1040-"
  - id: FR-106-AC-73
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: Over [TC-464](../test-cases/TC-464-snapshots-and-invocations-admit.md)
      step 8's model variant without `x`, storing the formerly declared `c1.x`
      identity in `r`, in the present value of `maybe`, or at element 0 of
      `refs` refuses
      `dangling_reference`/`absent-target-in-complete-population`, naming the
      snapshot, owner population `calls`, object `c1`, the affected field and
      nested value path, and that undeclared target identity, with no admitted
      observation or truth value. A target's clock-reference static type or
      native Clock target alone does not make it declared.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      A statically `Reference<Clock>`-typed value
      does not name a population member and adds no required population through
      its target; its declared-place validation is independent of population
      completeness.
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: |-
      `Reference<Clock>` target is checked against declared clock places
             under the same rule as an object field.
  - id: TC-464
    path: spec/test-cases/TC-464-snapshots-and-invocations-admit.md
    role: examined
    excerpt: Repeat with `c1.maybe` absent.
  - id: TC-464
    path: spec/test-cases/TC-464-snapshots-and-invocations-admit.md
    role: examined
    excerpt: |-
      Assert the full refusal record at
         that field's nested path.
```
