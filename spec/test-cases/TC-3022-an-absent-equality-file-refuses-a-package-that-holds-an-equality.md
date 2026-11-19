---
id: TC-3022
title: "An absent equality file refuses a package that holds an equality application"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: verifies
---
# TC-3022: An absent equality file refuses a package that holds an equality application

## Description

Verify that a `package` selection whose handoff directory lacks `equality.json` is refused at the request stage when the package holds an equality application, settles with the source route's record when the file is present, and is admitted without the file when the package holds no equality application. Scope: FR-850-AC-10.

## Test Procedure

1. Write the Counter unit's handoff directory through a compile request carrying the `handoff` member, remove its `equality.json`, and analyze it with the item `{"claim": "Bounded"}`.
2. Restore `equality.json` and analyze the same request, and the source request with the same items.
3. Write the handoff directory of the Counter unit whose `Inc` postcondition is `self.value > pre(self.value)`, remove its `equality.json`, and analyze the same request over it and over that unit's source.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `invalid_package`, `missing-file`, naming `equality.json`, exit 20, `items` empty, no engine run.
- Step 2: `Bounded` settles `proved` with a record equal to the source route's.
- Step 3: the package's `checked_term_data` is absent or its `equality_decisions` empty; `Bounded` settles with a record equal to the source route's, exit 0.
- Assertions compare typed members, never message text.
