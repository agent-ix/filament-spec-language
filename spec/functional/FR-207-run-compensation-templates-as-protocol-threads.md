---
id: FR-207
title: "Run compensation templates as protocol threads"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-056
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-057
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-058
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-427
    type: depends_on
---
# FR-207: Run compensation templates as protocol threads

## Description

The protocol system SHALL run compensation templates as FSpec FR-427
states: registration on the forward effect, activation on the first
eligible trigger, compensation attempts on a compensation thread under the
authored attempt count and retry relation, closure at the commit and
`cend` with its recovery status (ADR-027 CP-1 to CP-5, ST-11, ST-12),
folding registration and activation into the step that causes them
(ADR-027 FO-1).

## Use case

A verification operator models a payment that is refunded when a later
failure event occurs. They need the refund to be explored in every
interleaving where the failure happens, to run no more times than the
template allows, to stop being possible once the payment commits, and to
report whether the account recovered, so that a claim about recovery is
checked over every behaviour.

## Inputs

- A protocol state (FR-205) and the checked compensation templates of the
  protocol clause (FR-218): forward effect node, `as` binder, registration
  and activation captures, trigger type and activation guard, operation and
  role, attempt type, `attempts n`, retry relation, `commit` node or `commit
  never`, and recovery predicate.

## Outputs

- Compensation registrations in the state key's `control` member with
  status `registered`, `activated`, `recovered` or `unrecovered` and their
  captures; compensation threads with their attempt counts in
  `bounds` and their attempt records in `observations`; `cattempt(c, r)` and `cend(c, r)` steps.

## Behavior

- The protocol system SHALL apply FSpec FR-427's registration, activation,
  `cattempt`, `cend` and commit rules, keying each registration by
  (template, the effect's thread path, ordinal).
- The protocol system SHALL apply each `cattempt` through FR-206's
  `attempt` rule for the template's operation by its role, and SHALL take
  one `cattempt` step per record of the template's attempt type, at a retry
  only each record the retry relation admits with `earlier` bound to the
  thread's attempt record and `later` to the step's record, as FSpec FR-427
  states (ADR-027 ST-11). The step's record SHALL be the thread's attempt
  record and part of the step's transition identity.
- The protocol system SHALL keep `cend` enabled whether or not a retry is.
- The protocol system SHALL mark a protocol instance finished in its
  `finish` step, which FR-206 enables only after every other thread of the
  instance, compensation threads included, has left `running`, and SHALL
  remove the instance with its registrations in that step (FR-209).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-207-AC-1 | ADR-027 §7.1 (`Pay`, `x.bal = 0`), with the compensation thread's fail, timeout and expiration steps (ST-16 to ST-18, FR-893) and the registration keeping each attempt's outcome in `control` until the instance finishes: the reachable states are exactly 17 and the transitions 22. They are t0 to t8 of its table and eight more: from t4 (`bal` 1, at the attempt, count 0), `cattempt` to t6, `cfail` to F (`bal` 1, retry point, count 1, attempt `failed`), `ctimeout` to T (the same with attempt `timed-out`) and `cexpire` to E0 (`bal` 1, no thread, registration `timed-out`, no attempt); from t6, `cend` to t7 and `cexpire` to Es (`bal` 0, `timed-out`, attempt `successful`); from F, `cend` to RF (`bal` 1, `recovered`, attempt `failed`) and `cexpire` to EF (`bal` 1, `timed-out`, attempt `failed`); from T, `cend` to RT and `cexpire` to ET (the same with attempt `timed-out`); and `finish` from t7 and Es to t8 and from E0, RF, EF, RT and ET to t5, since a finished instance leaves only the model state. `event(Paid)` registers `Refund` in the same step; `event(Fail)` with `failed` true activates it in the same step and creates the compensation thread at count 0; with `failed` false the registration stays `registered` until `finish`, which removes the instance with its registration, so t5 holds no instance and no registration. | Test (TC-652) |
| FR-207-AC-2 | In §7.1, `finish` is not enabled at t4 while the compensation thread is `running`; t4 enables exactly `cattempt`, `cfail`, `ctimeout` and `cexpire`. At t6 the count has reached `attempts 1`, so no `cattempt`, `cfail` or `ctimeout` is enabled, and the steps are `cend` and `cexpire` (ST-18, which the count does not bound); the registration is `recovered` at t7 because the recovery predicate `triggered`, over the activation capture, holds, and `timed-out` after `cexpire`, with no recovery predicate evaluated. With the predicate changed to `not triggered`, the registration is `unrecovered` after `cend`. | Test (TC-652) |
| FR-207-AC-3 | With `attempts 2` and the retry relation `true`, `cattempt(Refund)`, `cfail(Refund)`, `ctimeout(Refund)`, `cend(Refund)` and `cexpire(Refund)` are enabled at the retry point after the first attempt, whatever that attempt's outcome; after the second attempt only `cend` and `cexpire` are. With the retry relation `false`, only `cend` and `cexpire` are enabled after the first attempt, since `cfail` and `ctimeout` satisfy the retry relation at a retry as `cattempt` does. | Test (TC-652) |
| FR-207-AC-4 | A variant of §7.1 with a `commit Ok` node between `Paid` and `Fail` and the template's `commit Main::Ok`: the registration closes at `Ok`'s step, `event(Fail)` with `failed` true activates nothing, and no state holds a compensation thread. A second effect of the template's forward node after `Ok` registers nothing. | Test (TC-652) |
| FR-207-AC-5 | With the recovery predicate `r.bal = 0`, which reads its recovery binder `r` of type `Recovery` (`bal: Int[0, 1]`), `cend` at t6 has exactly two successors: the registration `recovered` with `r.bal` 0 and `unrecovered` with `r.bal` 1, the two transition identities differing only in the record; no item settles `unsupported`, and a claim that every finished instance is `recovered` is refuted with a counterexample carrying `r.bal` 1 that replays. | Test (TC-652) |
| FR-207-AC-6 | A variant of §7.1 with `attempts 2 of M::RefundAttempt`, `RefundAttempt` with one field `ok: Boolean`, `refund()` with precondition `true`, and the retry relation `not e.ok` over (`e`, `l`): at t4 two `cattempt(Refund)` steps are enabled, whose transition identities differ only in `record`, one with `ok` true and one with `ok` false. At the retry point after the `ok` false attempt, two retry `cattempt` steps, one per record, and `cend` are enabled; after the `ok` true attempt only `cend` is. With the retry relation `l.ok`, the retry point after either first attempt enables one retry `cattempt`, whose record has `ok` true, and `cend`. These counts are of the `cattempt` and `cend` steps; beside them each attempt or retry point that enables a `cattempt` with a record also enables `cfail` and `ctimeout` with that record (ST-16, ST-17), and every attempt and retry point enables `cexpire` (ST-18). | Test (TC-652) |
| FR-207-AC-7 | Under `on each (x: Level)` (`Level` is `Int[0, 3]`) with the protocol capture `big: Boolean = x > 1`, a variant of §7.1's `Refund` with the registration capture `seen: Boolean = x > 2 and big`, the activation guard `f and big`, the activation capture `armed: Boolean = big` and the recovery predicate `seen and armed`: only the registration-capture stage reads the activation binder `x` (FR-381's stage table); the guard, the activation capture and the recovery predicate read the protocol capture `big`. After `activate(Level)` with the trigger 3 the registration is `recovered` after `cend`, with `seen` true in its captures; with the trigger 2 it is `unrecovered`, with `seen` false; with the trigger 0 or 1 the guard is false, the compensation never activates and the registration stays `registered` with `seen` false until `finish`. | Test (TC-652) |

## Dependencies

- ADR-027 §2.2 ST-11 and ST-12, §2.2a CP-1 to CP-5, §2.1 FO-1, §3.1 PB-4.
- FR-205 (registrations in the key), FR-206 (the `attempt` rule), FR-218
  (checked templates).
- FSpec owns compensation's meaning in a model subject (ADR-027 QS-10):
  FSpec FR-056, FR-057, FR-058.

## References

- Owning ticket: Linear QSL-396. FSpec half: FSpec FR-427 (Linear STD-140).
