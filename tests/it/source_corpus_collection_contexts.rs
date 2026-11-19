// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source cases for pure call contexts, layer gates and runtime bounds.

use ix_trace_rs::trace;

#[test]
#[trace("FR-110", "FR-230-AC-64", "FR-391-AC-8", "FR-391-AC-10", "FR-650")]
fn collection_contexts_and_runtime_boundaries() {
    let directory = super::corpus_root().join("collections/contexts");
    let report = super::run_directory(&directory).unwrap_or_else(|error| panic!("{error}"));
    println!("{report}");
}
