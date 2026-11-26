---
id: TC-3241
title: "An exact item with no initial state or no instance settles inconclusive"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: verifies
---
# TC-3241: An exact item with no initial state or no instance settles inconclusive

## Description

Verify that an exact probabilistic item over an empty `initial` list or an
empty instance set settles FR-125's cause, and that the certificate checker
derives the same cause from its own reading of the subject.

Scope: FR-203-AC-20, FR-203-AC-34 to FR-203-AC-37 and FR-201-AC-85.

## Test Procedure

1. Take three claims over one subject: one `on origin` with no parameter, one
   with an `over` parameter and one `on each`. Run each through `fsl analyze`
   with exact evidence over an empty `initial` list and bounded universes,
   then with those universes also empty.
2. Run them with an empty `initial` list and one universe removed.
3. Over a non-empty `initial` list whose states admit no key of the
   parameter's type, run the `over` claim and the `on each` claim, then a
   variant of each whose form EN-5 does not admit.
4. For the subjects of steps 1 and 3, call the certificate checker directly
   with a well-formed document of each claim's identity.

5. Run an exact claim over `Animal` over records typing `c` as `Animal` and
   `Dog`, then with a third overlapping population's universe removed.
6. Run a claim whose form EN-5 does not admit over an empty `initial` list.
7. Over an empty `initial` list and over a subject with no admitted key, call
   the wire form, the document form and `fsl-probability-check`; then give
   the document form another item's identity and truncated bytes.

Tag the tests `#[trace("TC-3241", "<criterion>")]`.

## Expected Results

- Step 1: every item `inconclusive`, `no-initial-state`; no diagnostic,
  certificate, counterexample or numeric entry.
- Step 2: `RequiresBound`, never `no-initial-state`.
- Step 3: every item `inconclusive`, `no-instance`.
- Step 4: `NoInitialState` for step 1's subjects and `NoInstance` for step
  3's, each written in the test as a literal expectation; no acceptance,
  rejection, `Malformed` or `Unsupported`.
- Step 5: refused `conflicting-identity` with no terminal record; then
  `RequiresBound`.
- Step 6: `inconclusive`, `no-initial-state`.
- Step 7: the typed member and `status` for each outcome; `IdentityMismatch`
  and `Malformed` for the two bad documents.
