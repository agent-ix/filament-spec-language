// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-5 (TC-1943): a package input supplies a model unit in place of
//! a domain package document, and intake admits it as it admits the compiled
//! document.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use ix_trace_rs::trace;

/// FR-710-AC-1's model unit.
const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total;
}
population orders: Order;
";

/// A unit selecting `identity` as `M`, with one invariant over `M::Order`.
fn unit(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         model M = \"{identity}\";\n\
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n"
    )
}

/// The unit's selections admitted against an input holding the model unit.
fn admit(identity: &str) -> Result<PackageDeclarations, String> {
    let text = unit(identity);
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "model-unit-input"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = package_input([SHOP.as_bytes()]).expect("a package input admits a model unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
#[ignore = "QSL-1980: a package input supplies a model unit in place of its document; not implemented yet"]
fn a_supplied_model_unit_is_admitted_as_its_compiled_document() {
    let mut declarations = admit("example/shop").expect("the model unit is admitted");
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .expect("dispatch links");
    declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .expect("the invariant over M::Order checks");
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
#[ignore = "QSL-1980: a model unit supplied for another identity is refused wrong-model-selection; not implemented yet"]
fn a_model_unit_for_another_identity_is_refused_wrong_model_selection() {
    let refusal = admit("example/other").expect_err("the identities differ");
    assert!(refusal.contains("WrongModelSelection"), "{refusal}");
}
