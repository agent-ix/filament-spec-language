// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-141 package emit and package read workloads over timed-model fixtures.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use fsl_bench::{package, widen};
use fsl_package::{
    emit_package, read_evaluation_emission, EmitLimits, SourceWitnessSupplier, V2ReadLimits,
};
use fsl_semantics::library::LibraryName;
use std::hint::black_box;
use std::time::Duration;

fn package_axes(criterion: &mut Criterion) {
    for count in package::MODEL_COUNTS {
        let fixture = package::fixture(count).expect("timed package fixture compiles");
        let mut emit_group = criterion.benchmark_group("package/emit");
        emit_group.sample_size(10);
        emit_group.measurement_time(Duration::from_secs(4));
        emit_group.throughput(Throughput::Elements(widen(count)));
        emit_group.bench_function(BenchmarkId::from_parameter(count), |b| {
            b.iter(|| {
                black_box(emit_package(
                    fixture.checked.package(),
                    |_| Some(fixture.region.clone()),
                    EmitLimits::new(1 << 32),
                ))
            });
        });
        emit_group.finish();
        let identity = LibraryName::new("package").expect("benchmark library identity");
        let read_limits = V2ReadLimits::bounded(1 << 24);
        let mut read_group = criterion.benchmark_group("package/read");
        read_group.sample_size(10);
        read_group.measurement_time(Duration::from_secs(4));
        read_group.throughput(Throughput::Elements(widen(count)));
        read_group.bench_function(BenchmarkId::from_parameter(count), |b| {
            b.iter(|| {
                black_box(read_evaluation_emission(
                    fixture.checked.package(),
                    &fixture.emission,
                    identity.clone(),
                    &fixture.packages,
                    read_limits,
                    EmitLimits::new(1 << 32),
                    SourceWitnessSupplier::new(&fixture.emission),
                ))
            });
        });
        read_group.finish();
    }
    let mut replay_group = criterion.benchmark_group("replay/request");
    replay_group.sample_size(10);
    replay_group.measurement_time(Duration::from_secs(4));
    for count in package::MODEL_COUNTS {
        let fixture = package::fixture(count).expect("timed replay fixture compiles");
        let source = package::source(count);
        let request = package::replay_request(&fixture, &source).expect("replay request builds");
        let limits = fsl_replay::ReplayLimits::default().with_input_bytes(1 << 30);
        replay_group.throughput(Throughput::Elements(widen(count)));
        let result = fsl_replay::replay(request.clone(), limits);
        assert!(result.is_ok(), "replay probe request succeeds: {result:?}");
        replay_group.bench_function(BenchmarkId::from_parameter(count), |b| {
            b.iter_batched(
                || request.clone(),
                |request| black_box(fsl_replay::replay(request, limits)),
                BatchSize::SmallInput,
            );
        });
    }
    replay_group.finish();
}

criterion_group!(benches, package_axes);
criterion_main!(benches);
