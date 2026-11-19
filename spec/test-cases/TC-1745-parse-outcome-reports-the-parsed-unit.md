---
id: TC-1745
title: "Parse outcome reports the parsed unit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: verifies
---
# TC-1745: Parse outcome reports the parsed unit

## Description

Verify the `parsed` result of the parse outcome document. Scope: FR-663-AC-1, FR-663-AC-2, FR-002-AC-7.

## Test Procedure

1. Parse `tests/fixtures/spine-compile.native` as `agent-ix`/`test:parent` and serialize through `OutcomeDocument::from_parse`.
2. Parse a unit holding `a < b < c`, and a call with a blank label.
3. Parse the fixture twice, and with `s1.tokens` set to its token count.

Tag the tests `#[trace("TC-1745", "<AC id>")]`.

## Expected Results

- Step 1: `result` kind `parsed` with the labels, path, byte count and four S1 usage rows in order; `s1.input_bytes` used equals `bytes`.
- Step 2: a `parsed` result beside the `invalid_syntax` diagnostic; `null` for the blank label.
- Step 3: equal results; `s1.tokens` used equals its bound.
