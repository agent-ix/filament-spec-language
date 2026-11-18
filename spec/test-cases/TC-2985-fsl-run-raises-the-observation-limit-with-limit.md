---
id: TC-2985
title: "fsl run raises the observation size limit with --limit observation.input_bytes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: verifies
---
# TC-2985: fsl run raises the observation size limit with --limit observation.input_bytes

## Description

Verify that the driver's `fsl run --limit observation.input_bytes=N` sets the
same ceiling as the request member `clause.observation_limits.document_bytes`
and reports it in the same shape. Scope: FR-287-AC-13.

## Test Procedure

1. Run FR-312-AC-9's request without `observation_limits`, its snapshot one
   byte over `N`, with `--limit observation.input_bytes=N`.
2. Run FR-312-AC-9's request with `document_bytes` `N` and no option.
3. Run step 1 with `--limit observation.document_bytes=N`.
4. Run step 2's request with `--limit observation.input_bytes=N` as well.

## Expected Results

- Steps 1 and 2: the same outcome document, exit 22, the item cause
  `stage_limit_exceeded` naming `observation.input_bytes`.
- Step 3: exit 20, the unknown-setting refusal naming the operand.
- Step 4: exit 20, `invalid-request` naming `observation_limits`.
