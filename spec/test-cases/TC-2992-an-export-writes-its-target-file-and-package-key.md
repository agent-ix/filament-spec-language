---
id: TC-2992
title: "An export writes its target file and its package key"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: verifies
---
# TC-2992: An export writes its target file and its package key

## Description

Verify that FSL writes the export's target file from the request member and from the request override, refuses an existing path and a doubled destination, and writes the FSpec FR-299 package key. Scope: FR-1015-AC-3 and AC-4.

## Test Procedure

1. Export FR-756-AC-1's counter with `output` `{"file": "counter.tla"}`.
2. Export the same request without the member through `RequestOverrides::output` naming `counter.tla` in a fresh directory.
3. Repeat step 1 with `counter.tla` present.
4. Export with the member and the override together.
5. Export the counter twice, then with `limits` `{"export.nodes": 99999}`, and read each `package` member.

## Expected Results

- Step 1: the file equals `target_text`; `output` names the path and `bytes_written` equals the text length; exit 0.
- Step 2: byte-identical file.
- Step 3: `invalid-request` naming the path, exit 20, `last_stage` null, bytes unchanged.
- Step 4: `invalid-request` naming `output` and `--output`, exit 20, no file.
- Step 5: the members hold `source_package`, `target_profile`, `generator` and `limits` with the three defaults; the first two are equal; the third differs only in `export.nodes`.
