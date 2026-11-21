---
id: FR-1042
title: "Certify value bounds with lower and upper certificates"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-197
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-198
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1041
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1043
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
---
# FR-1042: Certify value bounds with lower and upper certificates

## Description

When EN-5 proves a probability claim from a dyadic interval
(`ValueBounds`, FR-203), it SHALL certify the interval with two documents in
the closed profile `"filament.probability-objective/value-bounds/v1"`: a
`lower` document whose initial value is the interval's lower end and an
`upper` document whose initial value is its upper end. Each wraps, as its
`base`, the objective of the exact profile the claim would otherwise be
written in, and the checker SHALL check each by FSpec FR-412's `lower` or
`upper` conditions over the product that base defines. A value whose exact
rational is too long to carry (§15.1's has 23,254 bits) is then still
certified on both sides, rather than written as a rational it is not.

## Use case

§15.1's no-fault probability `(9999999/10000000)^1000` exceeds
`max_rational_bits` 1,024. EN-5 reruns backward induction over 64-bit dyadic
intervals and settles `proved`, `ValueBounds{…, BackwardInduction{64}}`. The
item carries a `lower` document, ranked by remaining horizon, whose initial
value is above 0.999, and an `upper` document; `fsl-probability-check`
accepts both, so the reported interval is certified at both ends.

## Inputs

- An EN-5 `Bounds` result (FR-197 dyadic backward induction, or FR-198
  interval iteration) with its lower and upper vectors and, over every
  scheduler, its policy.

## Outputs

- Two documents, `lower` and `upper`; FR-201's check result for each.

## Behavior

### Bases

- `base` SHALL be the complete objective, `profile` member included, of
  one of these exact profiles, admitted by that profile's own rules:
  FR-201's `reachability/v1` (both subsets, with the bounded event forms of
  FR-201's bounded subset), `state-predicate/v1`, `cyclic-dtmc-eventual/v1`,
  `cyclic-mdp-eventual/v1` and `bounded-fair-eventual/v1`; FR-1041's
  `cyclic-mdp-fair-eventual/v1`; and FR-1043's `finite-horizon-measure/v1`
  with any of its payoffs. The checker SHALL compare every member of `base`
  with its own derivation, as that profile does, before reading a value.
- A timed proof keeps FR-920's profile, which admits `lower` and `upper`
  itself. Expected-reward and long-run values are solved exactly (FR-198,
  FR-199) and have no dyadic method; when such a value exceeds
  `max_rational_bits`, EN-5 SHALL settle the item
  `Incomplete(LimitReached{MaxRationalBits, value, exact_prob.max_rational_bits})`,
  which raising that setting resolves, with no certificate and no truth
  value.

### Document

- The objective SHALL have exactly `profile` and `base`. The document SHALL
  have exactly `identity`, `objective`, `kind` (`"lower"` or `"upper"`),
  `values` and those of `ranking` and `policy` the table below requires, in
  FR-201's strict canonical JSON, `Rational` and `Count` encodings with
  complete product keys as the base profile writes them. A dyadic value is
  an ordinary reduced `Rational` whose denominator is a power of two; no
  precision member is written, since `ValueBounds.method` records it.
- `values` of an `upper` document SHALL cover exactly the base's stopped
  graph. A `lower` document SHALL name the initial key and any subset of
  the stopped graph; an unnamed successor counts as value 0 (FSpec FR-412),
  so a `lower` document may cover only the part of the product explored
  before a limit stopped exploration (FSpec FR-411 "Evidence on a partial
  product"). `ranking` and `policy` rows SHALL cover exactly the states the
  table names among the covered states.
- A `policy` row SHALL be `{state, choice}` with `choice` FSpec FR-412's
  `TransitionIdentity` of an enabled semantic action at that state (FR-187
  operation, receiver and non-random arguments), one deterministic choice;
  a randomized choice SHALL be rejected `Malformed` at `policy`, since a
  deterministic memoryless policy attains every bound below.
- Targets and accepting decided states SHALL carry 1, non-target terminals
  and rejecting decided states 0, the states of FR-1041's `U` 0, and a
  measure base's decided states their payoff (FR-1043). Every value of a
  probability base SHALL lie in [0,1].

### Conditions

`Q_a(x, s)` is FR-201's one-step value of action `a`; under a workload, the
single resolved distribution. "Progress" at `s` means a positive-probability
successor of strictly smaller rank whose value is positive (FSpec FR-412
`lower`), or, for the fair-minimum upper, whose value is below 1.

| Base optimization | `lower` (`x` at most the value) | `upper` (`y` at least the value) |
| --- | --- | --- |
| workload | `x(s) <= Q(x, s)`; `ranking`; progress at each named non-target state with `x > 0` | `Q(y, s) <= y(s)` |
| minimum | for every action `a`, `x(s) <= Q_a(x, s)` and progress under `a`; `ranking` | `policy` `σ`: `Q_σ(y, s) <= y(s)` |
| maximum (with or without fairness) | `policy` `σ`: `x(s) <= Q_σ(x, s)` and progress under `σ`; `ranking` | for every action `a`, `Q_a(y, s) <= y(s)` |
| fair minimum (FR-1041) | for every action `a`, `x(s) <= Q_a(x, s)`, `x = 0` on `U`; no `ranking` | `policy` `σ`: `Q_σ(y, s) <= y(s)` and progress under `σ` toward `U`, the targets and the states with `y = 1`; `ranking` |

- Over a bounded base (FR-201's bounded subset, the bounded fair profile,
  FR-1043's measure product) the undecided product is acyclic and decided
  states are absorbing. There the inequalities alone bound the value by
  induction on the remaining horizon, and a `lower` document's `ranking`
  SHALL instead equal the independently rederived remaining horizon, as
  FR-201's bounded rank rule states; no positive-value progress rule
  applies, so a measure's negative shifted payoffs are admitted.
- Why the conditions are sound: the value is the least fixed point of the
  one-step operator with the boundary values above. An `upper` vector is a
  pre-fixed point of the operator the table names (every action for a
  maximum, a policy's for a minimum, whose value is at least the minimum),
  so it dominates that least fixed point. A `lower` vector is a post-fixed
  point whose ranking makes the policy or every action reach the targets
  from every positive state with positive probability within its rank,
  which excludes the greatest-fixed-point solutions a cycle admits
  (FSpec FR-412, Funke, Jantsch and Baier). For the fair minimum the roles
  swap through `w = 1 − v` (FR-1041).
- A `Lower` vector from interval iteration from below, or from dyadic
  backward induction rounded down, satisfies its inequalities, and its rank
  is the first iteration or position at which a state's value becomes
  positive; an `Upper` vector rounded up satisfies its own. EN-5 SHALL check
  both documents before emitting them, and on a rejection raise the
  precision as FR-201 states for any rejected certificate.

### Pairing, side and results

- The document on the claim's side SHALL meet the threshold inclusively at
  the initial key: `lower` for `AtLeast` (and for the `<= c` quantile's
  shifted payoff, at least 0), `upper` for `AtMost` (and for the `>= c`
  quantile's shifted payoff, below 0, or 0 with FR-201's tight-action
  search finding no activated state). The other document SHALL meet its
  conditions and no threshold. A claim-side document of the wrong kind
  SHALL be rejected `Malformed` at `kind`; a missing counterpart, other than
  for a `lower` proof on a partial product, SHALL be rejected `Malformed`
  at `counterpart`.
- Both accepted SHALL settle `proved`,
  `ValueBounds{lower, upper, method, reductions}`, certified, with `lower`
  and `upper` the two documents' initial values; a `lower` proof on a
  partial product reports upper 1. The claim-side document missing its
  threshold SHALL make EN-5 raise the precision, up to
  `max_precision_bits`, and then settle `Incomplete(PrecisionBudget{lower,
  upper})` (FR-203). A rejection SHALL settle `inconclusive`,
  `CertificateRejected{rule, at}` with FR-201's rules and loci. The checker
  SHALL obey the caller's byte, state, edge, work, storage, representation
  and arithmetic limits and cancellation; a stop SHALL settle
  `incomplete` with its limit and no truth value.
- Identity SHALL be FR-201's structural `{node_id, model, ordinal}`; both
  documents carry the item's identity, and a mismatch in either SHALL
  refuse before enumeration. A changed `base` member SHALL refuse objective
  mismatch.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1042-AC-1 | Over the cyclic `link` chain (from the undelivered state `s`, `1/3` each to `s`, delivered `g` and the stopped terminal `f`; value `1/2`) under `Even`, `probability >= rational(2,5) [eventually holds(m.delivered)]` with `max_rational_bits` 1 settles `proved`, `ValueBounds{lower, upper, …}` with `lower <= 1/2 <= upper`, and its `lower` and `upper` documents, `base.profile` `cyclic-dtmc-eventual/v1`, are each accepted by `fsl-probability-check` in a separate process and rejected with one value altered. | Test (TC-2065) |
| FR-1042-AC-2 | Over AC-1's chain, offered documents: `lower` `s` `7/16`, rank 1, with counterpart `upper` `s` `9/16` is accepted for `>= rational(2,5)`; `lower` `5/8` and `upper` `7/16` are rejected `InequalityFails` at `s`; `lower` `7/16` for `>= rational(1,2)` is rejected `ThresholdMissed` at `s`; `lower` without `ranking` is rejected `Malformed` at `ranking`; an `upper` document offered as the claim side of that `AtLeast` proof is rejected `Malformed` at `kind`; `f` at `1/16` in the `upper` document is rejected `InequalityFails` at `f`. | Test (TC-3151) |
| FR-1042-AC-3 | Over `W` (FR-1041-AC-1) with no fairness set, maximum `1/2`: for `probability <= rational(3,5)`, `upper` `s` `3/5` (both actions satisfy `Q_a(y) <= y`) with counterpart `lower` `s` `7/16`, policy `go`, rank 1, settles `proved`; `lower` `s` 1 with policy `wait` is rejected `RankBroken` at `s`; `lower` without `policy` is rejected `Malformed` at `policy`. For the minimum (0), `upper` `s` `1/4` with policy `wait` is accepted and with policy `go` rejected `InequalityFails` at `s`. | Test (TC-3151) |
| FR-1042-AC-4 | Over `W` under `fair strong go` (fair minimum `1/2`), `lower` `s` `2/5` with no ranking and counterpart `upper` `s` `3/5` with policy `go` and rank 1 prove `probability >= rational(2,5)`; `upper` `s` `1/4` with policy `wait` is rejected `RankBroken` at `s`. | Test (TC-3151) |
| FR-1042-AC-5 | A `Health` chain whose `ok` state stays `ok` with `9/10` per step (otherwise a fault): `probability >= 0.75 [always[0,2] holds(ok)]` (exact `81/100`) is proved by `lower` values `25/32` and `7/8` at the undecided keys of remaining horizon 2 and 1, ranks 2 and 1, with counterpart `upper` `7/8` and `15/16`, settling `ValueBounds{25/32, 7/8}`; the `lower` rank 2 changed to 3 is rejected `RankBroken` at that key, and `upper` `13/16` at horizon 2 is rejected `InequalityFails` there (`(9/10)(15/16) = 27/32 > 13/16`). | Test (TC-3151) |
| FR-1042-AC-6 | §15.1 with `max_rational_bits` 1,024 and `precision_bits` 64 (FR-197-AC-3, FR-203-AC-1) settles `proved`, `ValueBounds{…, BackwardInduction{64}}`, whose `lower` document (base `reachability/v1`, bounded `always`) ranks every undecided key by its remaining horizon and whose initial lower value is above `999/1000`; both documents are accepted by `fsl-probability-check` from the emitted package. | Test (TC-2066) |
| FR-1042-AC-7 | FR-201-AC-27's `Steady` long-run claim (exact `1800/1801`) with `max_rational_bits` 4 settles `Incomplete(LimitReached{MaxRationalBits, 4, exact_prob.max_rational_bits})` with no certificate; FR-201-AC-35's expected-reward claim with `max_rational_bits` 1 settles the same way; neither settles `proved`, `unsupported` or `failed`. | Test (TC-3151) |
| FR-1042-AC-8 | Pre-cancellation, cancellation during either document's check, and exhaustion of each caller limit settle `incomplete` with the actual limit and no truth value; either document with another identity refuses before enumeration, and either with a changed `base` member refuses objective mismatch. | Test (TC-3151) |

## Dependencies

- [FR-201](FR-201-check-a-probability-certificate.md) supplies the base
  profiles and the rejection rules; [FR-197](FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
  and [FR-198](FR-198-decide-unbounded-reachability-and-expected-rewards.md)
  the dyadic results; FSpec FR-412 the `lower` and `upper` conditions.

## References

- F. Funke, S. Jantsch and C. Baier, farkas certificates and minimal
  witnesses for probabilistic reachability constraints, TACAS 2020;
  S. Haddad and B. Monmege, interval iteration, 2018.
- Ticket: Linear QSL-2158.
