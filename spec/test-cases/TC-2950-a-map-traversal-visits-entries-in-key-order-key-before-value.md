---
id: TC-2950
title: "A Map traversal visits entries in key order, key before value"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2950: A Map traversal visits entries in key order, key before value

## Description

A traversal of a Map value, a closure walk or a first-reference search, uses the run form's entry array order.

Scope: FR-650-AC-11.

## Test Procedure

Supply a `Map` whose entries are given in input order `b` then `a`, each holding a different reference, and walk it for the first reference. Repeat with a key that holds a reference and a value that holds another.

## Expected Results

- The reference in the entry for `a` is visited before the reference in the entry for `b`.
- Within an entry, the key's reference is visited before the value's.
