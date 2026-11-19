---
id: TC-1542
title: "Previous, next and weak until are refused naming their derived forms"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-611
    type: verifies
---
# TC-1542: Previous, next and weak until are refused naming their derived forms

## Description

Verify the S1 refusal of `previous`, `next` and `weak until` with messages naming the derived forms, that the words stay identifiers elsewhere, and that the derived forms carry strong previous, next and bounded weak until.

Scope: FR-611-AC-1 to FR-611-AC-3.

## Test Procedure

1. Parse `previous holds(c.level = 0)`, the same under `always ... implies`, and `next holds(c.level = 1)`.
2. Parse `holds(c.level = 0) weak until holds(c.level = 2)`, and a record with a field `next` read as `holds(c.next = 1)`.
3. Evaluate the derived forms over every `Counter` trace of length 1 to 4 with values 0..2 against a reference.

Tag the tests `#[trace("TC-1542", "FR-611-AC-n")]`.

## Expected Results

- Step 1: `invalid_syntax` at the word; messages contain `once[1,1]` and `eventually[1,1]`.
- Step 2: `invalid_syntax` at `weak`, message contains `or always`; the field read parses.
- Step 3: every outcome equals the reference.
