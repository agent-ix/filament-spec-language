---
id: FR-893
title: "Reach and report every compensation outcome in FSpec FR-236's partition"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-207
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-236
    type: implements
  - target: ix://agent-ix/filament-specification/FR-058
    type: implements
  - target: ix://agent-ix/filament-specification/FR-736
    type: implements
---
# FR-893: Reach and report every compensation outcome in FSpec FR-236's partition

## Description

The protocol system (FR-206, FR-207) SHALL reach every compensation outcome
that FSpec FR-236 defines, and SHALL report each one in FR-236's partition.
That partition has three kinds of fact, kept apart: a per-attempt outcome, a
recovery-relation aggregate fact, and a duplicate-business-effect fact. The
system SHALL also report FSpec FR-736's instance recovery summary.

**Per-attempt outcomes.** A compensation thread's attempt steps are ADR-027
ST-11, ST-16 and ST-17:

| Step | Outcome | Model state |
| --- | --- | --- |
| `cattempt(c, r)` (ST-11) | `successful` | the operation's post-state is written |
| `cfail(c, r)` (ST-16) | `failed` | unchanged |
| `ctimeout(c, r)` (ST-17) | `timed-out` | unchanged |

Each of these steps binds an attempt record of the template's attempt type,
satisfies the retry relation at a retry, and advances the attempt count, so
`attempts n` bounds all three together (FSpec FR-058-AC-8). An attempt keeps
its own outcome for ever. A later successful retry does not relabel an
earlier failed or timed-out attempt (FSpec FR-236-AC-4).

**Retry.** A *retried* attempt is any attempt after a registration's first.
It records `retry_of`, the index of its predecessor attempt, under the
retry relation. Being retried is a relation between two attempts, not an
outcome label, so a retried attempt still has one of the three outcomes
above.

**Recovery-relation aggregate fact.** Each registration has one aggregate
fact, derived from its status:

| Registration status | Aggregate fact |
| --- | --- |
| `activated`, thread still running | `pending` |
| `recovered` (`cend`, recovery predicate true) | `successful` |
| `unrecovered` (`cend`, recovery predicate false) | `failed` |
| `timed-out` (`cexpire`, ADR-027 ST-18) | `timed-out` |
| `registered`, or closed without activating | absent |

An absent fact is reported as absent, never as `pending` or `failed`
(FR-236 "Absence"). A registration that closes without activating is
*untriggered*. That is a lifecycle state (FSpec FR-173 `registered`, then
closed), not an outcome.

**Duplicate business effect.** In exploration, an effect occurrence is
established once per occurrence key (FR-894), so the protocol system never
produces a `duplicate` fact. The `duplicate` fact arises from a recorded
execution (FR-895), whose observations can carry two business-effect keys
for one occurrence.

**Instance summary.** Each protocol instance has FSpec FR-736's recovery
summary over its registrations. Every reachable state carries it for each
live instance, and the `finish` step carries it for the finished instance.

**Where the outcomes appear.** The outcomes, the `retry_of` relation, the
aggregate facts and the summary are carried in four places:

- the state key's `control` member, so two states that differ in an outcome
  are different states;
- the states of every protocol counterexample (FSpec FR-433), so replay
  reproduces them;
- the protocol-analysis record (FR-574);
- the protocol obligations a claim reads, under the names `outcome`,
  `retry_of`, `recovery` and `summary`.

### Decision: FR-236's partition is the vocabulary, and the five ticketed outcomes map onto it

A user asking for "failed, timed out, retried, untriggered and partially
recovered" is naming four different kinds of fact.

- *Failed* and *timed out* are per-attempt outcomes, and also aggregate
  labels.
- *Retried* is a relation between attempts.
- *Untriggered* is a lifecycle state with no fact.
- *Partially recovered* describes an instance in which some registrations
  recovered and others did not.

FSpec FR-236 already fixes the per-attempt and aggregate vocabulary, and it
forbids approximating one kind of fact from another. FSL therefore uses that
vocabulary unchanged, adds the attempt relation it lacks, and takes the
instance summary from FSpec FR-736, which the standard now holds because no
per-registration fact can express it.

Failure and timeout of an attempt are environment steps. The outcome of an
operation in a deployed system is reported by that system, as a recovery
record is (FSpec FR-730). The untimed protocol system admits a timeout at
every point, as it does for an `await`, so every deadline's behaviours are
explored.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-893-AC-1 | Over ADR-027 §7.1's `Pay` with `attempts 2`, the reachable states include a registration whose first attempt is `failed` (ST-16), one whose first attempt is `timed-out` (ST-17), and one whose second attempt is `successful` with `retry_of` 1, the first attempt keeping `failed`. No state holds three attempts. | Test (TC-2526) |
| FR-893-AC-2 | The reachable states include a registration with aggregate fact `successful` (`cend`, `r.bal = 0`), `failed` (`cend`, `r.bal = 1`), `timed-out` (`cexpire`) and `pending` (thread running), and, in the variant whose `event(Fail)` has `failed` false, a registration that closes at `finish` with no aggregate fact. | Test (TC-2526) |
| FR-893-AC-3 | A protocol with two templates, `RefundA` and `RefundB`, both activated, reaches a finished instance whose summary is `partially-recovered` (one `successful`, one `failed`), one that is `recovered`, one that is `unrecovered`, and, when neither activates, one that is `untriggered`. | Test (TC-2527) |
| FR-893-AC-4 | The per-attempt outcomes, aggregate facts and summaries of AC-1 to AC-3 are distinct values. No attempt carries `duplicate` or `pending`. No exploration produces a `duplicate` fact. Two states that differ only in one attempt's outcome have different keys. | Test (TC-2527) |
| FR-893-AC-5 | A saga in which no trigger occurs reports no per-attempt outcome, no aggregate fact and the summary `untriggered` for every finished instance. A claim that every finished instance's summary is `recovered` or `untriggered` is refuted with a counterexample that reaches `partially-recovered` and replays with the same outcomes. | Test (TC-2527) |

## Dependencies

- [FR-207](FR-207-run-compensation-templates-as-protocol-threads.md) and
  ADR-027 ST-11, ST-12 and ST-16 to ST-18: the compensation steps.
- [FR-894](FR-894-keep-message-delivery-and-business-effect-identities-apart.md):
  effect occurrence keys.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  recorded executions.
- FSpec FR-058, FR-173, FR-236 and FR-736.
