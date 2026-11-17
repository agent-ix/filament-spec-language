// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035-AC-7: under `1-draft`, `helper`, `rec`, `cast` and `Tuple` are
//! ordinary identifiers in every native declaration and binder name
//! position (FSpec FR-339-AC-5, QSL-1649); lowercase `tuple`, a complete-V1
//! reserved word, still is not.
use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const LANGUAGE: &str = "language \"ix:native\" edition \"1-draft\";\n";
const PROFILE: &str = "profile v = \"filament.value.complete/v1\";\n";
const SPELLINGS: [&str; 8] = [
    "helper", "rec", "cast", "Tuple", "helperz", "recount", "caster", "TupleX",
];

/// The function and predicate name and `using` alias positions. `@` marks
/// the name; the text is a whole `1-draft` unit.
const FUNCTION_POSITIONS: [(&str, &str); 4] = [
    (
        "function name",
        "{P}function @ using v(x: Integer): Integer pure { x + 1 }",
    ),
    (
        "predicate name",
        "{P}predicate @ using v(x: Integer): Boolean { x = 1 }",
    ),
    (
        "function `using` alias",
        "{P}function f using @(x: Integer): Integer pure { x + 1 }",
    ),
    (
        "predicate `using` alias",
        "{P}predicate p using @(x: Integer): Boolean { x = 1 }",
    ),
];

/// The other native name positions, one per family.
const OTHER_POSITIONS: [(&str, &str); 13] = [
    (
        "parameter",
        "{P}function f using v(@: Integer): Integer pure { 1 }",
    ),
    (
        "`let` binder",
        "{P}function f using v(x: Integer): Integer pure { let @ = x in 1 }",
    ),
    ("record name", "{P}record @ { datum: Integer; }"),
    ("record field", "{P}record R { @: Integer; }"),
    ("enum name", "{P}enum @ { a, b }"),
    ("enum member", "{P}enum E { a, @ }"),
    ("tuple name", "{P}tuple @(Integer, Integer);"),
    ("union name", "{P}union @ { a, b }"),
    ("union member", "{P}union U { a, @(Integer) }"),
    ("type alias", "{P}type @ = Integer;"),
    (
        "profile alias",
        "profile @ = \"filament.value.complete/v1\";\nrecord R { datum: Integer; }",
    ),
    (
        "import alias",
        "{P}import \"acme/base\" as @;\nrecord R { datum: Integer; }",
    ),
    (
        "model alias",
        "{P}model @ = \"acme/model\";\nrecord R { datum: Integer; }",
    ),
];

fn unit(template: &str, name: &str) -> String {
    format!(
        "{LANGUAGE}{}\n",
        template.replace("{P}", PROFILE).replace('@', name)
    )
}

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:ordinary-identifier-spellings"),
        "ordinary.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 within the source ceiling")
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn the_four_spellings_name_every_native_position() {
    for (position, template) in FUNCTION_POSITIONS.into_iter().chain(OTHER_POSITIONS) {
        for name in SPELLINGS {
            let text = unit(template, name);
            let parsed = parse(&text);
            assert!(
                parsed.is_admissible(),
                "{position}: {text}: {:?}",
                parsed.diagnostics()
            );
        }
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_qualified_member_name_admits_the_four_spellings() {
    for name in SPELLINGS {
        for declarations in [
            format!("model M = \"acme/model\";\nrecord R {{ datum: M::{name}; }}"),
            format!("function f using v(x: M::R): Integer pure {{ x.{name} }}"),
        ] {
            let text = format!("{LANGUAGE}{PROFILE}{declarations}\n");
            let parsed = parse(&text);
            assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        }
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn lowercase_tuple_still_refuses_as_a_record_name() {
    let text = unit("{P}record @ { datum: Integer; }", "tuple");
    let parsed = parse(&text);
    assert!(!parsed.is_admissible(), "{text}");
    let [refusal] = parsed.diagnostics() else {
        panic!("one refusal for {text}: {:?}", parsed.diagnostics());
    };
    assert_eq!(refusal.code, CompleteCode::InvalidSyntax);
    assert_eq!(refusal.cause, CompleteCause::UnexpectedToken);
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn collection_constructor_words_are_refused_as_parameter_names() {
    for name in ["set", "bag", "orderedSet"] {
        let text = unit(
            "{P}function f using v(@: Integer): Boolean pure { true }",
            name,
        );
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{text}");
        let [refusal] = parsed.diagnostics() else {
            panic!("one refusal for {text}: {:?}", parsed.diagnostics());
        };
        assert_eq!(refusal.code, CompleteCode::InvalidSyntax, "{text}");
        assert_eq!(refusal.cause, CompleteCause::UnexpectedToken, "{text}");
        assert_eq!(
            refusal.byte_span().expect("located").start,
            text.find(name).expect("parameter name"),
            "{text}"
        );
    }

    for name in ["set", "bag", "orderedSet"] {
        let text = unit(
            "{P}function f using v(x: Integer): Boolean pure { let @ = x in true }",
            name,
        );
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{text}");
        assert!(
            parsed.diagnostics().iter().any(|refusal| {
                refusal.code == CompleteCode::InvalidSyntax
                    && refusal.cause == CompleteCause::UnexpectedToken
                    && refusal.message.contains(name)
            }),
            "{text}: {:?}",
            parsed.diagnostics()
        );
    }

    for declarations in [
        "function f using v(n: Integer, setting: Integer): Boolean pure { let ok = n in set[ok, setting] }",
        "record R { values: Set<Integer>; }",
    ] {
        let text = format!("{LANGUAGE}{PROFILE}{declarations}\n");
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    }
}
