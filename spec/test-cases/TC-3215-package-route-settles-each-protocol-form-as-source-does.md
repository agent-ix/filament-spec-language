---
id: TC-3215
title: "The package route settles each protocol form as the source route does"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1048
    type: verifies
---
# TC-3215: The package route settles each protocol form as the source route does

## Description

Verify that each protocol form whose source behavior another requirement owns
is selected, executed and replayed from an admitted package with the source
route's records, and that no such form settles `unsupported` on the package
route. Scope: FR-1048-AC-1 to FR-1048-AC-7.

## Test Procedure

For each step, compile the unit with a handoff directory, run it as a
`program` request and as a `package` request over the handoff directory, and
compare the item records, counterexamples, replays and process exit.

1. Correlated `attempt` and `recommend` (FR-1048-AC-1), including the altered
   `arguments` replay and a package with a nonempty `related` and no clause
   member.
2. `effect`, `event`, `commit` and `fence` over a repeat, and the same actions
   behind `observe sequence Review` (FR-1048-AC-2).
3. A compensation template under `attempts 2`, a package with malformed
   template rows, and a template whose execution fails beside a sibling item
   (FR-1048-AC-3).
4. `repeat Retry` with the loop proof in its decreasing, unchanged and
   state-limited variants, the maximum-and-proof repeat and the maximum-only
   repeat (FR-1048-AC-4).
5. Replicated roles with each lifetime, `max_live_instances` 1 and 2, an
   undefined `until` and an empty universe (FR-1048-AC-5).
6. Symbolic capacity with `Bound` 1, 2, unbound, unknown, malformed and
   repeated, then with the source deleted (FR-1048-AC-6).
7. `join quorum(2)` under `cancel` and `continue`, packages with `count` 0,
   above the branches and absent, and a `predicate` join (FR-1048-AC-7).

## Expected Results

- Steps 1 to 6: equal item records, verdicts, counterexamples and replays on
  both routes; no item `unsupported`; refusals and item outcomes as the
  criteria state.
- Step 7: equal quorum results; `invalid_package`/`invalid-value` for the two range defects and the reader refusal for the absent count, all
  before any engine runs; the predicate join `unsupported`.
