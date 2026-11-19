// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-981 (TC-2773): typed size forms carry their result domain.
use fsl_cst::{CompleteCause, CompleteCode, ParsedSource};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n";

fn parse(declaration: &str) -> (String, ParsedSource) {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "typed-size"),
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

/// FR-981-AC-1: typed and untyped size both build a Size form.
#[trace("TC-2773", "FR-981-AC-1")]
#[test]
fn a_typed_size_builds_a_size_form_with_its_type_argument() {
    let (_, parsed) = parse(&function("size<Int[0, 4]>(c)"));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let typed = format!("{:?}", build_unit(&parsed).expect("S2 builds typed size"));
    assert!(typed.contains("Size"), "{typed}");
    assert!(typed.contains("type_argument: Some"), "{typed}");
    assert!(typed.contains("Builtin(Int)"), "{typed}");
    assert!(typed.contains("bounds: [\"0\", \"4\"]"), "{typed}");

    let (_, parsed) = parse(&function("size(c)"));
    let untyped = format!("{:?}", build_unit(&parsed).expect("S2 builds untyped size"));
    assert!(untyped.contains("type_argument: None"), "{untyped}");
}

/// FR-981-AC-2: malformed type and operand slots are rejected by S1.
#[trace("TC-2773", "FR-981-AC-2")]
#[test]
fn a_size_without_its_type_or_operand_refuses_at_s1() {
    for (body, token) in [("size<>(c)", ">"), ("size<Int[0, 4]>()", ")")] {
        let (text, parsed) = parse(&function(body));
        let diagnostic = parsed
            .diagnostics()
            .first()
            .expect("malformed size refuses");
        assert_eq!(
            (diagnostic.code, diagnostic.cause),
            (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
            "{body}"
        );
        let expected = text.find(body).expect("function body") + body.find(token).expect("token");
        assert_eq!(
            diagnostic.byte_span().expect("located refusal").start,
            expected,
            "{body}"
        );
    }
}
