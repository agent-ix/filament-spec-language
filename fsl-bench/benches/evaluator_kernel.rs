// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interpreter kernels paired with plain-Rust reference implementations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fsl_bench::kernel::{KernelCase, KernelKind, SIZES};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use std::hint::black_box;

fn evaluator_kernel(c: &mut Criterion) {
    let objects = ObjectEnvironment::default();
    let mut group = c.benchmark_group("evaluator/kernel");
    for kind in KernelKind::ALL {
        for size in SIZES {
            let case = KernelCase::new(kind, size);
            group.throughput(Throughput::Elements(case.operations));
            group.bench_function(
                BenchmarkId::new(format!("interpreter/{}", kind.name()), size),
                |b| b.iter(|| black_box(case.call(&objects))),
            );
            group.bench_function(
                BenchmarkId::new(format!("rust/{}", kind.name()), size),
                |b| b.iter(|| black_box(kind.rust_reference(size))),
            );
        }
    }
    group.finish();
}

criterion_group!(benches, evaluator_kernel);
criterion_main!(benches);
