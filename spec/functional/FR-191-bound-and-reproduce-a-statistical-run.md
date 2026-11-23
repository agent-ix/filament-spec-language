---
id: FR-191
title: "Bound and reproduce a statistical run"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-022
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-189
    type: depends_on
---
# FR-191: Bound and reproduce a statistical run

## Description

EN-4's cost SHALL be bounded only by caller-set budgets,
`StatisticalLimits{max_samples, max_draws, max_cycle_steps}`, each an
ADR-014 B-5 budget with a published default, plus time, the clause meter,
[FR-120](FR-120-simulate-a-checked-package-s-state-family.md)'s cumulative
candidate and clause work allowance, the local expansion ceiling and
cancellation (ADR-024 ST-8). Reaching a budget SHALL stop the run and name
the budget, its value and the request setting that raises it, `max_cycle_steps` included (FR-190). EN-4 SHALL
identify a statistical run by its provenance, from which the run reproduces
exactly (ADR-024 RP-1 to RP-3).

## Use case

An operator runs `NoFault` with `max_samples` set to 1,000,000, below
Okamoto's 9,210,341 samples, and the run stops there. The result says
`max_samples` was reached at 1,000,000 and that `statistical.max_samples` raises
it. The operator raises it
and reruns with the same seed, and the rerun reproduces every sample the
first run drew.

## Inputs

```rust
pub struct StatisticalLimits {
    pub max_samples: u64,      // counted samples or cycles per test; default 16_777_216 (2^24)
    pub max_draws: u64,        // samples drawn, counted or not, per test (steps taken, for the regenerative method); default 67_108_864 (2^26)
    pub max_cycle_steps: u64,  // steps in one regeneration cycle; default 16_777_216 (2^24)
}
pub enum StatisticalLimit { MaxSamples, MaxDraws, MaxCycleSteps }  // setting() = `statistical.<member>`
```

## Outputs

- `StatisticalOutcome::Stopped{stop, partial, provenance}` (FR-189), where
  `stop` is `Limit{limit, value}` for a budget (with `statistical.<member>`
  as the request setting that raises it), `Expansion(ExpansionStop)` for a
  stopped sample expansion such as the clause meter, whose own limit, value
  and setting FR-192 reports (ADR-024 SV-4), or `Cancelled(cause)` for a
  cancelled `Cancel` handle; and `partial`, the aggregate `samples`, `draws`
  and test count reached across the run.
- On every outcome (a `Completed` verdict, a `Stopped` and an `Unsupported`,
  each carrying the field, FR-189), `StatisticalProvenance{seed, sampler,
  tests}` with, per test, its index, initial state, binding and trace-index
  range. A refusal carries none: it is a function of the request, so
  repeating the request reproduces it.

## Behavior

### Budgets

- Each `StatisticalLimits` member SHALL be set by the request, with the
  published default above when the request omits it. Its setting name SHALL
  be `statistical.` followed by the member name (`statistical.max_samples`,
  `statistical.max_draws`, `statistical.max_cycle_steps`), by FR-255's
  convention. FSpec FR-408 owns each limit and its setting name; FSL
  publishes the default value. No member SHALL be
  fixed by the language, and none enters the obligation identity.
- Before drawing any sample, EN-4 SHALL compute the fixed sample count `N`
  required by Okamoto or Clopper–Pearson for every test. In test-index order,
  it SHALL compare `N` with that test's `max_samples`, then `max_draws`. If
  `N` exceeds either configured value, EN-4 SHALL return
  `Stopped{PreflightLimit{limit, value, required: N}, partial, provenance}`
  for the first exceeded limit, where `value` is the configured limit and
  `partial` reports zero samples and zero draws for every test. FR-192 SHALL
  settle it `incomplete`, naming the setting, configured value and required
  `N`; no sample or draw occurs. A limit equal to `N` admits the run.
- When a test has counted `max_samples` samples or cycles and its method
  needs another, EN-4 SHALL stop and return `Stopped` with
  `Limit{MaxSamples, value}`, before it merges the next sample (FR-191 "Workers" for physical draws); a method that decides
  at exactly `max_samples` completes. When a test has drawn `max_draws`
  samples (for the regenerative method, taken `max_draws` walk steps in total, as FR-190 counts steps as draws), or taken `max_cycle_steps` steps in one cycle, it SHALL stop the same
  way before the next draw or step. The stop carries the limit, its value, the
  setting name (`statistical.max_samples`, `statistical.max_draws`,
  `statistical.max_cycle_steps`) and the counts reached, and settles
  `incomplete`, `limit-reached{limit, value, setting}` (FSpec FR-408). A
  stopped sample expansion (the clause meter) and a cancelled `Cancel` handle
  (FR-276) SHALL stop the run the same way with their own stops.
- SPRT and regenerative runs, whose stopping sample count is not fixed by
  the request before sampling, SHALL retain stop-at-limit behavior. A stopped
  run's `partial.samples` SHALL report the total counted samples across tests
  and `partial.draws` the total physical draws through the last admitted trace
  index. `partial.tests` SHALL be the number of tests. `draws` includes a
  drawn sample whose evaluation is undefined; that sample does not increase
  `samples`. Its `UndefinedEvaluation`, including sampled location and
  catalogued cause, is reported in the completed test result (FR-189). For
  regeneration, `samples` counts completed positive-weight cycles and `draws`
  counts walk steps, including steps in an unfinished cycle (FR-190).
- A run stopped before its method completes SHALL never carry a decision; an
  Undecided decision is kept for a run that completed its method.
- The method, `min_cycles` and `max_witnesses` SHALL be request settings,
  recorded in the result, and none enters the obligation identity.

### Workers

The worker-execution contract below applies to routes that execute worker
threads. The native model probability route currently executes serially,
as [FR-741](FR-741-request-and-report-probability-items-through-analyze.md)
states; retaining a requested worker count does not establish parallel
execution. The cumulative logical admission order below also applies to
serial execution.

- A request SHALL carry `workers: usize` (`StatisticalRequest.workers`,
  FR-189), the number of threads that may draw samples at once, setting
  `statistical.workers`, default 1. It is not a budget and enters neither
  the obligation identity nor the provenance. On a route that executes
  workers, every admitted sampler and subject SHALL support `workers`
  above 1.
- `workers` 0 SHALL return `StatisticalRefusal::Workers`, catalog code
  `invalid_runtime_input`/`invalid-value`, before any sample is drawn.
- A sample is a function of the seed, the sampler and its trace index
  alone, so workers MAY draw samples of different trace indices
  concurrently. The run SHALL merge samples in trace-index order and SHALL
  charge `max_samples`, `max_draws` and `max_cycle_steps`, and run the
  method's stopping rule (Okamoto's `N`, SPRT's bounds, the regenerative
  cycle count), in that merge order. At most `workers` samples are drawn
  and not yet merged at any time, so a run stopped at a trace index has
  drawn at most `workers - 1` samples beyond it. Those samples SHALL be
  discarded and SHALL appear in no count, estimate, witness or provenance
  range; the "before it draws the next sample" of Budgets is a statement
  about the merge, not about the number of physical draws.
- For every `workers >= 1` a run SHALL return the same decision, counts,
  estimates, witnesses, provenance and stop (limit and value) as the
  one-worker run.
- For a model subject, admission of candidate and clause work against
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)'s cumulative
  allowance SHALL follow the one-worker logical order: tests in ascending
  test index, samples in trace-index order within each test, and each
  sample's existing deterministic expansion and clause-charge order.
  Earlier tests may exhaust the shared allowance before a later test
  begins; the run SHALL NOT divide it into fresh per-test quotas.
- Workers SHALL admit or reserve cumulative work before performing it,
  preserving the entitlement of every earlier logical charge and the same
  first unpaid charge as the one-worker run. Speculative samples SHALL
  NOT consume that entitlement or perform candidate or clause work outside
  the physical run allowance, even when later discarded under the sample
  merge rule above. Worker scheduling SHALL NOT choose the exhausted
  bound or change the stop, counts or provenance. These rules SHALL retain
  every supported positive worker count and the existing cancellation
  semantics, without prescribing a locking or scheduling mechanism.
- A run stopped by `Cancel` MAY stop at a different trace index from one
  run to the next, and its result SHALL be the one-worker result at some
  trace index, with every sample before it counted and none after it.

### Provenance and reproduction

- The provenance SHALL record the seed, the sampler identity, and
  per test its index, initial state index, `over` binding and the
  trace-index range it used (`i · m + j` for sample `i` of test `j`).
- Re-running EN-4 with the same subject, claim, method, `min_cycles`, seed,
  sampler and limits SHALL reproduce every sample, every count, every
  estimate and the decision.
- A rerun with only the limits raised SHALL draw, for every trace index both
  runs reach, the same sample as the stopped run.
- A rerun with a different seed SHALL draw independent samples of the same
  obligation, and its result SHALL have the same obligation identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-191-AC-1 | `NoFault` (ADR-024 §7.1) with Okamoto has required `N = 9,210,341`. With `max_samples` 1,000,000, it returns `PreflightLimit{MaxSamples, value: 1,000,000, required: 9,210,341}`, settles `incomplete` naming `statistical.max_samples`, configured value 1,000,000 and required N, and reports zero samples and draws. The same result occurs when only `max_draws` is 1,000,000, naming `statistical.max_draws`; no sample is drawn. With either bound at 9,210,341, it completes and decides. A request that omits `max_samples` runs with 16,777,216. | Test (TC-626) |
| FR-191-AC-2 | FR-189-AC-4's activation variant with SPRT and `max_draws` 1,000 stops naming `statistical.max_draws`, with `partial.samples = 0` and `partial.draws = 1,000`; a cancelled `Cancel` handle stops with `Cancelled`; neither carries a decision. FR-190-AC-2's transient variant stops with `Limit{MaxCycleSteps, value}` and no decision, settling `incomplete`, `limit-reached` naming `statistical.max_cycle_steps`, with its elapsed walk steps in `partial.draws`. A regenerative run with `max_draws` below the steps one cycle needs stops `Limit{MaxDraws, value}` with no decision, `partial.draws = value` and `partial.samples = 0`. | Test (TC-626) |
| FR-191-AC-3 | `P95` with SPRT and seed 7 run twice gives byte-equal results: provenance, per-test counts, estimates and decision. The run stopped by `max_samples` 100 and the rerun with the default limit draw equal samples at trace indices 0 to 99. A run with seed 8 records seed 8, the same obligation identity, and a different sample at some trace index. | Test (TC-626) |
| FR-191-AC-4 | A two-test run records per test its initial state index, binding and trace-index range, with test 1's first trace index equal to 1 and its step 2. Changing the method from Okamoto to SPRT changes the result's method and leaves its obligation identity unchanged. | Test (TC-626) |
| FR-191-AC-5 | `NoFault` with Okamoto and `max_samples` 9,210,341 completes; with 9,210,340 it returns `PreflightLimit{MaxSamples, value: 9,210,340, required: 9,210,341}` with zero samples and draws. An activation variant with fixed `N` above `max_draws` returns the corresponding `PreflightLimit` with the same required N and zero counts. A `Stopped` and an `Unsupported` outcome (FR-189-AC-7's `Wide`) each carry the run's `StatisticalProvenance`, and a cancelled handle stops `Cancelled`. | Test (TC-626) |
| FR-191-AC-6 | `P95` with SPRT and seed 7, and `NoFault` with Okamoto and `max_samples` 1,000 (a stop well below Okamoto's `N`), each run with `workers` 1, 2, 4 and 64, give results byte-equal to the one-worker run's: provenance, per-test counts, estimates, witnesses, decision and, for the second run, `Limit{MaxSamples, 1,000}` with 1,000 samples counted. A sampler test double that makes the sample at a higher trace index finish first changes no result. The run stopped by `max_samples` 100, with `workers` 4, draws samples at trace indices 0 to 99 equal to the one-worker run's samples at those indices and counts none beyond 99; the double records at most 103 draws. | Test (TC-1417) |
| FR-191-AC-7 | A run with `workers` 4 whose `Cancel` handle is cancelled after a delay returns `Cancelled` with counts and estimates equal to the one-worker run stopped at the same trace index, for the index it reached; the result carries no sample of a higher trace index. `workers` 0 returns `StatisticalRefusal::Workers` (`invalid_runtime_input`/`invalid-value`) and draws no sample. | Test (TC-1417) |
| FR-191-AC-8 | A two-test model-subject run with cumulative candidate and clause bound B, sufficient for each test alone but insufficient for both in ascending test order, spends the remainder on the later test and stops at the same first unpaid charge with workers 1, 2, 4 and 64. Reversing completion order of speculative samples changes no stop, counts, provenance, estimates or witnesses. Instrumented admitted candidate and clause work never exceeds B across all workers, including discarded samples; no later sample consumes an earlier sample's entitlement. If the first test alone exhausts B, the later test performs no work. Cancellation retains AC-7's prefix semantics. | Test |

## Dependencies

- ADR-024 ST-8, SV-4, RP-1 to RP-3; ADR-014 B-5, TR-1, TR-6.
- [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md).

## References

- FSpec half, which owns `StatisticalLimits` and its setting names in the
  request and the trace-index assignment, while FSL publishes the default
  values: FSpec FR-408 (Linear STD-137).
- Owning ticket: Linear QSL-371.
