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
        let value_source = source.replace(
            "profile v = \"filament.state.core/v1\";",
            "profile v = \"filament.value.complete/v1\";\nprofile law = \"filament.value.integer-division.euclidean/v1\";",
        );
        let checked = crate::support::front_end::checked(
            SourceIdentity::new("state-core-tests", "original:value-counterpart"),
            "value-counterpart.native",
            value_source.as_bytes(),
            &packages,
            &DependencyInput::default(),
        )
        .expect("the value profile admits the same function body");
        use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
        let evaluated = checked
            .package()
            .call(
                &QualifiedName::unqualified("f").unwrap(),
                vec![filament_exact::Value::Integer(8_i64.into())],
                &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
                &mut filament_exact::Meter::new(filament_exact::ScalarLimits {
                    integer_bits: 1_000_000,
                    decimal_digits: 1_000_000,
                    scale_expansion: 1_000_000,
                    text_input_bytes: 1_000_000,
                    text_scalars: 1_000_000,
                    normalized_scalars: 1_000_000,
                    unit_edges: 1_000_000,
                    value_occurrences: 1_000_000,
                    work_units: 1_000_000,
                    result_units: 1_000_000,
                }),
            )
            .expect("the checked counterpart runs");
        let filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
            filament_exact::Value::Integer(value),
        )) = evaluated.outcome
        else {
            panic!("the value function must complete: {evaluated:?}");
        };
        let expected = if body.starts_with("if") { 1_i64 } else { 2_i64 };
        assert_eq!(value, expected.into(), "{body}");
    }
}

#[test]
#[trace("FR-495-AC-4")]
fn state_core_locates_live_and_dead_division_in_each_state_clause() {
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the object is declared in the unit");
    for (kind, target) in [
        ("invariant", "Cell at current"),
        ("pre", "Cell::bump"),
        ("post", "Cell::bump"),
    ] {
        let mut cases = Vec::new();
        for operator in ["div", "rem", "mod"] {
            let expression = format!("7 {operator} 3");
            cases.push((format!("{expression} = 0"), operator));
            cases.push((
                format!("(if true then 1 else ({expression})) = 0"),
                operator,
            ));
        }
        // The operator inside a chain and under a negated operand.
        cases.push(("1 + 7 div 3 * 2 = 0".to_owned(), "div"));
        cases.push(("-7 mod 3 = 0".to_owned(), "mod"));
        for (body, operator) in cases {
            let source = format!(
                    "language \"ix:native\" edition \"1-draft\";\n\
                     profile v = \"filament.state.core/v1\";\n\
                     object Cell {{ n: Int[0, 9]; population cells; operation bump(): Boolean modifies self.n; }}\n\
                     {kind} Rule using v on {target} {{ {body} }}\n"
                );
            let refusal = crate::support::front_end::checked(
                SourceIdentity::new("state-core-tests", "original:state-clause"),
                "state-clause.native",
                source.as_bytes(),
                &packages,
                &DependencyInput::default(),
            )
            .expect_err("state clauses do not admit integer division");
            let diagnostics = refusal.outcome_diagnostics();
            assert_eq!(diagnostics.len(), 1, "{kind}, {body}: {diagnostics:?}");
            assert_eq!(diagnostics[0].code(), "unsupported_construct");
            assert_eq!(diagnostics[0].cause(), Some("expression-form"));
            let region = refusal.region().expect("the operator has its source locus");
            let start = usize::try_from(region.start()).unwrap();
            let end = usize::try_from(region.end()).unwrap();
            assert_eq!(&source[start..end], operator, "{kind}, {body}");
            assert!(diagnostics[0].message().contains("filament.state.core/v1"));
        }
    }
}
