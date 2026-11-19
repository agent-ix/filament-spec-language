---
id: TC-1595
title: "Analyze items reach external providers only through the registry"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-635
    type: verifies
---
# TC-1595: Analyze items reach external providers only through the registry

## Description

Verify FR-635's registry and provider routing. Scope: FR-635-AC-1 to FR-635-AC-3.

## Test Procedure

1. Start the workspace test plugin as a process provider for `value-validity` `unbounded`, pass it to `analyze_with_providers`, and analyze an item naming it.
2. Analyze `Bounded` with and without the provider.
3. Analyze with `backend` naming an unregistered id.

## Expected Results

- Step 1: the item settles `proved` on the provider with certification `trusted`; `registry` lists `en-1`, `en-7` and the provider.
- Step 2: byte-equal items.
- Step 3: `invalid-request` `unknown-backend`, exit 20.
- Assertions compare the outcome document's typed members and the exit code, never message text.
