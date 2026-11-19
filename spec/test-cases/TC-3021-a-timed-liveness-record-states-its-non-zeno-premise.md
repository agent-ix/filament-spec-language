---
id: TC-3021
title: "A timed liveness record states its non-Zeno premise"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: verifies
---
# TC-3021: A timed liveness record states its non-Zeno premise

## Description

Verify `ZoneCheckRecord::non_zeno` and its FSpec FR-416 wire member on each settled liveness-product record and its absence on other records. Scope: FR-235-AC-7, FR-235-AC-8 and FR-245-AC-10.

## Test Procedure

1. Settle FR-240-AC-1's response claim over `Rpc` with `T = 4 ms`, its `reply` self-loop variant and its `echo` variant through `settle_zone_check`.
2. Settle `always eventually holds(true)` over `Stall`, and `NoLateReply` over `Rpc`, and FR-239-AC-5's undefined-evaluation refutation.
3. Run `fsl analyze` over FR-632-AC-2's `Within` and `Early` on the zone and digital-clock routes.
4. Check the response proof's and the reply-loop proof's certificates, each with `non_zeno` swapped and removed, and `Rpc`'s time-lock-freedom certificate given a `non_zeno`.

## Expected Results

- Step 1: `NotRequired`, `Assumed`, then `Checked` on the refutation.
- Step 2: `Inconclusive(NoAdmittedBehaviour)` with `Refuted`; `NoLateReply` and the undefined-evaluation refutation with `None`.
- Step 3: `Within` carries the same `non_zeno` on both routes, `not-required` or `assumed`; `Early` carries `checked` on both.
- Step 4: the produced certificates carry `NotRequired` and `Assumed` and are `Accepted`; every altered certificate is `Rejected`, `NonZenoMismatch` at `CertificateMember{non_zeno}`.
