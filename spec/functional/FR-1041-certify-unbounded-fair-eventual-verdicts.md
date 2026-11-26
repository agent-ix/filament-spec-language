---
id: FR-1041
title: "Certify unbounded fair eventual verdicts"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-200
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
---
# FR-1041: Certify unbounded fair eventual verdicts

## Description

EN-5 SHALL certify every exact proof of an untimed infinite-trace
`probability op c [eventually holds(P)]` claim over every scheduler with a
nonempty checked fairness set in the closed profile
`"filament.probability-objective/cyclic-mdp-fair-eventual/v1"`, and the
checker SHALL accept it only after recomputing the maximal fair end
components itself (FR-200) and checking FSpec FR-412's "Fair minimum"
conditions or, for a maximum, FR-201's cyclic MDP maximum conditions. A fair
scheduler can loop in a cycle that an unfair one could not be stopped from
using, so the cyclic MDP profile's rule that every action of a
positive-valued state progresses is wrong under fairness: the fair
end components replace it.

## Use case

A designer proves that a retrying sender delivers with probability at least
`1/2` against every strongly fair scheduler of `go`. Over every scheduler
the minimum is 0, because the scheduler may `wait` forever; that run is
unfair, since `go` stays enabled and is never taken. The checker finds that
`{s}` is no fair end component, accepts value `1/2` at `s` through the tight
action `go`, and rejects a document that states 0 there.

## Inputs

- A checked claim as above, its fairness set (FR-195) and EN-5's fair
  reachability result (FR-200: values, the fair end components it used and
  its fair witness).

## Outputs

- The document below, carried as the record's exact certificate bytes, and
  FR-201's check result.

## Behavior

### Admission

- The profile SHALL admit the claims above over a finite untimed Markov
  subject with one initial state, at most one actual S3 binding, FR-196's
  single-post-per-draw restriction and source `Every` `AtLeast`
  (`"minimum"`) or `AtMost` (`"maximum"`). The original reachable graph MAY
  cycle and every live state MAY have several actions. An empty checked
  fairness set SHALL keep FR-201's cyclic MDP eventual profile with its
  bytes unchanged and SHALL NOT be written in this profile.
- Fairness constraints SHALL be the checked set, written as FR-201's bounded
  fair profile writes it: records `{fairness_kind, granularity, target}`,
  `target` FSpec FR-322-AC-293's closed operation variant, strictly ascending
  by canonical record octets. A constraint none of whose scheduled
  identities occurs in the reachable product is never enabled, excludes no
  component and stays in the set; it is not a refusal.
- Clocks, rewards, workloads, other temporal forms, pending multi-post
  choice and multiple initial states or bindings SHALL keep the admission
  outcome another profile or engine gives them; this profile does not
  encode them.

### Document

- The objective SHALL have exactly `profile`, `targets`, `optimization` and
  `fairness`, with FR-201's reachability meanings for `targets` and
  `optimization`. The document SHALL have exactly `identity`, `objective`,
  `kind` (`"exact"`), `values` and `ranking`, with FR-201's strict canonical
  JSON, `Rational` and `Count` encodings, complete FR-196 product keys in
  this no-reset profile (the original eight-member representation)
  (monitor, accumulator and pending null), canonical row order and exact
  coverage of the stopped graph (targets absorbing).
- The document SHALL carry no policy, component list, fair end component or
  qualitative set. The checker derives each of them; a transported list
  would be a second, unchecked statement of what it computes.

### Check

- The checker SHALL rebuild the stopped graph from the checked subject and
  evaluate P itself, as FR-201's cyclic MDP profile does, and derive
  `U`: the maximal end components of the sub-MDP on non-target states,
  using only actions whose whole support stays in the candidate, refined by
  FR-200's weak and strong rules with enabledness read from every action of
  the original state, including actions that reach a target. A non-target
  terminal's implicit stutter is an end component that holds no
  constraint's action and is enabled by none, so it is in `U`.
- Minimum: every target SHALL have value 1 and every state of `U` value 0.
  At every other state `s` the checker SHALL check
  `v(s) = min_a Q_a(v, s)` over every enabled action, `Q_a` as in FR-201's
  cyclic MDP profile. Targets, states of `U` and states with value 1 SHALL
  have rank 0; every other state SHALL have positive rank and some tight
  action (`Q_a(v, s) = v(s)` exactly) with a positive-probability successor
  of strictly smaller rank.
- Maximum: the checker SHALL apply FR-201's cyclic MDP maximum conditions
  unchanged (a tight progressing action at every positive non-target state,
  every action of a zero-valued live state staying at zero). A `policy` row is
  optional; at a positive state with no row the progressing action SHALL also
  be tight. It SHALL still compare the fairness records with the checked set.
- Why the minimum is sound: under a fair scheduler a run reaches the target
  or ends, with probability 1, in a fair end component of the non-target
  states (Baier and Kwiatkowska), so the fair infimum is
  `1 − max Pr(not target until U)` (FR-200). Write `w = 1 − v`. Bellman
  equality makes `w` a fixed point of the maximum operator with `w = 1` on
  `U` and 0 on targets, so the least fixed point, that maximum, is at most
  `w`. The tight progressing actions form a memoryless policy that, from
  every state with `w > 0`, reaches `U` or a state with `w = 0` almost
  surely; under it the value is exactly `w`, so the maximum is at least `w`.
  A scheduler that follows that policy, then stays fairly inside the fair
  end component it reaches (which a fair end component admits by
  definition) or continues uniformly at a state with `v = 1`, is fair and
  attains `v`. Hence `v` is the fair infimum, not one of the other Bellman
  fixed points that a `wait` loop admits.
- Why the maximum is sound: the supremum over fair schedulers equals the
  maximum over every scheduler (FSpec FR-411), and the attaining policy of
  FR-201's maximum rule, continued uniformly once the value is decided, is
  fair.
- The threshold SHALL be checked inclusively at the initial key. Rejections
  SHALL use FR-201's rules and first-failing-key loci: a wrong value at a
  state of `U`, a target or a Bellman state `InequalityFails`; a missing
  tight progressing action or a wrong boundary rank `RankBroken`; a missed
  threshold `ThresholdMissed`. A missing, extra, changed or reordered
  fairness record, or changed targets or optimization, SHALL refuse
  objective mismatch; duplicate or out-of-order records SHALL be rejected
  `Malformed` at `objective`; identity conflict SHALL refuse before
  enumeration.

### Bounds and results

- Stopped-graph expansion, end-component decomposition, each refinement
  round, tightness and progress passes, key construction and exact sums
  SHALL run under the caller's byte, state, edge, work, storage,
  representation and arithmetic limits and cancellation. One round of
  decomposition costs `O(V + E)`; refinement removes at least one state or
  component per round, so it ends within `V` rounds per constraint, and
  each round is charged to the work limit. A stop SHALL settle
  `Incomplete(LimitReached{…})` or `Incomplete(Cancelled{…})` with no truth
  value.
- An accepted document SHALL settle `proved`, `ExactValue`, certified. A
  rejected one SHALL settle `inconclusive`, `CertificateRejected{rule, at}`.
  When the exact values exceed `max_rational_bits`, EN-5 SHALL certify
  dyadic bounds in FR-1042's value-bounds profile with this profile's
  objective as its base. A refutation SHALL keep FR-202's fair witness
  evidence, a randomized fair scheduler checked against FR-200's
  fair-end-component condition.
- EN-5 SHALL emit `ranking` as, at each state outside the rank-0 set, the
  shortest number of steps to that set through tight actions, which its
  values determine; the checker requires only the progress property.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1041-AC-1 | A model `W` whose live state `s` has `wait` (back to `s`) and `go` (to delivered `g` or to the undelivered terminal `f`, `1/2` each): `probability >= 0.5 [eventually holds(delivered)]` under every scheduler `fair strong go` settles `proved`, `ExactValue{1/2}`, with a document whose `values` are `s` `1/2`, `g` 1, `f` 0 and whose `ranking` is 1, 0, 0, which `fsl-probability-check` accepts in a separate process; `fair weak go` gives the same value and a document whose fairness record differs. With no fairness set the claim settles `refuted` (minimum 0) in FR-201's cyclic MDP profile family. | Test (TC-3150) |
| FR-1041-AC-2 | AC-1's document with `s` at 0 or at `1/4` (each satisfying the minimum through `wait`) is rejected `RankBroken` at `s`; with `f` at `1/2` it is rejected `InequalityFails` at `f`; with its fairness record removed or changed to `go` of another model, it refuses objective mismatch; with two copies of the record it is rejected `Malformed` at `objective`. | Test (TC-3150) |
| FR-1041-AC-3 | Over `W` with a further action `drift` from `s` to `u`, whose only action loops at `u`: `go` is not enabled at `u`, so `{u}` is a fair end component and the fair minimum at `s` is 0; the document with values `s` 0, `u` 0, `g` 1, `f` 0 and ranks 1, 0, 0, 0 is accepted and `probability >= 0.25` misses its threshold, so the item settles `refuted` by FR-202's fair evidence (the path `drift` into `u`); a document with `u` at `1/2` is rejected `InequalityFails` at `u`. | Test (TC-3150) |
| FR-1041-AC-4 | `probability <= 0.5 [eventually holds(delivered)]` over `W` under `fair strong go` settles `proved`, `ExactValue{1/2}` (the maximum through `go`) with an accepted document in this profile; `probability <= 0.4` settles `refuted`. | Test (TC-3150) |
| FR-1041-AC-5 | FR-201-AC-51's unbounded fair eventual item, `probability >= rational(1,2) [eventually holds(m.delivered)]` over the cyclic `link` model under `every scheduler fair strong Msg::send_a`, settles `proved`, `ExactValue{1/2}`, with values `1/2` at the undelivered state, 1 at delivered and 0 at the stopped terminal, which `fsl-probability-check` accepts from the emitted package and rejects with one value altered. | Test (TC-2065) |
| FR-1041-AC-6 | Pre-cancellation, cancellation during refinement, and exhaustion of the state, edge, work or arithmetic limit during end-component refinement or progress checking settle `incomplete` with the actual limit and no truth value; a fairness constraint whose operation is never enabled is retained in the objective and changes no value. | Test (TC-3150) |
| FR-1041-AC-7 | A maximum takes FR-201's cyclic maximum conditions unchanged. Where `{u}` is a fair end component from which the target is reached with probability 1 (`return`, then a retried `go`), the maximum document with value 1 at `u` passes every condition but a missed threshold, and the same document with `u` at 0 is rejected `InequalityFails`. Where `i` begins into `s`, whose actions are a self-loop and `fail` into a non-target terminal, and no target is reachable (maximum 0), the document with `i` and `s` at `1/3`, ranks 2, 1, 0 and a `policy` naming only `i` is rejected `RankBroken` at `s`, since no tight action at `s` descends, both when `{s}` is a fair end component and when it is not. | Test (TC-3150) |

## Dependencies

- [FR-200](FR-200-decide-every-scheduler-claims-over-fair-schedulers.md)
  supplies the fair end components and the fair infimum reduction;
  [FR-201](FR-201-check-a-probability-certificate.md) the document rules and
  the cyclic MDP maximum conditions; FSpec FR-412 "Fair minimum".

## References

- C. Baier and M. Kwiatkowska, model checking for a probabilistic branching
  time logic with fairness, Distributed Computing 11, 1998.
- Ticket: Linear QSL-2158.
