// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-981 (TC-2773): `size<N>(c)` builds the `Size` form carrying its
//! result type, `size(c)` builds it with none, and the malformed spellings
//! refuse at S1.
use fsl_cst::{CompleteCause, CompleteCode, ParsedSource};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n";

fn parse(declarations: &str) -> (String, ParsedSource) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "typed-size.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    (text, parsed)
}

fn function(body: &str) -> String {
    format!("function f using v(c: Sequence<Integer>[0, 4]): Int[0, 4] pure {{ {body} }}")
}

/// FR-981-AC-1: both spellings build a `Size` form; only the typed one
/// carries a type argument.
#[trace("TC-2773", "FR-981-AC-1")]
#[test]
#[ignore = "QSL-1966: S2 refuses size<T>(..) as unrepresented; building Size with its type argument is not implemented yet"]
fn a_typed_size_builds_a_size_form_with_its_type_argument() {
    let (_, parsed) = parse(&function("size<Int[0, 4]>(c)"));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let typed = format!("{:?}", build_unit(&parsed).expect("S2 builds size<N>(c)"));
    assert!(typed.contains("Size"), "{typed}");
    assert!(typed.contains("type_argument: Some"), "{typed}");
    let (_, parsed) = parse(&function("size(c)"));
    let untyped = format!("{:?}", build_unit(&parsed).expect("S2 builds size(c)"));
    assert!(untyped.contains("type_argument: None"), "{untyped}");
}

/// FR-981-AC-2: an empty type argument list and a missing operand refuse at
/// the token the grammar requires.
#[trace("TC-2773", "FR-981-AC-2")]
#[test]
#[ignore = "QSL-1966: the located refusals of `size<>(c)` and `size<N>()` not implemented yet"]
fn a_size_without_its_type_or_operand_refuses_at_s1() {
    for (body, at) in [("size<>(c)", ">(c)"), ("size<Int[0, 4]>()", ")")] {
        let (text, parsed) = parse(&function(body));
        let first = parsed.diagnostics().first().expect("refused");
        assert_eq!(
            (first.code, first.cause),
            (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
            "{body}"
        );
        let start = text.find(body).expect("the body") + body.find(at).expect("the token");
        assert_eq!(first.byte_span().expect("located").start, start, "{body}");
    }
}
