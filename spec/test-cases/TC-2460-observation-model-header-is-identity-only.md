---
id: TC-2460
title: "An observation's model header carries its identity only"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-868
    type: verifies
---
# TC-2460: An observation's model header carries its identity only

## Description

Verify that a `version` or `digest` in an observation's `model` member is refused. Scope: FR-868-AC-1 to FR-868-AC-3.

## Test Procedure

1. Run FR-108's healthy-parent `ParentOrder` clause request with the snapshot's `model` carrying `version`, then `digest`, then both.
2. Run it with `model` `{"identity": "example/config-version"}` and with another identity.
3. Run a postcondition request whose invocation document's `model` carries `version`; compile with an unknown member in a `models` entry.

## Expected Results

- Step 1: `invalid_runtime_input`/`unknown-member` at stage `admit` naming `model.version`, `model.digest` and the first in member order; exit 20.
- Step 2: success, exit 0; `invalid_model_binding`/`wrong-model-selection`.
- Step 3: the invocation refuses naming `model.version`; the compile refuses the unknown member.

Test: `tests/it/observation_model_header.rs`.
