// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S3 checks a `verify` plan and a
//! `synthesis` declaration. Its refusing cases: an unbounded synthesis
//! domain refuses `unsupported_projection`/`unbounded-extent`, and a plan
//! whose `depends` edges form a cycle refuses
//! `invalid_package`/`definition-cycle`.

use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{CheckCause, CheckRefusal, PackageDeclarations};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\";\n\
     temporal Reaches using p over (c: Config::ConfigVersion) clock \"model-steps\" \
     on origin { eventually holds(c.versionNumber = 2) }\n";

/// The S3 refusals of a unit holding `HEADER` and `declarations`, or the
/// S1/S2 refusal that stopped it earlier, rendered by `Debug`.
fn check(declarations: &str) -> Result<Vec<CheckRefusal>, String> {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let text = format!("{HEADER}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:synthesis-verify"),
        "program.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(format!("S1: {:?}", parsed.diagnostics()));
    }
    let raw = parsed.source().reference().clone();
    let unit = fsl_forms::build_unit(&parsed).map_err(|refusal| format!("S2: {refusal:?}"))?;
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    let declarations =
        PackageDeclarations::assemble(raw, "fixture".to_string(), unit, models, Vec::new())
            .expect("the unit assembles");
    Ok(
        match declarations.check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        ) {
            Ok(_) => Vec::new(),
            Err(refusals) => refusals,
        },
    )
}

fn codes(refusals: &[CheckRefusal]) -> Vec<Option<CatalogCode>> {
    refusals
        .iter()
        .map(|refusal| refusal.cause.catalog_code())
        .collect()
}

fn assert_missing_name_span(refusals: &[CheckRefusal], source: &str, name: &str) {
    let [CheckRefusal { cause, .. }] = refusals else {
        panic!("expected one missing-name refusal: {refusals:?}");
    };
    let CheckCause::MissingQualifiedName(missing) = cause.as_ref() else {
        panic!("expected one missing-name refusal: {refusals:?}");
    };
    let start = source
        .find(name)
        .expect("the missing name is in the source");
    assert_eq!(missing.name, name);
    assert_eq!(
        (missing.span.start, missing.span.end),
        (start, start + name.len())
    );
}

/// FR-754-AC-2: a `synthesis` whose `domain` names `Integer`, which has no
/// finite bound, refuses `unsupported_projection`/`unbounded-extent`.
#[trace("TC-2104", "FR-754-AC-2")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-8")]
#[test]
fn an_unbounded_synthesis_domain_refuses_unbounded_extent() {
    let declaration = "type Wide = Integer;\n\
         synthesis Pick using v grammar Wide domain Wide satisfies { Pick * Pick = 49 };";
    let refusals = check(declaration)
        .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_eq!(
        codes(&refusals),
        [Some(CatalogCode::new(
            "unsupported_projection",
            "unbounded-extent"
        ))],
        "{refusals:?}"
    );
    let [CheckRefusal { cause, .. }] = refusals.as_slice() else {
        panic!("expected a located unbounded-domain refusal: {refusals:?}");
    };
    let fsl_semantics::check::CheckCause::SynthesisUnboundedDomain { span, .. } = cause.as_ref()
    else {
        panic!("expected a located unbounded-domain refusal: {refusals:?}");
    };
    let source = format!("{HEADER}{declaration}\n");
    assert_eq!(&source[span.start..span.end], "Wide");

    let builtin =
        "synthesis Pick using v grammar Integer domain Integer satisfies { Pick * Pick = 49 };";
    let refusals =
        check(builtin).unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_eq!(
        codes(&refusals),
        [Some(CatalogCode::new(
            "unsupported_projection",
            "unbounded-extent"
        ))],
        "{refusals:?}"
    );
    let [CheckRefusal { cause, .. }] = refusals.as_slice() else {
        panic!("expected a located unbounded-domain refusal: {refusals:?}");
    };
    let fsl_semantics::check::CheckCause::SynthesisUnboundedDomain { component, span } =
        cause.as_ref()
    else {
        panic!("expected a located unbounded-domain refusal: {refusals:?}");
    };
    assert_eq!(component, "domain.Integer");
    let source = format!("{HEADER}{builtin}\n");
    assert_eq!(&source[span.start..span.end], "Integer");
}

/// FR-754-AC-2: a record's first unbounded field and a recursive union's
/// recursive member both refuse at the authored `domain` name.
#[trace("TC-2104", "FR-754-AC-2")]
#[test]
fn composite_synthesis_domains_name_the_first_unbounded_component() {
    for (declaration, expected_domain, expected_component) in [
        (
            "record Packet { payload: String; }\n\
             synthesis Pick using v grammar Packet domain Packet satisfies { true };",
            "Packet",
            "domain.payload.String",
        ),
        (
            "union Tree { Leaf, Branch(Option<Tree>) }\n\
             synthesis Pick using v grammar Tree domain Tree satisfies { true };",
            "Tree",
            "domain.Branch[0].some.recursive composite",
        ),
    ] {
        let refusals = check(declaration)
            .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
        assert_eq!(
            codes(&refusals),
            [Some(CatalogCode::new(
                "unsupported_projection",
                "unbounded-extent"
            ))],
            "{refusals:?}"
        );
        let [CheckRefusal { cause, .. }] = refusals.as_slice() else {
            panic!("expected one unbounded-domain refusal: {refusals:?}");
        };
        let fsl_semantics::check::CheckCause::SynthesisUnboundedDomain { component, span } =
            cause.as_ref()
        else {
            panic!("expected one unbounded-domain refusal: {refusals:?}");
        };
        assert_eq!(component, expected_component);
        let source = format!("{HEADER}{declaration}\n");
        assert_eq!(&source[span.start..span.end], expected_domain);
    }
}

/// FR-754-AC-2 (positive half): the same declaration over a bounded domain
/// checks with no refusal.
#[trace("TC-2104", "FR-754-AC-2")]
#[test]
fn a_bounded_synthesis_domain_checks() {
    let refusals = check(
        "type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit satisfies { Pick * Pick = 49 };",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert!(refusals.is_empty(), "{refusals:?}");
}

/// FR-753-AC-2: steps `a depends [b]` and `b depends [a]` refuse
/// `invalid_package`/`definition-cycle`.
#[trace("TC-2103", "FR-753-AC-2")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-10")]
#[test]
fn a_plan_dependency_cycle_refuses_definition_cycle() {
    let refusals = check(
        "verify Loop using p {\n\
           check a claim Reaches method model_check domain Config::ConfigVersion depends [b];\n\
           check b claim Reaches method model_check domain Config::ConfigVersion depends [a];\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&refusals),
        [Some(CatalogCode::new(
            "invalid_package",
            "definition-cycle"
        ))],
        "{refusals:?}"
    );
    let [CheckRefusal { cause, .. }] = refusals.as_slice() else {
        panic!("the cycle refusal retains its edges: {refusals:?}");
    };
    let CheckCause::DefinitionCycle { edges } = cause.as_ref() else {
        panic!("the cycle refusal retains its edges: {refusals:?}");
    };
    assert_eq!(
        edges,
        &[
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "a".to_owned())
        ]
    );
}

/// FR-753-AC-2 (positive half): an acyclic plan over `Reaches` checks.
#[trace("TC-2103", "FR-753-AC-2")]
#[test]
fn an_acyclic_plan_checks() {
    let refusals = check(
        "verify Release using p {\n\
           check safe claim Reaches method model_check domain Config::ConfigVersion bound 20;\n\
           check later claim Reaches method model_check domain Config::ConfigVersion depends [safe];\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert!(refusals.is_empty(), "{refusals:?}");
}

/// FSpec FR-192-AC-9: a repeated step name refuses
/// `conflicting_declaration`/`duplicate-declaration`, and a `method` named
/// `fuzz` refuses `missing_declaration`/`missing-name`.
#[trace("TC-2103", "FR-753-AC-2")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-9")]
#[test]
fn plan_step_name_and_method_refusals() {
    let duplicate_source = format!(
        "{HEADER}verify Twice using p {{\n\
           check a claim Reaches method model_check domain Config::ConfigVersion;\n\
           check a claim Reaches method model_check domain Config::ConfigVersion;\n\
         }}"
    );
    let duplicate = check(
        "verify Twice using p {\n\
           check a claim Reaches method model_check domain Config::ConfigVersion;\n\
           check a claim Reaches method model_check domain Config::ConfigVersion;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&duplicate),
        [Some(CatalogCode::new(
            "conflicting_declaration",
            "duplicate-declaration"
        ))],
        "{duplicate:?}"
    );
    let [CheckRefusal { cause, .. }] = duplicate.as_slice() else {
        panic!("the duplicate refusal retains both spans: {duplicate:?}");
    };
    let CheckCause::DuplicateVerificationStep { first, second, .. } = cause.as_ref() else {
        panic!("the duplicate refusal retains both spans: {duplicate:?}");
    };
    let first_start = duplicate_source.find("check a").unwrap() + "check ".len();
    let second_start = duplicate_source.rfind("check a").unwrap() + "check ".len();
    assert_eq!((first.start, first.end), (first_start, first_start + 1));
    assert_eq!((second.start, second.end), (second_start, second_start + 1));
    let fuzz = check(
        "verify Fuzzed using p {\n\
           check a claim Reaches method fuzz domain Config::ConfigVersion;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&fuzz),
        [Some(CatalogCode::new(
            "missing_declaration",
            "missing-name"
        ))],
        "{fuzz:?}"
    );
    assert_missing_name_span(
        &fuzz,
        &format!(
            "{HEADER}verify Fuzzed using p {{\n\
               check a claim Reaches method fuzz domain Config::ConfigVersion;\n\
             }}"
        ),
        "fuzz",
    );
}

/// FSpec FR-192-AC-9/10: a step resolves its claim, method, domain and
/// prerequisite, and rejects a zero bound and an inapplicable method.
#[trace("TC-2103", "FR-753-AC-2")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-9", "FSpec-FR-192-AC-10")]
#[test]
fn plan_reference_bound_and_method_applicability_refusals() {
    let missing_claim_source = format!(
        "{HEADER}verify MissingClaim using p {{\n\
           check a claim Never method model_check domain Config::ConfigVersion;\n\
         }}"
    );
    let missing_claim = check(
        "verify MissingClaim using p {\n\
           check a claim Never method model_check domain Config::ConfigVersion;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&missing_claim),
        [Some(CatalogCode::new(
            "missing_declaration",
            "missing-name"
        ))],
        "{missing_claim:?}"
    );
    assert_missing_name_span(&missing_claim, &missing_claim_source, "Never");

    let missing_domain = check(
        "verify MissingDomain using p {\n\
           check a claim Reaches method model_check domain Nowhere;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&missing_domain),
        [Some(CatalogCode::new(
            "missing_declaration",
            "missing-name"
        ))],
        "{missing_domain:?}"
    );
    assert_missing_name_span(
        &missing_domain,
        &format!(
            "{HEADER}verify MissingDomain using p {{\n\
               check a claim Reaches method model_check domain Nowhere;\n\
             }}"
        ),
        "Nowhere",
    );

    let missing_step = check(
        "verify MissingStep using p {\n\
           check a claim Reaches method model_check domain Config::ConfigVersion depends [unknown_step];\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&missing_step),
        [Some(CatalogCode::new(
            "missing_declaration",
            "missing-name"
        ))],
        "{missing_step:?}"
    );
    assert_missing_name_span(
        &missing_step,
        &format!(
            "{HEADER}verify MissingStep using p {{\n\
               check a claim Reaches method model_check domain Config::ConfigVersion depends [unknown_step];\n\
             }}"
        ),
        "unknown_step",
    );

    let zero_bound = check(
        "verify ZeroBound using p {\n\
           check a claim Reaches method model_check domain Config::ConfigVersion bound 0;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&zero_bound),
        [Some(CatalogCode::new(
            "invalid_model_binding",
            "malformed-declaration"
        ))],
        "{zero_bound:?}"
    );
    let [CheckRefusal { cause, .. }] = zero_bound.as_slice() else {
        panic!("the bound refusal retains its span: {zero_bound:?}");
    };
    let CheckCause::VerificationPlanBoundZero { span } = cause.as_ref() else {
        panic!("the bound refusal retains its span: {zero_bound:?}");
    };
    let zero_source = format!(
        "{HEADER}verify ZeroBound using p {{\n\
           check a claim Reaches method model_check domain Config::ConfigVersion bound 0;\n\
         }}"
    );
    let zero_start = zero_source.find("bound 0").unwrap() + "bound ".len();
    assert_eq!((span.start, span.end), (zero_start, zero_start + 1));

    let inapplicable = check(
        "verify WrongMethod using p {\n\
           check a claim Reaches method schedulability domain Config::ConfigVersion;\n\
         }",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `verify`: {refusal}"));
    assert_eq!(
        codes(&inapplicable),
        [Some(CatalogCode::new(
            "invalid_capability",
            "inapplicable-kind"
        ))],
        "{inapplicable:?}"
    );
    let [CheckRefusal { cause, .. }] = inapplicable.as_slice() else {
        panic!("the inapplicable-method refusal retains its span: {inapplicable:?}");
    };
    let CheckCause::VerificationPlanInapplicableMethod { span, .. } = cause.as_ref() else {
        panic!("the inapplicable-method refusal retains its span: {inapplicable:?}");
    };
    let method_source = format!(
        "{HEADER}verify WrongMethod using p {{\n\
           check a claim Reaches method schedulability domain Config::ConfigVersion;\n\
         }}"
    );
    let method_start = method_source.find("schedulability").unwrap();
    let method_end = method_start + "schedulability".len();
    assert_eq!((span.start, span.end), (method_start, method_end));
}

/// FSpec FR-194-AC-9: a `grammar` naming nothing refuses
/// `missing_declaration`/`missing-name`; a `satisfies` block of type
/// `Integer` refuses `ill_typed`/`non-boolean-root`.
#[trace("TC-2104", "FR-754-AC-2")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-9")]
#[test]
fn synthesis_name_and_type_refusals() {
    let missing = check(
        "type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Nothing domain Digit satisfies { Pick = 1 };",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_eq!(
        codes(&missing),
        [Some(CatalogCode::new(
            "missing_declaration",
            "missing-name"
        ))],
        "{missing:?}"
    );
    assert_missing_name_span(
        &missing,
        &format!(
            "{HEADER}type Digit = Int[0, 9];\n\
             synthesis Pick using v grammar Nothing domain Digit satisfies {{ Pick = 1 }};\n"
        ),
        "Nothing",
    );
    let missing_domain = check(
        "type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Nowhere satisfies { Pick = 1 };",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_missing_name_span(
        &missing_domain,
        &format!(
            "{HEADER}type Digit = Int[0, 9];\n\
             synthesis Pick using v grammar Digit domain Nowhere satisfies {{ Pick = 1 }};\n"
        ),
        "Nowhere",
    );
    let integer = check(
        "type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit satisfies { Pick + 1 };",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_eq!(
        codes(&integer),
        [Some(CatalogCode::new("ill_typed", "non-boolean-root"))],
        "{integer:?}"
    );
}
