// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-680: accepting and ill-typed `.native` pairs for indexing, tuple
//! position reads, source maps, `Some`, quantity scaling and the equality-sum
//! rule, each run through S1 to S3 under `CheckMode::Linked`.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn check(name: &str, bytes: &[u8]) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("fsl-conformance", name),
        name,
        bytes,
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{name}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "conformance".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{name}: assembly succeeds: {refusal:?}"))
    .check(CheckMode::Linked, CheckingLimits::default())
}

/// The accepting case checks and the ill-typed case gives one refusal with
/// `code` and `cause`.
fn assert_pair(accepted: (&str, &[u8]), refused: (&str, &[u8]), code: Code, cause: &str) {
    if let Err(refusals) = check(accepted.0, accepted.1) {
        panic!("{} checks: {refusals:?}", accepted.0);
    }
    let refusals = check(refused.0, refused.1).expect_err("the ill-typed case is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("{}: one typed refusal: {refusals:?}", refused.0);
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

macro_rules! case {
    ($file:literal) => {
        (
            $file,
            include_bytes!(concat!("../../cases/", $file)).as_slice(),
        )
    };
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-31", "FSpec-FR-144-AC-32")]
#[test]
#[ignore = "QSL-2172: indexing a sequence and refusing a set index not implemented yet"]
fn index_pair() {
    assert_pair(
        case!("index-accepted.native"),
        case!("index-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1821", "FR-680-AC-2", "FSpec-FR-143-AC-22")]
#[test]
#[ignore = "QSL-2171: tuple position read p[n] not implemented yet"]
fn tuple_position_pair() {
    assert_pair(
        case!("tuple-position-accepted.native"),
        case!("tuple-position-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1822", "FR-680-AC-3", "FSpec-FR-144-AC-33", "FSpec-FR-149-AC-28")]
#[test]
#[ignore = "QSL-2171: source Map<K, V> type and map equality not implemented yet"]
fn map_pair() {
    assert_pair(
        case!("map-accepted.native"),
        case!("map-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
#[ignore = "QSL-2169: a Some literal payload adopting the expected element type not implemented yet"]
fn some_pair() {
    assert_pair(
        case!("some-accepted.native"),
        case!("some-ill-typed.native"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1824", "FR-680-AC-5", "FSpec-FR-142-AC-41", "FSpec-FR-142-AC-42")]
#[test]
#[ignore = "QSL-2168: quantity scaling by an exact number not implemented yet"]
fn scale_pair() {
    assert_pair(
        case!("scale-accepted.native"),
        case!("scale-ill-typed.native"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1825", "FR-680-AC-6", "FSpec-FR-149-AC-29")]
#[test]
fn eq_sum_pair() {
    assert_pair(
        case!("eq-sum-accepted.native"),
        case!("eq-sum-ill-typed.native"),
        Code::UndefinedExpression,
        "unproved-range",
    );
}
