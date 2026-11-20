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

Verify the compile request's `handoff` member and the handoff verify operation. Scope: FR-665-AC-1, FR-665-AC-2, FR-665-AC-3 and FR-665-AC-5: every `inspect --handoff` refusal carries an `input` locus naming the directory as written and the member file.

## Test Procedure

1. Compile `tests/fixtures/spine-compile.native` with FR-665-AC-1's `handoff` member.
2. Verify the directory; then a copy with an altered node count in `selection.json` and a copy with one changed byte in `checked-package.json`.
3. Send the request without `native_join`, with `corpus` `all` and with member `digest`; then repeat step 1 over the existing directory.
4. Run `inspect --handoff handoff` with `selection.json` removed, then as `inspect --handoff ./handoff/`; run `inspect --handoff absent`; inspect step 2's node-count copy and a copy whose `checked-package.json` `contract_version` is `filament.checked-package/v1` and read each diagnostic's `locus`.

Tag the tests `#[trace("TC-1748", "<AC id>")]`.

## Expected Results

- Step 1: exit 0; the directory holds the package bytes, `selection.json` of format `filament.checked-handoff.selection/1`, `identities.json`, `equality.json` and `inventory.json`, and no `mutations/`.
- Step 2: success with operation `inspect`; the count's `SelectionAxis` refusal; the strict reader's typed refusal.
- Step 3: `invalid-request` naming the member and no directory; the repeat refuses naming the path and leaves the files unchanged.
- Step 4: `handoff-read` with `{"kind": "input", "member": "selection.json", "operand": "handoff", "pointer": null}`, exit 20, then the same with `operand` `./handoff/`; `handoff-read` with `member` `null` for `absent`; `handoff-read-back-differs` at `member` `selection.json`; `handoff-reader` at `member` `checked-package.json`, `pointer` `/contract_version`, for the `v1` header copy; no `artifact` locus.
