// SPDX-License-Identifier: AGPL-3.0-or-later
//! Partial Cranelift JIT backend seed adapted to the current execution seam.
//! Its conservative coverage is the Boolean and integer subset published by
//! [`coverage`]; this crate does not claim the full FR-560 floor or complete
//! FR-561/FR-562 behavior.
//!
//! The backend translates the checked representation of a function
//! (`fsl_semantics::check::CheckedNode`) and nothing else: a [`Jit`] prepares
//! entries of a `CheckedPackage`. An entry with a construct outside
//! [`coverage`] is listed unsupported with FR-294's `Untranslatable` cause
//! and the interpreter runs it; the JIT never runs it and `execute` never
//! falls back (FR-296).
//!
//! Generated code reaches the kernel only through the shims of [`exec`]
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
