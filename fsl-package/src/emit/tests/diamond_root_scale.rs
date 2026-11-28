// SPDX-License-Identifier: AGPL-3.0-or-later
//! The E4 diamond rule over two selections of one library identity that
//! differ only in a dimension equivalence's `root_scale`, and so share the
//! equivalence's node id (FSpec FR-307-AC-2).

use super::*;

/// Library `pkg`, whose one source difference between calls is the ratio of
/// its dimension equivalence.
fn library(ratio: u8) -> CheckedPackage {
    package_from_text(&format!(
        "dimension Time;\ndimension Period;\nunit s : Time = rational(1, 1);\n\
         unit sec : Period = rational(1, 1);\ndimension Period ~ Time by 1 sec = {ratio} s;\n"
    ))
    .1
}

/// The root linking `test/left` and `test/right`, each of which imports its
/// own `pkg`.
fn diamond(left: u8, right: u8) -> Result<CheckedPackage, crate::LinkRefusal> {
    let branch = |name: &str, ratio: u8| linked(name, vec![import("pkg", library(ratio))]);
    CheckedPackage::link_with(
        graph_of_named("test/root", vec![t_read_from_text()]),
        vec![
            import("test/left", branch("test/left", left)),
            import("test/right", branch("test/right", right)),
        ],
        crate::EmitLimits::fixture(),
    )
}

/// Two `pkg` contents differing only in `root_scale` refuse at link as
/// `invalid_package`/`conflicting-definition` with both paths, while two
/// separately built equal contents unify.
#[trace("FSpec-FR-307-AC-2", "FSpec-TC-227")]
#[test]
fn e4_refuses_a_diamond_whose_selections_differ_only_in_root_scale() {
    let one = wire(&emit(&library(1)));
    let mut two = wire(&emit(&library(2)));
    let position = nodes(&one)
        .iter()
        .position(|node| node["semantic_form"] == "dimension_equivalence")
        .expect("the equivalence node");
    let scale = format!("/semantic_graph/nodes/{position}/body/root_scale/numerator");
    assert_eq!(one.pointer(&scale), Some(&json!("1")));
    assert_eq!(two.pointer(&scale), Some(&json!("2")));
    *two.pointer_mut(&scale).expect("the ratio") = json!("1");
    assert_eq!(one, two, "the ratio is the only difference");

    let agreeing = diamond(1, 1).expect("equal contents unify");
    assert_eq!(
        agreeing.dependency_selections()[&lib("pkg")].path(&lib("pkg")),
        path(&["test/left", "pkg"])
    );

    for (left, right) in [(1, 2), (2, 1)] {
        let refusal = diamond(left, right).expect_err("the two ratios do not unify");
        assert_eq!(refusal.code(), Code::InvalidPackage);
        assert_eq!(refusal.cause(), Some("conflicting-definition"));
        let crate::LinkRefusal::ConflictingDefinition {
            identity,
            selections,
        } = &refusal
        else {
            panic!("expected ConflictingDefinition, got {refusal:?}");
        };
        assert_eq!(*identity, lib("pkg"));
        assert_eq!(selections[0].path(identity), path(&["test/left", "pkg"]));
        assert_eq!(selections[1].path(identity), path(&["test/right", "pkg"]));
    }
}
