---
id: TC-2504
title: "A FIFO key that is not FIFO-key-stable refuses at S3 at the key"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-881
    type: verifies
---
# TC-2504: A FIFO key that is not FIFO-key-stable refuses at S3 at the key

## Description

Verify the S3 key rule, its locus and message, its agreement with the standard's reader, and per-protocol independence. Scope: FR-881-AC-1, FR-881-AC-2, FR-881-AC-3.

## Test Procedure

1. Check FIFO channels keyed by `Float64[nearest-even]`, `Option<Float32[nearest-even]>`, a float record field and an event-typed value.
2. Check every carries and key vector of FSpec TC-1770 and read each written package with the FR-726 reader.
3. Check a unit with a float-keyed protocol and a good protocol.

## Expected Results

- Step 1: `ill_typed`/`operator-ineligible` at the key span, retaining `=`, channel, key text and type; message `key type <T> of channel <C> is not FIFO-key-stable`; no S4 refusal.
- Step 2: S3 admits exactly the vectors the reader admits.
- Step 3: one refusal at the key for the first; the second protocol's nodes are written.
