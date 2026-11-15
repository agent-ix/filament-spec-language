// SPDX-License-Identifier: AGPL-3.0-or-later
//! Definition and model selection validation in the parser: each invalid
//! identity of a `profile`, and each invalid identity of an `import`, and each
//! invalid identity of a `model` declaration, is located at its own literal. Formerly in the root crate's
//! `complete::package_tests`; it parses and never resolves, so it is
//! layer 1's.
use fsl_cst::{parse, Limits};
use fsl_foundation::selection::{DefinitionRef, InvalidDefinitionComponent};
use fsl_foundation::{Code, SourceIdentity};
use ix_trace_rs::trace;

#[trace("FSpec-TC-180", "FSpec-FR-131-AC-2")]
#[test]
fn selection_validation_locates_each_invalid_component_for_every_declaration_kind() {
    assert_eq!(
        DefinitionRef::new("agent-ix", "").unwrap_err(),
        InvalidDefinitionComponent::Identity
    );
    assert_eq!(
        DefinitionRef::new("", "acme/definition").unwrap_err(),
        InvalidDefinitionComponent::Authority
    );

    for (declaration_kind, components) in [
        ("profile", &["identity"][..]),
        ("import", &["identity"][..]),
        ("model", &["identity"][..]),
    ] {
        for &invalid_component in components {
            let identity = if invalid_component == "identity" {
                ""
            } else {
                "acme/definition"
            };
            let declaration = match declaration_kind {
                "profile" => format!("profile Complete = \"{identity}\";"),
                "import" => format!("import \"{identity}\" as Base;"),
                "model" => {
                    format!("model M = \"{identity}\";")
                }
                _ => unreachable!("closed declaration-kind test table"),
            };
            let valid_profile = (declaration_kind != "profile")
                .then(|| "profile Complete = \"acme/profile\";\n".to_owned());
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n{}{declaration}\nrecord R {{ datum: Integer; }}",
                valid_profile.as_deref().unwrap_or_default(),
            );
            let invalid_literal = format!("\"{identity}\"");
            let expected_start = source.find(&invalid_literal).unwrap();
            let parsed = parse(
                SourceIdentity {
                    authority: "test".into(),
                    identity: format!("test:{declaration_kind}-{invalid_component}"),
                },
                "invalid-selection.native",
                source.as_bytes(),
                Limits::default(),
            )
            .unwrap();

            assert!(!parsed.is_admissible());
            assert_eq!(parsed.diagnostics().len(), 1);
            let diagnostic = &parsed.diagnostics()[0];
            assert_eq!(diagnostic.code, Code::InvalidIdentifier);
            assert_eq!(diagnostic.byte_span().unwrap().start, expected_start);
            assert_eq!(
                diagnostic.byte_span().unwrap().end,
                expected_start + invalid_literal.len()
            );
            assert!(diagnostic.message.contains(invalid_component));
        }
    }
}

/// FR-056 (FSpec FR-154): a `model` names its domain package by identity
/// alone, `model M = "<identity>";`. A `version` or `digest` token after the
/// identity is a syntax error and the selection is not recorded; a `model`
/// without a string identity is a located diagnostic, never a silent drop.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn a_model_names_only_its_domain_package_identity() {
    let parse_model = |model: &str| {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"acme/profile\";\n{model}\nrecord R {{ datum: Integer; }}"
        );
        parse(
            SourceIdentity::new("test", "test:model"),
            "model.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap()
    };
    let parsed = parse_model("model M = \"acme/orders\";");
    assert_eq!(parsed.diagnostics(), []);
    let selection = &parsed.selections().models[0];
    assert_eq!(selection.alias, "M");
    assert_eq!(selection.identity, "acme/orders");

    let hex = "c".repeat(64);
    for refused in [
        "model M = \"acme/orders\" version \"1\";".to_owned(),
        format!("model M = \"acme/orders\" digest \"sha256-jcs:{hex}\";"),
        format!("model M = \"acme/orders\" version \"1\" digest \"sha256-jcs:{hex}\";"),
        "model M = ;".to_owned(),
        "model M = acme;".to_owned(),
    ] {
        let parsed = parse_model(&refused);
        assert!(!parsed.is_admissible(), "{refused}");
        assert!(parsed.selections().models.is_empty(), "{refused}");
        assert!(!parsed.diagnostics().is_empty(), "{refused}");
    }
}

/// FR-099-AC-2 (TC-446 step 2): an `import` names its library by identity
/// alone, `import "L" [as a];`. A `version` or `digest` token after the
/// identity is a syntax error, and the selection is not recorded.
#[trace("FR-099-AC-2", "TC-446")]
#[trace("FSpec-FR-307-AC-6", "FSpec-TC-227")]
#[test]
fn an_import_names_only_its_library_identity() {
    let parse_import = |import: &str| {
        let source = import_unit(import);
        parse(
            SourceIdentity::new("a", "u"),
            "unit.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap()
    };
    let parsed = parse_import("import \"test/geometry\" as g;");
    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    let import = &parsed.selections().imports[0];
    assert_eq!(import.identity, "test/geometry");
    assert_eq!(import.alias.as_deref(), Some("g"));
    let bare = parse_import("import \"test/geometry\";");
    assert!(bare.diagnostics().is_empty());
    assert_eq!(bare.selections().imports[0].alias, None);

    let hex = "d".repeat(64);
    for old in [
        "import \"test/geometry\" version \"1\" as g;".to_owned(),
        format!("import \"test/geometry\" digest \"{hex}\" as g;"),
        format!("import \"test/geometry\" version \"1\" digest \"{hex}\" as g;"),
    ] {
        let parsed = parse_import(&old);
        assert!(!parsed.is_admissible(), "{old}");
        assert!(parsed.selections().imports.is_empty(), "{old}");
        assert_eq!(parsed.diagnostics()[0].code, Code::InvalidSyntax, "{old}");
        // FR-307-AC-6: the cause is `unexpected-token`, located at the
        // `version` or `digest` token itself.
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.cause, fsl_cst::CompleteCause::UnexpectedToken);
        let token = if old.contains("version") {
            "version"
        } else {
            "digest"
        };
        assert_eq!(
            diagnostic
                .region
                .as_ref()
                .map(|region| usize::try_from(region.start()).unwrap()),
            token_offset(&old, token),
            "{old}"
        );
    }
}

/// The unit text `an_import_names_only_its_library_identity` parses for one
/// `import` line.
fn import_unit(import: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {import}\n\
         record R {{ datum: Integer; }}\n"
    )
}

/// The byte offset of `token` in the unit text built from `import`.
fn token_offset(import: &str, token: &str) -> Option<usize> {
    let unit = import_unit(import);
    let line = unit.find(import)?;
    import.find(token).map(|at| line + at)
}

/// Parses `text` as a unit of the selection tests.
fn parse_unit(id: &str, text: &str) -> fsl_cst::ParsedSource {
    parse(
        SourceIdentity {
            authority: "test".into(),
            identity: format!("test:{id}"),
        },
        format!("{id}.native"),
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

/// FSpec FR-500-AC-1: `1-draft` is the only edition; any other edition string,
/// `0-draft` included, refuses `unknown_edition`/`unsupported-selection` at the
/// edition string literal, with no other diagnostic.
#[trace("FSpec-TC-1000", "FSpec-FR-500-AC-1")]
#[test]
fn an_edition_other_than_1_draft_refuses_unknown_edition_at_its_literal() {
    let body = "\nfunction f(): Boolean pure { true }";
    let admitted = parse_unit(
        "edition-1-draft",
        &format!("language \"ix:native\" edition \"1-draft\";{body}"),
    );
    assert!(admitted.is_admissible(), "{:?}", admitted.diagnostics());
    for (edition, end) in [("0-draft", 38), ("2-draft", 38), ("", 31)] {
        let text = format!("language \"ix:native\" edition \"{edition}\";{body}");
        let parsed = parse_unit("edition-other", &text);
        let diagnostics = parsed.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{edition}: {diagnostics:?}");
        assert_eq!(
            diagnostics[0].code,
            fsl_cst::CompleteCode::UnknownEdition,
            "{edition}"
        );
        assert_eq!(
            diagnostics[0].cause,
            fsl_cst::CompleteCause::UnsupportedSelection,
            "{edition}"
        );
        let span = diagnostics[0].byte_span().expect("located");
        assert_eq!((span.start, span.end), (29, end), "{edition}");
    }
}

/// FSpec FR-500-AC-2: a unit may declare no `profile`; it parses with no
/// diagnostic and no profile selection.
#[trace("FSpec-TC-1001", "FSpec-FR-500-AC-2")]
#[test]
fn a_unit_with_no_profile_parses() {
    let text = "language \"ix:native\" edition \"1-draft\";\nfunction f(): Boolean pure { true }";
    let parsed = parse_unit("no-profile", text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert!(parsed.diagnostics().is_empty());
    assert!(!parsed
        .cst()
        .nodes()
        .iter()
        .any(|node| node.production() == fsl_cst::Production::Profile));
}

/// FSpec FR-500-AC-4: a `function` and a `predicate` may omit `using`; a state
/// clause may not, and refuses `invalid_syntax`/`unexpected-token` at `on`.
#[trace("FSpec-TC-1001", "FSpec-FR-500-AC-4")]
#[test]
fn a_function_or_predicate_may_omit_using_and_a_state_clause_may_not() {
    let unit = |declaration: &str| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declaration}"
        )
    };
    for declaration in [
        "function g(x: Integer): Integer pure { x + 1 }",
        "predicate p(x: Integer): Boolean { x > 0 }",
    ] {
        let parsed = parse_unit("no-using", &unit(declaration));
        assert!(
            parsed.is_admissible(),
            "{declaration}: {:?}",
            parsed.diagnostics()
        );
    }
    let text = unit("invariant I on M::T at current { true }");
    let parsed = parse_unit("invariant-no-using", &text);
    let first = parsed.diagnostics().first().expect("refused");
    assert_eq!(first.code, fsl_cst::CompleteCode::InvalidSyntax);
    assert_eq!(first.cause, fsl_cst::CompleteCause::UnexpectedToken);
    assert_eq!(
        first.byte_span().expect("located").start,
        text.find(" on ").expect("the on") + 1
    );
}
