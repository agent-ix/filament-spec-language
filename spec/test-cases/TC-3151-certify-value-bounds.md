---
id: TC-3151
title: "Certify value bounds with lower and upper certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: verifies
---
# TC-3151: Certify value bounds with lower and upper certificates

## Description

Verify that a `ValueBounds` proof carries a `lower` and an `upper`
`value-bounds/v1` document, that the checker applies each kind's conditions
over the base profile's product, and that forms with no dyadic method settle
a typed incomplete result.

Scope: FR-1042-AC-2 to FR-1042-AC-5, FR-1042-AC-7, FR-1042-AC-8.

## Test Procedure

1. Over the cyclic `link` chain (`s` to `s`, `g`, `f`, `1/3` each) offer the
   documents of FR-1042-AC-2.
2. Over `W` with no fairness set offer the maximum and minimum documents of
   FR-1042-AC-3; under `fair strong go` the documents of FR-1042-AC-4.
3. Over the `Health` chain offer FR-1042-AC-5's `lower`/`upper` pair for
   `probability >= 0.75 [always[0,2] holds(ok)]`, then with the rank 2
   changed to 3, then with `upper` `13/16` at horizon 2.
4. Settle FR-201-AC-27's long-run claim with `max_rational_bits` 4 and
   FR-201-AC-35's expected-reward claim with `max_rational_bits` 1.
5. Run step 1 pre-cancelled and with each caller limit exhausted; offer
   either document with another identity and with a changed `base`.

Tag the tests `#[trace("TC-3151", "FR-1042-AC-n")]`.

## Expected Results

- Step 1: accepted; `InequalityFails` at `s` twice; `ThresholdMissed` at
  `s`; `Malformed` at `ranking`; `Malformed` at `kind`; `InequalityFails`
  at `f`.
- Step 2: `proved` with both accepted; `RankBroken` at `s`; `Malformed` at
  `policy`; the minimum `upper` accepted with `wait` and rejected
  `InequalityFails` with `go`; the fair pair accepted and `upper` `1/4`
  rejected `RankBroken` at `s`.
- Step 3: `ValueBounds{25/32, 7/8}`; `RankBroken`; `InequalityFails` at
  the horizon-2 key.
- Step 4: each `Incomplete(LimitReached{MaxRationalBits, …,
  exact_prob.max_rational_bits})`, no certificate.
- Step 5: `incomplete` with the actual limit; refusal before enumeration;
  objective mismatch.
