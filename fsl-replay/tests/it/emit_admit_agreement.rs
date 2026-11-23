// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2062: what the checker accepts and the emitter writes, the strict package reader
//! (IR's `read_checked_package`) admits.
//!
//! The ticket names `fsl-package/tests/emit_admit_agreement.rs`; the spine that takes a
//! source through S1 to S4 lives in `fsl-replay`, which sits above `fsl-package`, so the
//! suite lives here.
//!
//! Exclusions (generator forms the suite leaves out until the ticket fixing each lands):
//! QSL-1142 (a FIFO channel with a value-type key, reader rule `equality_stable`) and
//! QSL-1903 (a protocol capture initializer with a divisor that may be zero, FR-381).
//! QSL-1137 (bounded channel payloads) and QSL-1136 (a role on a whole model) are Done and
//! are not excluded; the typed units this suite generates are value functions, which reach
//! none of the four forms, so the four named per-form cases (`bounded_channel_payloads_agree`,
//! `value_key_fifo_agrees`, `unproved_divisor_in_capture_is_refused_by_both`,
//! `whole_model_role_agrees`) need protocol and model unit generators that QSL-2054 does not
//! provide and are not written here.
use std::collections::BTreeMap;

use filament_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageReadLimits,
};
use filament_exact::Cancel;
use fsl_bench::prop::{self, near_miss, unit};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    self, DependencyInput, EmittedUnit, FrontEndFailure, ParseRequest, PackageLimits, SpineLimits,
};
use proptest::prelude::*;

/// The generator forms left out, each with the ticket it waits on.
const KNOWN_EXCLUSIONS: &[&str] = &["QSL-1142", "QSL-1903"];

fn emit(source: &str) -> Result<EmittedUnit, FrontEndFailure> {
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("agent-ix", "emit-admit");
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "generated.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )?
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)?.into_value();
    let checked = spine::check(
        &parsed,
        "emit-admit".into(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )?
    .into_value();
    Ok(spine::package(&checked, PackageLimits::default(), &cancel)?.into_value())
}

fn read(emitted: &EmittedUnit) -> CheckedPackageDispatchResult {
    read_checked_package(
        emitted.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        emitted.evidence(),
    )
}

/// Provenance: QSL-2062 P1. A unit the checker accepts is emitted and the strict reader admits it.
#[test]
fn checker_accepted_units_are_emitted_and_admitted() {
    prop::run(
        "checker_accepted_units_are_emitted_and_admitted",
        prop::config(256),
        &unit::package_units(),
        |input| {
            let source = input.source();
            let emitted = emit(&source)
                .map_err(|error| TestCaseError::fail(format!("checker or emitter: {error:?}\n{source}")))?;
            match read(&emitted) {
                CheckedPackageDispatchResult::AdmittedV2(_) => Ok(()),
                other => Err(TestCaseError::fail(format!("reader: {other:?}\n{source}"))),
            }
        },
    )
    .unwrap();
}

/// Provenance: QSL-2062 P3. Emitting the same unit twice gives the same bytes and artifact
/// identity, and reading either gives the same admitted package.
#[test]
fn emitted_package_reread_has_the_same_identity() {
    prop::run(
        "emitted_package_reread_has_the_same_identity",
        prop::config(256),
        &unit::package_units(),
        |input| {
            let source = input.source();
            let first = emit(&source).map_err(|e| TestCaseError::fail(format!("{e:?}\n{source}")))?;
            let second = emit(&source).map_err(|e| TestCaseError::fail(format!("{e:?}\n{source}")))?;
            prop_assert_eq!(first.package().bytes(), second.package().bytes());
            prop_assert_eq!(
                format!("{:?}", first.emission().artifact_identity()),
                format!("{:?}", second.emission().artifact_identity())
            );
            prop_assert_eq!(format!("{:?}", read(&first)), format!("{:?}", read(&second)), "{}", source);
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2062 P2 (value forms). For a unit the checker refuses, nothing is emitted
/// and the refusal carries a catalog code; the strict reader is never reached with it.
#[test]
fn refused_units_are_not_emitted() {
    prop::run(
        "refused_units_are_not_emitted",
        prop::config(256),
        &near_miss::near_misses(),
        |input| {
            match emit(&input.source) {
                Ok(_) => Err(TestCaseError::fail(format!("emitted a refused unit:\n{}", input.source))),
                Err(failure) => {
                    let refusal = failure.into_refused().map_err(|stopped| {
                        TestCaseError::fail(format!("not a typed refusal: {stopped:?}"))
                    })?;
                    prop_assert_eq!(refusal.code(), input.code);
                    Ok(())
                }
            }
        },
    )
    .unwrap();
}

/// Provenance: QSL-2062 negative. A package with one byte changed is never read as an
/// admitted package unless the reader still finds it valid; every refusal is a typed result.
#[test]
fn corrupted_package_is_refused() {
    let source = unit::package_units()
        .new_tree(&mut proptest::test_runner::TestRunner::new(prop::config(1)))
        .unwrap();
    let source = proptest::strategy::ValueTree::current(&source).source();
    let emitted = emit(&source).expect("emits");
    let bytes = emitted.package().bytes().to_vec();
    let (mut refused, mut admitted) = (0, 0);
    for at in (0..bytes.len()).step_by((bytes.len() / 64).max(1)) {
        let mut corrupt = bytes.clone();
        corrupt[at] = corrupt[at].wrapping_add(1);
        match read_checked_package(&corrupt, CheckedPackageReadLimits::bounded(), emitted.evidence()) {
            CheckedPackageDispatchResult::AdmittedV2(_) => admitted += 1,
            _ => refused += 1,
        }
    }
    eprintln!("corrupted packages: {refused} refused, {admitted} still admitted");
    assert!(refused > admitted, "the reader admitted most corrupted packages");
    // Truncation is always refused.
    assert!(!matches!(
        read_checked_package(&bytes[..bytes.len() / 2], CheckedPackageReadLimits::bounded(), emitted.evidence()),
        CheckedPackageDispatchResult::AdmittedV2(_)
    ));
}

/// Provenance: QSL-2062. The exclusions are exactly the open tickets.
#[test]
fn known_exclusions_are_exactly_the_open_tickets() {
    assert_eq!(KNOWN_EXCLUSIONS, ["QSL-1142", "QSL-1903"]);
}
