---
id: TC-3184
title: "Emit cyclic until and always proof documents"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3184: Emit cyclic until and always proof documents

## Description

Verify FR-201-AC-62: EN-5 proves cyclic until and always claims with the new
profiles instead of reporting them unsupported, and the codec keeps its
strict rules and the profiles' boundaries.

## Test Procedure

Analyze FR-201-AC-59's `probability >= 1/4` and FR-201-AC-60's
`probability >= 1/2` claims. Round-trip each emitted document and check it
with `fsl-probability-check` in a process with no `fsl-analyze` linkage.
Analyze an acyclic until and always subject. Offer a cyclic subject's
document under `state-predicate/v1`. Mutate `event`, `optimization`,
`fairness`, `profile`, identity, keys, bindings, row coverage and order,
rationals and canonical bytes. Analyze fair, timed, reward, nested,
bounded, event-predicate, multiple-initial, multiple-binding and
unresolved-workload variants, and offer `lower`, `upper` or interval
documents.

## Expected Results

Both claims settle `proved` with exact values `1/4` and `1/2`; the documents
round-trip byte for byte and the separate checker accepts them; neither
settles `unsupported`. The acyclic subject keeps `state-predicate/v1` bytes
and verdicts; the cyclic subject is unsupported under that profile. Each
mutation prevents acceptance with the existing rule and locus. The excluded
variants and kinds issue no proof in these profiles.
