// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared value-validity package construction helpers.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::value::{admit_integer_division, DefinitionRole};

pub(crate) fn package_of(declarations: &str) -> CheckedPackage {
    package_with_source(declarations).0
}

pub(crate) fn package_with_source(declarations: &str) -> (CheckedPackage, Vec<u8>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}\n{text}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds package declarations");
    declarations.integer_division = Some(
        admit_integer_division(
            &[DefinitionRole::IntegerDivisionEuclidean.reference()],
            None,
        )
        .expect("Euclidean division is admitted"),
    );
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Kernel,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|error| panic!("the package checks: {error:?}\n{text}")),
    );
    (package, text.into_bytes())
}
