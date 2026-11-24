// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared counted-step protocol source helper.

/// A `Claim` over `k: M::Cell` under infinite-trace.
pub(crate) fn tp3(activation: &str, formula: &str) -> String {
    format!(
        "temporal Claim using t0 over (k: M::Cell) clock \"model-steps\" {activation} \
         {{ {formula} }}\n"
    )
}
