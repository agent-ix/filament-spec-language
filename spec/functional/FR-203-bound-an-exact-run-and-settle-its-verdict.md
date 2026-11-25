---
id: FR-203
title: "Bound an exact run and settle its verdict"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-412
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: references
---
# FR-203: Bound an exact run and settle its verdict

## Description

EN-5's cost SHALL be bounded only by `ExactProbLimits`,
[FR-120](FR-120-simulate-a-checked-package-s-state-family.md)'s cumulative
candidate and clause work allowance, its local expansion ceiling and the
existing cancellation and clause-meter limits, ADR-014 B-5 budgets
that the request sets, each with a published default (ADR-028 LM-1 to
LM-3). Reaching one SHALL stop the run and name the budget, its value and
the request setting that raises it. FSL SHALL settle every item routed to
EN-5 as exactly one FR-331 terminal record with ADR-018's verdict kinds and
the numeric `ProofBasis` members `ExactValue` and `ValueBounds` (XV-1 to
XV-7), or the existing value-free `Exhaustive` basis for certified
conditional vacuity below, never with ADR-024's `measured`.

## Use case

An operator's exact check of a large model stops at `max_states`. The result
names `max_states`, its value and `exact_prob.max_states`; the operator raises
it and reruns. A completed numeric proof settles `proved` with `ExactValue`
and the least favourable initial state, or a refutation settles `refuted`
after its witness replays. A conditional claim with no reachable activation
settles a certified, value-free `Exhaustive` proof after its vacuity
certificate is independently accepted.

## Inputs

```rust
pub struct ExactProbLimits {
        pub model: ModelCheckLimits,      // FR-126: max_states, max_transitions, max_automaton_states, its defaults

    pub max_iterations: u64,          // interval-iteration sweeps; default 1_000_000
    pub precision_bits: u32,          // starting dyadic precision; default 64
    pub max_precision_bits: u32,      // default 4_096
    pub max_rational_bits: u64,       // default 65_536
    pub max_policy_iterations: u64,   // default 10_000
    pub max_witness_paths: u64,       // default 1_024
}
pub enum ExactProbLimit { MaxStates, MaxTransitions, MaxAutomatonStates, MaxIterations, MaxPrecisionBits, MaxRationalBits, MaxPolicyIterations, Time, ClauseMeter, MaxCandidates, Cancelled }
```

- An EN-5 outcome (FR-197 to FR-200) with its certificate check (FR-201) or
  witness replay (FR-202).

## Outputs

```rust
// ProofBasis gains:
//   ExactValue { value: Rational, reductions: Vec<AppliedReduction> },
//   ValueBounds { lower: Rational, upper: ExtRational, method: BoundsMethod, reductions: Vec<AppliedReduction>, exploration: Exploration },
// Exploration: Complete, or Partial for a lower proof found on a partial product (FR-1042), whose
//   upper is the objective's least upper bound (+∞ for an expected reward) and which has no counterpart
pub enum BoundsMethod { BackwardInduction { precision_bits: u32 }, IntervalIteration { precision_bits: u32, sweeps: u64 } }
// IncompleteCause gains: PrecisionBudget { lower: Rational, upper: Rational }

// UnsupportedCause gains: DelayDistribution (exact evidence over a continuous
//   delay distribution only; FR-204). Zero-weight components (FR-199), strict
//   constraints, form shapes, zero-delay cycles and time-locks (FR-204) settle
//   verdicts.
// ExactEntry belongs to numeric results only; certified conditional vacuity has no numeric entries.
pub struct ExactEntry { pub initial: u32, pub binding: Option<Binding>, pub instance: Option<Binding>, pub value: ExactOrBounds, pub extremum: Option<Extremum> } // Extremum: Min or Max
```

## Behavior

### Budgets

- Each `ExactProbLimits` member SHALL be set by the request, with the
  default above when omitted. Every member's setting name SHALL be
  `exact_prob.` followed by the member name, the `model` members included
  (`exact_prob.max_states`, `exact_prob.max_transitions`,
  `exact_prob.max_automaton_states`), as FSpec FR-411's flat limits object
  names them (FR-255's engine table); the `model` members keep FR-126's
  defaults. FSpec FR-411 owns each limit and its setting name; FSL
  publishes the default values. No member SHALL be fixed by the language, and none
  enters the obligation identity. The method choices the budgets drive
  SHALL be recorded in the result.
- Reaching a budget before the method completes SHALL settle V-7:
  `Incomplete(PrecisionBudget{lower, upper})` when an interval still
  straddles the threshold at `max_precision_bits`, and
  `Incomplete(LimitReached{limit, value, setting})` naming the budget, its
  value and its setting otherwise, unless partial evidence already settles the
  item (FR-202).
- EN-5 SHALL debit all `work_units` charges made while settling one exact item
  against the same cumulative `accounting.work_units` allowance defined by
  FR-120, including product construction, exact certificate production and
  checking, counterexample witness production and replay, and any undefined
  re-exploration used to classify that item's outcome. None of these phases
  SHALL receive a replenished allowance. Their local ceilings, document
  storage limits, and cancellation checks remain independently enforced. A
  separate exact item begins with its own allowance as specified by FR-120.
  If the next charge would exceed the allowance, EN-5 SHALL stop before doing
  that charged work and settle `incomplete` with
  `Incomplete(LimitReached{work_units, B, accounting.work_units})`, where `B`
  is the configured bound; it SHALL emit no proof, refutation, accepted
  certificate, or replayed witness for that item.

### Product admission before materialization

- EN-5 SHALL check the remaining `exact_prob.max_transitions` capacity
  before constructing outgoing planned edges for an undecided product
  node. With no capacity remaining it SHALL stop with the existing
  `MaxTransitions` limit before invoking successor construction for that
  node.
- While constructing successors, EN-5 SHALL admit each next edge against
  the remaining transition capacity before materializing that planned
  edge, and each new product-state key against
  `exact_prob.max_states` before constructing or retaining its planned
  target representation. Only the work needed to establish the target key
  and whether it already exists may precede state admission. A target whose key
  already exists consumes no new state slot. The builder SHALL NOT
  construct the complete planned-edge or target vector before applying
  these ceilings. Candidate work needed to discover a key SHALL remain
  bounded by the cumulative work allowance and cancellation, even when
  an outer state or transition slot remains.
- A denied state or edge admission SHALL retain the existing typed limit,
  configured bound and `exact_prob.` setting in incomplete settlement;
  it SHALL NOT produce a truth value or broaden engine admission.

### Settlement

- This conditional-vacuity settlement SHALL apply to every conditional
  reading admitted by FR-201, including bounded quantiles under a workload
  or every scheduler, with or without fairness. When complete exploration
  of such a claim finds no activating state reachable, so its condition has probability 0, EN-5 SHALL settle
  `proved`, `Certified`, with the existing `Exhaustive` basis only after
  FR-201's independent checker accepts the same item's `Conditional`
  certificate in `Vacuous` mode. The checker SHALL rederive absence of
  reachable activation for every initial state and binding under the
  claim's admission rules. Settlement SHALL require opaque acceptance of
  that certificate bound to the checked package, subject, claim and
  binding; an engine's vacuity assertion alone SHALL NOT authorize proof.
- This vacuous proof SHALL carry its accepted FR-412 conditional
  certificate (`kind` `conditional`, `conditional` exactly
  `{"mode": "vacuous"}`, empty `values`, no conditional parts) as the
  result's `certificate`. Its wire basis SHALL be `{"type": "exhaustive"}`.
  It SHALL carry no `ExactValue`, `ValueBounds`, numerical result entries,
  numeric value, bounds, counterpart or generic `evidence` member.
  Settlement SHALL NOT fabricate 0, `0/0`, or any other numerical
  placeholder for an undefined conditional value, or introduce a new basis.
- For an `exhaustive` result, the only permitted certificate SHALL be that
  conditional `vacuous` certificate; all other `exhaustive` results SHALL
  carry neither `certificate` nor `evidence`. A `ratio`, `optimum`, `exact`
  or other certificate SHALL NOT be accepted as the vacuity certificate.
  An `exact-value` basis SHALL still require its valid `Rational` value;
  omission, a scalar zero or a zero denominator SHALL refuse the result
  under FR-331's result contract.
- Partial exploration SHALL NOT establish vacuity. A cancelled or
  limit-stopped build or check SHALL retain its existing typed incomplete
  outcome, with no accepted vacuity evidence or proof record. A certificate
  offered in `Vacuous` mode when activation is reachable SHALL be rejected
  `InequalityFails` at FR-201's locus and settle `inconclusive`,
  `CertificateRejected{rule, at}`. Other malformed certificates, identity
  mismatches and refusals SHALL retain FR-201's existing typed rule and
  locus; none SHALL produce vacuity acceptance.

- The map from EN-5's outcome to `TerminalValue` SHALL be exhaustive, with
  no `_` arm:

| Outcome | FR-331 value | FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- |
| Complete conditional exploration finds no reachable activation, `Conditional` `Vacuous` certificate independently accepted | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}`, value-free, carrying only the permitted certificate | success |
| Exact value on the claim's side, certificate accepted | `proved` | `closed-scope` | `Proved{basis: ExactValue{…}, certification: Certified}` | success |
| Sound interval on the claim's side, its `lower` and `upper` documents accepted, or a `lower` proof found on a partial product with `exploration: Partial` and the objective's least upper bound, its one document accepted (FR-1042) | `proved` | `closed-scope` | `Proved{basis: ValueBounds{…}, certification: Certified}` | success |
| `ValueBounds` whose claim-side document is accepted and whose counterpart is rejected (FR-1042) | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, at})` with the counterpart's rule and locus | inconclusive |
| `ValueBounds` that reaches settlement with no counterpart and without `exploration: Partial` (FR-1042) | `failed`, diagnostic `runtime_invariant`/`established-invariant-broken` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| Bound fails, witness replays | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| Undefined evaluation at a state of positive probability, `Undefined` path replays | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| Interval straddles at the budget | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(PrecisionBudget{…})` | incomplete |
| Another limit reached | `incomplete`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7) | incomplete |
| Cancelled through the `Cancel` handle | `incomplete`, `cancelled{source}` | `unavailable` | `Incomplete(Cancelled{source})` (ADR-018 V-7) | incomplete |
| Certificate rejected; replay disagrees or refuses | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, at})`, `Inconclusive(ReplayParity)`, `Inconclusive(ReplayRefused)` | inconclusive |
| `NotMarkov`, `DelayDistribution` (FR-204) | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- A `proved` record SHALL carry its accepted certificate (FR-201) and a
  `refuted` record its replayed witness (FR-202) as members of the item's
  result in the `fsl analyze` result document (FR-741), so the evidence that
  settled the item leaves the run with it and can be checked again later by
  `fsl-probability-check`.

### Build refusals and stops

- EN-5 SHALL settle a stop or refusal of FR-196's product build as the
  following table states, by the existing rules for the same kinds
  (FR-127, FR-192):

| Build result | FR-331 value | `TerminalValue` | Rule |
| --- | --- | --- | --- |
| `Stopped(Limit{limit, value})` for `MaxStates`, `MaxTransitions` or `MaxAutomatonStates` | `incomplete`, execution `resource-incomplete` | `Incomplete(LimitReached{limit, value, setting})` with its `exact_prob.` setting name | FR-127 V-7 |
| `Stopped(Cancelled(cause))` | `incomplete`, `cancelled{source}` | `Incomplete(Cancelled{source})` | FR-127 V-7 |
| `UndecidedSuccessor` | `inconclusive` | `Inconclusive(UndecidedSuccessor)` | FR-127 V-6 |
| `Expansion(stop)` naming no limit, an internal failure | `failed`, diagnostic `runtime_invariant`/`established-invariant-broken` | `Failed` | FR-192 |
| `Expansion(stop)` naming no limit, not an internal failure | `failed`, the stop's own `ExpansionStop.cause` as the one diagnostic | `Failed` carrying that diagnostic | FR-192 |
| `Internal(fault)` | `failed`, diagnostic `runtime_invariant`/`established-invariant-broken` | `Failed` | FR-127 |
| `NotMarkov` | `unsupported` | `Unsupported(NotMarkov)` | the `NotMarkov` row above |
| `Expansion(stop)` naming `MaxCandidates` | `incomplete`, execution `resource-incomplete` | `Incomplete(LimitReached{limit, value, setting})` with `limit` written `max_candidates` (the `expansion.max_candidates` row's limit name, FR-255), `value` the request's `max_candidates` and `setting` `expansion.max_candidates` | FR-192, FR-255 |
| `Expansion(stop)` naming `EvaluationMeter` | `incomplete`, execution `resource-incomplete` | `Incomplete(LimitReached{limit, value, setting})` for the typed `ClauseMeter`, naming the exhausted accounting counter by FR-255 Behavior 9's rule (its `limit` and `setting` the counter's name) and `value` that counter's configured bound | FR-192, FR-255 Behavior 9 |
| `PredicateRefused(KernelRefused(r))`, `r` not `CheckedInvariant`, or `PredicateRefused(FamilyRefused(code))` | `failed`, the refusal's catalog code and cause as the one diagnostic | `Failed` carrying that diagnostic | FR-100's refusal table |
| `PredicateRefused(Incomplete(i))` | `incomplete`, execution `resource-incomplete` | `Incomplete(LimitReached{limit, value, setting})` for the typed `ClauseMeter`, naming `i`'s counter by FR-255 Behavior 9's rule and `value` that counter's configured bound | FR-255 Behavior 9 |
| `PredicateRefused` holding `KernelRefused(CheckedInvariant)`, `NotBoolean`, `KernelUndefined` or `FamilyUndefined` | `failed`, diagnostic `runtime_invariant`/`established-invariant-broken` | `Failed` | FR-127 |
| `Translation(error)` | `failed`, diagnostic `runtime_invariant`/`established-invariant-broken` | `Failed` | FR-127 |
| `Admission(failure)` or `NotSimulated(RequiresBound)` | none: a request refusal, returned to the caller | no terminal record | FR-126, FR-196 |

- An undefined read is never a `PredicateRefused` (FR-196 returns
  `ProbBuild::Undefined`), and a checked claim's predicate completes a
  Boolean, so a `PredicateRefused` holding an undefined or non-Boolean
  outcome, or the kernel `CheckedInvariant` refusal, is a broken invariant.
- An automaton translation's own limit stops the build as
  `Stopped(Limit{MaxAutomatonStates, value})`, so a `Translation` error is
  never a reached limit.

- `TerminalValue::category` SHALL map both numeric `ProofBasis` members and
  the certified conditional-vacuity `Exhaustive` outcome to success, and
  proof accounting SHALL count them as proof.
- The record SHALL carry FR-242 truth and an FR-243 basis, and no `measured`
  value. For a numeric result, the basis SHALL state the least favourable
  (initial state, binding) pair; the result SHALL keep one entry per
  activating pair with its value or interval and, over every scheduler,
  whether it is a minimum or a maximum, and, with a fairness set, the set.
  For a conditional numeric result with at least one activating pair, the
  independent certificate acceptance SHALL account for every admitted
  (initial state, binding) pair, either by its checked numeric value or by
  independently established unreachable activation under FR-201's per-key
  rule. Settlement SHALL emit exactly one numeric entry for each activating
  pair and omit every independently checked nonactivating pair, without
  substituting 0, `0/0` or another numerical placeholder. Entries and the
  least favourable pair SHALL retain their actual admitted initial-state
  indices and bindings, including when the only activating index is not 0.
  An entry for a nonactivating pair, a missing activating entry, a duplicate
  entry or any mismatch between entries and accepted activating coverage
  SHALL settle `failed`, diagnostic
  `runtime_invariant`/`established-invariant-broken`, never `proved`.
  Coverage SHALL remain part of the independently checked acceptance,
  derived from the existing certificate values and admitted bindings;
  it SHALL NOT add an external coverage member or change the certificate
  wire shape. If no pair activates, settlement SHALL require the accepted
  `Conditional` `Vacuous` certificate and value-free `Exhaustive` basis
  above, without numerical entries. A certified conditional-vacuity proof
  SHALL retain the same claim and subject identity through that acceptance.
- The obligation identity SHALL bind the subject, the claim, its scheduler or
  `every`, its fairness set and its stated confidence parameters; the
  evidence kind SHALL enter the item's request, not the claim.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-203-AC-1 | §15.2 at `5 ms` settles `proved`, `closed-scope`, `Proved{ExactValue{24233/25000}, Certified}`, success, with one entry. §15.1 with `max_rational_bits` 1,024 settles `Proved{ValueBounds{…, BackwardInduction{64}}, Certified}` with FR-1042's `lower` and `upper` documents (FR-1042-AC-6). §15.4's `Deliver` settles `refuted`, `decisive-counterexample`, after FR-202's replay, with an entry marked `Min`. | Test (TC-638) |
| FR-203-AC-2 | FR-197-AC-4's straddling run settles `Incomplete(PrecisionBudget{lower, upper})` with `lower < 99/100 < upper`. §15.3's per-window claim with `max_states` 1,000 stops before the product completes and no partial evidence settles it: `Incomplete(ResourceExhausted)` naming `limits.max_states` and 1,000. A request omitting every limit runs with the defaults above. | Test (TC-638) |
| FR-203-AC-3 | FR-201-AC-1's rejected certificate settles `inconclusive`, `CertificateRejected`; FR-202-AC-2's prefix-duplicated path set settles `inconclusive`, `ReplayParity`; FR-204-AC-3's exponential delay with exact evidence requested settles `unsupported`, `DelayDistribution`. None of these records carries a `measured` value. | Test (TC-638) |
| FR-203-AC-4 | A request whose proof summary holds AC-1's two proofs counts 2 proved items. `P95` with and without stated confidence parameters have different obligation identities, and the same claim requested with `exact` and with a changed `precision_bits` has one obligation identity. | Test (TC-638) |
| FR-203-AC-5 | FR-196-AC-5's refutation settles `refuted`, `decisive-counterexample`, `Refuted`, category violation, with cause `UndefinedEvaluation{where: 1, cause: absent-key}`, after FR-202's replay of its FSpec FR-413 `undefined-path` evidence (FR-202-AC-24 for its checked-subject replay); the claim's threshold plays no part. | Test (TC-641) |
| FR-203-AC-6 | A build stopped at `max_automaton_states` settles `incomplete`, `Incomplete(LimitReached{MaxAutomatonStates, value, exact_prob.max_automaton_states})`; a cancelled build settles `Incomplete(Cancelled{source})`; a build that meets an undecided successor settles `inconclusive`, `Inconclusive(UndecidedSuccessor)`; an expansion stop naming no limit settles `failed`, as an internal fault `runtime_invariant`/`established-invariant-broken` and otherwise with the stop's own cause; an `Internal` refusal settles `failed`. | Test (TC-638) |
| FR-203-AC-7 | The build-result map is exhaustive over `ProductRefusal` and `ClauseOutcome` with no `_` arm. A build whose expansion stops at `max_candidates` 2 settles `incomplete`, `Incomplete(LimitReached{max_candidates, 2, expansion.max_candidates})`. A build whose `ProductRefusal`, asserted before settling, is `Expansion(stop)` naming `EvaluationMeter` (a subject whose successor expansion charges the meter before the claim reads any predicate, under an accounting bound `B` of `work_units` its first expansion exceeds) settles `Incomplete(LimitReached{work_units, B, accounting.work_units})`, naming the exhausted counter, `B` and the setting `accounting.<counter>` by FR-255 Behavior 9's rule (FSpec FR-461-AC-5); a build whose `ProductRefusal`, asserted before settling, is `PredicateRefused(Incomplete(i))` (the expansion fits under `B`, the predicate read's charge is the first one denied) settles the same way for `i`'s counter. A predicate refused `integer_out_of_domain`/`outside-domain` settles `failed` with that diagnostic; a constructed `PredicateRefused(NotBoolean)`, `PredicateRefused(KernelRefused(CheckedInvariant))` and `Translation` error each settle `failed`, `runtime_invariant`/`established-invariant-broken`; an `Admission` failure and a `RequiresBound` refusal return no terminal record. | Test (TC-1166) |
| FR-203-AC-8 | The `proved` record of FR-203-AC-1 carries its `Exact` certificate in its item result, and those certificate bytes are accepted by `fsl-probability-check` in a later process; the `refuted` record of FR-203-AC-5 carries its witness, which replays. | Test (TC-2065) |
| FR-203-AC-9 | A `ValueBounds` proof of FR-1042-AC-2's claim whose claim-side `lower` document `7/16` is accepted and whose counterpart `upper` `7/16` is rejected settles `inconclusive`, `CertificateRejected{InequalityFails, s}`; the same proof handed to settlement with no counterpart and no `exploration: Partial` settles `failed`, `runtime_invariant`/`established-invariant-broken`, category failed; with `exploration: Partial` and upper 1 it settles `proved`, `ValueBounds{7/16, 1, …}`. | Test (TC-3151) |
| FR-203-AC-10 | The untimed claim `quantile 0.5 of (steps from holds(false) until holds(true) within 4) <= 2`, checked separately under a resolved workload and under every scheduler (with and without an admitted fairness set) has no activating state in the complete reachable product. Native checking independently accepts its `Conditional` `Vacuous` certificate and settlement returns `proved`, `closed-scope`, `Certified`, `Exhaustive`, category success, carrying that certificate with empty `values` and `conditional` exactly `{"mode": "vacuous"}`. The emitted result passes FR-331's contract with basis `{"type": "exhaustive"}` and no numeric value, numeric entries, bounds, counterpart or generic `evidence`; it contains neither `ExactValue` nor a fabricated 0 or `0/0`. | Test |
| FR-203-AC-11 | Changing AC-10's activation predicate to `holds(true)` makes the offered vacuous certificate reject `InequalityFails` and settle `inconclusive`, `CertificateRejected`, without vacuity acceptance. Offering an otherwise well-shaped `exact`, `ratio` or `optimum` certificate on `exhaustive`, or generic `evidence` with or without its permitted vacuous certificate, refuses the FR-331 result contract. An `exact-value` basis missing its value, using scalar zero or denominator zero refuses; AC-1's ordinary exact result with a valid `Rational` value remains admitted. An engine-only assertion, partial exploration, cancellation before or during checking, or a caller limit reached before checking completes produces no accepted vacuity evidence or proved record; cancellation and limits retain their typed incomplete causes. | Test |
| FR-203-AC-12 | `Retry` has `Msg.delivered`, terminal when delivered, and the resolved `Steady` workload whose `fast` step delivers with probability `1/2` and whose `slow` weight is 0. With request initial states ordered as index 0 `{delivered: true}` and index 1 `{delivered: false}`, the same admitted binding `m`, and `quantile 0.5 of (steps from holds(not m.delivered) until holds(m.delivered) within 4) <= 2`, the independent conditional certificate check accounts for both pairs: index 0 has unreachable activation with numerator and denominator 0, while index 1 activates immediately with numerator `3/4`, denominator 1 and ratio `3/4`. Settlement is certified `proved`, `ExactValue{3/4}`, with exactly one numeric entry for index 1 and the least favourable activating pair correctly naming index 1; index 0 is omitted without evaluating or emitting `0/0` or a scalar placeholder. Adding an entry for index 0, omitting index 1, duplicating its entry, or substituting index 0 for it settles `failed`, diagnostic `runtime_invariant`/`established-invariant-broken`, instead of proof. Both pairs remain accounted for by checked acceptance and the existing certificate values and binding derivation, without a new external coverage field. | Test |
| FR-203-AC-13 | In a model-subject build, two expansions, two claim reads, and an expansion followed by a claim read each fit the caller's `accounting.work_units = B` individually but exceed B cumulatively. Each run stops at its first unpaid cumulative charge and settles `Incomplete(LimitReached{work_units, B, accounting.work_units})` with no truth. A separate run has a fresh allowance. A local candidate denial continues to settle `Incomplete(LimitReached{max_candidates, C, expansion.max_candidates})` at its configured C; cancellation retains `Incomplete(Cancelled{source})`. | Test |
| FR-203-AC-14 | On a nonterminal product node with zero remaining transition capacity, an instrumented successor provider observes no successor-construction call before `MaxTransitions` incomplete settlement. With one edge slot remaining and multiple possible edges, only that slot is admitted before the next planned edge is denied; the full planned-edge vector is never materialized. At a full state ceiling an existing target key reuses its slot, while the first new key stops `MaxStates` before its planned target representation is constructed or retained, after only the work needed to establish the key. Stops name the configured bounds and `exact_prob.max_transitions` or `exact_prob.max_states`; work needed to discover candidates remains subject to FR-120's shared allowance. | Test |
| FR-203-AC-15 | When an expected-reward claim `expected total <= 1` has exact engine value `+infinity`, the item settles `refuted`, `decisive-counterexample`, `Refuted`, category violation only after FR-202 replay reproduces a lower certificate whose initial bound is `> 1`. The item's result carries the replayed witness and does not add an infinite numeric result entry or encode infinity as a finite `ExactValue`; a bare engine infinity remains unsettled. | Test (TC-3160) |
| FR-203-AC-16 | When FR-204's digital timed-product builder detects a reachable zero-delay cycle as `ProductRefusal::ZeroDelayCycle`, the item settles `unsupported` with the existing `zero-delay-cycle` cause and the cycle's ordered product-state identities. A cycle for which the existing time-divergence or subsystem rules decide the initial outcome remains `proved` or `refuted` under those rules; in particular FR-202-AC-42 and AC-46's initial-decided cyclic claims still settle `refuted` after replay. This criterion applies only to the timed-product refusal, not the exact graph check in AC-18. | Test |
| FR-203-AC-17 | When exact EN-5 encounters a digital timed state whose free delay is not stochastic, the item settles `unsupported` with the existing `not-stochastic` cause carrying that state's canonical product key and the typed transition identity (QSL-3143). This is one refusal family and does not turn a supported time-divergent zero-delay component into a refusal. | Test |
| FR-203-AC-18 | When the exact certificate/claim graph check returns `ExactProbabilityGraphError::Cycle` (outside FR-204's timed-product builder), the item settles `unsupported` with the existing `unsupported_construct/not-yet-implemented` cause, identifies the selected claim and cycle product form at the claim/form locus (QSL-3143), and emits no proof, counterexample, or numeric entry. FR-202-AC-42 and AC-46's initial-decided cyclic claims remain refuted through subsystem replay and do not take this graph-check refusal path. | Test |
| FR-203-AC-19 | When exact EN-5 cannot represent a required draw, the item settles `unsupported` with `unsupported_construct/not-yet-implemented` at the selected claim's draw-form locus and names the unsupported draw form (QSL-3143); no proof, counterexample, or numeric entry is emitted. | Test |
| FR-203-AC-20 | When exact EN-5's simulation input has an empty initial-state vector and returns `NotSimulated::EmptyInitial`, the item settles `failed` with diagnostic `invalid_runtime_input/invalid-value` and no verdict, certificate, counterexample, or numeric entry. This diagnostic is item-level and carries no finer field locus. If an initial-state value is rejected earlier by model admission or reading, the item retains that existing typed cause and its actual locus; EN-5 does not rewrite every unreadable initial state to `invalid_runtime_input/invalid-value`. An absent initial vector is never treated as probability zero or as an empty universal set. | Test |
| FR-203-AC-21 | When exact EN-5 cannot build a selected claim binding, the item settles `unsupported` with `unsupported_construct/not-yet-implemented` at that claim's binding-form locus and names the unsupported binding form (QSL-3143); no proof, counterexample, or numeric entry is emitted. | Test |
| FR-203-AC-22 | When exact EN-5 cannot translate the selected claim form to a product monitor, the item settles `unsupported` with `unsupported_construct/not-yet-implemented` at that claim's form locus and names the unsupported claim form (QSL-3143); no proof, counterexample, or numeric entry is emitted. | Test |
| FR-203-AC-23 | For an unconditional reachability claim, EN-5 computes exact value `v_i` separately from every admitted initial product state `i`, and emits an entry for each `i` with its initial-state identity and value. The aggregate exact claim value is `min_i(v_i)` for `>=` and `max_i(v_i)` for `<=` (QSL-3142); with one initial state the aggregate equals that state's value. | Test |
| FR-203-AC-24 | For an unconditional expected-reward claim, EN-5 evaluates every admitted initial product state and settles the claim-side objective using the minimum across initial states for `>=` and the maximum for `<=`; entries retain every initial-state identity and its value (QSL-3142). An infinite extremum retains FR-203-AC-15's replay requirement for a refutation. | Test |
| FR-203-AC-25 | For an unconditional long-run-fraction claim, EN-5 evaluates every admitted initial product state and settles the claim-side objective using the minimum across initial states for `>=` and the maximum for `<=`; entries retain every initial-state identity and its value (QSL-3142). | Test |
| FR-203-AC-26 | For an unconditional subsystem claim, EN-5 evaluates every admitted initial product state and settles the claim-side objective using the minimum across initial states for `>=` and the maximum for `<=`; entries retain every initial-state identity and its value (QSL-3142). | Test |
| FR-203-AC-27 | For an unconditional elapsed-time claim, EN-5 evaluates every admitted initial product state and settles the claim-side objective using the minimum across initial states for `>=` and the maximum for `<=`; entries retain every initial-state identity and its value (QSL-3142). | Test |
| FR-203-AC-28 | The exact solver refuses before it builds a dense linear system and stops when cancelled while it solves one. Policy evaluation, reward evaluation, stationary distribution and bias each check the inline bytes of a `rows x (rows + 1)` system, counted for the rational system and for the integer system the elimination builds from it, against `certificate_check.max_working_bytes` (`ExactProbLimits.document.storage_bytes`) before the first row is allocated; digits a cell holds on the heap are not counted. A system over the bound settles `incomplete`, `Incomplete(LimitReached{CertificateMaxWorkingBytes, bound, certificate_check.max_working_bytes})`. The bound is exact to the byte, a request for 2^40 rows is refused without allocating, and a system at or under the bound is solved as before. The elimination polls the run's cancel handle at every matrix row and every pivot row, and a cancelled handle settles `incomplete`, `Incomplete(Cancelled{source})`. | Test (TC-638) |
| FR-203-AC-29 | With `accounting.work_units = B`, an exact item whose product construction consumes `P` units and whose certificate production and independent checking together require `C` more units, where `P <= B`, `C <= B` and `P + C > B`, stops at the first unpaid certificate-phase charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; the certificate is not accepted or emitted and the item has no truth value. The same subject and claim with a bound of `P + C` completes as before. | Test |
| FR-203-AC-30 | With `accounting.work_units = B`, an exact item whose product construction consumes `P` units and whose counterexample witness production and replay together require `W` more units, where `P <= B`, `W <= B` and `P + W > B`, stops at the first unpaid witness-phase charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`; it does not settle `refuted` or emit a replayed witness. The same subject and claim with a bound of `P + W` settles as before. | Test |
| FR-203-AC-31 | When an exact item's undefined-evaluation classification performs re-exploration after its initial product work, the re-exploration charges the same `accounting.work_units` allowance. A fixture in which the initial work and re-exploration each fit bound `B` separately but exceed it cumulatively stops at the first unpaid re-exploration charge with `incomplete`, `Incomplete(LimitReached{work_units, B, accounting.work_units})`, and no truth value; a separate exact item starts with a fresh allowance. | Test |

## Dependencies

- ADR-028 LM-1 to LM-3, XV-1 to XV-7, AR-4, SP-5; ADR-014 B-5 and ADR-013
  O-16, O-24 as amended by ADR-028.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the terminal record and `ProofBasis` it extends),
  [FR-201](FR-201-check-a-probability-certificate.md),
  [FR-202](FR-202-replay-a-probabilistic-witness.md).

## References

- [FSpec FR-331: Backend provider envelope](ix://agent-ix/filament-specification/FR-331)
  owns the value-free `exhaustive` result carrier and its sole permitted
  conditional `vacuous` certificate exception, with `evidence` forbidden.
  [FSpec FR-412: Probability certificate and its checker](ix://agent-ix/filament-specification/FR-412)
  owns the conditional `vacuous` certificate and the independent
  unreachable-activation check. The certified vacuity settlement above
  depends on both contracts.
- FSpec half, which owns `ExactProbLimits` and its setting names in the
  request, while FSL publishes the default values, and the result content
  onto FR-331 and FR-243: FSpec FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.
