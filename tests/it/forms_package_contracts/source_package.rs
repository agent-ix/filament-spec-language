// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declared function identity through real S1–S4 and I2 admission.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::SourceIdentity;
use fsl_package::{CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use fsl_semantics::library::LibraryName;
use ix_trace_rs::trace;

fn check(declarations: &str) -> CheckedGraph {
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "function-identity"),
        "functions.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source fits parse limits");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let forms = fsl_forms::build_unit(&parsed).expect("real S2 forms");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "function-identity".to_owned(),
        forms,
        Vec::new(),
        Vec::new(),
    )
    .expect("real package assembly")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    )
    .expect("well-typed functions")
}

#[trace("FR-065-AC-2", "TC-163")]
#[test]
fn function_identities_survive_linking_strict_read_and_unrelated_reordering() {
    let callee = "function callee using v(x: Integer): Integer pure { x }";
    let caller = "function caller using v(x: Integer): Integer pure { callee(x) }";
    let unrelated = "function unrelated using v(flag: Boolean): Boolean pure { not flag }";
    let sources = [
        format!("{callee}\n{caller}\n{unrelated}"),
        format!("{unrelated}\n{callee}\n{caller}"),
    ];
    let mut identities = Vec::new();
    for source in sources {
        let graph = check(&source);
        let callee_id = graph.function_identity("callee").expect("declared callee");
        let caller_id = graph.function_identity("caller").expect("declared caller");
        let unrelated_id = graph
            .function_identity("unrelated")
            .expect("declared control");
        assert_ne!(
            callee_id, caller_id,
            "distinct declarations cannot collapse"
        );
        assert_ne!(callee_id, unrelated_id);
        assert!(graph.function_identity("missing").is_none());
        let package = CheckedPackage::link(graph);
        assert_eq!(
            package.graph().function_identity("callee"),
            Some(callee_id.clone())
        );
        assert_eq!(
            package.graph().function_identity("caller"),
            Some(caller_id.clone())
        );
        let read = fsl_package::read_evaluation_package(
            &package,
            LibraryName::new("function-identity").expect("valid package name"),
            &Default::default(),
            V2ReadLimits::fixture(),
            EmitLimits::fixture(),
        )
        .expect("emitted package passes strict I2 admission");
        for identity in [&callee_id, &caller_id, &unrelated_id] {
            let structural = fsl_semantics::check::wire_id::structural_id(identity);
            let node = read
                .admitted()
                .resolved_graph()
                .nodes
                .iter()
                .find(|node| node.node_id == structural)
                .expect("strict read retains each actual checked function identity");
            assert_eq!(
                fsl_semantics::check::wire_id::node_key(&node.node_id)
                    .expect("declared identity resolves"),
                *identity
            );
        }
        identities.push((callee_id, caller_id, unrelated_id));
    }
    assert_eq!(
        identities[0], identities[1],
        "unrelated declaration order changes no function identity at any boundary"
    );
}
