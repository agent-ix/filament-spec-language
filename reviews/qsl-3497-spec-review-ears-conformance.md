---
id: SR-967194
title: "ears-conformance review of PR 4064 spec changes"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; spec/functional/FR-189, FR-192, FR-203, FR-315, FR-896; spec/test-cases/TC-624, TC-900, TC-3241"
review_set: subset
---

## Summary

Ticket: QSL-3497. PR 4064 at a8b116f24c243dbacdb05ec6ae6559c00900359c. 0 finding(s).

## Verdict

The new and edited statements name an actor, a trigger where one applies and a SHALL response.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

- examined: FR-189 (spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)

```text
- EN-4 SHALL take a claim's instances, and its instancing outcomes in their
  order, from `ModelSystem::claim_instances` (FR-125 "Instancing"):
  `RequiresBound`, then `NoInitialState`, then a conflicting identity, then
  `NoInstance`. A claim with no `over` has one instance, with no bound object.
- If the subject's `initial` list is empty, then EN-4 SHALL refuse with
  `StatisticalRefusal::NoInitialState`; if the subject has an initial state
  and the claim's `over` type has no admitted instance, then EN-4 SHALL refuse
  with `StatisticalRefusal::NoInstance`. FR-192 settles each `inconclusive
```

- examined: FR-203 (spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)

```text
### Initial states and instances

- Before it admits the claim's form, builds a product, writes a certificate
  or reads one, EN-5 SHALL decide FR-125's instancing outcomes in FR-125's
  order: `RequiresBound`, `NoInitialState`, `conflicting-identity`,
  `NoInstance`. This holds for every claim form and activation.
- When the subject's `initial` list is empty and no universe is missing,
  EN-5 SHALL report `NoInitialState`, whatever the universes hold.
- When the `initial` list is non-empty and the claim has no instance, since
  its `over` parameter or its `on each` activation type has no admi
```

- examined: FR-315 (spec/functional/FR-315-encode-the-smt-lib-transition-relation.md)

```text
  apply the checks in the order of this table, one row at a time, and SHALL
  refuse on the first row that applies. Rows 5 to 8 are FR-125's instancing
  outcomes in FR-125's order:

```

