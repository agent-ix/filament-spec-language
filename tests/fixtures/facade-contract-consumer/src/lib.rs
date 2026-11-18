// SPDX-License-Identifier: AGPL-3.0-or-later
//! The consumer cannot reach package internals through a public module.
//!
//! ```compile_fail,E0603
//! fn private_relation_helper_is_not_a_consumer_api() {
//!     let _ = fsl_package::protocol_view::relation_over;
//! }
//! ```
