// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 limit messages identify the source by its logical identity.

use fsl_cst::Limits;
use fsl_foundation::diagnostic::Locus;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"filament.value.complete/v1\";\n";

/// Trace: FR-255-AC-5, source loci label logical identity separately from file path.
#[trace("TC-422", "FR-255-AC-5")]
#[test]
fn limit_message_does_not_render_source_identity_as_a_path() {
    let source_text = format!(
        "{HEADER}function f using Complete (x: Integer): Boolean pure {{ {}true }}\n",
        "not ".repeat(20)
    );
    let error = fsl_cst::parse(
        SourceIdentity::new("issuer", "logical-document"),
        "/tmp/physical-source.native",
        source_text.as_bytes(),
        Limits::default().with_nodes(1),
    )
    .expect_err("the one-node ceiling must refuse the nested expression");
    let region = error
        .region
        .as_ref()
        .expect("the nested-expression limit has a source locus");
    let rendered = Locus::Region(region.clone()).to_string();
    assert!(
        rendered.contains("source identity `logical-document`"),
        "{rendered}"
    );
    assert!(!rendered.contains("physical-source.native"), "{rendered}");
}

/// Trace: FR-255-AC-4, source-byte refusals name the setting that raises the budget.
#[trace("TC-422", "FR-255-AC-4")]
#[test]
fn source_byte_limit_message_names_its_setting() {
    let error = fsl_cst::parse(
        SourceIdentity::new("issuer", "logical-document"),
        "physical-source.native",
        b"more than one byte",
        Limits::default().with_source_bytes(1),
    )
    .expect_err("the one-byte source ceiling must refuse");
    assert!(
        error.message.contains("s1.input_bytes"),
        "{}",
        error.message
    );
    assert!(error.message.contains("--limit"), "{}", error.message);
}
