---
id: TC-2772
title: "An annotated let formats idempotently and keeps its checked identity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-980
    type: verifies
---
# TC-2772: An annotated let formats idempotently and keeps its checked identity

## Description

An annotated let formats idempotently and keeps its checked identity.

Scope: FR-980-AC-5.

## Test Procedure

Format FR-980-AC-5's function, format the output again, and check both sources. Test: `fsl-cst/tests/it/annotated_let.rs` `an_annotated_let_formats_and_keeps_its_tokens`.

## Expected Results

- The token order is kept, the second pass is byte-identical, and the checked package identities are equal.
