---
id: FR-877
title: "Certify elapsed-quantile verdicts over timed subjects"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-920
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-753
    type: depends_on
---
# FR-877: Certify elapsed-quantile verdicts over timed subjects

## Description

EN-5 SHALL decide `quantile q of (elapsed from holds(A) until holds(B) within
W) <= c` and `>= c` over a timed subject by ADR-028 XF-2's transform over the
product with the measure's elapsed clock (FR-920), certify it in
`timed-product/v1` with `form: "elapsed-quantile"`, and the checker SHALL
rebuild the elapsed clock, activation, completion, censoring and the
transform's terminal reward itself.

## Use case

Over `Retx` the first `send` comes by 2 ms and succeeds with `9/10`, so
every scheduler delivers within 2 ms with probability at least `9/10`:
`quantile 0.5 of M <= 2 ms` holds, the transform's minimum being
`9/10 − 1/2 = 2/5`. A scheduler may delay the first send to 2 ms, so
`quantile 0.5 of M <= 1 ms` fails.

## Inputs

- A quantile claim over an elapsed measure of a timed subject.

## Outputs

- A `timed-product/v1` document; FR-921's check result.

## Behavior

- The measure SHALL activate at the first observation where `A` holds,
  complete at the first later observation where `B` holds, and be censored
  when the elapsed clock passes `W` first; the elapsed clock SHALL be reset
  at activation, advance with every delay while active, freeze at completion
  or censoring, and be capped at `W + 1`.
- `<= c` SHALL be decided as `min E[[activated] · ([M <= c] − q)] >= 0`,
  certified by an `exact` or `lower` document of that minimum. `>= c` SHALL
  be decided as `max E[[activated] · ([M < c] − q)] < 0`, certified by an
  `upper` document whose initial value is below 0; at exactly 0 the checker
  SHALL search the tight actions from the initial state (FR-201) and reject
  at the first activated state reached.
- The comparison `M < c` is strict, so a `>=` claim takes the region route
  (FR-922); `M <= c` keeps the digital route when every other constraint is
  closed.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-877-AC-1 | Over `Retx` (guard `self.x >= 1 ms`) under every scheduler, with `M` = `elapsed from holds(not c.late) until holds(c.late) within 4 ms`: `quantile 0.5 of (M) <= 2 ms` settles `proved` with a digital `exact` document of initial value `2/5` that the checker accepts. | Test (TC-2494) |
| FR-877-AC-2 | `quantile 0.5 of (M) <= 1 ms` settles `refuted`, the transform's minimum being `−1/2`; AC-1's item with its initial value set to `1/2` settles `inconclusive`, `CertificateRejected`, rule `InequalityFails`. | Test (TC-2494) |
| FR-877-AC-3 | `quantile 0.5 of (M) >= 1 ms` settles `proved` with a `region` `upper` document of initial value `−1/2` that the checker accepts. | Test (TC-2494) |
| FR-877-AC-4 | `quantile 0.9 of (M) >= 2 ms` settles `refuted` (sending at 1 ms gives `Pr(M < 2 ms) = 9/10`): the transform's maximum is exactly 0 and the tight actions reach the activated initial state, so no `upper` document of initial value 0 proves it (FSpec FR-753-AC-5). | Test (TC-2494) |

## Dependencies

- ADR-028 XF-2, CE-4; [FR-186](FR-186-check-probabilistic-claim-forms-at-s3.md)
  admits the form; FSpec FR-753.
