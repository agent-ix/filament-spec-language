// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark: wall time of EN-4's Okamoto sampling of the `Service` model's
//! `P95` claim (`en4/sample/<eps>`), where `eps` is the indifference in
//! ten-thousandths: 400, 120 and 48 draw about 10^3, 10^4 and 10^5 samples.
//! Throughput is counted samples.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fsl_bench::mc;
use std::time::Duration;

/// Indifference in ten-thousandths.
const EPSILONS: [u32; 3] = [400, 120, 48];

fn sample(c: &mut Criterion) {
    let mut group = c.benchmark_group("en4/sample");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    for epsilon in EPSILONS {
        let world = mc::service(epsilon).expect("the model builds");
        let work = mc::run_sample(&world).expect("the claim completes");
        group.throughput(Throughput::Elements(work.samples));
        group.bench_with_input(BenchmarkId::from_parameter(epsilon), &(), |b, ()| {
            b.iter(|| mc::run_sample(&world))
        });
    }
    group.finish();
}

criterion_group!(benches, sample);
criterion_main!(benches);
