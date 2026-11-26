---
id: SR-967175026302197768900474181941935084872
title: "Gap analysis — QSL-3381 trim proof rows to the stopped graph"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-3381; PR #4015 head 69dd2dec71; diff-scoped implementation, tests and owning criteria"
review_set: subset
---

## Summary

Ticket: QSL-3381. The changed producer behaviour is owned by FR-201-AC-11 and by FR-201's "Closed reachability document profile", which says "Values and ranks SHALL cover exactly this stopped graph". The bounds side is owned by FR-1042-AC-25. I searched the spec for the cause's wording ("stopped graph", "past a target"). It appears in FR-201 (lines 347, 434, 496) and FR-1042 (line 130, AC-25), and the code agrees with both. Both new tests carry correct trace tags, and `quire matrix` lists FR-201-AC-10, FR-201-AC-11 and FR-1042-AC-25 as `tagged`.

## Verdict

**PASS** for the changed scope. The audit is diff-scoped: no gap was found between the criteria, the tests and the code. No stubs, tautological assertions or behaviour without an owning criterion were found in the four changed files. The expected values in the tests (`7/16`, `9/16`, value `1/2`, three rows) are hand-written from AC-25's chain and not copied from product output. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No changed-scope gap found (placeholder) | - |

## Coverage

Plan completion: not assessed.

`quire matrix --scope . --format json` exited 0 and reports FR-201-AC-10, FR-201-AC-11 and FR-1042-AC-25 as `tagged`. `quire trace --file` lists both new tests under TC-636, TC-3151 and the ACs above. A semantic review was done for the changed functions only. A full-repository reverse-gap and stub sweep was not done, because this run is diff-scoped.

## Examined units and bindings

```yaml
clean: true
scope:
  - {id: FR-201-AC-11, path: spec/functional/FR-201-check-a-probability-certificate.md, role: examined, excerpt: "Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass the `reachability/v1` profile checker at an attained threshold; a threshold strictly beyond the certified initial value fails."}
  - {id: FR-201-AC-10, path: spec/functional/FR-201-check-a-probability-certificate.md, role: examined, excerpt: "Altering a value fails equality; removing or breaking positive-value rank progress fails ranking even when equalities hold."}
  - {id: FR-1042-AC-25, path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, role: examined, excerpt: "Over the acyclic chain under a workload ... `lower` `s` `7/16`, rank 1, and counterpart `upper` `s` `9/16` are accepted; ... Where `max_states` stops exploration past a target but admits the graph that stops there, EN-5's `lower` keeps that base, names that graph's states alone and settles `exploration: Partial` with no counterpart."}
findings: []
bindings: []
```
