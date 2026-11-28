---
id: TC-3219
title: "Hyper object parameters instance over admitted keys"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-151
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-172
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-176
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-181
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-183
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-315
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-3219: Hyper object parameters instance over admitted keys

## Description

Verify that a hyper object parameter has one instance per admitted key of the
populations that overlap its type, and that no route proves, refutes or
accepts a key that no initial state admits.

Scope: FR-125-AC-19 to FR-125-AC-22, FR-128-AC-17, FR-151-AC-5,
FR-172-AC-10, FR-172-AC-11, FR-176-AC-6 to FR-176-AC-8, FR-181-AC-8 to
FR-181-AC-14, FR-183-AC-13, FR-315-AC-59, FR-621-AC-23 to FR-621-AC-26.

## Test Procedure

Expected values are written by hand from the criteria, not read from the
implementation.

1. Run FR-181-AC-8's fixtures through the source `native-analyze/1` route and
   through the library engine, settlement and replay.
2. Run FR-181-AC-9 under the event-position profile and FR-181-AC-10 through
   package selection.
3. Run FR-176-AC-6's universal clause through the source route and
   `check_hyper`, and FR-176-AC-8's two requests.
4. Check and run FR-172-AC-10, FR-172-AC-11, FR-125-AC-19, FR-125-AC-20 and
   FR-125-AC-22.
5. Run each FR-125-AC-21 variant and FR-181-AC-11 to FR-181-AC-14.
6. Replay FR-183-AC-13's witness and counterexample, and build FR-315-AC-59's
   identities.
7. Run FR-176-AC-7 over source and package, FR-128-AC-17's two envelopes,
   FR-151-AC-5's declaration and FR-621-AC-23's request.
8. Settle FR-621-AC-24's two instance records in each order.
9. Combine each pair of proved records FR-621-AC-25 gives.
10. Settle FR-621-AC-26's two replaced counterexamples.

Tag the tests `#[trace("TC-3219", "FR-xxx-AC-n")]` with the criterion each
asserts.

## Expected Results

- Step 1: one instance `l`, `proved`, `excluded_keys` `[m]`; the control is
  `refuted` at `l`.
- Step 2: `proved` at `l` alone; the package item is
  `unsupported_construct`/`not-yet-implemented`, exit 21.
- Step 3: `Holds` for `l` alone, `proved`, exit 0; FR-176-AC-8's requests are
  `refuted` at `p` and then at `q`.
- Step 4: the instances, excluded keys, outcomes and verdicts each criterion
  states.
- Step 5: the outcomes of FR-125-AC-21 in its order, none a fault, and the
  records and refusals FR-181-AC-11 to FR-181-AC-14 state.
- Step 6: the unadmitted key is refused and the admitted one is not.
- Step 7: the records, refusal, instance and binding each criterion states.
- Step 8: `refuted` from the confirmed instance, and `replay-parity` only when
  none is confirmed.
- Step 9: the certification, basis and instance each case states.
- Step 10: each item is `inconclusive`, `replay-parity`.
