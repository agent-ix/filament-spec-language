---
id: TC-1417
title: "Worker count, a hash-indexed seen set and a packed state leave results unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: verifies
---
# TC-1417: Worker count, a hash-indexed seen set and a packed state leave results unchanged

## Description

Verify that exploration, model checking and statistical sampling return the
same result for every worker count, that budgets stop a run at the same
state whatever the count, that a hash-indexed seen set never coalesces two
states with different key bytes, and that a packed state layout is a
bijection with the canonical key bytes, which stay every output's identity.
Scope: FR-101-AC-16 to FR-101-AC-22, FR-126-AC-20, FR-191-AC-6 and
FR-191-AC-7.

## Test Procedure

Integration tests in `qsl-eval/tests/it/` through the public entries
(`explore_request`, `explore_request_parallel`, `replay`, `check_model` and the
statistical run) and, for steps 5 to 7, the `SeenSetProbe` and `PackedLayout`
test seam of FR-101 under each explorer crate's `test-support` feature. The
statistical runs stay small (`max_samples` at most 1,000) so the case is cheap. Each
step runs the same request with `workers` 1, 2, 4 and 64 and compares the
results, except where it says otherwise.

1. A test system of at least 200 reachable states with findings on some;
   then the same system whose `successors` sleeps a pseudo-random time per
   state.
2. `workers` 0; a run stopped by `max_states` 5 on a recording system.
3. A system with a wide first level run with `max_states` stop,
   `max_transitions` stop, and two states that stop with different causes,
   the later in canonical order made to finish first.
4. A run whose poll answers `true` after a delay, `workers` 4.
5. Through the probe, on each explorer (`explore_request`, `explore_request_parallel`, the FR-126 and FR-142 product explorers): a constant hash, over 100 distinct states and over two initial states with equal hashes and different keys; then two other hash functions.
6. Pack and unpack every reachable state of a finite system and compare every pair
   of 200 states, including `float64` zero and negative zero and each end of
   a declared integer range; then a state with a value outside the layout.
7. On each explorer that has a `PackedLayout` (FR-101-AC-22), `Packing::Auto` and `Packing::Off`, and replay of each run's trace against the other.
8. `check_model` over the `Counter` subject with a violated claim, a holding
   claim with a certificate and a claim stopped by `max_states` 5; with
   `max_candidates` set so the one-worker run stops at its seventh expanded
   state, with a later state made to finish first; with a delayed cancel;
   and with `workers` 0.
9. The statistical runs of FR-191-AC-6 (`max_samples` 1,000 and 100) with a sampler double that finishes
   higher trace indices first; a delayed cancel with `workers` 4; `workers` 0.

Tag each test `#[trace("TC-1417", "<AC id>")]` with its AC.

## Expected Results

- Step 1: equal `Exploration` values for every count, order included.
- Step 2: `NotSimulated::Workers` with `invalid_runtime_input`/`invalid-value`
  and no system call; the stopped run has `Stats.states` 5, no finding past
  the stop, and equals the one-worker run whatever the call count.
- Step 3: the same limit and value, the same stopped state and cause, the
  same frontier for every count.
- Step 4: `Cancelled` with the stats, findings and frontier of the
  one-worker run cancelled after `n` merged states, for the reached `n`.
- Step 5: `Stats.states` 100 and results equal to the default hash's;
  both initial states admitted.
- Step 6: `unpack(pack(s))` equals `s`, with equal canonical key bytes;
  equal packed forms exactly when key bytes are equal; zero and negative
  zero stay two states; the out-of-layout state coalesces by key bytes.
- Step 7: byte-equal results, no packed form in any output, replay succeeds
  both ways.
- Step 8: equal verdict, counterexample, certificate bytes and stop for
  every count; the same `MaxCandidates` stop at the seventh state; `Stopped{Cancelled, None}`
  with counts and frontier of a one-worker prefix for the delayed cancel; the `workers` 0 refusal.
- Step 9: byte-equal results and provenance; the `max_samples` 100 run counts
  no sample beyond trace index 99; the cancelled run equals the one-worker
  result at its trace index; the refusal.
