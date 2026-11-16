---
id: FR-921
title: "Check a timed-product certificate without the engine"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-920
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-751
    type: depends_on
---
# FR-921: Check a timed-product certificate without the engine

## Description

`fsl_replay::check_probability_certificate_document`, the wire form
`check_probability_certificate` and the binary `fsl-probability-check` SHALL
check a `timed-product/v1` document by rebuilding the timed product from the
checked subject and claim with qualified-core code only (ADR-028 CE-7), and
SHALL accept it only when FR-201's conditions, with FR-922 to FR-924's and
FR-877's timed rules, hold on that product. The checker links no
`fsl-analyze` code and reuses no EN-5 product, value or classification.

## Use case

A certificate whose values were computed by an engine that dropped a
`discrete` distribution from a race reaches the checker. The checker rebuilds
the race from the `delay_distribution` node, finds the drawn outcome with
probability `1/2` the certificate's values ignore, and rejects the
certificate `InequalityFails` at the first such state; the item settles
`inconclusive`, `CertificateRejected`.

## Inputs

- The document, the `ModelSystem` over the checked package and the claims
  (checked-subject form), or FR-098's wire and byte provision (wire form).

## Outputs

- `ExactProbabilityCertificateCheck` (`Accepted` or `Rejected`) or a
  `ProbabilityDocumentError` refusal.

## Behavior

### Re-derived by the checker

The checker SHALL compute, with code in the qualified core that EN-5 does not
share:

1. the route, digital exactly when every clock constraint of every guard and
   time invariant, every timed interval of the claim and every elapsed
   comparison is non-strict, the transform's `M < c` counting as strict;
2. the unit, the scale (FR-196's least common denominator) and each clock's
   cap;
3. the digital moves, or each region's time successor and each guard's
   reading on regions, the TS-5 tail moves and, for boundary regions, each
   delay edge's elapsed time;
4. each race's drawn vectors with their exact probabilities, conditioned on
   their windows, from the checked `delay_distribution` nodes, and the
   scheduler's remaining choices (FR-923);
5. the maximal end components whose actions take zero time, their exits, and
   the time-locks (FR-924);
6. the deadline and elapsed clocks, activation, completion, censoring and the
   quantile transform's terminal reward (FR-877);
7. FR-201's targets, decided states, `Prob0`, `Prob1` and tight actions.

### Not trusted

The stated `route`, `unit`, `scale` and `form` SHALL only be compared; a
difference refuses `ObjectiveMismatch` before any value is read. Every
reachable state of the rebuilt product SHALL have a row in an `exact` or
`upper` document; a missing row, or a row for a key the rebuilt product does
not reach, rejects `Malformed` at `values`. A key with a wrong cap, clock
set, unit or scale rejects `Malformed` at `values`. No transition, race,
component, time-lock or phase is read from the document, because it has none
(FR-920).

### Order

Identity, then member set, then objective comparison, then rebuild, then the
conditions in canonical key order, then the threshold; the first failure is
the result.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-921-AC-1 | FR-920-AC-1's document with one value altered is rejected `InequalityFails` at a `ProbabilisticProductState`; with its identity changed it is refused `IdentityMismatch` before the product is rebuilt. | Test (TC-2490) |
| FR-921-AC-2 | The same document with `route` set to `"region"`, or `scale` set to `"2"`, is refused `ObjectiveMismatch`; with the cap of `x` in one key changed from 3 to 2 it is rejected `Malformed` at `values`. | Test (TC-2490) |
| FR-921-AC-3 | The same document with one reachable row removed, or with one added row for a key the product does not reach, is rejected `Malformed` at `values`. | Test (TC-2490) |
| FR-921-AC-4 | `fsl-probability-check`, a separate process with no `fsl-analyze` linkage, accepts the documents of FR-922-AC-1, FR-923-AC-1, FR-924-AC-1 and FR-877-AC-1 from their bytes and the byte provision, and rejects each with one value altered. | Test (TC-2495) |

## Dependencies

- [FR-920](FR-920-emit-a-timed-product-probability-certificate.md) defines
  the document; [FR-201](FR-201-check-a-probability-certificate.md) the
  general conditions and entry points.
- FSpec FR-751 is the standard's half.
