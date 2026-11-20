# QSL-2090 scope

The cases exercise the current public source, checked-package reader, run,
call-site, replay, and model-analysis paths. Expected values, source slices,
operator families, binding names, and finite Counter paths are independent
oracles. No proof result or successful replay is manufactured.

- Expression lowering: a shared literal with three occurrences; parameter and
  let references; operator families and arities; literal narrowing and explicit
  rational conversion; flat-map identity; nested and sibling binder levels.
- Named calls: parameter-name order, missing/duplicate/foreign names, malformed
  selections and work exhaustion.
- Header profiles: an empty clause-selection lock for the value role and exact
  source regions for unknown and wrong-role selections.
- Replay: actual call-site parameter ids, source-only provision, reordered
  assignments, Boolean admission, missing and foreign parameter ids, and actual
  value and settlement outcomes.
- State clauses: selecting a new postcondition, distinct current/pre reads,
  and an entire guarded parent expression evaluated in the pre observation.
- Model behaviours: authored bounded each-activation horizons, false extension
  versus infinite stuttering, and a real terminal counterexample and replay.

## Pending features

QSL-2709 blocks QSL-2090. FR-093-AC-4's permitted function-result widening
emits bytes that the public strict reader refuses with IllTyped/TypeMismatch.
The original failure is retained. Its strict desired-success case is ignored
and excluded from passing coverage; the independently split bounded-literal
case remains active. No production fix or guessed conversion is included.

QSL-2714 also blocks QSL-2090. Two actual source/model journeys under
event-position false-extension settle `failed`/`internal-failure` instead of
their required EN-1 verdicts. The reached Enough item explored four states
and three transitions; ThroughFive reports `runtime_invariant`. The separate
infinite-profile bounded-operator counterpart also settles internal failure
after six states and three transitions. These three strict cases are ignored
and uncounted. The original Short-item assertions were not reached, so no
verdict is claimed for it. No shared root cause is invented. No more bounded
operator/profile variants are probed.

Operand references are compared using the public reader's resolved typed
structural keys. Node-held operand-slot bytes are not compared with a
document-held node id's graph-index bytes (FSpec FR-322).

This slice does not claim exhaustive completion of all historical criterion
counts. It does not rerun the matrix or duplicate the accepted key, declaration,
native-value, evaluator-clause, or replay work merely to add tags. Retired
criteria remain retired. Existing pending implementation owners stay pending.
