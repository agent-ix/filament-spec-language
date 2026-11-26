---
id: FR-243
title: "Route timed items to the zone engine or the digital-clock route"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
---
# FR-243: Route timed items to the zone engine or the digital-clock route

## Description

The request writer SHALL route every non-probabilistic timed item (TT-1 to
TT-4, the deadlock-freedom item and the time-lock-freedom item) to EN-6's
zone search (FR-239), so a timed invariant is proved by `Holds` with a CF-1
certificate (ADR-026 EZ-10). Only a probabilistic claim over a timed
subject that names `exact` evidence takes the digital-clock route, EN-5's
reading of a closed subject through integer-valued clocks (ADR-028 TA-1 to
TA-3, FR-204), whose results settle by FR-203.

## Use case

A verification operator's model has only non-strict guards and deadlines.
Its timed invariant is proved by the zone engine with a certificate. A
probabilistic deadline claim over the same model with `exact` evidence is
decided on the digital-clock route with an exact value.

## Inputs

- A timed item, its capability kind, its `options.method` and, for a probabilistic claim, its
  requested evidence kind.

## Outputs

- The routed engine for the item: EN-6, EN-5's digital-clock route for an exact probabilistic claim, or EN-1 over the digital-clock reading for a `temporal-satisfaction` item that names `options.method` `digital-clocks`.

## Behavior

- When a timed item's capability kind is `temporal-satisfaction` and its
  `options.method` is absent or `zones`, negotiation SHALL route it to EN-6,
  whatever its constraints.
- `options.method` is a provider option, negotiated against each
  candidate's manifest like every other provider option (FR-635, FR-075).
  For a timed subject, EN-6's manifest publishes `method` with the one value
  `zones`, its default, and EN-1's manifest publishes `method` with the one
  value `digital-clocks`. An item over a timed subject that names
  `options.method` `digital-clocks` (FR-632) therefore has EN-1 as its only
  candidate and negotiates to EN-1's digital-clock route: EN-1's
  explicit-state search (FR-126) over the subject's integer-clock reading
  (ADR-028 TA-1 to TA-3), settling with basis `exhaustive`
  (FR-235). The caller asks for it by name, as a cross-check of the zone
  verdict; EZ-10's default route is unchanged, and EN-6 still never
  digitises (FR-239).
- EN-1's digital-clock route SHALL check, before any search, that the
  subject is closed and that digitisation preserves the item's dense-time
  verdict. The subject is closed when every clock constraint of its guards
  and time invariants is non-strict. When it is not, or the claim carries a
  strict (open or half-open) interval, the route SHALL decline the item
  `invalid-input` naming the strict constraint or interval.
- The **normal form** of a temporal claim is the normal form of the claim
  itself, not of its negation: `implies` and `iff` expanded and every
  negation pushed to the atoms. Each occurrence of an operator or atom in it
  keeps the source span it came from. A temporal claim is **digitisable**
  when, in the normal form of the claim:
  - (a) every timed interval is closed;
  - (b) no atom reads a clock;
  - (c) at most one occurrence of `eventually`, `until`, `once` or `since`
    carries an interval other than `[0, *]`;
  - (d) each such occurrence's interval has lower bound 0 or no upper bound;
  - (e) no such occurrence is inside the operand of any `eventually` or
    `once`, an operand of any `until` or `since`, or the left operand of any
    `release` or `triggered`; and
  - (f) when the claim declares a fairness constraint, the subject has no
    clock constraint.

  EN-1's digital-clock route SHALL decline a claim that is not digitisable
  `invalid-input`, naming the one part that fails the rule.
- EN-1's digital-clock route SHALL choose the named part as follows. The occurrences of the normal
  form are read in the source order of the claim: by the start of their
  source spans, and of two with the same start, the longer span first. The
  first source span with an occurrence that breaks one of (a) to (e) is
  named, with the first rule in (a) to (e) order that it breaks; (c) is
  broken by every counted occurrence after the first in that order. When (a)
  to (e) hold and (f) does not, the first clock constraint of the subject in
  source order is named, with rule (f).
- Of the state-graph forms (FR-852), EN-1's digital-clock route SHALL decide
  `possible` and SHALL decline `always possible` and `unique path`
  `invalid-input` naming the form: each quantifies over every reachable
  timed state, and an integer-clock reading does not visit them all.
- Every decline by EN-1's digital-clock route settles `declined` with
  `settled_by` `result` and `backend` `en-1`, and no search runs. A decline
  for a claim that is not digitisable, or for a state-graph form, carries the
  catalogued cause `invalid_runtime_input`/`invalid-value`. EN-6, the route
  of an item whose `options.method` is absent or `zones`, decides such an
  item.
- When a `probabilistic-satisfaction` item over a timed subject names
  `exact` evidence, the request writer SHALL route it to EN-5's
  digital-clock route (FR-204); when it names `statistical` evidence, to
  EN-4 (FR-254).
- The route SHALL be a function of the capability kind, the requested
  evidence kind and the item's `options` alone, negotiated against the
  published manifests, so one item always reaches one engine.

### Decision: digitise only what one rounding preserves

Digitisation rounds every time stamp of a dense behaviour to an integer with
one rounding threshold. For a closed subject every threshold gives a
behaviour of the subject. A claim's verdict carries over only when a
violating dense behaviour has some threshold whose rounded behaviour also
violates it. "Some later position lies in a closed window" holds under every
threshold, as does a statement about every later position. "Every position
within the interval `I` of one position holds" needs the positions outside
`I` to stay outside it, which one threshold gives when `I` is bounded on one
side only. A two-sided `I`, two such statements, or one read at many
positions can need two thresholds at once. In the two-step subject of
FR-243-AC-5, `first` at a time strictly between 0 and 1 leaves no position in
`[1, 2]`, and no integer time stamps reproduce that.

Rule (b): rounding keeps non-strict clock comparisons only, and a negated
clock atom is a strict one.

Rule (f): fairness asks whether an operation is enabled at every instant. A
disabled constrained operation can be a strict region such as `x > b`, which
the integer grid can miss. Take an operation `loop`, guarded by `x <= 1 ms`,
that resets `x`, the time invariant `x <= 1 ms`, and an operation `t`,
guarded by `x >= 1 ms`, that reaches a quiescent state `done`. Taking `loop`
every half millisecond never enables `t`, so that behaviour is fair under
`strong t` and refutes `eventually holds(done)`; every integer-clock
behaviour that diverges without `t` enables `t` again and again, so none of
them is fair.

The rule is conservative: it declines some claims that digitisation would
preserve.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-243-AC-1 | `NoLateReply` over `Rpc` with `T = 4 ms` (all closed) and the deadlock-freedom item over the same subject route to EN-6 and return `Holds` with a certificate (FR-239-AC-1). | Test (TC-698) |
| FR-243-AC-2 | ADR-028 §15.5's `Deadline` over `Retx` with `exact` evidence routes to EN-5's digital-clock route and settles `proved`, `ExactValue{99/100}` (FR-204-AC-1); with `statistical` evidence it routes to EN-4. | Test (TC-698) |
| FR-243-AC-3 | ADR-026 §9's retry model, with strict constraints, routes `always holds(not retried)` to EN-6 and is refuted (FR-239-AC-2); the all-closed `Rpc` routes the same claim kind to EN-6 too. | Test (TC-698) |
| FR-243-AC-4 | `NoLateReply` over the all-closed `Rpc` with `T = 4 ms` and `options.method` `digital-clocks` routes to EN-1's explicit-state search over the integer-clock reading and settles `proved`, basis `exhaustive`, as the zone route proves it; the same item over ADR-026 §9's strict-constraint retry model settles `declined` `invalid-input` with `settled_by` `result` and `backend` `en-1`, and no search runs; with `options.method` absent it negotiates to EN-6. | Test (TC-2290) |
| FR-243-AC-5 | Over the two-step closed subject (`first`, taken by `x <= 1 ms`, resets `x`; `second` is taken at exactly `x = 2 ms`), `eventually[1 ms, 2 ms] holds(c.phase >= 0)`, `holds(c.phase >= 0) until[1 ms, 2 ms] holds(c.phase >= 0)`, `eventually[0 ms, 0 ms] eventually[1 ms, 2 ms] holds(c.phase >= 0)` and `eventually (holds(c.phase = 0) and eventually[1 ms, 2 ms] holds(c.phase >= 0))` are each refuted by EN-6 and declined `invalid-input` by EN-1, naming the failing operator, never `proved`; `eventually[0 ms, 2 ms] holds(c.phase >= 1)` and `always (holds(c.phase = 1) implies eventually[0 ms, 2 ms] holds(c.phase = 2))` are proved by both. | Test (TC-2290) |
| FR-243-AC-6 | An `always possible` and a `unique path` item with `options.method` `digital-clocks` are declined `invalid-input` naming the form, with `backend` `en-1`, and no search runs; a `possible` item is decided. | Test (TC-2412) |
| FR-243-AC-7 | Over the two-step closed subject, a temporal claim that declares a fairness constraint is declined `invalid-input` by EN-1, naming a clock constraint of the subject, and no search runs. | Test (TC-2290) |

## Dependencies

- ADR-026 §8 EZ-10, §8.1 CF-4; ADR-028 TA-1 to TA-3 (EN-5's digital route).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  1992; J. Ouaknine and J. Worrell, 2003 (ADR-026 References).
