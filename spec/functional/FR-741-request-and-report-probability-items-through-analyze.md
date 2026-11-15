---
id: FR-741
title: "Request and report probability items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
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
  - target: ix://agent-ix/filament-spec-language/FR-254
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-740
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

## Outputs

### Result members

- `record`: for statistical evidence, FR-192's settlement of the
  `StatisticalOutcome`: `measured` with its verdict, or `incomplete`,
  `unsupported` or `invalid-request` with its cause; for exact evidence,
  FR-203's settlement: `proved` with `ExactValue` or `ValueBounds` and
  certification `certified`, `refuted`, `incomplete`, `inconclusive` or
  `unsupported`.
- `measurement`, for statistical evidence: the method, the seed, the sampler
  identity, and per test its index, initial state, binding, trace-index
  range, counted samples, drawn samples, estimate and interval (no interval
  for `sprt`), as FR-189 and FR-191 define them.
- `witnesses`, for statistical evidence: up to `max_witnesses` sampled
  witnesses (FR-193), each with its trace index and the outcome it records,
  replayable from the item and the emitted package with no engine present.
- `certificate`, for an exact `proved` record: FR-621's `{identity, checker,
  accepted}`, plus `kind` (`exact`, `lower`, `upper` or `long-run`) and
  `document`, the certificate in FSpec FR-331's `ProbabilityCertificate`
  spelling (FR-201's closed profile where one applies), so that
  `fsl-probability-check` accepts the same document in a later process from
  it, the emitted package's byte provision and the subject binding.
- `counterexample`, for an exact `refuted` record: FR-202's probabilistic
  witness in FSpec FR-413's spelling, with `replay` present only after FR-202
  replay reproduced it; under every scheduler it carries the witness
  scheduler.
- `entries`, for exact evidence: FR-203's `ExactEntry` per initial state and
  binding, with the extremum under every scheduler.

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
  `exact_prob.<member>` setting that raises it; a statistical run reports the
  counts it reached in `measurement`.
- **Cancellation.** A cancelled or expired run settles `incomplete` with the
  cancellation or deadline cause and reports what it reached.
- **Reproducibility.** Two runs of one request with one seed SHALL give equal
  item results, whatever `workers` is (FR-191).
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
| FR-741-AC-1 | Over FR-185-AC-1's `Service` compiled from source, the items `{"claim": "P95", "probabilistic": {"evidence": "statistical", "method": {"type": T}, "seed": "7"}}` for `T` each of `okamoto`, `sprt` and `clopper-pearson`, and `{"claim": "LongRun", "probabilistic": {"evidence": "statistical", "method": {"type": "regenerative", "min_cycles": 100}, "seed": "7"}}`, each settle `measured` with a `measurement` naming the method and seed, and the same request gives equal item results with `workers` 1 and 4; each witness it reports replays. | Test (TC-2066) |
| FR-741-AC-2 | Over the same `Service`, an exact item for `P95` with no confidence parameters settles `proved` `ExactValue{24233/25000}` with a `certificate` of kind `exact` whose `document` `fsl-probability-check` accepts in a separate process; over ADR-028 §15.4's `Link`, exact unbounded reachability, until, always and expected-reward items, an every-scheduler item, a fair every-scheduler item and §15.3's long-run item each settle with an accepted certificate, the long-run one of kind `long-run`; §15.1's claim with `max_rational_bits` 1,024 settles with `lower` and `upper` certificates. | Test (TC-2066) |
| FR-741-AC-3 | An exact item for a claim whose exact value is below its threshold settles `refuted` with a `counterexample` whose `replay` is present and reproduced; FSpec FR-421-AC-1's timed claim settles through digital clocks with an accepted certificate. | Test (TC-2066) |
| FR-741-AC-4 | `probability >= 0.99 [eventually holds(p.done)]` under the infinite-trace profile with `statistical` evidence settles `unsupported` `exact-only-form` with no sample drawn; a `mean of` claim requested with the method `sprt` settles `invalid-request` `not-statistical`; `statistical.max_samples` 10 settles `incomplete` `LimitReached` naming `statistical.max_samples`. | Test (TC-2066) |

## Dependencies

- [FR-620](FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md) and
  [FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md)
  own the request and result documents; FSpec FR-331 the item members and
  FSpec FR-641 the required item classes.
- EN-4: [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)
  to FR-193 and FR-254; EN-5: [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md)
  to FR-204; [FR-740](FR-740-emit-random-parameters-workloads-rewards-and-delays-as-v2-probabilistic-nodes.md)
  supplies the package.
