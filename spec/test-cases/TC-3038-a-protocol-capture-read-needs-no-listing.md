---
id: TC-3038
title: "A protocol capture read needs no listing"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-381
    type: verifies
---
# TC-3038: A protocol capture read needs no listing

## Description

Verify that S3 never refuses a read of a protocol capture
`wrong_snapshot`/`wrong-observation`, that a guard reading another role's
state in place of a capture is refused `wrong-observation` at the read, and
that FR-381's read classification is the only refusal of a capture read.
Scope: FR-218-AC-48 and FR-381-AC-16.

## Test Procedure

1. Check a protocol whose activation captures
   `capture amount: M::Money = input.amount;` with a `case when amount > 0`
   guard at a decision owned by each role, and no `visible` entry.
2. Replace one guard's `amount` with the field of another role's binder that
   the capture would have snapshotted, at a decision of the first role.
3. Add a capture that reads a capture written after it.
4. In the refund template of TC-1057, guard a decision of each role on the
   activation capture `amount`; add a capture reading itself, one reading a
   later capture and a registration capture reading another template's
   capture.

## Expected Results

- Step 1: the protocol checks, with no refusal.
- Step 2: `wrong_snapshot`/`wrong-observation` at the replaced read, reason
  `other-role`.
- Step 3: `missing_declaration`/`missing-name` with `read` `forward` at the
  name written.
- Step 4: the guards check with no `wrong_snapshot` refusal; the three
  captures are refused `missing_declaration`/`missing-name` at the name
  written with `read` `forward`, `forward` and `foreign`.
