// SPDX-License-Identifier: AGPL-3.0-or-later
//! A package the strict reader refuses surfaces as the reader's own typed
//! refusal, never as a fault of a later step.

use super::*;
use crate::PackageAdmissionRefusal;

/// The emission of a package with one function, one member of whose wire
/// is replaced by an unknown member the reader refuses.
fn corrupted() -> (CheckedPackage, Emission) {
    let package = CheckedPackage::link(converted_equality::checked_graph(
        "function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }",
    ));
    let mut emission = emit(&package);
    let mut wire: Value = serde_json::from_slice(emission.package().bytes()).expect("wire");
    wire.as_object_mut()
        .expect("object")
        .insert("zz_unknown".to_owned(), json!(1));
    let bytes = serde_json::to_vec(&wire).expect("bytes");
    let name = emission.package().name().to_owned();
    emission.package = EmittedPackage::new(&name, |_| Ok::<_, ()>(bytes)).expect("package");
    (package, emission)
}

#[test]
#[trace("FR-286-AC-2")]
fn a_refused_read_carries_the_readers_code_cause_and_pointer() {
    let (package, emission) = corrupted();
    let refusal = crate::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &BTreeMap::new(),
        V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::SourceWitnessSupplier::new(&emission),
    )
    .err()
    .expect("the reader refuses the corrupted package");
    let typed = PackageAdmissionRefusal::of_import_view(&refusal, None).expect("a reader refusal");
    assert_eq!(typed.code, Code::InvalidPackage);
    assert_eq!(typed.cause, None);
    assert_eq!(typed.pointer.as_deref(), Some("/zz_unknown"));
}

#[test]
#[trace("FR-286-AC-2")]
fn a_refusal_spells_the_readers_cause_or_its_codes_default() {
    let refusal = |code, cause: Option<&str>| PackageAdmissionRefusal {
        limit: None,
        code,
        cause: cause.map(str::to_owned),
        pointer: Some("/claim_names/2".to_owned()),
        identity: None,
    };
    assert_eq!(
        refusal(Code::InvalidPackage, Some("missing-member")).catalog_cause(),
        "missing-member"
    );
    assert_eq!(
        refusal(Code::InvalidPackage, None).catalog_cause(),
        "invalid-value"
    );
    assert_eq!(
        refusal(Code::ResourceExhausted, None).catalog_cause(),
        "insufficient-next-charge"
    );
    assert_eq!(
        refusal(Code::MissingImport, None).catalog_cause(),
        "missing-selection"
    );
}
