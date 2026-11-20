// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-only admission rejects arithmetic even when control flow cannot reach it.

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;

#[test]
#[trace("FR-495-AC-4")]
fn state_core_refuses_each_division_as_one_function_declaration() {
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("no selected model is needed");
    for body in [
        "a div 3",
        "a rem 3",
        "a mod 3",
        "if true then 1 else (7 div 3)",
    ] {
        let declaration = format!("function f using v(a: Int[0, 9]): Integer pure {{ {body} }}");
        let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.state.core/v1\";\n{declaration}\n");
        let refusal = crate::support::front_end::checked(
            SourceIdentity::new("state-core-tests", "original:state-only"),
            "state-only.native",
            source.as_bytes(),
            &packages,
            &DependencyInput::default(),
        )
        .expect_err("the state profile does not admit value functions");
        let diagnostics = refusal.outcome_diagnostics();
        assert_eq!(diagnostics.len(), 1, "{body}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code(), "unsupported_construct");
        assert_eq!(diagnostics[0].cause(), Some("declaration-form"));
        let region = refusal
            .region()
            .expect("the refused declaration has its source locus");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(&source[start..end], declaration, "{body}");
        assert!(diagnostics[0].message().contains("filament.state.core/v1"));
    }
}
