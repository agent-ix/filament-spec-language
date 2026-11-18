---
id: TC-2953
title: "A reference literal builds one S2 value form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
---
# TC-2953: A reference literal builds one S2 value form

## Description

Verify that S2 maps the S1 `ReferenceLiteral` node to the value form
`ReferenceLiteral{type_name, key}` with the decoded key and the source span.
Scope: FR-1000-AC-7.

Catches an S2 that maps the literal to a name, a call or a string, keeps the
key's escaped spelling, or normalizes it.

## Test Procedure

1. Build a function whose body is `M::T#"s1"` through S1 and S2.
2. Build the same with the body `M::T#"é"`.

## Expected Results

- Step 1: the body's root is `ReferenceLiteral{type_name: "M::T", key: "s1"}`
  with the literal's source span.
- Step 2: the key is the one scalar `é`.
