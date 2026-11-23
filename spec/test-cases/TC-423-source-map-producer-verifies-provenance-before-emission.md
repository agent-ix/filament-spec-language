---
id: TC-423
title: "Source-map producer verifies provenance before emission"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-095
    type: verifies
---

# TC-423: Source-map producer verifies provenance before emission

## Description

Verify the producer/package-reader/consumer trust boundary for occurrence-keyed
source-map regions.

## Test Procedure

1. Compile one source occurrence under its selected definition with an exact
   region. Independently change the end beyond the source, split a multibyte
   UTF-8 scalar, point at another occurrence, and change the source identity.
2. Give the I2 reader a package whose altered region remains structurally
   valid but whose end differs from the original source length. The reader is
   given only package bytes.
3. Ask a package consumer to use that source map without binding the selected
   definition, then repeat with the matching admitted definition bound.

## Expected Results

The exact producer input emits, and each mutation refuses before emission.
The package-only reader admits and retains the structurally valid region but
does not authenticate it against absent source bytes. The consumer refuses
source-level use before definition binding and may proceed only after the
matching definition is admitted and bound.
