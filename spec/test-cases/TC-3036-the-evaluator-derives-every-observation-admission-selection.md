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
maximum a limit, and that the package revision and digest are equal on the
source and bare-package routes. Scope: FR-944-AC-8, FR-944-AC-9 and
FR-944-AC-10.

## Test Procedure

1. Build the admission request for FR-501-AC-13's counter-cover request under
   `ms`, with one position carrying arrival 2, one an order key and one an
   interval stamp; call `admit`; build the request a second time; vary one
   valuation, one order key and one interval, each alone; add a second
   position equal to the first in document, stamp, key and interval. Build
   the request for an event-position clause.
2. Inspect the `fsl-eval` producer bundle and admit it twice in one process;
   mutate a derived request (the binding source, the population identity, one
   record identity), each alone, before `admit` and run it through the
   evaluator; run a trace of `max_population_entries + 1` positions and a
   trace with one position whose value text is one byte over
   `max_string_bytes`.
3. Compile one package, evaluate it on the source route, emit it
   canonically, read the bytes back (and a reordered, whitespace-padded copy)
   and evaluate on the bare-package route; compile a package differing in one
   node.

## Expected Results

- Step 1: every member equals FR-944-AC-8's value and `admit` returns
  `Available`; the two builds are equal; each variation changes exactly that
  record's identity; the duplicate fact refuses at stage `admit`,
  `invalid_runtime_input`/`invalid-value` at the later position; the
  event-position request uses `EventPosition` anchors and range, unit
  `position`, and is `Available`.
- Step 2: the bundle members and digests equal FR-944-AC-9's and it is
  admitted once; each mutation ends `runtime_invariant`/
  `established-invariant-broken`, invariant `observation-admission`, with the
  owner's refusal cause, exit `30`, no clause outcome; the two over-maximum
  traces refuse `stage_limit_exceeded` with `node-count-exceeded` and
  `input-bytes-exceeded`, the owner maximum as `bound`, and the
  `filament_observation::authority::Limits` member as `setting`.
- Step 3: equal `canonical_bytes()`, `revision()` (`sha256:<hex>` of those
  bytes), `package` selections and `Context` authorities on both routes and
  for the non-canonical copy; the one-node change gives a different revision
  and digest.
