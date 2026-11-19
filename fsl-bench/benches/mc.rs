// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark: wall time of the model checker's axes, each a criterion group
//! named for its axis (`cargo bench --bench mc -- mc/state_graph` runs one).
//!
//! - `mc/state_graph/<plain|symmetric>/<n>x<k>`: FR-167 `check_state_graph`
//!   over `n` ring objects of `k` values ([`fsl_bench::mc::ring_state_graph`]).
//! - `mc/safety/<invariant|deadlock>/<n>x<k>`: `check_model`
//!   ([`fsl_bench::mc::run_safety`]).
//! - `mc/liveness/<n>x<k>`: weak-fair `always eventually`.
//! - `mc/frame_candidates/<m>x<r>`: one operation writing `m` fields of
//!   `Int[0, r]`.
//! - `mc/observation/<R>`: a fixed two-state model beside `R` unrelated
//!   package records.
//!
//! Throughput is the engine's own state count. Worlds are built outside the
//! timing.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fsl_bench::mc::{self, Work};
use std::time::Duration;

/// Ring sizes `(objects, values)`.
const RINGS: [(usize, usize); 3] = [(2, 2), (3, 3), (4, 4)];
/// Wide sizes `(fields, range)`.
const WIDES: [(usize, usize); 3] = [(2, 1), (3, 3), (4, 5)];
/// Unrelated record counts.
const UNRELATED: [usize; 3] = [1, 50, 400];

fn group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group
}

fn states(work: &Work) -> Throughput {
    Throughput::Elements(work.states)
}

fn state_graph(c: &mut Criterion) {
    let mut group = group(c, "mc/state_graph");
    for symmetric in [false, true] {
        let label = if symmetric { "symmetric" } else { "plain" };
        for (n, k) in RINGS {
            let world = mc::ring_state_graph(n, k, symmetric).expect("the ring builds");
            let work = mc::run_state_graph(&world, symmetric).expect("the graph completes");
            group.throughput(states(&work));
            group.bench_with_input(BenchmarkId::new(label, format!("{n}x{k}")), &(), |b, ()| {
                b.iter(|| mc::run_state_graph(&world, symmetric))
            });
        }
    }
    group.finish();
}

fn safety(c: &mut Criterion) {
    let mut group = group(c, "mc/safety");
    for deadlock in [false, true] {
        let label = if deadlock { "deadlock" } else { "invariant" };
        for (n, k) in RINGS {
            let world = mc::ring_safety(n, k).expect("the ring builds");
            let (work, _) = mc::run_safety(&world, deadlock).expect("the item holds");
            group.throughput(states(&work));
            group.bench_with_input(BenchmarkId::new(label, format!("{n}x{k}")), &(), |b, ()| {
                b.iter(|| mc::run_safety(&world, deadlock))
            });
        }
    }
    group.finish();
}

fn liveness(c: &mut Criterion) {
    let mut group = group(c, "mc/liveness");
    for (n, k) in RINGS {
        let world = mc::ring_liveness(n, k).expect("the ring builds");
        let (work, _) = mc::run_liveness(&world).expect("the clause holds");
        group.throughput(states(&work));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{k}")),
            &(),
            |b, ()| b.iter(|| mc::run_liveness(&world)),
        );
    }
    group.finish();
}

fn frame_candidates(c: &mut Criterion) {
    let mut group = group(c, "mc/frame_candidates");
    for (m, r) in WIDES {
        let world = mc::wide(m, r).expect("the model builds");
        let work = mc::run_frame_candidates(&world).expect("the item holds");
        group.throughput(states(&work));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{m}x{r}")),
            &(),
            |b, ()| b.iter(|| mc::run_frame_candidates(&world)),
        );
    }
    group.finish();
}

fn observation(c: &mut Criterion) {
    let mut group = group(c, "mc/observation");
    for records in UNRELATED {
        let world = mc::observation(records).expect("the model builds");
        let work = mc::run_observation(&world).expect("the item holds");
        group.throughput(states(&work));
        group.bench_with_input(BenchmarkId::from_parameter(records), &(), |b, ()| {
            b.iter(|| mc::run_observation(&world))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    state_graph,
    safety,
    liveness,
    frame_candidates,
    observation
);
criterion_main!(benches);
