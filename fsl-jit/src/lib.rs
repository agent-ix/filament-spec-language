// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cranelift JIT backend adapted to the current execution seam. Its published
//! FR-560 coverage includes Boolean and integer operations; Rational, Decimal,
//! and Text values and operations; record, tuple, collection, and reference
//! values; and the listed field, query, collection, and fold operations. The
//! exact supported cases are published by [`coverage`]. Checked entries
//! outside those forms, value types, or operations are listed unsupported.
//! This is the backend's declared FR-560 coverage floor, not a claim that every
//! FR-560, FR-561, or FR-562 behavior is implemented.
//!
//! The backend translates the checked representation of a function
//! (`fsl_semantics::check::CheckedNode`) and nothing else: a [`Jit`] prepares
//! entries of a `CheckedPackage`. An entry with a construct outside
//! [`coverage`] is listed unsupported with FR-294's `Untranslatable` cause
//! and the interpreter runs it; the JIT never runs it and `execute` never
//! falls back (FR-296).
//!
//! Generated code reaches the kernel only through the shims of [`exec`].
//! The kernel shims are adapted from the historical QSL backend. Their parity
//! behavior remains subject to the current FR-563 gate. The crate denies
//! unsafe code outside the isolated executable-memory module.

mod coverage;
#[allow(unsafe_code)]
mod exec;
mod kernel;
mod prepared;
mod translate;

pub use coverage::coverage;
pub use prepared::{CallableEntry, Jit, PreparedJit};
