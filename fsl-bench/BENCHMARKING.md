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
| Evaluator kernels against plain Rust | `make bench-evaluator_kernel` | `benches/evaluator_kernel.rs` | `src/kernel.rs` |
| Checker: Text-reachable recursive cluster | `make bench-text_cluster` | `benches/text_cluster.rs` | `src/text_cluster.rs` |
| Model check: `mc/state_graph` (plain and symmetric), `mc/safety` (invariant and deadlock), `mc/liveness`, `mc/frame_candidates`, `mc/observation` | `make bench-mc` | `benches/mc.rs` | `src/mc.rs`, `src/world.rs` |
| Protocol reach `reach/workers` at 1, 2, 4 and all workers | `make bench-reach` | `benches/reach.rs` | `src/mc.rs`, `src/world.rs` |
| Canonical state-key encoding `key/encode` | `make bench-key` | `benches/key.rs` | `src/mc.rs`, `src/world.rs` |
| EN-4 Okamoto sampling `en4/sample` | `make bench-en4` | `benches/en4.rs` | `src/mc.rs`, `src/world.rs` |
| Package emit/read and end-to-end replay request | `make bench-package` | `benches/package.rs` | `src/package.rs`, `src/world.rs` |
| Counts, refusal boundaries, one-shot large inputs and peak RSS | `make bench-probe` | `src/bin/fsl-bench-probe.rs` | all of the above |

`make bench` runs all twelve criterion suites. Each target runs
`cargo bench --locked -p fsl-bench --bench <axis>`, and criterion options go
through `BENCH_ARGS`. Run benches with `-p fsl-bench`, never `--workspace`;
other crates' dev-dependencies can enable `filament-exact/test-support` and make
the meter log every charge.

The evaluator-kernel target also runs its Rust report tool before and after
Criterion so it can identify and summarize only estimates from that invocation.

### Evaluator kernels

`make bench-evaluator_kernel` runs the `evaluator/kernel` group. It measures
`sum(n) = if n > 0 then n + sum(n - 1) else 0`, a three-comparison recursive
kernel, and a sequence fold, each at sizes 5 and 10. The generator is
`fsl_bench::kernel::KernelCase`; each interpreter call is paired with its
plain-Rust reference. After Criterion completes, the target prints the live
estimates in ns per frame and ns per logical operation, plus the
interpreter-to-Rust ns-per-operation ratio. Recursive kernels use `size + 1`
frames and `size` logical operations; the fold uses one frame and one logical
operation per sequence element. The report reads only estimates written by
that invocation, so a filtered run does not reuse an earlier estimate. The
checked package and arguments are exposed by `KernelCase` for reuse by another
evaluator backend.

Each input is built through the layers' public API. A change behind that API
needs no benchmark change; an API change requires updating its generator. Each
generator has a smallest-size test so invalid inputs fail in `make ci` before
benchmark time.

### Package emit and read (NFR-141)

`make bench-package` runs the separately selectable `package/emit`,
`package/read` and `replay/request` groups at 1, 10 and 100 timed models. Each model has one object
type with one clock and one integer value, one operation, one pre/post timed
transition and one temporal claim. `src/package.rs` prepares the models and
checked package before timing; the emit row calls the package emitter and the
read row admits the already emitted bytes through the evaluation-package
reader. The replay row executes one end-to-end function replay request over
the same source package. The invocation-local reporter prints Criterion's
live estimates as milliseconds per package for emit/read and milliseconds per
request for replay; emit/read also print the 1→10→100 scaling exponent.
`BENCH_ARGS='package/emit'`, `package/read` or `replay/request` selects one
axis. Parse-call counts are collected separately with the `counters` feature
at the replay spine's parser boundary using a caller-owned counter, so timed
Criterion replay does not include counter overhead.

The isolated replay RSS probe starts a fixture-preparation child and then a
fresh replay child. Preparation compiles the checked fixture and writes a
benchmark-local request artifact. The replay child reconstructs the prepared
request from that artifact before starting its wall-time timer; it does not
build the checked package or emission. Its peak RSS is process-wide and
includes artifact decoding and request reconstruction, but excludes the
fixture-preparation process. The report requires numeric positive RSS in KiB.

For the five-round A/B protocol, build both variants in this ticket's single
worktree. A is current `origin/main` plus only this PR's benchmark harness and
replay-counter seam; B is the candidate commit. Start with a clean worktree
and fetch the current main. Record each source SHA in the evidence alongside
the generated result JSON files. The report JSON intentionally records side
labels and measurements only; it does not infer a binary's source from the
checkout that happens to run the report.

```sh
git fetch origin main
git status --short
mkdir -p target/package-a target/package-b
git rev-parse HEAD > target/package-b/source-revision
git diff --binary origin/main...HEAD -- Cargo.toml Cargo.lock fsl-bench fsl-replay > target/package-ab-harness.patch
git switch --detach origin/main
git rev-parse HEAD > target/package-a/source-revision
git apply target/package-ab-harness.patch
CARGO_TARGET_DIR="$PWD/target/package-a" cargo build --locked --release -j 4 --bench package
CARGO_TARGET_DIR="$PWD/target/package-a" cargo build --locked --release -j 4 -p fsl-bench --features counters --bin fsl-bench-probe
git reset --hard origin/main
git switch <candidate-branch>
CARGO_TARGET_DIR="$PWD/target/package-b" cargo build --locked --release -j 4 --bench package
CARGO_TARGET_DIR="$PWD/target/package-b" cargo build --locked --release -j 4 -p fsl-bench --features counters --bin fsl-bench-probe
make bench-package-ab BENCH_PACKAGE_A_TARGET=target/package-a BENCH_PACKAGE_B_TARGET=target/package-b BENCH_PACKAGE_A_RESULTS=target/package-a-results.json BENCH_PACKAGE_B_RESULTS=target/package-b-results.json
```

Proceed only when `git status --short` is empty. If the PR branch needs to be
rebased onto the fetched main, do that on the same branch before building; if
the harness patch does not apply cleanly to current main, stop and reconcile
the branch before collecting any measurements.

The baseline patch carries the same benchmark generator and the PR's counter
seam on both sides. Criterion binaries are built without `counters`; probe
binaries are built with it. The reporter alternates A then B for five rounds,
records load average before and after each side's round, collects every
required Criterion estimate and isolated probe at M=1, 10, 100, and writes
generated side result files without overwriting existing files.
`report-ab <A-results.json> <B-results.json>` prints per-round measurements,
medians, session noise and the emit/read scaling exponents. Missing rows,
failed runs, missing load records, or incomplete rounds invalidate the report.
Criterion uses 10 samples per estimate in this harness.

Do not rerun a failed or invalid full session under new result filenames.
Preserve the original raw result files and diagnostics. A retest requires
explicit authorization for a whole new session; record the prior result paths
and the retest authorization with the new evidence. Never replace a failed
round with a loaded rerun or select a subset of rounds.

## Comparing a candidate with current main

For package axes, the commands above are the source/build procedure. The
general steps below define the five-round comparison and threshold only.

1. Use the ticket's single worktree. Build current `origin/main` (A) with the
   identical benchmark-only harness and a target directory inside that
   worktree, then build candidate (B) in the same worktree with a different
   target directory. Record each production source SHA; the benchmark harness
   revision is identical for A and B.
2. In one session, alternate A and B for five rounds each on the same machine;
   record load average before and after every round. For Criterion axes, each
   round runs the named axis at every required size. For isolated probes, each
   round starts a fresh process at every required size.
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
