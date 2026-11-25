---
id: NFR-141
org: agent-ix
title: "Measure performance-program paths with repeatable benchmark axes"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-011
    type: depends_on
---
# NFR-141: Measure performance-program paths with repeatable benchmark axes

## Statement

When the FSL benchmark gate evaluates a change for QSL-1919, QSL-1921,
QSL-1922, QSL-1923, QSL-1924, QSL-1925, QSL-1926, QSL-1927, QSL-1930, or
the region-mapping cost group in QSL-2854,
the FSL benchmark gate SHALL run the applicable named axis, check its
acceptance criterion below, and compare candidate B with current
`origin/main` A using live measurements from the same session as specified in
`fsl-bench/BENCHMARKING.md`. Stored measurements are not thresholds.

## Scope

This requirement defines the benchmark inputs and acceptance criteria for the
the named QSL performance-program tickets and the region-mapping cost group
in QSL-2854. A baseline-relative comparison is
used where a ticket gives no fixed numeric target; no historical measurement
is a threshold.

## Rationale

Naming the axes, generators, literal sizes, correctness conditions and live
comparison boundaries makes each result reproducible and catches regressions
without turning a past measurement into a fixed threshold.

## Measurement and Evaluation

`noise` for an estimate is the largest of A's and B's MAD divided by its
median and, for Criterion runs, the widest 95% confidence-interval half-width
divided by its estimate. Isolated probe runs use the two sides' MAD/median
because they have no Criterion interval. This is the session-noise calculation
in step 4 of `fsl-bench/BENCHMARKING.md`. For a performance metric with no
explicit ticket-specific threshold, a candidate is non-regressing only when
B's median is no greater than A's median multiplied by `1 + noise`; it is
improving only when B's median is below A's median by more than `noise`. The
session-noise allowance in QSL-1924 uses this same formula, applied to its
measured exponent estimates. Stored figures in tickets and prior sessions are
not thresholds.

Each AC below is evaluated independently. A live A/B result records all five
alternating rounds, load average before and after each round, per-round
estimates, both medians, the computed noise, and the pass/fail comparison. A
measurement session is immutable: every raw result file from the session,
including failed, loaded, or invalid results, SHALL be preserved and reported.
A rerun requires explicit lead authorization recorded in its report. No axis
result may be selectively replaced within a session.

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| NFR-141-AC-1, package emission | Milliseconds per emitted package at 1, 10 and 100 timed models | B median <= A median × (1 + noise) at each size | Five-round live A/B |
| NFR-141-AC-2, package read | Milliseconds per package read at 1, 10 and 100 timed models | B median <= A median × (1 + noise) at each size | Five-round live A/B |
| NFR-141-AC-3, replay wall time | Milliseconds per replay request at 1, 10 and 100 timed models | B median <= A median × (1 + noise) at each size | Five-round live A/B |
| NFR-141-AC-4, replay peak RSS | Peak RSS at 1, 10 and 100 timed models | B <= A × (1 + noise) at each size | Isolated live A/B probes |
| NFR-141-AC-5, parse calls | Calls per source document and shared counter availability | B calls per document <= A at each size | Live A/B counter report |
| NFR-141-AC-6, child traversal correctness | Zero child-traversal allocations; identical outcomes and charges across all callers | Exactly zero allocations and identical results | Allocation probe and correctness checks |
| NFR-141-AC-7, evaluator cost | Wall time, child-traversal allocation count and allocation share at specified sizes | B median <= A median × (1 + noise); count and share remain zero | Five-round live A/B and profile |
| NFR-141-AC-8, parser bookkeeping correctness | IDs recorded; descriptions formatted only on diagnostic; corpus output unchanged | Exact tree, diagnostic and expected-set-order equality | Parser-corpus comparison |
| NFR-141-AC-9, parser cost | Wall time for parser/volume and cst/parse specified inputs | B median < A median × (1 − noise) | Five-round live A/B |
| NFR-141-AC-10, grammar construction correctness | Default grammar reused; caller-supplied forms still build; outputs unchanged | Exact tree and diagnostic equality | Construction counter and parser comparison |
| NFR-141-AC-11, grammar construction cost | Wall time for the 150-byte parser/depth/0 input | B median < A median × (1 − noise) | Five-round live A/B |
| NFR-141-AC-12, enum lowering | Identical output; exponent from 100 to 1,000 members | Exponent <= 1 + noise | Output check and live A/B scaling |
| NFR-141-AC-13, checker axes | Axes and sizes listed in AC-13; report wall time at each completed size, exponent when at least two sizes complete, and the 5,000-type refusal boundary | Identical outputs and per-size candidate median <= baseline median × (1 + noise), as AC-14 states | Live A/B and output/refusal comparison |
| NFR-141-AC-14, checker output/cost | Same checked output and node keys; cause recorded and fixed if in QSL | B median <= A median × (1 + noise) at each completed size | Live A/B scaling and cause analysis |
| NFR-141-AC-15, memory/API | Public tree API unchanged; peak RSS for both ticket workloads | B peak RSS <= A peak RSS × (1 + noise) | Heap profile and isolated live A/B probes |
| NFR-141-AC-16, assembly/normalization | Before/after exponent or the full linear-growth table | Identical outputs if fixed; follow the ticket's super-linear/linear branch | Live A/B scaling and output comparison |
| NFR-141-AC-17, workspace structure | One `it` binary each and duplicate major versions removed | Exactly one each; zero duplicate `syn`/`jsonschema` majors | Build graph and binary inventory |
| NFR-141-AC-18, build time | Cold/incremental build and test no-run wall time | Each B median <= A median × (1 + noise), and one < A median × (1 − noise) | Five-round live A/B build measurements |
| NFR-141-AC-19, region lookup work | Source-map segment-group index build visits and lookup probes for nested prefix spans at 5,000, 10,000 and 20,000 segments | At most N index visits and N × (2 × ceil(log2 N) + 4) query probes at each size; exact mapped-region results equal the existing multi-region mapping filtered to exactly one region | Deterministic operation-count test and exact output comparison |
| NFR-141-AC-20, baseline provenance | FSL production source revision used for A | `origin/main` at session start | Live A/B provenance report |
| NFR-141-AC-21, candidate provenance | FSL production source revision used for B | Candidate production sources | Live A/B provenance report |
| NFR-141-AC-22, timing instrumentation | Replay-counter seam on AC-1 through AC-3 wall-time axes | No seam on either side | Live A/B harness and provenance inspection |
| NFR-141-AC-23, parse-counter instrumentation | Identical benchmark-only seam on AC-5 isolated probe | Same unchanged harness files on A and B; seam revision reported separately from production source revisions | Isolated probe and provenance report |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-141-AC-1 | For QSL-1919, `package/emit` is selectable by name and listed in `fsl-bench/BENCHMARKING.md`; it emits checked fixtures with M timed models at M = 1, 10 and 100 (one object type, one clock, one operation, one timed transition and one claim per model), and reports wall time in milliseconds per emitted package and the exponent across M. Candidate B's median wall time is no greater than A's median multiplied by `1 + noise` at each M. | Live five-round A/B benchmark and generated-package comparison |
| NFR-141-AC-2 | For QSL-1919, `package/read` is separately selectable and listed in the guide; it reads the same checked package fixtures at M = 1, 10 and 100, reports wall time in milliseconds per package and the exponent across M, and B's median at each M is no greater than A's median multiplied by `1 + noise`. | Live five-round A/B benchmark and checked-package comparison |
| NFR-141-AC-3 | For QSL-1919, `replay/request` is separately selectable and listed in the guide; it executes one end-to-end replay request over a source package containing 1, 10 or 100 timed models, reports wall time in milliseconds per request, and B's median at each size is no greater than A's median multiplied by `1 + noise`. | Live five-round A/B benchmark and replay outcome comparison |
| NFR-141-AC-4 | For QSL-1919, the same `replay/request` input sizes (1, 10 and 100 timed models) each have an isolated peak-RSS probe; B's peak RSS is no greater than A's peak RSS multiplied by `1 + noise` at each size. | Isolated live A/B peak-RSS probe |
| NFR-141-AC-5 | For QSL-1919, `replay/request` reports parse calls per source document using the parse-call counter feature shared with the model-check counters; the counter is selectable in the benchmark output and B's calls per document do not exceed A's for any of the three package sizes. | Live A/B counter report and counter-unit check |
| NFR-141-AC-6 | For QSL-1921, visiting `CheckedNode::children` by the evaluator and every other caller allocates zero bytes within the child-traversal operation; evaluator results and charges remain identical. The benchmark/profile covers `evaluator/kernel` using `KernelCase` sizes 5 and 10 and `evaluator/call_chain` using `linked_chain` sizes 1, 10, 100 and 1,000 with one call to `f0(5)`. | Allocation-count probe, evaluator correctness checks, and live A/B measurements |
| NFR-141-AC-7 | For QSL-1921, `evaluator/kernel` and `evaluator/call_chain` report wall time, child-traversal allocation count and allocation share at every AC-6 size; B's median wall time is no greater than A's median multiplied by `1 + noise`, and both the child-traversal allocation count and share remain zero. | Live five-round A/B benchmark and allocation profile |
| NFR-141-AC-8 | For QSL-1922, parser failure bookkeeping records expected-terminal identifiers; terminal descriptions are formatted only when a diagnostic is produced. Trees, diagnostic text and expected-set order remain identical on the parser corpus. `parser/volume` uses 1,000 one-line functions; `cst/parse` uses `nested_source` depths 0, 1, 2, 3 and 4 and `volume_source(1,000)`. | Parser-corpus comparison and live A/B benchmark |
| NFR-141-AC-9 | For QSL-1922, `parser/volume` and `cst/parse` report wall time for every AC-8 input; B's median for each input is below A's by more than session noise. | Live five-round A/B benchmark |
| NFR-141-AC-10 | For QSL-1923, the default grammar tables are constructed once per process and shared; grammars with caller-supplied forms still build correctly as today. Trees and diagnostics remain identical. `parser/depth/0` uses the ticket's 150-byte source input. | Grammar-construction counter, parser-corpus comparison and live A/B benchmark |
| NFR-141-AC-11 | For QSL-1923, `parser/depth/0` reports wall time for the 150-byte input; B's median is below A's by more than session noise. | Live five-round A/B benchmark |
| NFR-141-AC-12 | For QSL-1924, `checker/enum_members` checks the same 1,000 functions over enum sizes 10, 100 and 1,000, with equality lookup for two members per function. Checked output is identical, and the measured exponent from 100 to 1,000 members is at most `1 + noise` as defined above. | Checked-output comparison and live A/B scaling benchmark |
| NFR-141-AC-13 | For QSL-1925, the generator inputs are literal: `checker/chain` uses 250, 1,000 and 2,000 functions; `checker/independent` uses 1,000 and 5,000 functions; `checker/text_cluster` uses 3 through 8 records plus the 9-record refusal boundary; and `checker/object_chain` uses 500 and 1,000 admitted types for the exponent, plus the 5,000-type default-budget refusal boundary. Each axis reports wall time and its exponent where at least two sizes complete. | Live A/B benchmark and output/refusal comparison |
| NFR-141-AC-14 | For QSL-1925, checked outputs and node keys are identical on `checker/chain`, `checker/independent` and admitted `checker/text_cluster` inputs. The `checker/object_chain` exponent cause is recorded; if the cause is in QSL, it is fixed in the ticket change. Each axis's candidate median is no greater than the baseline median multiplied by `1 + noise` at every completed size. | Output/key comparison, cause analysis, and live A/B scaling benchmark |
| NFR-141-AC-15 | For QSL-1926, the heap profiles identify the largest per-node contributors for parsing 200,000 one-line functions (12.9 MB source) and checking an 8,000-function `call_chain`; the fixes target the largest contributors and leave the public tree API unchanged. Peak RSS is reported before and after for both inputs, and B is no greater than A multiplied by `1 + noise`. | Heap profile, public API comparison, and isolated live A/B peak-RSS probes |
| NFR-141-AC-16 | For QSL-1927, `bench-model` includes widening redefinitions and package-typed fields at 1,000, 4,000 and 16,000 types, reports the before exponent, and either (a) for super-linear growth fixes the cause, preserves outputs and reports the after exponent, or (b) for linear growth closes with the complete three-size table. The intake alias-chain and map-key scans in the ticket remain context only, not benchmark axes. | Live A/B scaling benchmark and output comparison |
| NFR-141-AC-17 | For QSL-1930, qsl-replay and qsl-package each build exactly one `it` test binary; duplicate `syn` and `jsonschema` major versions are removed from the resolved workspace dependency graph. | Cargo build graph and test-binary inventory |
| NFR-141-AC-18 | For QSL-1930, cold and incremental `cargo build --timings` and `cargo test --no-run` wall time are measured before and after; each candidate median is no greater than its baseline median plus session noise, and at least one is lower by more than session noise. | Live five-round A/B build measurements |
| NFR-141-AC-19 | For the region-mapping cost group in QSL-2854, construct a `SourceMap` with N = 5,000, 10,000 and 20,000 ordered one-line body segments separated by an allowed two-byte indentation deletion in the original source, then query each of the N nonempty body prefixes `[0, 2k)` for `1 <= k <= N`. Building the contiguous original-region group index visits at most N input segments. Across the N queries at each size, binary-search and examined-group probes total at most `N × (2 × ceil(log2 N) + 4)`. In this fixture, the exact result is the original first-line region for k = 1 and `None` for k > 1; a boundary differential test compares the specialized result with `map_regions` for every span of a 12-segment fixture. The old scan-per-query implementation fails this bound: at N = 5,000, the deterministic probe recorded 12,577,499 segment steps against a bound of 150,000. | Focused deterministic operation-count unit test at all three sizes and exact boundary comparison against `map_regions` |
| NFR-141-AC-20 | For each QSL-1919 live A/B session, baseline A uses the FSL production sources at `origin/main` when the session begins; the report records A's production source revision. | Live A/B provenance report |
| NFR-141-AC-21 | For each QSL-1919 live A/B session, candidate B uses the FSL candidate production sources; the report records B's production source revision. | Live A/B provenance report |
| NFR-141-AC-22 | For QSL-1919, the `package/emit`, `package/read` and `replay/request` wall-time axes in AC-1 through AC-3 use no replay-counter seam on either side. | Live A/B harness and provenance inspection |
| NFR-141-AC-23 | For QSL-1919 AC-5, the isolated parse-counter probe applies identical benchmark-only replay-counter harness files unchanged to baseline A and candidate B; the report records the seam revision separately from each production source revision. | Isolated probe and provenance report |

## Verification

The named generators and sizes above are part of the benchmark contract. The
benchmark gate records each AC's correctness result and the live A/B evidence
required by `fsl-bench/BENCHMARKING.md`; a missing axis, size, metric, restored
scope condition, or failed threshold fails that AC.
