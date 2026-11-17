// SPDX-License-Identifier: AGPL-3.0-or-later
//! Input generators shared by this crate's criterion benchmarks
//! (`benches/*.rs`) and its one-shot probe (`src/bin/fsl-bench-probe.rs`).
//!
//! Each module builds one axis's input at a caller-chosen size `N` through
//! the layer's public API only, so the benchmarks keep compiling when a
//! layer is rewritten behind that API:
//!
//! - [`parse`]: complete-V1 source text for the S1 parser (`fsl_cst::parse`)
//!   -- N nested parentheses, or N one-line functions.
//! - [`check`]: `PackageDeclarations` for the S3 checker -- an N-function
//!   call chain, or N independent functions.
//! - [`kernel`]: checked evaluator kernels for the `evaluator/kernel` axis.
//! - [`deep_input`]: complete-V1 units nested 1 to 100,000 levels deep,
//!   built from fuzzer bytes and compiled through S1 to S4, for the
//!   `fuzz/` deep-input target (FR-356-AC-7).
//! - [`model`]: a Semantic IR 2.0.0 document with N object types and a
//!   population, carried through model intake (`admit` ->
//!   `read_domain_package`), then normalization and population admission.
//! - [`recursion`]: N self-recursive functions, N recursive components.
//! - [`rss`]: the process's peak resident set size, for the probe.
//! - [`world`]: a parametrised one-type domain package, native unit and
//!   initial snapshot, checked and linked.
//! - [`mc`]: the model-check, state-graph, protocol-reach, key-encoding and
//!   statistical-sampling axes built on [`world`].
//! - [`counters`] (feature `counters`): the workload counters those axes
//!   assert.
//! - [`text_cluster`]: N mutually referencing, Text-reachable records and
//!   one structural equality, for FR-093's text-leaf walk.
//!
//! No generator panics on a refusal from the layer it feeds: a refusal is a
//! measured result, reported by the probe, not a harness failure.

#![forbid(unsafe_code)]

/// `count` as the `u64` criterion's throughput and the probe's counts take.
///
/// # Panics
///
/// Panics on a count above `u64::MAX`, which no benchmark input reaches.
pub fn widen(count: usize) -> u64 {
    u64::try_from(count).expect("a benchmark count fits in u64")
}

pub mod check;
#[cfg(feature = "counters")]
pub mod counters;
pub mod deep_input;
pub mod kernel;
pub mod mc;
pub mod model;
pub mod parse;
pub mod recursion;
pub mod rss;
pub mod text_cluster;
pub mod world;
