// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-264 Behavior 5, FR-255 `emit.output_bytes` (TC-738, TC-721): the
//! caller-owned ceiling on one emission's canonical bytes, driven through
//! zero, `N - 1` and `N` for the package's measured length `N`.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::diagnostic::LimitKind;
use fsl_foundation::{Setting, SettingLimits};

use super::*;
use crate::EmitLimits;

/// The refusal an emission under `bound` returned, whose limit is the one
/// `emit.output_bytes` reached.
fn limit_of(refusal: EmitRefusal) -> fsl_foundation::diagnostic::LimitExceeded {
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert_eq!(refusal.cause(), Some("input-bytes-exceeded"));
    match refusal {
        EmitRefusal::OutputLimit(limit) => limit,
        other => panic!("expected the output limit, got {other:?}"),
    }
}

fn emitted_at(package: &CheckedPackage, bound: u64) -> Result<Emission, EmitRefusal> {
    crate::emit::emit_package(package, whole_unit, EmitLimits::new(bound))
}

const UNIT_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                            profile v = \"filament.value.complete/v1\";\n\
                            function f using v(): Boolean pure { ";

/// A pure Boolean function whose body is `depth` nested `not` expressions,
/// read and checked from source with front-end ceilings raised to fit.
fn nested_not_package(depth: usize) -> CheckedPackage {
    checked_unit(&format!("{UNIT_HEADER}{}true }}\n", "not ".repeat(depth)))
}

/// A pure Boolean function whose body is `depth` nested `let`s, which hold no
/// term and so carry no body-location row.
fn nested_let_package(depth: usize) -> CheckedPackage {
    let bindings: String = (0..depth)
        .map(|level| format!("let b{level} = true in "))
        .collect();
    checked_unit(&format!("{UNIT_HEADER}{bindings}true }}\n"))
}

/// `source` read and checked with front-end ceilings raised to fit it.
fn checked_unit(source: &str) -> CheckedPackage {
    let source_limits = fsl_cst::Limits {
        source_bytes: source.len(),
        tokens: source.len(),
        nodes: source.len().saturating_mul(16),
        ..fsl_cst::Limits::default()
    };
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "nested-not.native"),
        "nested-not.native",
        source.as_bytes(),
        source_limits,
    )
    .expect("nested-not source parses");
    assert!(parsed.is_admissible(), "nested-not source is admissible");
    let forms = fsl_forms::build_unit(&parsed).expect("nested-not source has forms");
    let declarations = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "nested-not".to_owned(),
        forms,
        Vec::new(),
        Vec::new(),
    )
    .expect("nested-not source assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::new(u64::MAX)
                .with_input_bytes(u64::MAX)
                .with_work_budget(u64::MAX),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("nested-not source checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

fn emitted_byte_count(depth: usize) -> usize {
    let package = nested_let_package(depth);
    let emission =
        crate::emit::emit_checked(&package, EmitLimits::new(u64::MAX)).unwrap_or_else(|refusal| {
            panic!("depth {depth} emits under a raised output limit: {refusal}")
        });
    emission.package().bytes().len()
}

/// The bound is inclusive: `N` bytes under bound `N` are emitted, byte for
/// byte as under a larger bound; `N - 1` and `0` refuse, naming the setting,
/// the configured bound and the first unpaid byte, and return no partial package.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_package_is_emitted_at_its_exact_length_and_refused_one_below_and_at_zero() {
    let package = package(vec![t()]);
    let larger = emitted_at(&package, u64::MAX).expect("an unbounded ceiling emits");
    let bytes = larger.package().bytes();
    let n = u64::try_from(bytes.len()).unwrap();
    assert!(n > 1, "the fixture package has bytes to bound");

    let exact = emitted_at(&package, n).expect("a ceiling of exactly N bytes emits");
    assert_eq!(exact.package().bytes(), bytes);

    for bound in [n - 1, 0] {
        let refusal = emitted_at(&package, bound).expect_err("a ceiling below N refuses");
        let limit = limit_of(refusal);
        assert_eq!(limit.setting(), Setting::EmitOutputBytes);
        assert_eq!(limit.kind(), LimitKind::InputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert_eq!(
            limit.actual(),
            u128::from(bound) + 1,
            "a fresh emission reports its first unpaid byte"
        );
        assert!(
            limit.to_string().contains("emit.output_bytes"),
            "the refusal names the setting that raises it: {limit}"
        );
    }
}

/// An emission remembered by an earlier call counts the same bytes against
/// the bound the later call supplies: refused below `N` with actual exactly
/// `bound + 1`, returned unchanged at `N`.
#[trace("TC-738", "FR-264-AC-7")]
#[test]
fn a_remembered_emission_is_held_to_the_bound_of_the_later_call() {
    let package = package(vec![t_read_from_text()]);
    let first =
        crate::emit::emit_checked(&package, EmitLimits::fixture()).expect("the package emits");
    let bytes = first.package().bytes().to_vec();
    let n = u64::try_from(bytes.len()).unwrap();

    let at = crate::emit::emit_checked(&package, EmitLimits::new(n))
        .expect("the remembered bytes fit N");
    assert_eq!(at.package().bytes(), bytes);

    for bound in [n - 1, 0] {
        let limit = limit_of(
            crate::emit::emit_checked(&package, EmitLimits::new(bound))
                .expect_err("N bytes exceed the bound"),
        );
        assert_eq!(limit.setting(), Setting::EmitOutputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert_eq!(limit.actual(), u128::from(bound) + 1);
    }
}

/// The dependency emissions a link makes run under the caller's ceiling: a
/// dependency whose bytes exceed it refuses the link with the output limit.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_link_refuses_a_dependency_whose_emission_exceeds_the_ceiling() {
    let refused = CheckedPackage::link_with(
        root_graph(),
        vec![import("test/units", dependency("test/units"))],
        EmitLimits::new(1),
    )
    .expect_err("the dependency does not fit one byte");
    let crate::LinkRefusal::DependencyEmission {
        identity,
        refusal: Some(refusal),
    } = refused
    else {
        panic!("expected a dependency emission refusal, got {refused:?}");
    };
    assert_eq!(identity, lib("test/units"));
    let limit = limit_of(*refusal);
    assert_eq!(limit.configured_bound(), 1);
    CheckedPackage::link_with(
        root_graph(),
        vec![import("test/units", dependency("test/units"))],
        EmitLimits::fixture(),
    )
    .expect("the same link under the fixture ceiling succeeds");
}

/// FR-255-AC-20: the setting names exactly the output-byte field, accepts
/// zero and `u64::MAX`, and no other setting changes it.
#[trace("TC-721", "FR-255-AC-20")]
#[test]
fn emit_output_bytes_sets_only_the_output_byte_bound() {
    let mut limits = EmitLimits::new(7);
    assert_eq!(limits.bounds(), vec![(Setting::EmitOutputBytes, 7)]);
    for bound in [0, u64::MAX] {
        assert!(limits.set_bound(Setting::EmitOutputBytes, bound));
        assert_eq!(limits.output_bytes, bound);
    }
    assert!(!limits.set_bound(Setting::I2InputBytes, 1));
    assert_eq!(limits.output_bytes, u64::MAX);
    assert_eq!(Setting::EmitOutputBytes.name(), "emit.output_bytes");
    assert_eq!(Setting::EmitOutputBytes.kind(), LimitKind::InputBytes);
    assert_eq!(EmitLimits::new(1).with_output_bytes(9).output_bytes, 9);
}

/// QSL-1985: canonical emission cost for a body that holds no term row scales
/// with its nested `let` count; the measured bytes per level stay within 5%.
/// A body of nested terms also writes a body-location row per term, each row
/// carrying a compact relative path (FR-531); the next test exercises its
/// measured output-byte boundary.
#[trace("FR-264-AC-6", "FR-255-AC-1")]
#[test]
fn nested_let_emission_bytes_are_linear_at_one_and_five_thousand_levels() {
    let at_one_thousand = emitted_byte_count(1_000);
    let at_five_thousand = emitted_byte_count(5_000);
    let per_let_at_one_thousand = at_one_thousand / 1_000;
    let per_let_at_five_thousand = at_five_thousand / 5_000;
    let difference = per_let_at_one_thousand.abs_diff(per_let_at_five_thousand);

    eprintln!(
        "measured canonical bytes per let: depth 1,000 => {per_let_at_one_thousand}, depth 5,000 => {per_let_at_five_thousand}"
    );
    assert!(
        per_let_at_one_thousand > 0,
        "the emitted package is nonempty"
    );
    assert!(
        difference * 100 <= per_let_at_one_thousand.max(per_let_at_five_thousand) * 5,
        "bytes per let should remain linear: 1,000 => {at_one_thousand} bytes ({per_let_at_one_thousand}/let), 5,000 => {at_five_thousand} bytes ({per_let_at_five_thousand}/let)"
    );
}

/// FR-264: relative body locations keep 4,500 nested `not`s below the default
/// ceiling. An explicit ceiling one byte below their output still refuses at
/// a source occurrence, and raising that named ceiling admits the same bytes.
#[trace("FR-264-AC-6", "FR-255-AC-4")]
#[test]
fn a_deep_nested_not_names_and_obeys_the_output_byte_limit() {
    const DEFAULT_OUTPUT_BYTES: u64 = 16_777_216;
    let shallow = nested_not_package(100);
    crate::emit::emit_checked(&shallow, EmitLimits::new(DEFAULT_OUTPUT_BYTES))
        .expect("100 nested nots emit under the default output limit");

    let deep = nested_not_package(4_500);
    let admitted = crate::emit::emit_checked(&deep, EmitLimits::new(DEFAULT_OUTPUT_BYTES))
        .expect("relative body locations fit the default output ceiling");
    let bytes = u64::try_from(admitted.package().bytes().len()).expect("a fixture length");
    let refusal = crate::emit::emit_checked(
        &deep,
        EmitLimits::new(bytes.checked_sub(1).expect("a nonempty package")),
    )
    .expect_err("one byte below the emitted package refuses");
    let EmitRefusal::OutputLimit(limit) = &refusal else {
        panic!("expected emit.output_bytes refusal, got {refusal:?}");
    };
    assert_eq!(limit.setting(), Setting::EmitOutputBytes);
    assert_eq!(limit.kind(), LimitKind::InputBytes);
    assert!(
        limit.locus().is_some(),
        "the emit refusal identifies a source occurrence"
    );
    let message = refusal.to_string();
    assert!(message.contains("output byte limit"), "{message}");
    assert!(message.contains("emit.output_bytes"), "{message}");

    let exact = crate::emit::emit_checked(&deep, EmitLimits::new(bytes))
        .expect("raising emit.output_bytes admits the 4,500-level package");
    assert_eq!(exact.package().bytes(), admitted.package().bytes());
}

/// FR-264-AC-6 over a body that writes body-location rows: a package of `N`
/// canonical bytes is emitted at bound `N`, byte for byte as under a larger
/// bound, so the byte charge of its rows never exceeds the bytes written; at
/// `N - 1` it is refused.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_package_with_body_location_rows_is_emitted_at_its_exact_length() {
    let package = nested_not_package(300);
    let larger = crate::emit::emit_checked(&package, EmitLimits::new(u64::MAX))
        .expect("an unbounded ceiling emits");
    let rows = serde_json::from_slice::<serde_json::Value>(larger.package().bytes())
        .expect("the package is JSON")["checked_term_data"]["body_locations"]
        .as_array()
        .map_or(0, Vec::len);
    assert!(rows >= 300, "the body writes a row per term: {rows}");
    let n = u64::try_from(larger.package().bytes().len()).unwrap();

    let exact = crate::emit::emit_checked(&package, EmitLimits::new(n))
        .expect("a ceiling of exactly N bytes emits");
    assert_eq!(exact.package().bytes(), larger.package().bytes());
    let limit = limit_of(
        crate::emit::emit_checked(&package, EmitLimits::new(n - 1))
            .expect_err("a ceiling below N refuses"),
    );
    assert_eq!(limit.configured_bound(), n - 1);
    assert!(limit.actual() > u128::from(n - 1));
}

/// QSL-2849: the source map is written verbatim, so a bound below its bytes
/// refuses before any typed row is built. Allocations are counted, not time: a
/// refusal the source map causes (bound N/10) allocates under two thirds of what a
/// refusal at the final encode (bound `N - 1`) does; the first unpaid byte is
/// exactly `bound + 1` in both.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_bound_below_the_source_map_refuses_before_the_typed_rows_are_built() {
    // A `let` chain has no body-location rows, which main already meters per
    // row, so only the source-map charge can refuse early here.
    let package = nested_let_package(1_000);
    let bytes = u64::try_from(emitted_byte_count_of(&package)).expect("a fixture length");

    let allocations = |bound: u64| {
        let mut outcome = None;
        let info = allocation_counter::measure(|| {
            outcome = Some(emitted_at(&package, bound));
        });
        let refusal = outcome
            .expect("emission ran")
            .expect_err("a bound below N refuses");
        let limit = limit_of(refusal);
        assert_eq!(limit.configured_bound(), bound);
        assert_eq!(limit.actual(), u128::from(bound) + 1);
        info.count_total
    };
    // Initialize the measurement machinery before observing the emitter.
    let _ = allocation_counter::measure(|| {});

    let early = allocations(bytes / 10);
    let late = allocations(bytes - 1);
    assert!(
        early.saturating_mul(3) < late.saturating_mul(2),
        "an early refusal skips the typed rows: {early} against {late}"
    );
}

/// QSL-2849: typed rows are charged as each section is built. A package whose
/// source map fits but whose claim-name rows overflow refuses before the
/// sections after them are built (allocations are counted, not time), with the
/// first unpaid byte exactly `bound + 1`.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_bound_below_the_typed_rows_refuses_before_the_later_rows_are_built() {
    let claims: Vec<String> = (0..150)
        .map(|index| super::probabilistic::claim(&format!("C{index}_{}", "x".repeat(2_000)), "0.5"))
        .collect();
    let refs: Vec<&str> = claims.iter().map(String::as_str).collect();
    let package = super::probabilistic::claims_package(&refs);
    let emit = |bound: u64| {
        crate::emit::emit_checked_uncached(&package, &Cancel::new(), EmitLimits::new(bound))
    };
    let bytes = u64::try_from(
        emit(u64::MAX)
            .expect("a raised output limit emits")
            .package()
            .bytes()
            .len(),
    )
    .expect("a fixture length");

    let allocations = |bound: u64| {
        let mut outcome = None;
        let info = allocation_counter::measure(|| {
            outcome = Some(emit(bound));
        });
        let limit = limit_of(
            outcome
                .expect("emission ran")
                .expect_err("a bound below N refuses"),
        );
        assert_eq!(limit.configured_bound(), bound);
        assert_eq!(limit.actual(), u128::from(bound) + 1);
        info.count_total
    };
    let _ = allocation_counter::measure(|| {});
    let wire: serde_json::Value =
        serde_json::from_slice(emit(u64::MAX).expect("emits").package().bytes())
            .expect("the package is JSON");
    let section = |name: &str| serde_json::to_vec(&wire[name]).expect("a section").len() as u64;
    let (map, names) = (section("source_map"), section("claim_names"));
    assert!(map < names, "the claim-name rows are the larger section");
    // A bound the source map fits under and the claim-name rows overflow.
    let bound = map + names / 2;
    let early = allocations(bound);
    let late = allocations(bytes - 1);
    assert!(
        early.saturating_mul(4) < late.saturating_mul(3),
        "refusing at the claim-name rows skips the rows after them: {early} against {late}"
    );
}

fn emitted_byte_count_of(package: &CheckedPackage) -> usize {
    emitted_at(package, u64::MAX)
        .expect("a raised output limit emits")
        .package()
        .bytes()
        .len()
}
