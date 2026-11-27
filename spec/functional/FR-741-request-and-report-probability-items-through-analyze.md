---
id: FR-741
title: "Request and report probability items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-189
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-193
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-195
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1041
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1043
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-254
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-413
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-741: Request and report probability items through analyze

## Description

A `native-analyze/1` item (FR-620) that names a `probabilistic` claim SHALL run
on EN-4, statistical model checking (FR-189 to FR-193, FR-254), or on EN-5,
exact probabilistic checking (FR-195 to FR-204), as its evidence kind selects,
over the checked package S4 emits with the model's random parameters,
workloads, rewards and delays (FR-740), and SHALL report its verdict and its
evidence in the FR-621 item members. Every item class FSpec FR-641 requires
is reachable this way; the library request types are not the public
contract, this requirement is.

## Inputs

### Request item

- `claim`: the declared name of a `probabilistic` claim; `kind`:
  `probabilistic-satisfaction`, the only kind of that family and so the
  default.
- `probabilistic.evidence`: `statistical` or `exact`, in FSpec FR-331's
  spelling. Absent means the evidence kind S3 recorded on the claim's
  requirement record (FR-195): `exact` for an exact-only claim, `statistical`
  otherwise.
- For `statistical`: `probabilistic.method`, FSpec FR-331's
  `{"type": T}` with `T` one of `okamoto`, `sprt` and `clopper-pearson`, or
  `{"type": "regenerative", "min_cycles": n}`; absent, FSL writes
  `okamoto` for a bounded form and `regenerative` with `min_cycles` 100 for a
  long-run fraction (FR-189, FR-190); `probabilistic.seed`, a decimal string,
  absent meaning `"0"`; `max_witnesses`, absent meaning 8; `workers` (default 1); over a timed subject the grid exponent
  `q` and `max_sample_steps` (FR-254); and `probabilistic.limits`, the
  `StatisticalLimits` members (FR-191).
- For `exact`: `probabilistic.limits`, the `ExactProbLimits` members
  (FR-203).
- FSL writes every member FSpec FR-331 requires of the item that the caller
  leaves out, with the defaults above, so the item it hands to negotiation is
  a complete FR-331 item.
- Each absent limit takes its published default (FSpec FR-461). The method
  is a request setting: it enters neither the claim's normal form nor its
  obligation identity (FR-186).
- For either engine's model subject, the native adapter SHALL use the
  finite local `expansion.max_candidates` default of 1,000,000 and the
  cumulative `accounting.work_units` default of 1,000,000 under
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) and
  [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md).
  The adapter SHALL preserve the caller's accounting limits instead of
  replacing them with defaults or an unlimited allowance.

## Outputs

### Result members

- `record`: for statistical evidence, FR-192's settlement of the
  `StatisticalOutcome`: `measured` with its verdict, or `incomplete`,
  `unsupported` or `invalid-request` with its cause; for exact evidence,
  FR-203's settlement: `proved` with `ExactValue` or `ValueBounds` and
  certification `certified`, `refuted`, `incomplete`, `inconclusive` or
  `unsupported`.
- `verdict`, for a `measured` record (FSpec FR-331's `measured | verdict`): the
  FR-410 `StatisticalVerdict`, holding the method, the seed, the sampler
  identity, and per test its index, initial state, binding, instance,
  trace-index range, counted samples, drawn samples, estimate and interval
  (no interval for `sprt`), as FR-189 and FR-191 define them, and its
  `witnesses`: up to `max_witnesses` sampled witnesses (FR-193), each with its
  trace index and the outcome it records, replayable from the item and the
  emitted package with no engine present.
- `certificate`, for an exact `proved` record: FR-621's `{identity, checker,
  accepted}`, plus `kind` (`exact`, `lower`, `upper`, `long-run` or
  `conditional`) and
  `document`, the certificate in FSpec FR-331's `ProbabilityCertificate`
  spelling (FR-201's closed profile where one applies), so that
  `fsl-probability-check` accepts the same document in a later process from
  it, the emitted package's byte provision and the subject binding. For a
  `ValueBounds` proof, `kind` and `document` are the claim side's (`lower`
  for `AtLeast`, `upper` for `AtMost`), and `counterpart` is the other
  side's `{kind, document, accepted}`, both in FR-1042's `value-bounds/v1`
  profile; a `lower` proof found on a partial product has no `counterpart`
  and its record says `exploration: Partial`. A quantile under a workload has kind `conditional`, mode
  `ratio`, with FR-1043 parts.
- `counterexample`, for an exact `refuted` record: FR-202's probabilistic
  witness in FSpec FR-413's spelling, with the item's `replay` member
  (FR-621) present beside it only after FR-202 replay reproduced it; under
  every scheduler it carries the witness scheduler. An undefined evaluation
  refutes with FR-202's `Undefined` evidence in FSpec FR-413's
  `undefined-path` spelling, and the record's cause
  equals its `undefined`. An undefined read of an on-each claim's guard or target is not yet built (FR-875 "Unbuilt on-each settlements (current)"). A refuted `on each`
  claim's counterexample names the failing instance in its binding's
  `instance` member: the first failing instance in key order (FR-875).
- `entries`, for exact evidence: FR-203's `ExactEntry` per initial state and
  binding, with the extremum under every scheduler.

### Binding

Every binding a result member names, in `verdict`'s tests, in
`counterexample` and in `entries`, has two members: the `over` binding
and `instance`, the FR-181 reference of the `on each` object the
instance is read for, the key FR-196 places in the product key. `instance`
is null for an `on origin` claim. FSpec FR-331 spells them as the
`binding` and `instance` members of its statistical test row and of FSpec
FR-413's counterexample. Two instances of one `over` binding are two
bindings, each with its own test row or entry.

## Behavior

- **Classes.** The driver SHALL route each item to the engine its evidence
  kind selects, through FR-075 negotiation, for every class of FSpec FR-641:
  bounded events, quantiles and means of fractions; unbounded reachability,
  until and always (FR-198); expected rewards; long-run fractions (FR-199);
  every scheduler with and without fairness sets (FR-200); and timed claims,
  by sampling (FR-254) or through digital clocks (FR-204).
- **Unbounded exact claims.** An item naming one of FR-195's unbounded
  forms with `statistical` evidence settles `unsupported` `exact-only-form`
  with no sample drawn (FR-195), and with `exact` evidence runs on EN-5.
- **Budgets.** A run that reaches a limit stops and settles `incomplete` with
  `LimitReached` naming the limit, its value and the `statistical.<member>` or
  `exact_prob.<member>` setting that raises it, or `expansion.max_candidates`
  for the independent local candidate ceiling or `accounting.<counter>`
  for an accounting stop. Candidate and clause work SHALL consume one
  cumulative allowance for the item, rather than a renewed allowance per
  expansion or clause. A denied charge SHALL stop before that work is
  performed and retain the actual typed bound through
  [FR-192](FR-192-settle-a-statistical-result-on-the-measured-axis.md) or
  [FR-203](FR-203-bound-an-exact-run-and-settle-its-verdict.md) to the native
  result, with no truth value or additional wall-clock cutoff.
- **Cancellation.** A cancelled or expired run settles `incomplete` with the
  cancellation or deadline cause and reports what it reached.
- **Reproducibility.** Two runs of one request with one seed SHALL give equal
  item results, whatever `workers` is (FR-191).
- **Native model execution (current).** The native model probability route
  executes serially today, without a worker pool. It retains and reports
  the request's `workers` value but does not forward it to the model
  sampler; it does not coerce the input to 1. Serial execution follows
  [FR-191](FR-191-bound-and-reproduce-a-statistical-run.md)'s logical
  admission order directly. Equal results for requested worker counts do
  not establish multiworker execution on this route.
- **Refusals.** A claim `under every scheduler`, an exact-only form, or a
  claim with no confidence parameters given `statistical` evidence settles
  `unsupported` with `every-scheduler`, `exact-only-form` or
  `missing-confidence` (FR-195); a method the form does not admit settles
  `invalid-request` `not-statistical` (FR-189); a rejected certificate
  settles `inconclusive` `certificate-rejected{rule, at}` and a witness
  replay that disagrees settles `inconclusive` `replay-parity` (FR-203).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-741-AC-1 | Over FR-185-AC-1's `Service` compiled from source, the items `{"claim": "P95", "probabilistic": {"evidence": "statistical", "method": {"type": T}, "seed": "7"}}` for `T` each of `okamoto`, `sprt` and `clopper-pearson`, and `{"claim": "LongRun", "probabilistic": {"evidence": "statistical", "method": {"type": "regenerative", "min_cycles": 100}, "seed": "7"}}`, each settle `measured` with a `verdict` naming the method and seed, and the same request gives equal item results with `workers` 1 and 4; each witness it reports replays. | Test (TC-2066) |
| FR-741-AC-2 | Over the same `Service`, an exact item for `P95` with no confidence parameters settles `proved` `ExactValue{24233/25000}` with a `certificate` of kind `conditional`, mode `ratio`, whose parts are FR-1043 `finite-horizon-measure/v1` documents (numerator `24233/25000`, denominator 1 at the initial key) and whose `document` `fsl-probability-check` accepts in a separate process; over ADR-028 §15.4's `Link`, exact unbounded reachability, until, always and expected-reward items, an every-scheduler item and §15.3's long-run item each settle with an accepted certificate, the long-run one of kind `long-run`; a fair every-scheduler unbounded eventual item settles with an accepted FR-1041 `cyclic-mdp-fair-eventual/v1` certificate; §15.1's claim with `max_rational_bits` 1,024 settles `ValueBounds` with a `lower` `certificate` and an `upper` `counterpart`, both FR-1042 `value-bounds/v1` documents accepted by `fsl-probability-check`. | Test (TC-2066) |
| FR-741-AC-3 | An exact item for a claim whose exact value is below its threshold settles `refuted` with a `counterexample` and an item-level `replay` that is present and reproduced; FSpec FR-421-AC-1's timed claim settles through digital clocks with an accepted certificate; FR-203-AC-5's undefined claim settles `refuted` with an FSpec FR-413 `undefined-path` `counterexample` whose `replay` is present and reproduced, at either threshold. | Test (TC-2066) |
| FR-741-AC-4 | `probability >= 0.99 [eventually holds(p.done)]` under the infinite-trace profile with `statistical` evidence settles `unsupported` `exact-only-form` with no sample drawn; a `mean of` claim requested with the method `sprt` settles `invalid-request` `not-statistical`; a bounded claim requested with Okamoto and `statistical.max_samples` 10, below its required `N`, settles `incomplete` before sampling, naming `statistical.max_samples`, required N and configured value 10, with zero samples and draws. | Test (TC-2066) |
| FR-741-AC-5 | Through `fsl analyze` over the `Link` model under `Even`, with a start snapshot holding `m` undelivered and `n` delivered, FR-875's claim `E` at `rational(9, 10)` settles `proved` with a `certificate` of kind `conditional`, mode `ratio`, whose numerator is `391/400` and denominator 1 at `m`'s initial key, `n`'s confirmed vacuous, and whose `document` `fsl-probability-check` accepts in a separate process; at `rational(99, 100)` it settles `refuted` with a `counterexample` whose `instance` is `m`'s reference and whose `binding` is null. | Test (TC-3062) |
| FR-741-AC-6 | For each engine, a source-model probability item requiring more than 1000000 cumulative candidate and clause work units under otherwise sufficient limits settles `incomplete`, `LimitReached{work_units, 1000000, accounting.work_units}` before the first unpaid charge, even though each expansion fits the local ceiling. Omitting the two limits and explicitly selecting their defaults through caller limits give equal item results. A caller accounting bound B is preserved and reported as B when reached; a local candidate denial instead reports `max_candidates`, its local bound and `expansion.max_candidates`. Neither stop carries truth or a measured decision. | Test |

## Dependencies

- [FR-620](FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md) and
  [FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md)
  own the request and result documents; FSpec FR-331 the item members and
  FSpec FR-641 the required item classes.
- EN-4: [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)
  to FR-193 and FR-254; EN-5: [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md)
  to FR-204; [FR-740](FR-740-emit-random-parameters-workloads-rewards-and-delays-as-v2-probabilistic-nodes.md)
  supplies the package.
