// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark: wall time of `ProtocolSystem::reachable` under `Sc` at 1, 2, 4
//! and all available workers (`reach/workers/<w>/<b>`), over `b` parallel
//! attempts ([`fsl_bench::mc::protocol`]). Throughput is reached states.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fsl_bench::mc;
use std::num::NonZeroUsize;
use std::time::Duration;

/// Parallel branch counts.
const BRANCHES: [usize; 3] = [2, 4, 6];

/// The worker limits: 1, 2, 4 and every available core.
fn worker_limits() -> Vec<(String, NonZeroUsize)> {
    let all = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
    let fixed = |workers: usize| NonZeroUsize::new(workers).unwrap_or(NonZeroUsize::MIN);
    vec![
        ("1".to_owned(), fixed(1)),
        ("2".to_owned(), fixed(2)),
        ("4".to_owned(), fixed(4)),
        ("all".to_owned(), all),
    ]
}

fn workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("reach/workers");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    for branches in BRANCHES {
        let world = mc::protocol(branches).expect("the protocol builds");
        for (label, limit) in worker_limits() {
            let work = mc::run_reach(&world, limit).expect("the search completes");
            group.throughput(Throughput::Elements(work.states));
            group.bench_with_input(BenchmarkId::new(label, branches), &(), |b, ()| {
                b.iter(|| mc::run_reach(&world, limit))
            });
        }
    }
    group.finish();
}

criterion_group!(benches, workers);
criterion_main!(benches);
