// SPDX-License-Identifier: AGPL-3.0-or-later
//! The README's first rule, checked against the model it imports.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::StateClauseKind;
use fsl_foundation::{diagnostic::Code, SourceIdentity};
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use ix_trace_rs::trace;

const FIRST_RULE: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
model Config = "example/config-version";

invariant ParentOrder using v on Config::ConfigVersion at current {
  present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber
}

invariant NoCycle using v on Config::ConfigVersion at current {
  not reaches(self, self, parent)
}

post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {
  self.versionNumber = pre(self.versionNumber)
}
"#;

fn first_rule() -> &'static str {
    let readme = include_str!("../../../README.md");
    let section = readme
        .split_once("## A first rule\n")
        .expect("the README has a first-rule section")
        .1;
    let rule = section
        .split_once("```\n")
        .expect("the first rule starts in a code fence")
        .1
        .split_once("```")
        .expect("the first rule ends in a code fence")
        .0;
    assert_eq!(rule, FIRST_RULE, "the checked fixture is the README's rule");
    rule
}

fn check(source: &str) -> Result<CheckedGraph, Vec<(Code, Option<&'static str>)>> {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "readme-first-rule"),
        "readme-first-rule.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the README rule");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the README rule");
    let document = include_bytes!("../../../examples/config-version/model.semantic-ir.json");
    let packages = package_input([document.as_slice()]).expect("the named model admits");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the README selects its supplied model");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .map_err(|refusal| {
        refusal
            .errors
            .iter()
            .map(|error| (error.cause.code(), Some(error.cause.catalog_code().cause())))
            .collect::<Vec<_>>()
    })?;
    declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
                .collect()
        })
}

/// The documented model layer checks all three clauses of the first rule.
#[trace("TC-490", "FR-110-AC-7", "TC-1015", "FR-361-AC-13")]
#[test]
fn the_readme_first_rule_compiles_against_its_model() {
    let graph = check(first_rule()).expect("the README rule checks against its model");
    let clauses: Vec<_> = graph
        .state_clauses()
        .iter()
        .map(|clause| (clause.name(), clause.kind()))
        .collect();
    assert_eq!(
        clauses,
        [
            ("ParentOrder", StateClauseKind::Invariant),
            ("NoCycle", StateClauseKind::Invariant),
            ("VersionUnchanged", StateClauseKind::Postcondition),
        ]
    );
}

/// A value-only layer cannot admit the rule's object-graph expressions.
#[trace("TC-1015", "FR-361-AC-13")]
#[test]
fn the_same_rule_under_value_complete_refuses_expression_form() {
    let source = first_rule().replace("filament.model.complete/v1", "filament.value.complete/v1");
    let refusals = check(&source).expect_err("the value layer refuses graph expressions");
    assert!(!refusals.is_empty(), "a refusal has at least one cause");
    for refusal in refusals {
        assert_eq!(
            refusal,
            (Code::UnsupportedConstruct, Some("expression-form"))
        );
    }
}
