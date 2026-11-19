---
id: TC-1746
title: "Refusal records carry a locus"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: verifies
---
# TC-1746: Refusal records carry a locus

## Description

Verify the `locus` member of refusal records. Scope: FR-663-AC-3.

## Test Procedure

1. Run a clause over a current snapshot whose `child` lacks `parent`.
2. Add an unknown member `extra` to `child` instead.
3. Run with an absent request file.

Tag the tests `#[trace("TC-1746", "<AC id>")]`.

## Expected Results

- Step 1: `missing-member` with a document locus: the snapshot's labels, the pointer to `child`, `object` `{config_history, child}`, `member` `parent`.
- Step 2: `unknown-member` with `member` `extra` and its pointer.
- Step 3: `locus` present and `null`.
