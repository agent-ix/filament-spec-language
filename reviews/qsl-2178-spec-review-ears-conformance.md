---
id: SR-17952655000005
title: "QSL-2178 spec-review/ears-conformance of frozen PR 3303"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2178 QSL-2717; frozen PR 3303"
review_set: subset
---

## Summary

Ticket: QSL-2178; linked QSL-2717. EARS review of the changed FR-106 requirement-bearing blocks. Conditions, admission actor and measurable response are stated; the engine reports no grammar finding.

## Verdict

PASS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

quire validate --scope . spec/functional/FR-106-admit-snapshots-and-invocations.md spec/test-cases/TC-464-snapshots-and-invocations-admit.md --summary exited 0; /private/tmp/qsl-third-specvalidate.log records 2/2 grammar-clean and 0 findings. TC-464 procedural prose is excluded from the EARS requirement lens. Semantic checks found no event/state confusion, vague response, missing actor or ambiguity changing implementation.

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
```
