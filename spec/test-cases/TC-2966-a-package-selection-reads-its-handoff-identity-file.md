---
id: TC-2966
title: "A package selection reads its handoff directory's identity file and refuses without it"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
---
# TC-2966: A package selection reads its handoff directory's identity file and refuses without it

## Description

Verify that an analyze request's `package` selection names an FR-517 handoff
directory, that the analysis attaches the identity rows of its
`identities.json` and the equality rows of its optional `equality.json`
through the strict readers, and that a missing or refused identity file, a
refused equality file and an absent directory each refuse the request
`invalid_package` before any engine runs. Scope: FR-850-AC-6.

## Test Procedure

1. Write the Counter unit's handoff directory through a compile request
   carrying the `handoff` member, and remove its `identities.json`.
2. Replace `identities.json` with `{}`, then change its `format`.
3. Remove `equality.json`, then replace it with `not json`.
4. Select `{"directory": "absent"}`.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `invalid_package`, `missing-file`, naming `identities.json`,
  exit 20, `items` empty.
- Step 2: `invalid_package`, `malformed-file`, naming `identities.json`;
  then `unknown-format`.
- Step 3: the request analyzes as FR-850-AC-1 states; then
  `invalid_package`, `refused-file`, naming `equality.json`.
- Step 4: `invalid_package`, `missing-file`, naming the path.
- Assertions compare typed members, never message text.
