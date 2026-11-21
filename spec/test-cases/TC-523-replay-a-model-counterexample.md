---
id: TC-523
title: "The replay facade replays a model counterexample through ModelSystem"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: verifies
---
# TC-523: The replay facade replays a model counterexample through ModelSystem

## Description

Verify `replay_model_trace`: lassos, prefixes, stutter loops and deadlock
counterexamples reproduce; the recorded state key selects among several
post-states of one transition identity; an engine disagreement settles
`inconclusive`; a loop that does not close, an unfair lasso, a disabled step, a bad initial state and a misplaced
`terminal-stutter` step refuse.

Scope: FR-128-AC-1 to FR-128-AC-4 and FR-330-AC-20.

## Test Procedure

Take the counterexamples of TC-521 steps 1 to 3, with the subject's initial
snapshot and source in the byte provision and its universe in the request.

1. Replay the lasso under the constraint with no granularity, the bounded
   prefix and the stutter lasso. Over the `Branch` subject of FR-128-AC-1,
   replay a one-step prefix recording the state key of `v = 2`.
2. Replay the deadlock counterexample; the same payload in an envelope
   carrying the identities of the `Counter` unit whose `When` member covers
   value 3; the original payload truncated to end at value 2.
3. Replay each defective envelope of FR-128-AC-3.
4. Replay a lasso of `upd(a)` steps bound to `c = a`; step 1's lasso twice.

5. Take TC-840 step 7's source and checked-package model-arm witnesses.
   Keep the checked package, original admitted initial snapshot, universe
   inputs, authoritative byte provision and request identity; delete the
   original source files. Replay the witnesses through FR-128's model route
   using those retained bytes, including its existing package/source
   authentication. Repeat the branching case with the actually taken
   successor's key and the initially terminal empty-prefix case.
6. Independently replace a recorded post-state with a well-shaped key of a
   state that is not a successor of that transition, replace a transition
   with a disabled transition, replace the binding with a typed reference
   outside the admitted subject, break loop closure and supply an unfair
   loop. Preserve the rest of each request's identity and inputs. Use the
   structural malformed-arm controls from TC-1160 separately.

Tag the tests `#[trace("TC-523", "FR-128-AC-n")]` and the added model-run
replay checks `#[trace("TC-523", "FR-330-AC-20")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness` three times, the lasso with
  `trace_position` 0; the `Branch` prefix reproduces with `trace_position`
  1.
- Step 2: `reproduced-with-evaluated-witness`; `inconclusive`, `ReplayParity`;
  `inconclusive`, `ReplayParity`.
- Step 3: each refuses as FR-128-AC-3 states, with no result.
- Step 4: `inconclusive`, `ReplayParity`; equal results.
- Step 5: replay reproduces the actual model witness from retained bytes,
  with equal evaluated values and trace positions for source and package
  routes, even with no source file remaining. The branch key selects the
  actual successor; the initially terminal case replays its actual terminal
  key without requiring a fabricated observed snapshot.
- Step 6: every semantic defect refuses replay with no reproduced result.
  A native reader's acceptance of a well-shaped state key or typed binding
  does not count as successful model re-execution or subject authentication.
