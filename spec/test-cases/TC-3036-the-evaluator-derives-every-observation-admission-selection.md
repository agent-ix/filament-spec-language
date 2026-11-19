---
id: TC-3036
title: "The evaluator derives every observation admission selection"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-944
    type: verifies
---
# TC-3036: The evaluator derives every observation admission selection

## Description

Verify that `fsl-eval` builds the filament-observation admission request from
the checked package, the clause occurrence, the trace with its clock profile
and its own crate-constant producer bundle alone, that the owner admits it,
that an owner refusal of that request is an internal failure and an owner
maximum a limit, and that the package selection and its in-memory digest
are equal on the source and bare-package routes. Scope: FR-944-AC-8,
FR-944-AC-9, FR-944-AC-10 and the relabelling case of FR-944-AC-4.

## Test Procedure

1. Build the admission request for FR-501-AC-13's counter-cover request under
   `ms`, with one position carrying arrival 2, one an order key and one an
   interval stamp; call `admit`; build the request a second time; vary one
   valuation, one order key and one interval, each alone; add a second
   observation equal to the first in content, stamp, key, interval and
   arrival under another FR-106 identity; inspect a position without an
   arrival. Build the request for an event-position clause.
2. Inspect the `fsl-eval` producer bundle and admit it twice in one process;
   mutate a derived request (the binding source, the population identity, one
   record identity), each alone, before `admit` and run it through the
   evaluator; run a trace of `max_population_entries + 1` positions and a
   trace with one position whose value text is one byte over
   `max_string_bytes`.
3. Compile one package, evaluate it on the source route, emit it
   canonically, read the bytes back and evaluate on the bare-package route;
   feed the reader a reordered, whitespace-padded copy; inspect every
   document and file the run writes.
4. With a ledger retaining `Complete { through: T }` that covers one
   position, deliver a position equal to it in content, stamp, order key,
   interval and arrival under a new FR-106 identity, then the same position
   under its covered FR-106 identity.

## Expected Results

- Step 1: every member equals FR-944-AC-8's value and `admit` returns
  `Available`; the two builds are equal; each variation changes exactly that
  record's identity; no record value carries an FR-106 identity; the equal
  observation admits as the same single record and member, and reports
  about it name both FR-106 identities; the arrival-less position carries
  its event time as ingestion (interim rule) and no report carries it; the
  event-position request uses `EventPosition` anchors and range, unit
  `position`, and is `Available`.
- Step 2: the bundle members and digests equal FR-944-AC-9's and it is
  admitted once; each mutation ends `runtime_invariant`/
  `established-invariant-broken`, invariant `observation-admission`, with the
  owner's refusal cause, exit `30`, no clause outcome; the two over-maximum
  traces refuse `stage_limit_exceeded` with `node-count-exceeded` and
  `input-bytes-exceeded`, the owner maximum as `bound`, and the
  `filament_observation::authority::Limits` member as `setting`.
- Step 3: `canonical_bytes()` equals the emitted and consumed bytes exactly
  on both routes; equal `package` selections (revision
  `filament.checked-package/v2`, equal digest) and `Context` authorities;
  the non-canonical copy is refused `noncanonical_wire` and admits nothing;
  no document or file FSL writes carries the digest.
- Step 4: the relabelled position is refused before `revise` is called, a
  contradiction on `Completeness`, `invalid_runtime_input`/`invalid-value`,
  with the clock name, the retained assertion and the position, the ledger
  unchanged; the position under its covered identity passes the check and
  `revise` accepts it.
