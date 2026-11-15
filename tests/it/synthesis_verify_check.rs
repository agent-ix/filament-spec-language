// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S3 checks a `verify` plan and a
//! `synthesis` declaration. Its refusing cases: an unbounded synthesis
//! domain refuses `unsupported_projection`/`unbounded-extent`, and a plan
//! whose `depends` edges form a cycle refuses
//! `invalid_package`/`definition-cycle`.

use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
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
        match declarations.check(CheckMode::Linked, CheckingLimits::default()) {
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

/// FR-754-AC-2: a `synthesis` whose `domain` names `Integer`, which has no
/// finite bound, refuses `unsupported_projection`/`unbounded-extent`.
#[trace("TC-2104", "FR-754-AC-2")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-8")]
#[test]
#[ignore = "QSL-2235: S3 finite-domain rule for `synthesis` not implemented yet"]
fn an_unbounded_synthesis_domain_refuses_unbounded_extent() {
    let refusals = check(
        "type Wide = Integer;\n\
         synthesis Pick using v grammar Wide domain Wide satisfies { Pick * Pick = 49 };",
    )
    .unwrap_or_else(|refusal| panic!("S1 and S2 admit `synthesis`: {refusal}"));
    assert_eq!(
        codes(&refusals),
        [Some(CatalogCode::new(
            "unsupported_projection",
            "unbounded-extent"
        ))],
        "{refusals:?}"
    );
}

/// FR-754-AC-2 (positive half): the same declaration over a bounded domain
/// checks with no refusal.
#[trace("TC-2104", "FR-754-AC-2")]
#[test]
#[ignore = "QSL-2235: S2/S3 `synthesis` declaration not implemented yet"]
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
#[ignore = "QSL-2236: S3 `verify` plan check not implemented yet"]
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
}

/// FR-753-AC-2 (positive half): an acyclic plan over `Reaches` checks.
#[trace("TC-2103", "FR-753-AC-2")]
#[test]
#[ignore = "QSL-2236: S2/S3 `verify` plan not implemented yet"]
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
#[ignore = "QSL-2236: S3 `verify` plan check not implemented yet"]
fn plan_step_name_and_method_refusals() {
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
}

/// FSpec FR-194-AC-9: a `grammar` naming nothing refuses
/// `missing_declaration`/`missing-name`; a `satisfies` block of type
/// `Integer` refuses `ill_typed`/`non-boolean-root`.
#[trace("TC-2104", "FR-754-AC-2")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-9")]
#[test]
#[ignore = "QSL-2235: S3 `synthesis` name and type checks not implemented yet"]
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
