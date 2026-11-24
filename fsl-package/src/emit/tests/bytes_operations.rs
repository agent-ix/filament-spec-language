// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-78 / TC-1231: Bytes operations emit and pass strict v2 admission.

use super::converted_equality::emitted;
use super::*;
use crate::checked_v2::read_checked_package_v2;
use fsl_foundation::diagnostic::Staged;
use ix_trace_rs::trace;

#[trace("FR-093-AC-78", "TC-1231", "FSpec-FR-322-AC-103", "FSpec-TC-280")]
#[test]
fn bytes_operations_emit_and_pass_strict_checked_package_v2_admission() {
    let emission = emitted(
        "function len using v(a: Bytes, b: Bytes): Integer pure { length(b) }\n\
         function cat using v(a: Bytes, b: Bytes): Bytes pure { concat(a, b) }",
    );
    let package: Value =
        serde_json::from_slice(emission.package.bytes()).expect("the package is JSON");
    let graph = WireGraph::of_package_value(&package).expect("the emitted operand graph");
    let bodies: Vec<Value> = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .enumerate()
        .map(|(holder, row)| {
            graph
                .structural_value_in_row(holder, &row["body"])
                .expect("a structural node body")
        })
        .collect();
    for identity in ["filament.op.bytes.length", "filament.op.bytes.concat"] {
        assert_eq!(
            bodies
                .iter()
                .filter(|body| body["operation"]["identity"] == identity)
                .count(),
            1,
            "the package emits {identity}"
        );
    }

    read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &read_evidence(&emission),
    )
    .map(Staged::into_value)
    .unwrap_or_else(|refusal| panic!("strict v2 reader refuses the emitted package: {refusal:?}"));
}
