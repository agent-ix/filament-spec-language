---
id: TC-1744
title: "Run command reaches Markdown extraction in its default build"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: verifies
---
# TC-1744: Run command reaches Markdown extraction in its default build

## Description

Verify that the command owner's default build admits FR-031's extraction mode. Scope: FR-662-AC-3.

## Test Procedure

1. Through `fsl_command::command::run`, send a clause-run request carrying `program.extraction` and two `clauses` bindings.
2. Send one binding naming a clause the document lacks.

Tag the tests `#[trace("TC-1744", "<AC id>")]`.

## Expected Results

- Step 1: `extraction-clause-count` `{"actual": 2}` at stage `request`, exit 20, not `invalid-request`.
- Step 2: FR-030's error at stage `extraction-selection` with the original identity and binding.
