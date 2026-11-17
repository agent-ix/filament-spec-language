---
id: TC-2833
title: "A reference literal parses as one node and formats unchanged"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
---
# TC-2833: A reference literal parses as one node and formats unchanged

## Description

Verify the reference-literal production and its malformed spellings.
Scope: FR-1000-AC-1.

Catches a parser that refuses `#` as an invalid token, and one that accepts a
literal with no type or no quoted key.

## Test Procedure

1. Parse a clause whose body holds `M::T#"s1"`; render and format it.
2. Parse `M::T#s1`, `M::T#` and `#"s1"` in the same position.

## Expected Results

- Step 1: one `ReferenceLiteral` node spanning the literal; the render equals
  the input bytes; formatting is idempotent with an unchanged token stream.
- Step 2: each refuses `invalid_syntax`, at the token after `#`, the token
  after `#` and `#` respectively.
