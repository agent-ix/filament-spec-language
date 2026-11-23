---
id: SR-3603004
title: "EARS review of the on-each reachability conditional profile (filament-spec-language PR 3603)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language@32ca5b98116c0ca890097e2604f1e680ed721a1f; spec/functional/FR-201-check-a-probability-certificate.md (Closed on-each reachability conditional profile); spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md (Instances)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: references
---

# EARS review of the on-each reachability conditional profile (PR 3603)

## Summary

Ticket: QSL-2979. The changed statements are the new profile section's
bullets, which are event/state-driven SHALL statements, and the FR-875
Instances bullet, where only the cause literal changed (it stays
"FSL SHALL settle ... with cause"). The opening sentence is a well-formed
state-driven requirement ("For a checked ... claim whose ..., the codec SHALL
admit ...").

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three normative clauses have no SHALL: "Every other on-each form stays unsupported by this profile", "An initial key whose denominator is 0 is confirmed vacuous ... and holds vacuously", and "a `Ratio` document of this profile is rejected `InequalityFails`". The last restates "Conditional certificates", but as written the first two read as narrative. Use "SHALL stay unsupported", "the checker SHALL confirm ... and the instance SHALL hold vacuously" and "SHALL be rejected". | spec/functional/FR-201-check-a-probability-certificate.md:873-907 |

## Verdict

EARS-conformant apart from FND-001's three clauses that are missing SHALL.
