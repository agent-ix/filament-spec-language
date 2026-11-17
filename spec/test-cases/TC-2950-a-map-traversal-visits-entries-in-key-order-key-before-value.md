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

A traversal of a Map value, a closure walk or a first-reference search, uses FSpec FR-144 canonical key order, mathematical for Int and Decimal keys, which can differ from the run form array order.

Scope: FR-650-AC-17.

## Test Procedure

Supply a `Map<Integer, ...>` whose entries are given in input order key 10 then key 9, each holding a different reference, and walk it for the first reference. Repeat with a `Map<Config::Link, ...>` whose key is a record with a `to` reference field and whose value is another reference, two entries in non-canonical input order.

## Expected Results

- The reference in the entry for key 9 is visited before the reference in the entry for key 10.
- In the record-keyed map, the first reference found is the key's `to` reference of the canonical-first entry, before that entry's value reference.
