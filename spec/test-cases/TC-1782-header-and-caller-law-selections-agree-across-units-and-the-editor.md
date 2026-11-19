---
id: TC-1782
title: "Header and caller law selections agree across units and the editor"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-670
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: verifies
---
# TC-1782: Header and caller law selections agree across units and the editor

## Description

Verify how a header law combines with a caller-supplied law, that a law
header without `div` or `rem` leaves the package identity unchanged, that a
library keeps its own law, and that the editor admits law headers. Scope:
FR-670-AC-5 to FR-670-AC-7 and FR-110-AC-12's law identities.

## Test Procedure

1. Through the public compile facade, compile FR-670-AC-1's unit with a
   floor law header and a caller-supplied floor selection; with a
   caller-supplied truncating selection; then with no header and a
   caller-supplied truncating selection.
2. Compile a unit with a floor law header and only `mod` and `+`, and the
   same unit without the header; compare the two package identities and read
   the package selection.
3. Compile a library whose header selects truncating and declares `ld`, and
   a unit selecting floor that imports it and declares `q`; call `ld(-7, 3)`
   and `q(-7, 3)`.
4. Run `complete::parse_with_catalog` with `complete::header_profile_catalog()`
   over a unit whose header names each law identity, then
   `test:unknown-profile`.

## Expected Results

- Step 1: checks with floor; `invalid_package`/`conflicting-definition` at
  the header's identity literal before checking; checks with truncating.
- Step 2: both check, the identities are equal and the selection holds no
  integer-division role.
- Step 3: `-2` and `-3`.
- Step 4: no `unknown_profile` diagnostic for the law identities;
  `unknown_profile`/`unsupported-selection` for `test:unknown-profile`.
