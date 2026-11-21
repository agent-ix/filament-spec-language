// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small authored ConfigVersion model for source planning requests.

const MODEL: &str = r#"language "ix:native" edition "1-draft";
domain "example/config-version";
type VersionNumber = Int[0, 3];
object ConfigVersion {
  versionNumber: VersionNumber;
  parent: ConfigVersion?;
  operation attemptUpdate(): Boolean modifies self.versionNumber;
}
population config_history: ConfigVersion;
"#;

/// Compile the planning model through the admitted model-unit input route.
/// The snapshot's two objects and parent reference use the same declaration
/// identities as the spine; their version domain is actually bounded to four
/// values, rather than filtering a larger domain with an invariant.
pub(crate) fn document() -> Vec<u8> {
    compile(MODEL)
}

/// Compile one explicitly authored finite planning model.
pub(crate) fn compile(model: &str) -> Vec<u8> {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("test", "planning-model"),
        "planning-model.native",
        model.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the authored planning model source is admitted");
    let packages = fsl_semantics::model::intake::package_input_from_sources([(
        "example/config-version",
        &source,
    )])
    .expect("the small planning model compiles to package input");
    packages["example/config-version"].clone()
}

/// Write the named finite planning model beside the genuine spine corpus.
pub(crate) fn write(directory: &std::path::Path) {
    super::config_version_spine::spine::write(
        directory,
        super::config_version_spine::Case::Healthy,
    )
    .expect("the spine supplies the starting object data");
    std::fs::write(
        directory.join("planning-model.semantic-ir.json"),
        document(),
    )
    .expect("the authored finite planning model is written under its own name");
    write_initial(directory, "planning-initial.json");
}

/// Make an initialization observation from the healthy spine's object data.
/// The corpus's handler snapshot and model document remain unchanged.
pub(crate) fn write_initial(directory: &std::path::Path, file: &str) {
    let bytes =
        std::fs::read(directory.join("spine-snapshot.json")).expect("the spine snapshot exists");
    let mut snapshot: serde_json::Value = serde_json::from_slice(&bytes).expect("a JSON snapshot");
    snapshot["anchor"] = serde_json::json!({"kind": "initialization", "name": "start"});
    std::fs::write(
        directory.join(file),
        serde_json::to_vec(&snapshot).expect("snapshot JSON"),
    )
    .expect("the initialization snapshot is written under its own name");
}
