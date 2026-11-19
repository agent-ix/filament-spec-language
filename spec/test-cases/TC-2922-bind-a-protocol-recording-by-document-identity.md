---
id: TC-2922
title: "Bind a protocol recording to the analyze request by document identity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1032
    type: verifies
---
# TC-2922: Bind a protocol recording to the analyze request by document identity

## Description

Verify the recording's document-identity binding in `native-analyze/1`. Scope: FR-1032-AC-1 to AC-4.

## Test Procedure

1. Supply the FR-895-AC-7 recording with a stated identity, then declaring it and supplied as `{file}`.
2. Bind it under a second identity.
3. State a malformed identity, a differing identity, no identity, and a snapshot's identity.
4. Name no document, a snapshot, and a `temporal-satisfaction` item in `recording`.

## Expected Results

- Step 1: `proved`, every obligation `satisfied`, `recording` echoed; equal results both ways.
- Step 2: equal `closure.premises` and `obligations`.
- Step 3: `invalid-request`, exit 20, at the named pointer, `items` empty.
- Step 4: `invalid-request` at `/items/0/recording`, exit 20.
