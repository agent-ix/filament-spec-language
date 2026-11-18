---
id: TC-2952
title: "A malformed reference literal refuses at the offending token"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
---
# TC-2952: A malformed reference literal refuses at the offending token

## Description

Verify the solid-spelling rule, the string-only key, the `#` token's
disambiguation from strings and comments, and the CST children of an
admitted literal. Scope: FR-1000-AC-6.

Catches a parser that skips whitespace around `#`, admits an integer key,
reads `#` inside a string or comment as a token, or builds the literal with
other children.

## Test Procedure

1. In a clause body, parse `M::T #"s1"`, `(x)#"s1"`, `f(x)#"s1"`,
   `M::T#"a"#"b"`, `M::T# "s1"`, `M::T#1`, `M::T#)` and `M::T#"\q"`.
2. Parse a source that ends in `M::T#`.
3. Parse bodies holding `"a#b"`, a `// a#b` comment and `M::T#""`.
4. Parse `M::T#"s1"` and read its `ReferenceLiteral` node's children.

## Expected Results

- Step 1: `invalid_syntax`/`unexpected-token` at the first `#` for the first
  three, at the second `#` for the fourth, at `"s1"`, `1` and `)` for the
  next three, and `invalid_syntax`/`invalid-escape` at the string for the
  last.
- Step 2: `invalid_syntax`/`unexpected-end`.
- Step 3: each is admissible; the string and the comment hold no `#` token.
- Step 4: the children are the `QualifiedName` node over `M::T`, the `#`
  token and the string token, in that order.
