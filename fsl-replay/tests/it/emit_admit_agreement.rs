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
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageReadLimits, CheckedPackageV2,
};
use filament_exact::Cancel;
use fsl_bench::prop::{self, near_miss, unit};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    self, DependencyInput, EmittedUnit, FrontEndFailure, PackageLimits, ParseRequest, SpineLimits,
};
use proptest::prelude::*;

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
            let emitted = emit(&source).map_err(|error| {
                TestCaseError::fail(format!("checker or emitter: {error:?}\n{source}"))
            })?;
            match read(&emitted) {
                CheckedPackageDispatchResult::AdmittedV2(_) => Ok(()),
                other => Err(TestCaseError::fail(format!("reader: {other:?}\n{source}"))),
            }
        },
    )
    .unwrap();
}

/// The package the strict reader admits from `emitted`'s bytes.
fn reread(emitted: &EmittedUnit) -> Result<Box<CheckedPackageV2>, TestCaseError> {
    match read(emitted) {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
        other => Err(TestCaseError::fail(format!("reader: {other:?}"))),
    }
}

/// Provenance: QSL-2062 P3. Emitting the same unit twice gives the same bytes and artifact
/// identity, and reading those bytes back through the strict reader (three reads: each
/// emission once, the first again) gives the same admitted package: its name is the one the
/// checker was given and its whole read-back value is equal.
#[test]
fn emitted_package_reread_has_the_same_identity() {
    prop::run(
        "emitted_package_reread_has_the_same_identity",
        prop::config(256),
        &unit::package_units(),
        |input| {
            let source = input.source();
            let first =
                emit(&source).map_err(|e| TestCaseError::fail(format!("{e:?}\n{source}")))?;
            let second =
                emit(&source).map_err(|e| TestCaseError::fail(format!("{e:?}\n{source}")))?;
            prop_assert_eq!(first.package().bytes(), second.package().bytes());
            prop_assert_eq!(
                format!("{:?}", first.emission().artifact_identity()),
                format!("{:?}", second.emission().artifact_identity())
            );
            let (a, b, again) = (reread(&first)?, reread(&second)?, reread(&first)?);
            prop_assert_eq!(a.name(), "emit-admit", "{}", source);
            prop_assert_eq!(a.name(), b.name());
            prop_assert_eq!(a.node_kinds(), b.node_kinds());
            prop_assert_eq!(format!("{a:?}"), format!("{b:?}"), "{}", source);
            prop_assert_eq!(format!("{a:?}"), format!("{again:?}"), "{}", source);
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
        |input| match emit(&input.source) {
            Ok(_) => Err(TestCaseError::fail(format!(
                "emitted a refused unit:\n{}",
                input.source
            ))),
            Err(failure) => {
                let refusal = failure.into_refused().map_err(|stopped| {
                    TestCaseError::fail(format!("not a typed refusal: {stopped:?}"))
                })?;
                prop_assert_eq!(refusal.code().as_str(), input.code);
                Ok(())
            }
        },
    )
    .unwrap();
}

/// Provenance: QSL-2062 negative. A package with one byte changed is refused by the strict
/// reader, or admitted as the very same package. The corruption adds one to a byte at 64
/// evenly spaced positions of a generated unit's package; a corrupted package the reader
/// admits must read back equal to the original (the change was not semantic). A corrupted
/// package that is admitted and differs is a defect: the reader would accept a package other
/// than the one emitted. "Equal" is the `Debug` of the admitted package, which may omit fields
/// the reader keeps opaque: the admitted-although-corrupted positions are printed so that the
/// bytes can be judged (at the time of writing: a rule identity digit and a source-map region
/// end, which the reader does not cross-check).
#[test]
fn corrupted_package_is_refused() {
    let source = unit::package_units()
        .new_tree(&mut proptest::test_runner::TestRunner::new(prop::config(1)))
        .unwrap();
    let source = proptest::strategy::ValueTree::current(&source).source();
    let emitted = emit(&source).expect("emits");
    let bytes = emitted.package().bytes().to_vec();
    let original = format!(
        "{:?}",
        reread(&emitted).expect("the emitted package is admitted")
    );
    let (mut refused, mut same) = (0, 0);
    let mut different = Vec::new();
    for at in (0..bytes.len()).step_by((bytes.len() / 64).max(1)) {
        let mut corrupt = bytes.clone();
        corrupt[at] = corrupt[at].wrapping_add(1);
        match read_checked_package(
            &corrupt,
            CheckedPackageReadLimits::bounded(),
            emitted.evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => {
                if format!("{package:?}") == original {
                    same += 1;
                    eprintln!("admitted although corrupted, reads back equal, at {at}: {:?} -> {:?} in {:?}", bytes[at] as char, corrupt[at] as char, String::from_utf8_lossy(&bytes[at.saturating_sub(40)..(at + 40).min(bytes.len())]));
                } else {
                    different.push((at, corrupt));
                }
            }
            _ => refused += 1,
        }
    }
    eprintln!(
        "corrupted packages: {refused} refused, {same} admitted unchanged, {} admitted different",
        different.len()
    );
    assert!(refused > 0, "no corruption was refused");
    assert!(
        different.is_empty(),
        "the reader admitted a different package for a corrupted input at byte {:?}:\n{}",
        different.first().map(|(at, _)| at),
        different
            .first()
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default()
    );
    // QSL-2947: a changed source-map region end is INTENTIONALLY admitted by the strict
    // reader. contract-ir FR-038 (#1149) makes source-map rows retained package data: the
    // reader checks their wire shape and joins but SHALL NOT claim a region is the exact
    // span of the named syntax, because the package does not carry the source bytes; the
    // emitter verifies regions (FSpec FR-203). The admitted package does retain the changed
    // end, so `Debug` equality hides it: the retained ends are compared here. Positions are
    // located by content, not by offset.
    let ends_of = |package: &CheckedPackageV2| -> Vec<u64> {
        package
            .source_map()
            .iter()
            .flat_map(|entry| entry.regions.iter().map(|region| region.end))
            .collect()
    };
    let original_ends = match read_checked_package(
        &bytes,
        CheckedPackageReadLimits::bounded(),
        emitted.evidence(),
    ) {
        CheckedPackageDispatchResult::AdmittedV2(package) => ends_of(&package),
        _ => panic!("the emitted package is admitted"),
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let map_at = text
        .find("\"source_map\"")
        .expect("the emitted package carries a source map");
    let mut admitted_with_changed_end = 0;
    for (found, _) in text[map_at..].match_indices("\"end\":") {
        let mut digit = map_at + found + "\"end\":".len();
        if !bytes[digit].is_ascii_digit() {
            continue;
        }
        while bytes[digit + 1].is_ascii_digit() {
            digit += 1;
        }
        let mut corrupt = bytes.clone();
        corrupt[digit] = if corrupt[digit] == b'9' {
            b'8'
        } else {
            corrupt[digit] + 1
        };
        if let CheckedPackageDispatchResult::AdmittedV2(package) = read_checked_package(
            &corrupt,
            CheckedPackageReadLimits::bounded(),
            emitted.evidence(),
        ) {
            assert_ne!(
                ends_of(&package),
                original_ends,
                "an admitted source-map end change at byte {digit} is retained"
            );
            admitted_with_changed_end += 1;
        }
    }
    assert!(
        admitted_with_changed_end > 0,
        "FR-038: a changed source-map region end is admitted and retained"
    );
    // A rule identity in `lock.definition_selections` is an opaque `DefinitionRef`
    // (FR-110: `{authority, identity}` rows; no FR makes the strict reader check an
    // identity against a registry), so a changed identity character is INTENTIONALLY
    // admitted. FR gap: no requirement says whether the reader must refuse an unknown
    // definition identity (QSL-3106).
    let lock_at = text
        .find("\"definition_selections\"")
        .expect("the lock carries definition selections");
    let rule = text[lock_at..]
        .find("filament.rule.fr-")
        .map(|found| lock_at + found + "filament.rule.fr-".len())
        .expect("a rule definition is selected");
    let mut corrupt = bytes.clone();
    corrupt[rule] = if corrupt[rule] == b'9' {
        b'8'
    } else {
        corrupt[rule] + 1
    };
    assert!(
        matches!(
            read_checked_package(
                &corrupt,
                CheckedPackageReadLimits::bounded(),
                emitted.evidence()
            ),
            CheckedPackageDispatchResult::AdmittedV2(_)
        ),
        "a changed rule identity is an opaque DefinitionRef and is admitted"
    );
    // Truncation is always refused.
    assert!(!matches!(
        read_checked_package(
            &bytes[..bytes.len() / 2],
            CheckedPackageReadLimits::bounded(),
            emitted.evidence()
        ),
        CheckedPackageDispatchResult::AdmittedV2(_)
    ));
}
