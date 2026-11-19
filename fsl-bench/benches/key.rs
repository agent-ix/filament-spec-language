// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark: wall time of one canonical state-key encoding
//! (`key/encode/<n>x<f>`): a state of `n` objects of `f` integer fields
//! ([`fsl_bench::mc::keys`]). Throughput is key bytes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fsl_bench::mc;
use fsl_eval::simulation::model::ModelSystem;

/// State shapes `(objects, fields)`.
const SHAPES: [(usize, usize); 3] = [(1, 1), (50, 4), (500, 8)];

fn encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("key/encode");
    for (n, f) in SHAPES {
        let world = mc::keys(n, f).expect("the state builds");
        let system = ModelSystem::new(world.simulation()).expect("the subject admits");
        let work = mc::run_encode(&system).expect("the key encodes");
        group.throughput(Throughput::Bytes(work.key_bytes));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}x{f}")),
            &(),
            |b, ()| b.iter(|| mc::run_encode(&system)),
        );
    }
    group.finish();
}

criterion_group!(benches, encode);
criterion_main!(benches);
