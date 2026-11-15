# Benchmarking

This guide defines how to run benchmarks and compare a candidate with current
main. Decisions use only measurements collected live for the current A/B run;
no stored measurements or past results are comparison inputs.

## Running the benchmarks

| Axis | Command | Bench file | Input generator |
| --- | --- | --- | --- |
| Parser: nesting depth and volume | `make bench-parser` | `benches/parser.rs` | `src/parse.rs` |
| Checker: call chain and independent functions | `make bench-checker` | `benches/checker.rs` | `src/check.rs` |
| CST identity hashing against source bytes | `make bench-cst` | `benches/cst.rs` | `src/parse.rs` |
| Model layer on an N-type `DomainPackage` built through FR-154 intake | `make bench-model` | `benches/model.rs` | `src/model.rs` |
| Evaluator cost per call frame | `make bench-evaluator` | `benches/evaluator.rs` | `src/check.rs` |
| Checker: Text-reachable recursive cluster | `make bench-text_cluster` | `benches/text_cluster.rs` | `src/text_cluster.rs` |
| Model check: `mc/state_graph` (plain and symmetric), `mc/safety` (invariant and deadlock), `mc/liveness`, `mc/frame_candidates`, `mc/observation` | `make bench-mc` | `benches/mc.rs` | `src/mc.rs`, `src/world.rs` |
| Protocol reach `reach/workers` at 1, 2, 4 and all workers | `make bench-reach` | `benches/reach.rs` | `src/mc.rs`, `src/world.rs` |
| Canonical state-key encoding `key/encode` | `make bench-key` | `benches/key.rs` | `src/mc.rs`, `src/world.rs` |
| EN-4 Okamoto sampling `en4/sample` | `make bench-en4` | `benches/en4.rs` | `src/mc.rs`, `src/world.rs` |
| Counts, refusal boundaries, one-shot large inputs and peak RSS | `make bench-probe` | `src/bin/fsl-bench-probe.rs` | all of the above |

`make bench` runs all ten criterion suites. Each target runs
`cargo bench --locked -p fsl-bench --bench <axis>`, and criterion options go
through `BENCH_ARGS`. Run benches with `-p fsl-bench`, never `--workspace`;
other crates' dev-dependencies can enable `filament-exact/test-support` and make
the meter log every charge.

Each input is built through the layers' public API. A change behind that API
needs no benchmark change; an API change requires updating its generator. Each
generator has a smallest-size test so invalid inputs fail in `make ci` before
benchmark time.

## Comparing a candidate with current main

1. Build current `origin/main` (A) and the candidate (B) in separate worktrees
   with separate target directories.
2. In one session, alternate A and B for five rounds each on the same machine;
   record load average before and after every round.
3. For each benchmark, calculate each side's median of its five point estimates.
4. Calculate the session noise from those ten live observations: the largest
   of A's and B's MAD/median and, for criterion runs, the widest 95%
   confidence-interval half-width divided by its estimate. Probe runs use the
   two sides' MAD/median because they do not report criterion intervals.
5. B is faster only if its median is below A's median by more than that session
   noise. A smaller difference is no change.
6. Record every round for both sides. Do not select favorable rounds or reuse
   measurements from an earlier session as a threshold.

The A/B runs are the comparison. No fixed table variance or earlier
measurement collection supplies a decision threshold.

## Model-check, reach, key and sampling axes

Each axis has an input generator in `src/mc.rs`, a size knob, a
smallest-size test that fails in `make ci` when the generator stops
producing a valid input, a measurement plan (`spec/assurance/MP-007` to
`MP-011`) and a name under `make bench` (a criterion group, selectable with
`BENCH_ARGS='<axis>'`, for example `make bench-mc BENCH_ARGS='mc/state_graph'`)
and under `make bench-probe` (`fsl-bench-probe mc <axis> <sizes>`).

| Axis (criterion group; probe axis) | Input generator and size knob | Unit |
| --- | --- | --- |
| `mc/state_graph` (`state_graph`, `state_graph_symmetric`) | `ring_state_graph(n, k, symmetric)`: `n` objects each cycling `k` values, `k^n` states; symmetric declares one class of all `n` keys | seconds per call; states and edges per second |
| `mc/safety` (`safety`, `deadlock`) | `ring_safety(n, k)`: the same ring with the invariant `always holds(c.v <= k-1)`; `deadlock` checks deadlock freedom | seconds per call; states and edges per second |
| `mc/liveness` (`liveness`) | `ring_liveness(n, k)`: `always eventually holds(c.v = 0)` under `fair weak each step`, over the product with the property automaton | seconds per call; states and edges per second |
| `mc/frame_candidates` (`frame_candidates`) | `wide(m, r)`: one object, one contract-free operation writing `m` fields of `Int[0, r]`, `(r+1)^m` candidates per expansion | seconds per call; edges per second |
| `mc/observation` (`observation`) | `observation(R)`: a two-state model beside `R` unrelated object types and populations in the same domain package | seconds per call |
| `reach/workers` (`reach_1`, `reach_2`, `reach_4`, `reach_all`) | `protocol(b)`: `b` parallel attempts of one operation, reached under `Sc` with 1, 2, 4 and all available workers | seconds per call; states and edges per second |
| `key/encode` (`encode`) | `keys(n, f)`: a state of `n` objects of `f` integer fields | seconds per call; key bytes |
| `en4/sample` (`sample`) | `service(eps)`: the `Service` model, `P95` by Okamoto, indifference `eps` in ten-thousandths; 400, 120 and 48 draw about 10^3, 10^4 and 10^5 samples | seconds per call; samples per second |

The probe reports, per run, states, edges and samples with their rates over
the timed run (building the input is excluded) and peak RSS. `fsl-bench-probe
mc-marginal <axis> <small sizes> -- <large sizes>` runs the small input and
then the large one in one process and prints the peak RSS the large run added
per state it added; peak RSS is monotone, so the pair must ascend.

Peak RSS prints a number on Linux (`VmHWM`) and macOS (`getrusage(RUSAGE_SELF)`
through nix's safe API). Darwin reports `ru_maxrss` in bytes; the probe converts
it to KiB. Compare runs on the same OS, and treat macOS measurements made with
the earlier `vmmap` reader as a separate series.

### Workload counters

`cargo test -p fsl-bench --features counters` enables the `counters` feature
(off by default). It adds `fsl_bench::counters`, which counts states, edges,
generated candidates, canonical encodes, atom evaluations and expansions of
each axis's smallest input, and a test asserts those totals so that a
generator change cannot silently change the workload. The qualified core
forbids a mutable global (FR-284), so the counts are taken at public seams: a
breadth-first search through `TransitionSystem` for expansions, edges,
candidates, encodes and states, and the bench's own `ModelLetters` for atom
evaluations. A candidate an expansion generates and a contract rejects is not
visible at that seam, so generated candidates equal edges. The `reach/workers`
and `en4/sample` axes report the engine's own states, edges and samples
instead.

These counters fingerprint the generator, not the engine run being timed:
expansions, edges, generated candidates, canonical encodes and states come
from the bench's own search over the plain model, so they do not move when an
engine change makes the engine expand, encode or generate less, and they
cannot show that improvement. For `mc/liveness` at 2x2 the search counts 4
states and 8 edges while `check_model` explores 12 product states and 28
edges. The engine-reported states and edges (`Work`) are what the probe
rates and the criterion throughput use.

### Symmetry on `mc/state_graph`

A state-graph claim is decided per key of the population. With a symmetry
class over all keys, the run keeps one orbit representative as the claim's
instance and quotients by the subgroup that fixes it. For `n` interchangeable
rings of `k` values that is `k * C(k + n - 2, n - 1)` states (18 at 3x3), not
the 10 orbits of the full group, and two objects reduce nothing. A
`symmetric` size therefore measures this reduced graph.
