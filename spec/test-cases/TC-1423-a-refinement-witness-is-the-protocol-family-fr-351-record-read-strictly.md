---
id: TC-1423
title: "A refinement witness is the protocol family's FR-351 record, read strictly"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-578
    type: verifies
  - target: ix://agent-ix/filament-specification/FR-351
    type: references
---
# TC-1423: A refinement witness is the protocol family's FR-351 record, read strictly

## Description

Verify that the refinement witness is the record realizability writes and
that its reader refuses repairs.

Scope: FR-578-AC-5, FR-578-AC-6; FSpec FR-351-AC-5.

## Test Procedure

1. Refute realizability of a protocol with a hidden choice and build a
   refinement witness for its counterexample's obligation node and trace
   position.
2. Write the witness, read it back, and build one at the next position.
3. Read the witness bytes with spacing added, with a leading-zero trace
   position, with an `index` member, with no `trace_position`, and with a
   deciding element that is a plain name.

Tag the tests `#[trace("TC-1423", "FR-578-AC-n")]`.

## Expected Results

- Step 1: the witness is the counterexample's FR-351 record on the wire.
- Step 2: the read witness is equal, the bytes carry the node's canonical
  text and the decimal position, and the next position is a different
  record.
- Step 3: each is refused.
