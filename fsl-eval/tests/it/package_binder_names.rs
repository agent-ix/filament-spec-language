// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-400-AC-13 and FR-931-AC-13 (TC-3039), package route: the reader-backed
//! read of a function from its written bytes gives every `case` arm, `let`
//! and query binding site the declared names the source declared, the `name`
//! of each binder's parameter node.

use filament_checked::node::NodeKind;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;

/// The names the reader gives `f`'s binding sites, in arena order.
fn reader_names(body: &str) -> (Vec<String>, Vec<String>, Vec<Vec<Vec<String>>>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         union Shape {{ Circle(Integer), Rect(Integer, Integer), Empty }}\n\
         function f using v(s: Shape): Boolean pure {{ {body} }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the unit");
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
        .expect("the checked package emits");
    let no_domains = package_input(std::iter::empty::<&[u8]>()).expect("empty model closure");
    let read = fsl_package::read_evaluation_emission(
        &package,
        &emission,
        LibraryName::new("fixture").expect("a library name"),
        &no_domains,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emission),
    )
    .expect("the reader admits the package");
    let v2 = read.admitted();
    let budget = || fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(v2, &mut budget())
        .expect("the identity rows materialize");
    let equality = emission
        .equality_witness(v2, &mut budget())
        .expect("the equality witness materializes");
    let bare = fsl_package::BarePackage::new(v2)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget(),
        )
        .expect("the fixture budget admits the supply");
    let projection = bare
        .project_function("f")
        .expect("the function projects")
        .expect("the package holds f");
    let (mut lets, mut queries, mut arms) = (Vec::new(), Vec::new(), Vec::new());
    let mut index = 0;
    while let Some(kind) = projection.kind(index) {
        match kind {
            NodeKind::Let { name, .. } => lets.push(name.clone()),
            NodeKind::Query { name, .. } => queries.push(name.clone()),
            NodeKind::Case { arms: found, .. } => {
                arms.push(found.iter().map(|arm| arm.names.clone()).collect());
            }
            _ => {}
        }
        index += 1;
    }
    (lets, queries, arms)
}

/// Every arm binder, `let` binder and nested arm binder keeps its declared
/// name on the package route, `_` included, and a renamed binder is renamed.
#[trace("TC-3039", "FR-400-AC-13")]
#[trace("TC-3039", "FR-931-AC-13")]
#[test]
fn the_reader_gives_case_arm_and_let_binders_their_declared_names() {
    let (lets, queries, arms) =
        reader_names("case s { Empty: true; Rect(_, h): let k = h in k <= 4; Circle(r): r <= 2; }");
    assert_eq!(lets, ["k"]);
    assert!(queries.is_empty());
    assert_eq!(arms.len(), 1);
    assert_eq!(
        arms[0],
        [
            vec!["r".to_owned()],
            vec!["_".to_owned(), "h".to_owned()],
            vec![]
        ]
    );
    let (lets, _, arms) = reader_names(
        "case s { Empty: true; Rect(width, h): let kk = h in kk <= 4; Circle(r): r <= 2; }",
    );
    assert_eq!(lets, ["kk"]);
    assert_eq!(arms[0][1], ["width", "h"]);
}
