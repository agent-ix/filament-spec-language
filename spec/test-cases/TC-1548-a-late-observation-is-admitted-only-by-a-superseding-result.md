---
id: TC-1548
title: "A late observation is admitted only by a superseding result"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: verifies
---
# TC-1548: A late observation is admitted only by a superseding result

## Description

Verify that a late observation never rewrites the result it arrives after and is admitted by a `Superseding` request.

Scope: FR-526-AC-31.

## Test Procedure

1. Evaluate FR-501-AC-29's request as `Original` (`r1`).
2. Send `Superseding(r1)` with the same trace and cutoff and `admit_late` naming the observation's identity.
3. Re-send `r1`'s own request as `Superseding(r1)`.

Tag the tests `#[trace("TC-1548", "FR-526-AC-31")]`.

## Expected Results

- Step 1: `r1` lists the observation as late; the ledger holds `r1`.
- Step 2: lineage 1, the observation read and listed in `admitted_late`, `r1` unchanged.
- Step 3: `unchanged-input`.
