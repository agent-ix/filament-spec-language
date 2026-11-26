---
id: TC-3240
title: "A require-certified request settles a proof that is not certified inconclusive"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1052
    type: verifies
---
# TC-3240: A require-certified request settles a proof that is not certified inconclusive

## Description

Verify that `require_certified` turns an `uncertified` or `trusted` proof into
`inconclusive`, `proof-not-certified`, on every route, and changes nothing
else.

Scope: FR-1052-AC-1 to FR-1052-AC-8.

## Test Procedure

Run each fixture of the criteria once without the policy and once with it,
through `native-analyze/1` for the request member and through `analyze` and
`provider::run` for the library members. Expected values are written by hand
from the criteria. Tag each test with `#[trace("TC-3240", ...)]` and the
criterion it asserts.

## Expected Results

- Without the policy every record is `proved` with its own label.
- With it, an `uncertified` or `trusted` proof is `inconclusive`,
  `proof-not-certified`, with its basis and certification and no
  `certification` member; a `certified` proof and every other result are
  unchanged.
- A `require_certified` that is not a boolean refuses the request.
