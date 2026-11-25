---
id: TC-1387
title: "The parity runner fails coverage gaps, empty comparisons and reference faults, refuses an empty scope, and excuses nothing"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-563
    type: verifies
---
# TC-1387: The parity runner fails coverage gaps, empty comparisons and reference faults, refuses an empty scope, and excuses nothing

## Description

Verify the failure policy: coverage gaps, prepare failures, empty
comparisons, a faulting or limit-reaching reference, generator exhaustion and
unbuildable corpus cases fail; an empty scope and an unreadable corpus refuse;
and no setting excuses a finding.

Scope: FR-563-AC-4, FR-563-AC-5, FR-563-AC-6, FR-563-AC-8 and FR-563-AC-10
to FR-563-AC-14.

## Test Procedure

1. Run `run_parity` with a test backend whose coverage lists `let` and which
   lists every `let` entry unsupported; then with the same backend and `let`
   removed from its coverage.
2. Run it with a backend whose `prepare` returns `PrepareFailure::Limit` for
   one generated unit, and with a backend that lists every entry unsupported
   and supplies no coverage.
3. Under `test-support`, run `run_parity_with_reference` with a reference
   that returns `Fault` for one entry; run `run_parity` with no
   non-interpreter backend, and with a corpus location that does not exist.
4. Inspect `ParityInputs`, `ParityLimits` and `cargo xtask parity`'s options
   for a member that allows, skips, retries or tolerates a finding or selects
   a reference; build the `compile_fail` doctest naming
   `run_parity_with_reference` without the feature and its control with it;
   run `cargo xtask parity --no-such-option`.
5. Run it over the FR-563-AC-10 valid corpus and over each of its six
   document-level defects, counting backend `prepare` calls.
6. Run it, with two delegating backends and count 3, over the FR-563-AC-11
   corpus.
7. Repeat step 6 with the twelve defective vectors expecting a refusal, then
   with only the seven whose cause an expected refusal covers; run the two
   class-against-disposition unbuildable vectors, and a refusal-expecting
   vector that builds, with a delegating and with an altering backend.
8. Run one seed and count with `max_discarded` at `u64::MAX`, then at the
   discard count N and at N − 1, with two delegating backends; read the
   default `max_discarded`.
9. Under `test-support`, with two delegating backends, run a reference whose
   `prepare` returns `PrepareFailure::Limit` for the first of two corpus
   units, then for one generated unit of two or more entries, and a reference
   whose `prepare` returns `Fault` for the first corpus unit.

Tag the tests `#[trace("TC-1387", "<AC id>")]`.

## Expected Results

- Step 1: one `coverage-gap` per `let` entry; then a pass, with those entries
  listed by their `UnsupportedEntry`.
- Step 2: `prepare-failed` naming the unit; `empty-comparison`.
- Step 3: one `reference-fault` finding naming the entry, exit 1; then
  `empty-scope` before any unit and the corpus refusal, each with no verdict
  and exit 2.
- Step 4: no such member exists; the doctest fails to compile without the
  feature and the control compiles with it; the unknown option refuses,
  exit 2, naming it.
- Step 5: a report for the valid corpus; each defect refuses the corpus as
  unreadable, exit 2, with no `prepare` call.
- Step 6: twelve vectors listed by id and cause and counted, none of them
  counted not selected; twelve `unbuildable-case` findings per backend and no
  other; `inc` and the generated units compared; exit 1.
- Step 7: the twelve listed and counted, with five findings per backend for the
  `UnreadableFile`, `UnsafePath` and `MissingMember` vectors, exit 1; then the
  seven listed and counted, no finding, exit 0; the class `refusal` vector
  expecting a value is a finding and the class `positive` vector expecting a
  refusal is not; the built refusal vector passes with the delegating backend
  and is a `difference` with the altering one.
- Step 8: no finding at N; at N − 1 one `generator-exhausted` finding per
  backend naming the N-th refused draw, fewer units than a full run, exit 1;
  the default is 10,000.
- Step 9: one `reference-prepare-limit` finding per entry per backend, naming
  the limit's setting, with the other unit compared and exit 1; then one
  `reference-fault` per entry per backend with the second unit compared. No
  run refuses.
