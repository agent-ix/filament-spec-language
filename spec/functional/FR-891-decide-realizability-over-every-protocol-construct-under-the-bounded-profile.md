---
id: FR-891
title: "Decide realizability over every protocol construct under the bounded profile"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-890
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-175
    type: implements
  - target: ix://agent-ix/filament-specification/FR-735
    type: implements
  - target: ix://agent-ix/filament-specification/FR-785
    type: depends_on
---
# FR-891: Decide realizability over every protocol construct under the bounded profile

## Description

Under the profile `bounded`, realizability (FR-571) SHALL decide every checked
protocol over the `fsl-eval` protocol system's arena, which steps every
construct (FR-882), explored to the bounds and to `max_live_instances`
(ADR-036 PA-33). It SHALL settle `proved`, `refuted`, `inconclusive` or
`failed`. It SHALL settle `unsupported` only as `evaluator-unsupported`, for
`UnenumerableRecord` and `WeakMemory`.

- **Environment moves** are PA-16's, plus:
  - `activate`, `spawn` and `retire`;
  - the record and post-state of every `attempt`, `effect`, `event`,
    `commit` and compensation attempt;
  - the step at which a `join` policy fires;
  - `timeout`, which the untimed arena admits whenever its `await` waits.
- **Role moves** are the role's own enabled actions. At a compensation
  thread's retry point, the template's role also chooses between `cattempt`
  and `cend`. This choice is an owner choice, so PA-17's OR nodes and
  PA-18's avoidability conditions apply.
- **Observation.** A role's observation (PA-16) also holds:
  - its own internal actions with their records;
  - its timeouts;
  - per protocol instance, the instance ordinal and the `activate` record;
  - for a replicated role instance, its own object reference.
  A replicated class has one table, and each live instance reads that table
  over its own observation.
- **Symbolic capacity.** Each named capacity takes its value from the
  subject's `capacities` binding (FSpec FR-785), supplied with the subject's
  other bindings; it is the binding the protocol system steps under, recorded
  in premise group (a). A name the binding does not bind is recorded as not
  stated, so the key member is present, and the result is `inconclusive`,
  cause `premise-not-stated` (`PremiseNotStated`) naming the member
  `capacities`, never `requires-bound`. A stated value is part of the key
  (FR-572).
- **Instance bound.** A run that reaches `max_live_instances` without a
  counterexample settles `inconclusive`, cause `instance-bound-reached` naming
  `max_live_instances` and its value. A spawn past a role's authored `max n`
  is never enabled (ADR-027 ST-14), so it yields no refusal and no step.

Witnesses, counterexamples, the in-engine replay gate before `proved`, the
FR-351 record on a refutation and two-run determinism are FR-571's,
unchanged.

### Decision: one arena, owner choices only where a role decides

The arena is already the one transition system that steps every construct.
Realizability over it needs only two answers for each move: who makes the
move, and what the mover sees. Most new moves belong to the environment,
because no role controls them. An operation's outcome, a domain event's
record, which branch of a join finishes first, when a timer fires and when
a trigger arrives are all outside every role's control. Treating them as the
environment's is the conservative reading that FSpec FR-175 requires: a
strategy has to win against every one of them.

A compensation's retry is different. Retrying or giving up is what the
template's role decides, and the retry relation constrains a retry but never
forces one (FSpec FR-058). If the environment made that choice, a protocol
that a sensible role realizes could be refuted, which would be a false
`refuted`. The retry is therefore an owner choice. PA-18 already keeps a
refutation that rests on an avoidable owner choice `inconclusive`.

## Inputs

A realizability request (FR-571) with profile `bounded`, a subject whose
`capacities` binding (FSpec FR-785) carries a value for each symbolic
capacity the protocol names, and the
bounds including `max_live_instances`.

## Outputs

FR-571's outcomes and FR-573's settlement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-891-AC-1 | Under `bounded`, the FR-890-AC-1 protocols that project to `proved` each settle realizability `proved` with a table the in-engine walk accepts, or `inconclusive` with a named cause, never `unsupported`. A protocol whose only decision point is FR-890-AC-2's unmerged `repeat` settles `refuted` with `IndistinguishableHistories` for `Seller`, and its counterexample replays. | Test (TC-2523) |
| FR-891-AC-2 | A channel of capacity `symbolic(Bound)` with no entry for `Bound` in the subject's `capacities` binding (FSpec FR-785) settles `inconclusive`, cause `premise-not-stated`, naming the member `capacities`. With `Bound` = 1 and two sends before any receive under `overflow reject`, the result equals the result for literal capacity 1. Changing the value changes the key. | Test (TC-2523) |
| FR-891-AC-3 | A protocol `on each (t: M::Cell)` whose instances each send one `Order` from `Buyer` to `Seller` settles `proved` with `max_live_instances` 2 over two cells. `Seller`'s table distinguishes the two instances by ordinal. With `max_live_instances` 1 and two triggers admitted, the result is `inconclusive`, cause `instance-bound-reached` naming `max_live_instances` = 1. | Test (TC-2524) |
| FR-891-AC-4 | A compensation whose template role can end its thread after a failed attempt, under a recovery claim that only a retry satisfies, settles `proved`, with a table whose committed action at the retry point is `cattempt`. The same protocol with `attempts 1` settles `refuted` by `EnvironmentDefeat`, and the defeating play replays. | Test (TC-2524) |
| FR-891-AC-5 | A replicated role `Pool ... max 2` whose session instance receives `Job` and sends `Done` settles `proved` with one table for the class. A `spawn` beyond `max 2` never occurs in any explored play. | Test (TC-2524) |
| FR-891-AC-6 | A protocol under `bounded` with an `await` whose timeout branch leads to a terminal violation settles `refuted` by `EnvironmentDefeat`, the defeating play taking `timeout`. With the timeout branch repaired, it settles `proved`. | Test (TC-2523) |

## Dependencies

- ADR-036 §7, PA-16 to PA-19 and PA-33.
- [FR-571](FR-571-decide-whether-local-strategies-realize-a-global-protocol.md):
  the realizability request, witness and counterexamples.
- [FR-890](FR-890-project-every-protocol-construct-under-the-bounded-profile.md):
  the decision points and their owners.
- [FR-882](FR-882-step-every-specified-protocol-construct.md): the arena.
- FSpec FR-175 and FR-735.
