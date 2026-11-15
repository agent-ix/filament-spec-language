// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-670-AC-7 and FR-110-AC-12: the editor's production header catalog
//! admits the three integer-division law identities.
use filament_spec_language::complete;
use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const LAWS: [&str; 3] = [
    "filament.value.integer-division.euclidean/v1",
    "filament.value.integer-division.floor/v1",
    "filament.value.integer-division.truncating/v1",
];

#[trace("TC-1782", "FR-670-AC-7", "FR-110-AC-12")]
#[test]
#[ignore = "QSL-1989: the header profile catalog holds only layer identities"]
fn the_header_profile_catalog_admits_each_integer_division_law() {
    let catalog = complete::header_profile_catalog();
    for law in LAWS {
        assert!(catalog.contains(law), "missing {law}");
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             profile d = \"{law}\";\n\
             record Reading {{ datum: Integer; }}\n"
        );
        let parsed = complete::parse_with_catalog(
            SourceIdentity::new("test", "test:law-editor"),
            "law.native",
            source.as_bytes(),
            &catalog,
            Limits::default(),
        )
        .expect("the source is readable");
        assert!(parsed.is_admissible(), "{law}: {:?}", parsed.diagnostics());
    }
    assert!(!catalog.contains("test:unknown-profile"));
}
