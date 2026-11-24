// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source adaptation of TC-187's authored unit diagnostic vectors.

use filament_exact::{Cancel, Integer, Rational};
use filament_specification::{QualificationVector, TestCase, QUALIFICATION_VECTORS};
use fsl_semantics::check::{
    AssemblyCause, AssemblyLimits, AssemblyRefusal, PackageDeclarations, TopologyFault,
    TopologyRetained,
};
use ix_trace_rs::trace;

fn vector(id: &str) -> &'static QualificationVector {
    QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.id == id)
        .expect("published qualification vector")
}

fn quoted(text: &str) -> Vec<&str> {
    text.split('`').skip(1).step_by(2).collect()
}

fn assert_expected(vector: &QualificationVector, expected: &str) {
    assert!(
        vector.expected.contains(expected),
        "{} expected cell {} did not contain {expected:?}",
        vector.id,
        vector.expected
    );
}

fn adapt_unit(snippet: &str) -> String {
    let (declaration, expression) = snippet
        .trim()
        .trim_end_matches(';')
        .split_once('=')
        .expect("unit declaration has a scale");
    let (amount, target) = expression
        .trim()
        .split_once(" * ")
        .map_or((expression.trim(), None), |(amount, target)| {
            (amount, Some(target))
        });
    let rational = match amount.trim() {
        "0" => "rational(0, 1)",
        "1" => "rational(1, 1)",
        "2" | "1 + 1" => "rational(2, 1)",
        "1/100" => "rational(1, 100)",
        other => panic!("unadapted TC-187 scale {other:?}"),
    };
    match target {
        Some(target) => format!("{declaration}= {rational} * {target};\n"),
        None => format!("{declaration}= {rational};\n"),
    }
}

fn unit_document(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}"
    )
}

fn assemble_refusal(documents: &[(&str, &str)]) -> (Vec<String>, AssemblyRefusal) {
    let mut texts = Vec::with_capacity(documents.len());
    let mut units = Vec::with_capacity(documents.len());
    for (identity, declarations) in documents {
        let text = unit_document(declarations);
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("qualification", identity),
            "unit-diagnostic.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source intake");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        units.push((
            parsed.source().reference().clone(),
            fsl_forms::build_unit(&parsed).expect("forms of admitted source"),
        ));
        texts.push(text);
    }
    let refusal = PackageDeclarations::assemble_units(
        "qualification-source".to_owned(),
        units,
        Vec::new(),
        Vec::new(),
        AssemblyLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the authored vector contains a refusing unit graph");
    (texts, refusal)
}

fn assert_diagnostic(
    text: &str,
    refusal: &AssemblyRefusal,
    code: &str,
    span: &str,
    cause: AssemblyCause,
) {
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    let error = &refusal.errors[0];
    assert_eq!(error.cause.catalog_code().to_string(), code);
    assert_eq!(error.cause, cause);
    assert_eq!(&text[error.span.start..error.span.end], span);
}

fn edge(from: &str, to: &str) -> TopologyRetained {
    TopologyRetained::Edge {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

fn cycle(edges: Vec<TopologyRetained>) -> AssemblyCause {
    AssemblyCause::UnitGraphTopology {
        fault: TopologyFault::TargetCycle,
        retained: edges,
    }
}

/// FSpec FR-142-AC-15 and FSL FR-091-AC-71: cycle vectors adapt their exact
/// scales while retaining the row's public diagnostic, ordered edges, span.
#[trace("FSpec-TC-187", "FSpec-FR-142-AC-15", "FR-091-AC-71")]
#[test]
fn u11b_f_l_adapt_cycle_vectors_to_the_authored_diagnostics() {
    for (id, prefix, names, start) in [
        ("U11b", "dimension L;\n", ["m", "cm"], "m"),
        ("U11l", "dimension L;\n", ["cm", "m"], "cm"),
    ] {
        let vector = vector(id);
        assert_eq!(vector.test_case, TestCase::Tc187);
        assert_expected(vector, "`invalid_package`/`definition-cycle`");
        let inputs = quoted(vector.inputs);
        assert_eq!(inputs.len(), 2, "{id}");
        let source = format!("{prefix}{}{}", adapt_unit(inputs[0]), adapt_unit(inputs[1]));
        let (texts, refusal) = assemble_refusal(&[("u11-cycle", &source)]);
        assert_diagnostic(
            &texts[0],
            &refusal,
            "invalid_package/definition-cycle",
            start,
            cycle(vec![edge(names[0], names[1]), edge(names[1], names[0])]),
        );
    }

    let vector = vector("U11f");
    assert_expected(vector, "`invalid_package`/`definition-cycle`");
    let inputs = quoted(vector.inputs);
    assert_eq!(inputs.len(), 1, "U11f");
    let source = format!("dimension L;\n{}", adapt_unit(inputs[0]));
    let (texts, refusal) = assemble_refusal(&[("u11-self", &source)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/definition-cycle",
        "u_self",
        cycle(vec![edge("u_self", "u_self")]),
    );
}

/// FSpec FR-142-AC-15: both U11e scales, including vector arithmetic, adapt
/// to the same exact scale and zero offset in the refusal payload.
#[trace("FSpec-TC-187", "FSpec-FR-142-AC-15", "FR-091-AC-71")]
#[test]
fn u11e_both_vector_scales_adapt_to_the_same_exact_source_diagnostic() {
    let vector = vector("U11e");
    assert_eq!(vector.test_case, TestCase::Tc187);
    let inputs = quoted(vector.inputs);
    assert_eq!(inputs.len(), 2, "U11e has two independent scale spellings");
    assert_expected(vector, "`invalid_package`/`invalid-value`");

    for input in inputs {
        let source = format!("dimension L;\n{}", adapt_unit(input));
        let (texts, refusal) = assemble_refusal(&[("u11-scale", &source)]);
        assert_diagnostic(
            &texts[0],
            &refusal,
            "invalid_package/invalid-value",
            "m",
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::NonIdentityRoot,
                retained: vec![
                    TopologyRetained::Unit {
                        name: "m".to_owned(),
                        key: None,
                    },
                    TopologyRetained::Scale(Rational::from_integer(Integer::from(2_i64))),
                    TopologyRetained::Offset(Rational::from_integer(Integer::zero())),
                ],
            },
        );
    }
}

/// FSpec FR-142-AC-15: the duplicate root in U11j wins over its later cycle.
#[trace("FSpec-TC-187", "FSpec-FR-142-AC-15", "FR-091-AC-71")]
#[test]
fn u11j_adapts_both_roots_and_preserves_the_first_defect() {
    let vector = vector("U11j");
    let inputs = quoted(vector.inputs);
    assert_eq!(
        inputs.len(),
        4,
        "U11j includes its dimension and three units"
    );
    assert_expected(vector, "`invalid_package`/`conflicting-definition`");
    let source = format!(
        "dimension {};\n{}{}{}",
        inputs[0],
        adapt_unit(inputs[1]),
        adapt_unit(inputs[2]),
        adapt_unit(inputs[3])
    );
    let (texts, refusal) = assemble_refusal(&[("u11-roots", &source)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/conflicting-definition",
        "m",
        AssemblyCause::UnitGraphTopology {
            fault: TopologyFault::TwoRoots,
            retained: vec![
                TopologyRetained::Dimension {
                    name: "L".to_owned(),
                    key: None,
                },
                TopologyRetained::Unit {
                    name: "m_alt".to_owned(),
                    key: None,
                },
                TopologyRetained::Unit {
                    name: "m".to_owned(),
                    key: None,
                },
            ],
        },
    );
}

/// FSpec FR-142-AC-15: U11k's earlier dimension cycle preempts the later zero.
#[trace("FSpec-TC-187", "FSpec-FR-142-AC-15", "FR-091-AC-71")]
#[test]
fn u11k_adapts_the_earlier_cycle_before_the_later_zero_scale() {
    let vector = vector("U11k");
    let inputs = quoted(vector.inputs);
    assert_eq!(inputs.len(), 4, "U11k");
    assert_expected(vector, "`invalid_package`/`definition-cycle`");
    let source = format!(
        "dimension {};\ndimension {};\n{}{}",
        inputs[0],
        inputs[1],
        adapt_unit(inputs[2]),
        adapt_unit(inputs[3])
    );
    let (texts, refusal) = assemble_refusal(&[("u11-order", &source)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/definition-cycle",
        "u_self",
        cycle(vec![edge("u_self", "u_self")]),
    );
}

/// FSpec FR-142-AC-16: both U11m orderings retain their own first diagnostic.
#[trace("FSpec-TC-187", "FSpec-FR-142-AC-16", "FR-091-AC-71")]
#[test]
fn u11m_adapts_both_dimension_orders_and_their_diagnostic_payloads() {
    let vector = vector("U11m");
    let inputs = quoted(vector.inputs);
    assert_eq!(inputs.len(), 4, "U11m");
    assert_expected(vector, "`invalid_package`/`definition-cycle`");
    assert_expected(vector, "`missing_declaration`/`missing-name`");
    let cycle_source = format!(
        "{}\n{}\n{}{}",
        inputs[0],
        inputs[1],
        adapt_unit(inputs[2]),
        adapt_unit(inputs[3])
    );
    let (texts, refusal) = assemble_refusal(&[("u11m-area-first", &cycle_source)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/definition-cycle",
        "a",
        cycle(vec![edge("a", "b"), edge("b", "a")]),
    );

    let base_source = format!(
        "{}\n{}\n{}{}",
        inputs[1],
        inputs[0],
        adapt_unit(inputs[2]),
        adapt_unit(inputs[3])
    );
    let (texts, refusal) = assemble_refusal(&[("u11m-base-first", &base_source)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "missing_declaration/missing-name",
        "L",
        AssemblyCause::UnanchoredBaseDimension {
            dimension: "L".to_owned(),
            unit: "a".to_owned(),
        },
    );
}

/// FSL FR-480-AC-6 / FR-514-AC-5: adapt both U11q document orders and retain
/// the first document's exact source location and topology fields.
#[trace(
    "FSpec-TC-187",
    "FSpec-FR-142-AC-17",
    "FR-091-AC-71",
    "FR-480-AC-6",
    "FR-514-AC-5"
)]
#[test]
fn u11q_adapts_each_document_order_and_reports_its_first_defect() {
    let vector = vector("U11q");
    let inputs = quoted(vector.inputs);
    assert_eq!(inputs.len(), 4, "U11q");
    assert_expected(vector, "`invalid_package`/`invalid-value`");
    assert_expected(vector, "`invalid_package`/`definition-cycle`");
    let time = format!("{}\n{}", inputs[0], adapt_unit(inputs[1]));
    let length = format!("{}\n{}", inputs[2], adapt_unit(inputs[3]));

    let (texts, refusal) = assemble_refusal(&[("doc1", &time), ("doc2", &length)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/invalid-value",
        "z",
        AssemblyCause::UnitGraphTopology {
            fault: TopologyFault::ZeroScale,
            retained: vec![
                TopologyRetained::Unit {
                    name: "z".to_owned(),
                    key: None,
                },
                TopologyRetained::Scale(Rational::from_integer(Integer::zero())),
            ],
        },
    );

    let (texts, refusal) = assemble_refusal(&[("doc2", &length), ("doc1", &time)]);
    assert_diagnostic(
        &texts[0],
        &refusal,
        "invalid_package/definition-cycle",
        "u_self",
        cycle(vec![edge("u_self", "u_self")]),
    );
}
