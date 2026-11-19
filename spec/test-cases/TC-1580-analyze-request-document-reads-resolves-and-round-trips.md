---
id: TC-1580
title: "An analyze request document is read, resolved and round-trips"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: verifies
---
# TC-1580: An analyze request document is read, resolved and round-trips

## Description

Verify that a `native-analyze/1` document is read under FR-026's envelope, that its items resolve by declared name, and that an admitted item round-trips through its echoed FR-331 request item. Scope: FR-620-AC-1 to FR-620-AC-9, FR-281-AC-23.

## Test Procedure

1. Over the Counter fixture, analyze the item `{"claim": "Bounded"}` and read the item's `request` member.
2. Rewrite that member as a document item (drop `request_index`, `extent`, `candidates`; claim by name) and analyze it again.
3. Analyze documents whose item carries `request_index`, `extent`, `candidates`, `temporal.fairness` or an unknown member, and one whose request carries an unknown member.
4. Analyze the items `Bounded` and `Missing` together.
5. Analyze the Counter document with no `items`.
6. Analyze with `deadline_ms` 0, then without it.
7. Analyze two `Bounded` items, the second with `temporal.limits.max_states` 1.
8. List `Engines::fsl()` registrations.
9. Hand the analyze document to the run reader and a `native-run/1` document to the analyze reader.

## Expected Results

- Step 1's `request` has `request_index` 0, the clause occurrence key, kind `temporal-satisfaction`, `item_kind` `formula`, and every limit at EN-1's default.
- Step 2's `request` equals step 1's.
- Step 3 refuses each at the request stage with `invalid-request` naming the member, exit 20, no item run.
- Step 4 settles `Missing` `invalid-request` `unknown-claim` and `Bounded` `proved`, exit 20.
- Step 5 lists each claim in declaration order, then the derived deadlock-freedom item.
- Step 6 settles `incomplete` with the deadline cause, exit 22, then `proved`.
- Step 7 settles `proved` then `incomplete` `LimitReached{max_states}`.
- Step 8 includes `en-7` advertising `schedulability`.
- Step 9 refuses both with `invalid-request`, exit 20.
- Assertions compare the outcome document's typed members and the exit code, never message text.
