// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-099-AC-8 (TC-894): an imported function call discharges its argument
//! obligations from the importing unit's facts exactly as a local call does,
//! and the callee's own body obligations are discharged when the library is
//! compiled, never at the call site.
//!
//! The library is compiled here through the same assembler and checker the
//! spine drives: its `CheckedGraph` and an `ImportView` over its exported
//! function identity are what the importing unit's assembly admits.
//! The spine-level `CompileRefusal::Dependency` wrapper belongs to
//! `fsl-replay`, a layer above this crate, and is not reachable here; the
//! library's own refusal (code, cause, locus) is asserted instead.

use std::sync::Arc;

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{AdmittedImport, CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::library::{ImportView, LibraryName, LibraryPackage};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

pub(crate) const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n";

/// Source text compiled through S1, S2, assembly and S3.
pub(crate) fn compile(
    source: &str,
    imports: Vec<AdmittedImport>,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "imported-call"),
        "imported-call.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        imports,
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
}

/// The library `identity` compiled from `body`, as the importing unit's
/// admitted import: its checked graph and the import view of the exports
/// named in `exports` (each the identity the checker minted for that
/// function).
pub(crate) fn admitted_library(
    identity: &str,
    body: &str,
    exports: &[&str],
) -> Result<AdmittedImport, Vec<CheckRefusal>> {
    let graph = compile(&format!("{HEADER}{body}"), Vec::new())?;
    let library = LibraryName::new(identity).expect("a non-empty identity");
    let package = LibraryPackage {
        library: library.clone(),
        imports: Vec::new(),
        exports: exports
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    graph
                        .function_identity(name)
                        .unwrap_or_else(|| panic!("the library declares {name}")),
                )
            })
            .collect(),
    };
    Ok(AdmittedImport {
        identity: library,
        view: ImportView::of(&package).expect("a valid import view"),
        graph: Arc::new(graph),
    })
}

const GEOMETRY_F: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
const LOCAL_F2: &str = "function f2 using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

/// Checks one body that calls `f` as `g::f` and again as the local `f2`,
/// each over the same argument shape; returns the two outcomes.
type Checked = Result<CheckedGraph, Vec<CheckRefusal>>;

fn pair(parameter: &str, argument: &str, wrap: impl Fn(&str) -> String) -> (Checked, Checked) {
    let library = admitted_library("test/geometry", GEOMETRY_F, &["f"]).expect("library checks");
    let imported = compile(
        &format!(
            "{HEADER}import \"test/geometry\" as g;\n\
             function caller using v({parameter}): Boolean pure {{ {} }}\n",
            wrap(&format!("g::f({argument})"))
        ),
        vec![library],
    );
    let local = compile(
        &format!(
            "{HEADER}{LOCAL_F2}\
             function caller using v({parameter}): Boolean pure {{ {} }}\n",
            wrap(&format!("f2({argument})"))
        ),
        Vec::new(),
    );
    (imported, local)
}

fn plain(call: &str) -> String {
    call.to_owned()
}

/// Asserts both sides give one verdict and, when refused, one code, cause and
/// locus path.
fn assert_same_verdict(label: &str, (imported, local): (Checked, Checked), refused: bool) {
    match (imported, local) {
        (Ok(_), Ok(_)) => assert!(!refused, "{label}: both checked, expected a refusal"),
        (Err(imported), Err(local)) => {
            assert!(refused, "{label}: both refused: {imported:?}");
            assert_eq!(imported.len(), 1, "{label}: {imported:?}");
            assert_eq!(local.len(), 1, "{label}: {local:?}");
            assert_eq!(imported[0].cause.code(), local[0].cause.code(), "{label}");
            assert_eq!(imported[0].cause.cause(), local[0].cause.cause(), "{label}");
            assert_eq!(
                imported[0].location.path(),
                local[0].location.path(),
                "{label}: same locus"
            );
            assert_eq!(imported[0].cause.code(), Code::UndefinedExpression);
        }
        (imported, local) => panic!(
            "{label}: imported and local verdicts differ: imported ok={} local ok={}",
            imported.is_ok(),
            local.is_ok()
        ),
    }
}

#[trace("TC-894", "FR-099-AC-8")]
#[test]
fn an_imported_call_has_the_verdict_of_the_same_local_call() {
    assert_same_verdict("y: Int[0, 9]", pair("y: Int[0, 9]", "y", plain), false);
    assert_same_verdict("literal 3", pair("", "3", plain), false);
    assert_same_verdict("literal 12", pair("", "12", plain), true);
    assert_same_verdict("y: Int[0, 20]", pair("y: Int[0, 20]", "y", plain), true);
    assert_same_verdict(
        "y: Int[0, 20] under if y <= 9",
        pair("y: Int[0, 20]", "y", |call| {
            format!("if y <= 9 then {call} else false")
        }),
        false,
    );
}

#[trace("TC-894", "FR-099-AC-8")]
#[test]
fn a_refused_imported_call_locates_the_argument_not_the_callee() {
    let (imported, local) = pair("", "12", plain);
    let imported = imported.expect_err("12 is outside Int[0, 9]");
    let local = local.expect_err("12 is outside Int[0, 9]");
    // The refusal is the argument's own, on a node strictly below the call.
    assert!(
        !imported[0].location.path().is_empty(),
        "the refusal names the argument, not the whole body: {:?}",
        imported[0]
    );
    assert_eq!(imported[0].location.path(), local[0].location.path());
}

#[trace("TC-894", "FR-099-AC-8")]
#[test]
fn a_callee_body_obligation_is_discharged_by_the_library_not_the_call_site() {
    // The library whose body divides by a possibly-zero `x` is refused when it
    // is compiled, at the divisor, as `unproved-nonzero`.
    let refusals = admitted_library(
        "test/geometry",
        "profile law = \"filament.value.integer-division.euclidean/v1\";\n\
         function f using v(x: Int[0, 9]): Boolean pure { (10 div x) < 5 }\n",
        &["f"],
    )
    .expect_err("the library body has an unproved divisor");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        (refusals[0].cause.code(), refusals[0].cause.cause()),
        (Code::UndefinedExpression, Some("unproved-nonzero")),
        "{refusals:?}"
    );
    // With the divisor guarded the same library compiles, and the call site
    // `g::f(y)` with `y: Int[0, 9]` raises nothing of its own.
    let library = admitted_library(
        "test/geometry",
        "profile law = \"filament.value.integer-division.euclidean/v1\";\n\
         function f using v(x: Int[0, 9]): Boolean pure { if x = 0 then false else (10 div x) < 5 }\n",
        &["f"],
    )
    .expect("a guarded divisor compiles");
    compile(
        &format!(
            "{HEADER}import \"test/geometry\" as g;\n\
             function caller using v(y: Int[0, 9]): Boolean pure {{ g::f(y) }}\n"
        ),
        vec![library],
    )
    .expect("no obligation is raised at the call site");
}
