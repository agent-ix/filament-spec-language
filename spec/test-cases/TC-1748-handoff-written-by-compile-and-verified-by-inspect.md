---
id: TC-1748
title: "Handoff written by compile and verified by inspect"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: verifies
---
# TC-1748: Handoff written by compile and verified by inspect

## Description

Verify the compile request's `handoff` member and the handoff verify operation. Scope: FR-665-AC-1, FR-665-AC-2, FR-665-AC-3.

## Test Procedure

1. Compile `tests/fixtures/spine-compile.native` with FR-665-AC-1's `handoff` member.
2. Verify the directory; then a copy with an altered node count in `selection.json` and a copy with one changed byte in `checked-package.json`.
3. Send the request without `native_join`, with `corpus` `all` and with member `digest`; then repeat step 1 over the existing directory.

Tag the tests `#[trace("TC-1748", "<AC id>")]`.

## Expected Results

- Step 1: exit 0; the directory holds the package bytes, `selection.json` of format `filament.checked-handoff.selection/1`, `identities.json`, `equality.json` and `inventory.json`, and no `mutations/`.
- Step 2: success with operation `inspect`; the count's `SelectionAxis` refusal; the strict reader's typed refusal.
- Step 3: `invalid-request` naming the member and no directory; the repeat refuses naming the path and leaves the files unchanged.
