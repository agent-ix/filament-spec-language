// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-390 (FR-090-AC-9) and TC-386 step 3 (FR-090-AC-5): `FamilyOutcome`,
//! `FamilyResult` and `EvalOutcome` are defined once, in the layer-E leaf
//! `filament-eval` (FR-401), so no FSL crate defines them; no module below the
//! check core names them;
//! the core names no family cause type; no crate below layer 3 depends on
//! this crate; and `fsl-semantics`, layer 3 itself, depends on layers 2, F,
//! SV and K only. Uses the resolved-import and definition scans of TC-256, TC-170 and
//! TC-176 (`xtask::import_graph`, `xtask::definition_scan`), so a
//! fully-qualified inline path is caught as well as a `use` line.

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

const CORE_TYPES: [&str; 3] = ["FamilyOutcome", "FamilyResult", "EvalOutcome"];

const QUALIFIED_CORE_CRATES: [&str; 9] = [
    "fsl-foundation",
    "fsl-cst",
    "fsl-source",
    "fsl-forms",
    "fsl-semantics",
    "fsl-package",
    "fsl-route",
    "fsl-eval",
    "fsl-replay",
];

/// The family cause types, which the `check` core holds only through
/// `CatalogCoded`/`UndefinedCoded`.
const CAUSE_TYPES: [&str; 4] = [
    "ProtocolClauseSnapshot",
    "ModelRefusal",
    "ModelQueryRefusal",
    "StateModelUndefined",
];

/// The modules ordered below the `check` core (ADR-011 §6.1, §6.2): layer 1
/// `fsl-cst`, layer 2 `fsl-forms`, and the layer-3 `semantic_value`, `model`
/// and `library` modules, plus the layer-3 `value` modules that sit beside the
/// evaluator (`value::model_query`) and the type environment
/// (`value::environment_stage`). The layer-SV crate `filament-semantic-value` is
/// its own repository and is not scanned here.
const BELOW_CORE: [&str; 9] = [
    "fsl-cst/src",
    "fsl-forms/src",
    "fsl-semantics/src/model",
    "fsl-semantics/src/library",
    "fsl-semantics/src/value/definition.rs",
    "fsl-semantics/src/value/enumeration.rs",
    "fsl-semantics/src/value/unit.rs",
    "fsl-semantics/src/value/model_query.rs",
    "fsl-semantics/src/value/environment_stage.rs",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Step 1 (FR-401-AC-3): the one definition of each type is `filament-eval`'s,
/// so no FSL crate defines it, `fsl-semantics/src/family/` included.
#[trace("FR-090-AC-9", "TC-390", "FR-401-AC-3")]
#[test]
fn family_outcome_types_are_defined_in_no_fsl_crate() {
    let definitions = xtask::definition_scan::scan_dirs(
        &workspace_root(),
        &[
            "src",
            "fsl-foundation/src",
            "fsl-cst/src",
            "fsl-forms/src",
            "fsl-semantics/src",
            "fsl-package/src",
            "fsl-eval/src",
            "fsl-replay/src",
        ],
    )
    .expect("the definition scan runs cleanly");
    for name in CORE_TYPES {
        let locations = definitions.items.get(name).cloned().unwrap_or_default();
        assert!(
            locations.is_empty(),
            "{name} is defined in filament-eval only, found in the workspace: {locations:?}"
        );
    }
}

/// Step 2: no `use` edge or inline path below the core names one of the
/// three types, or globs the `family` module.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn no_module_below_the_check_core_names_a_family_outcome_type() {
    for root in BELOW_CORE {
        assert!(
            workspace_root().join(root).exists(),
            "{root} does not exist"
        );
    }
    let paths = xtask::import_graph::resolved_paths(&workspace_root(), &BELOW_CORE)
        .expect("the resolved-import scan runs cleanly");
    assert!(!paths.is_empty(), "the scan found no paths at all");
    let offending: Vec<_> = paths
        .iter()
        .filter(|path| {
            path.segments
                .iter()
                .any(|segment| CORE_TYPES.contains(&segment.as_str()))
                || (path.is_glob && path.segments.last().is_some_and(|last| last == "family"))
        })
        .collect();
    assert!(
        offending.is_empty(),
        "a module below the check core names a family outcome type: {offending:#?}"
    );
}

/// Step 3: no `use` edge or inline path under the core names a family
/// cause type.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn the_check_core_names_no_family_cause_type() {
    let paths =
        xtask::import_graph::resolved_paths(&workspace_root(), &["fsl-semantics/src/family"])
            .expect("the resolved-import scan runs cleanly");
    assert!(
        !paths.is_empty(),
        "the scan found no paths under fsl-semantics/src/family"
    );
    let offending: Vec<_> = paths
        .iter()
        .filter(|path| {
            path.segments
                .iter()
                .any(|segment| CAUSE_TYPES.contains(&segment.as_str()))
        })
        .collect();
    assert!(
        offending.is_empty(),
        "the check core names a family cause type: {offending:#?}"
    );
}

/// One workspace package's normal (`[dependencies]`), dev
/// (`[dev-dependencies]`) and build (`[build-dependencies]`) dependency
/// names.
struct PackageDependencies {
    name: String,
    normal: Vec<String>,
    dev: Vec<String>,
    build: Vec<String>,
    /// Each normal (and build) dependency with the features it enables.
    shipped_features: Vec<(String, Vec<String>)>,
    /// The tables (`[dependencies]`, `[dev-dependencies]`,
    /// `[build-dependencies]`) in which the package enables `filament-checked`'s
    /// `construct` feature.
    construct_tables: Vec<&'static str>,
    /// The package's own `default` feature list.
    default_features: Vec<String>,
    /// Every one of the package's own `[features]` entries, `default`
    /// included, with the values it enables.
    features: Vec<(String, Vec<String>)>,
}

/// Every workspace package's dependency names, from `cargo metadata`, so
/// `x.workspace = true` keys, `[dependencies.x]` tables and renamed
/// dependencies resolve to the real package name like any other entry.
fn workspace_dependencies() -> Vec<PackageDependencies> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(workspace_root())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    metadata["packages"]
        .as_array()
        .expect("cargo metadata lists packages")
        .iter()
        .map(|package| {
            let dependencies = package["dependencies"]
                .as_array()
                .expect("a dependency list");
            let names_of_kind = |kind: Option<&str>| -> Vec<String> {
                dependencies
                    .iter()
                    .filter(|dependency| dependency["kind"].as_str() == kind)
                    .map(|dependency| {
                        dependency["name"]
                            .as_str()
                            .expect("a dependency name")
                            .to_owned()
                    })
                    .collect()
            };
            let strings = |value: &serde_json::Value| -> Vec<String> {
                value
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_str().expect("a feature name").to_owned())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            PackageDependencies {
                name: package["name"].as_str().expect("a package name").to_owned(),
                normal: names_of_kind(None),
                dev: names_of_kind(Some("dev")),
                build: names_of_kind(Some("build")),
                shipped_features: dependencies
                    .iter()
                    .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
                    .map(|dependency| {
                        (
                            dependency["name"]
                                .as_str()
                                .expect("a dependency name")
                                .to_owned(),
                            strings(&dependency["features"]),
                        )
                    })
                    .collect(),
                construct_tables: dependencies
                    .iter()
                    .filter(|dependency| dependency["name"].as_str() == Some("filament-checked"))
                    .filter(|dependency| {
                        strings(&dependency["features"])
                            .iter()
                            .any(|feature| feature == "construct")
                    })
                    .map(|dependency| match dependency["kind"].as_str() {
                        Some("dev") => "[dev-dependencies]",
                        Some("build") => "[build-dependencies]",
                        _ => "[dependencies]",
                    })
                    .collect(),
                default_features: strings(&package["features"]["default"]),
                features: package["features"]
                    .as_object()
                    .expect("a feature table")
                    .iter()
                    .map(|(name, values)| (name.clone(), strings(values)))
                    .collect(),
            }
        })
        .collect()
}

/// Step 4 of TC-390, step 3 of TC-386 and TC-398's crate edges: each crate
/// below layer 3 depends only on the workspace crates below it, in its
/// `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]` alike
/// (a build dependency on a higher layer or on this crate fails as a normal
/// one does). `fsl-semantics`
/// (layer 3) names exactly `fsl-forms` and `fsl-foundation` among
/// the workspace crates in `[dependencies]`, never `fsl-cst`; its `[dev-dependencies]` may name
/// `fsl-cst` so the FR-091
/// assembler's tests can run S1 (FR-091-AC-20), and only
/// `model::intake` names its FCD dependencies. `filament-exact` and
/// `filament-semantic-value` are external git dependencies of their own
/// repositories, so no workspace check reaches them; `fsl-foundation` may name `filament-exact`,
/// `fsl-cst` may name
/// `fsl-foundation` and `filament-exact`, and `fsl-forms` names exactly
/// `ix-trace-rs`, `fsl-cst`, `fsl-foundation`, `filament-exact`,
/// `filament-semantic-value` and `filament-walk` in `[dependencies]` (ADR-011
/// §6.1). `fsl-package` (layer 4)
/// names exactly `fsl-foundation`, `fsl-semantics`, `filament-exact` and
/// `filament-semantic-value` among the workspace crates in `[dependencies]`; its
/// `[dev-dependencies]` may also name the lower layers `fsl-forms` and
/// `fsl-cst` (the FR-091 S1-to-S4 round trip). `fsl-route` (layer R)
/// names exactly `fsl-foundation`, `fsl-semantics` and `thiserror`
/// in `[dependencies]`, has no `[build-dependencies]`, and its
/// `[dev-dependencies]` may also name layer K's `filament-exact`. `fsl-eval` (layer
/// 5) names exactly `fsl-foundation`, `fsl-package`,
/// `fsl-semantics`, `filament-exact`, `filament-semantic-value` and `thiserror` in
/// `[dependencies]`, has no `[build-dependencies]`, and its `[dev-dependencies]` may also name the lower layers
/// `fsl-forms` and `fsl-cst` (FR-091-AC-13's call of a function compiled
/// from source; `serde_json` moved here too, once its one shipped
/// caller -- the deleted v2 function-package codec -- was deleted). Every other
/// workspace crate, `fsl-source` and this crate included, is refused. Cargo already
/// refuses a normal-dependency cycle back to this crate, but accepts a
/// dev-dependency one, so the dev table is checked here.
#[trace(
    "FR-090-AC-9",
    "TC-390",
    "FR-090-AC-5",
    "TC-386",
    "FR-091-AC-11",
    "TC-398",
    "FR-068-AC-3",
    "TC-172"
)]
#[test]
fn no_crate_below_layer_three_depends_on_the_check_core() {
    let packages = workspace_dependencies();
    let workspace_crates: Vec<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    for required in [
        "filament-spec-language",
        "fsl-source",
        "fsl-cst",
        "fsl-forms",
        "fsl-semantics",
        "fsl-package",
        "fsl-route",
        "fsl-eval",
    ] {
        assert!(
            workspace_crates.contains(&required),
            "cargo metadata lists no {required}: {workspace_crates:?}"
        );
    }
    for (crate_name, allowed, dev_only) in [
        ("fsl-foundation", &["filament-exact"][..], &[][..]),
        (
            "fsl-cst",
            &["fsl-foundation", "filament-exact"][..],
            &[][..],
        ),
        (
            "fsl-forms",
            &[
                "filament-exact",
                "filament-semantic-value",
                "filament-walk",
                "fsl-cst",
                "fsl-foundation",
                "ix-trace-rs",
            ][..],
            &[][..],
        ),
        (
            "fsl-semantics",
            &["fsl-forms", "fsl-foundation"][..],
            &["fsl-cst"][..],
        ),
        (
            "fsl-package",
            &[
                "fsl-foundation",
                "fsl-semantics",
                "filament-exact",
                "filament-semantic-value",
            ][..],
            &["fsl-cst", "fsl-forms"][..],
        ),
        (
            "fsl-route",
            &["fsl-foundation", "fsl-semantics"][..],
            &["filament-exact"][..],
        ),
        (
            "fsl-eval",
            &[
                "fsl-foundation",
                "fsl-package",
                "fsl-semantics",
                "filament-exact",
                "filament-semantic-value",
            ][..],
            &["fsl-cst", "fsl-forms"][..],
        ),
    ] {
        let package = packages
            .iter()
            .find(|package| package.name == crate_name)
            .unwrap_or_else(|| panic!("cargo metadata lists no {crate_name}"));
        assert!(
            !package.normal.is_empty(),
            "{crate_name} lists no dependency"
        );
        if crate_name == "fsl-forms" {
            // Layer 2's "Depends on" cell names the four layer crates, so
            // its `[dependencies]` are exactly those plus `ix-trace-rs`
            // (ADR-012 section 9), which owns the `#[string_edge]` marker and
            // is exempt from the layer order (FR-064): a zero-cost attribute
            // `xtask string-edge` reads, not a semantic dependency.
            let mut normal: Vec<&str> = package.normal.iter().map(String::as_str).collect();
            normal.sort_unstable();
            assert_eq!(normal, allowed, "{crate_name}'s [dependencies]");
        }
        if crate_name == "fsl-semantics" {
            // Layer 3: its workspace-crate `[dependencies]` are
            // exactly layers 2 and F (K and SV are external) -- no `fsl-cst` (layer 1) and no
            // root crate. Its other entries are third-party crates and the
            // FCD crates, which `fcd_is_named_by_model_intake_only` below
            // confines to `model::intake`.
            let mut workspace_normal: Vec<&str> = package
                .normal
                .iter()
                .map(String::as_str)
                .filter(|dependency| workspace_crates.contains(dependency))
                .collect();
            workspace_normal.sort_unstable();
            assert_eq!(workspace_normal, allowed, "{crate_name}'s [dependencies]");
            assert!(
                !package
                    .normal
                    .iter()
                    .any(|dependency| dependency == "fsl-cst"),
                "{crate_name} depends on layer-1 fsl-cst"
            );
            fcd_is_named_by_model_intake_only(&package.normal);
            // ADR-013 §2, ADR-013:113: shipped code encodes identities
            // through `quire-canonical`; `sha2` is no dependency of this
            // crate, so no shipped module can hash on its own.
            assert!(
                package
                    .normal
                    .iter()
                    .any(|dependency| dependency == "quire-canonical"),
                "{crate_name} does not depend on quire-canonical"
            );
            assert!(
                !package.normal.iter().any(|dependency| dependency == "sha2"),
                "{crate_name} has a normal dependency on sha2"
            );
        }
        if crate_name == "fsl-package" {
            // Layer 4: "3, F, K; `filament-contract-model` for the v2
            // wire contract" (ADR-011 X-7). The whole shipped table
            // is fixed: layer 3, F, K (the kernel occurrence `Origin` the
            // I2 reader keys the v2 source map by, ADR-013 O-07),
            // the v2 wire contract, `quire-canonical` (ADR-013 §2's one
            // RFC 8785 encoder, for the `package_id` preimage),
            // and `thiserror`; `serde` deserializes semantic wire values
            // built by layer 3. The wire contract, canonical encoder,
            // `filament-exact` and `thiserror` are optional, enabled by `std`
            // (NFR-015). `serde_json` is optional too, enabled by `std`: the
            // hybrid reader reads a typed application operation through its
            // public `Serialize` form.
            // `ix-trace-rs`
            // (ADR-012 section 9) owns the `#[string_edge]` marker and is
            // exempt from the layer order below (FR-064): it is a
            // zero-cost attribute `xtask string-edge` reads, not a semantic
            // dependency.
            let mut normal: Vec<&str> = package.normal.iter().map(String::as_str).collect();
            normal.sort_unstable();
            assert_eq!(
                normal,
                [
                    "filament-checked",
                    "filament-contract-model",
                    "filament-exact",
                    "filament-semantic-value",
                    "fsl-foundation",
                    "fsl-semantics",
                    "ix-trace-rs",
                    "quire-canonical",
                    "serde",
                    "serde_json",
                    "thiserror"
                ],
                "{crate_name}'s [dependencies]"
            );
        }
        if crate_name == "fsl-eval" {
            // Layer 5: "4, 3, F, K". The whole shipped table is
            // fixed: the four layer crates, `thiserror` for its error types,
            // and `quire-canonical`/`serde`/`serde_json` for the FR-101 simulation state
            // key and the pinned sampler's draw preimage (ADR-011
            // X-8). `ix-trace-rs` (ADR-012 section 9) owns the
            // `#[string_edge]` marker and is exempt from the layer order
            // below (FR-064).
            let mut normal: Vec<&str> = package.normal.iter().map(String::as_str).collect();
            normal.sort_unstable();
            assert_eq!(
                normal,
                [
                    "filament-checked",
                    "filament-eval",
                    "filament-exact",
                    "filament-semantic-value",
                    "fsl-foundation",
                    "fsl-package",
                    "fsl-semantics",
                    "ix-trace-rs",
                    "quire-canonical",
                    "serde",
                    "serde_json",
                    "thiserror"
                ],
                "{crate_name}'s [dependencies]"
            );
            assert!(
                package.build.is_empty(),
                "{crate_name}'s [build-dependencies]: {:?}",
                package.build
            );
        }
        if crate_name == "fsl-route" {
            // Layer R (ADR-011 §6.1 row "4, 3, F, SV, K"): `route` names
            // layer 3's `Capability`, F's `CatalogCode` and
            // `MemoryModelName` (FR-226),
            // SV's `PropertyForm` for the FR-124 deadlock-freedom item, and
            // `thiserror` for `RegistrationRefusal`, `ix-trace-rs` for the
            // required FR-064 string-edge marker, and C's `DeclarationKey`
            // (`filament-checked`, X-9) for the population a request's symmetry
            // declaration names (FR-159), `filament-exact`'s `NodeKey` for the
            // structural node identity a request names; nothing of layer 4,
            // K or the root crate. The whole `[dependencies]` table is checked, so a
            // planted third-party crate fails it too.
            let mut normal: Vec<&str> = package.normal.iter().map(String::as_str).collect();
            normal.sort_unstable();
            assert_eq!(
                normal,
                [
                    "filament-checked",
                    "filament-exact",
                    "filament-semantic-value",
                    "fsl-foundation",
                    "fsl-semantics",
                    "ix-trace-rs",
                    "thiserror"
                ],
                "{crate_name}'s [dependencies]"
            );
            assert!(
                package.build.is_empty(),
                "{crate_name}'s [build-dependencies]: {:?}",
                package.build
            );
        }
        for (kind, dependencies, extra) in [
            ("normal", &package.normal, &[][..]),
            ("dev", &package.dev, dev_only),
            ("build", &package.build, &[][..]),
        ] {
            for dependency in dependencies {
                assert!(
                    !workspace_crates.contains(&dependency.as_str())
                        || allowed.contains(&dependency.as_str())
                        || extra.contains(&dependency.as_str()),
                    "{crate_name} has a {kind} dependency on workspace crate {dependency}"
                );
            }
        }
    }
    // ADR-011 §7.1: no shipped root-crate module calls `route`,
    // so the root crate names `fsl-route` as a dev dependency only. This
    // line comes out when a shipped caller (the T-13 driver) makes the
    // normal edge real.
    let root = packages
        .iter()
        .find(|package| package.name == "filament-spec-language")
        .expect("cargo metadata lists the root crate");
    for (kind, dependencies) in [("normal", &root.normal), ("build", &root.build)] {
        assert!(
            !dependencies
                .iter()
                .any(|dependency| dependency == "fsl-route"),
            "the root crate has a {kind} dependency on fsl-route; it is a dev dependency only"
        );
    }
    assert!(
        root.dev.iter().any(|dependency| dependency == "fsl-route"),
        "the root crate's tests name fsl-route through a dev dependency"
    );
    // No shipped root-crate module evaluates temporal evidence: the root
    // crate names `fsl-eval` as a dev dependency only.
    assert!(
        !root
            .normal
            .iter()
            .any(|dependency| dependency == "fsl-eval"),
        "the root crate has a normal dependency on fsl-eval; it is a dev dependency only"
    );
    assert!(
        root.dev.iter().any(|dependency| dependency == "fsl-eval"),
        "the root crate's tests drive the temporal evidence call through a dev dependency"
    );
    assert!(
        !root.build.iter().any(|dependency| dependency == "fsl-eval"),
        "no build script evaluates temporal evidence"
    );
}

/// The FCD crates `fsl-semantics` depends on are named by
/// `fsl-semantics/src/model/intake.rs` and by no other file of that crate
/// (ADR-011 §6.1: FCD from `model::intake` only). A plain text search for
/// each crate's Rust name, so a `use`, an inline path and a macro argument
/// are all caught.
fn fcd_is_named_by_model_intake_only(dependencies: &[String]) {
    let fcd: Vec<String> = dependencies
        .iter()
        .filter(|dependency| dependency.starts_with("agent-ix-"))
        .map(|dependency| dependency.replace('-', "_"))
        .collect();
    assert!(!fcd.is_empty(), "fsl-semantics names no FCD crate");
    let root = workspace_root().join("fsl-semantics/src");
    let mut pending = vec![root.clone()];
    let mut naming = std::collections::BTreeSet::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the source tree lists") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).expect("a source file reads");
                if fcd.iter().any(|name| source.contains(name.as_str())) {
                    let relative = path.strip_prefix(&root).expect("under the root");
                    naming.insert(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    assert_eq!(
        naming.into_iter().collect::<Vec<_>>(),
        ["model/intake.rs"],
        "files of fsl-semantics naming an FCD crate"
    );
}

/// Whether a `[features]` value forwards to a workspace crate's
/// `test-support`: `<crate>/test-support` or `<crate>?/test-support`.
fn forwards_to_test_support(value: &str, workspace_crates: &[&str]) -> bool {
    value.split_once('/').is_some_and(|(dependency, feature)| {
        feature == "test-support"
            && workspace_crates.contains(&dependency.strip_suffix('?').unwrap_or(dependency))
    })
}

/// #371 review L8, #372 review M1: `test-support` turns on fixture
/// constructors that forge crate-issued capabilities
/// (`ReaderAuthority::fixture`) and test-only views of `pub(crate)` items.
/// It is for tests only:
///
/// - no workspace package enables its own `test-support` by default;
/// - no normal or build dependency edge enables it on a workspace crate (a
///   dev-dependency may, which is how a crate's own tests reach it);
/// - no `[features]` entry other than a package's own `test-support`
///   forwards to a workspace crate's `test-support` (`default =
///   ["fsl-semantics/test-support"]`, or any feature a shipped dependent
///   could enable). Forwarding `test-support` to `test-support` is allowed:
///   only a test build can turn the outer one on, by the two rules above.
///
/// Cargo feature unification would otherwise switch it on in a shipped
/// build. Measured: a root `default = ["fsl-semantics/test-support"]` fails
/// this test.
#[trace("FR-087-AC-1", "TC-243", "FR-400-AC-9")]
#[test]
fn no_shipped_dependency_enables_test_support() {
    let packages = workspace_dependencies();
    // FR-400-AC-9: `filament-checked` is outside the workspace but its
    // `test-support` fixtures forge checked terms, so it is guarded too.
    let mut workspace_crates: Vec<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    workspace_crates.push("filament-checked");
    assert!(workspace_crates.contains(&"filament-spec-language"));
    // The matcher sees both spellings, including the `?` one Cargo accepts
    // only for an optional dependency, and nothing else.
    for (value, forwards) in [
        ("fsl-semantics/test-support", true),
        ("fsl-semantics?/test-support", true),
        ("filament-checked/test-support", true),
        ("fsl-semantics/other", false),
        ("serde/test-support", false),
        ("dep:fsl-semantics", false),
    ] {
        assert_eq!(
            forwards_to_test_support(value, &workspace_crates),
            forwards,
            "{value}"
        );
    }
    for package in &packages {
        assert!(
            !package
                .default_features
                .iter()
                .any(|feature| feature == "test-support"),
            "{} enables test-support by default",
            package.name
        );
        for (feature, values) in &package.features {
            if feature == "test-support" {
                continue;
            }
            for value in values {
                assert!(
                    !forwards_to_test_support(value, &workspace_crates),
                    "{}'s feature `{feature}` enables {value}, which a shipped build can reach",
                    package.name
                );
            }
        }
        for (dependency, features) in &package.shipped_features {
            assert!(
                !(workspace_crates.contains(&dependency.as_str())
                    && features.iter().any(|feature| feature == "test-support")),
                "{} enables {dependency}/test-support through a normal or build dependency",
                package.name
            );
        }
    }
}

/// Layer A's `fsl-analyze` sits above the qualified core (ADR-011 §6.1,
/// ADR-029 CB-3): no qualified-core crate (layers K to 6, SV or R), nor any
/// tools other than `xtask` and `fsl-inspect`, name it in any dependency
/// table. `xtask` is outside the stage DAG and depends on it only for
/// FR-489's provider snapshot. `fsl-inspect` is layer P and renders retained
/// state-graph results. In particular `fsl-replay`, whose zone
/// certificate checker keeps its own DBM separate from the zone engine's
/// (ADR-026 CF-6, FR-245), cannot reach the engine's DBM through a
/// dependency edge.
#[trace("TC-700", "FR-245-AC-6")]
#[test]
fn no_core_crate_depends_on_fsl_analyze() {
    let packages = workspace_dependencies();
    assert!(
        packages.iter().any(|package| package.name == "fsl-analyze"),
        "cargo metadata lists no fsl-analyze"
    );
    for core in QUALIFIED_CORE_CRATES.into_iter().chain(["arch-lint"]) {
        let package = packages
            .iter()
            .find(|package| package.name == core)
            .unwrap_or_else(|| panic!("cargo metadata lists no {core}"));
        for (kind, dependencies) in [
            ("normal", &package.normal),
            ("dev", &package.dev),
            ("build", &package.build),
        ] {
            assert!(
                !dependencies
                    .iter()
                    .any(|dependency| dependency == "fsl-analyze"),
                "{core} has a {kind} dependency on layer-A fsl-analyze"
            );
        }
    }

    // ADR-011 §6.1: xtask is build tooling, not part of the stage DAG, and no
    // stage crate may depend on it. The root uses it only from its tests.
    let root = packages
        .iter()
        .find(|package| package.name == "filament-spec-language")
        .expect("cargo metadata lists the root crate");
    assert!(
        root.dev.iter().any(|dependency| dependency == "xtask"),
        "the root test suite uses xtask through a dev dependency"
    );
    for package in &packages {
        for (kind, dependencies) in [
            ("normal", &package.normal),
            ("dev", &package.dev),
            ("build", &package.build),
        ] {
            let root_test_dependency = package.name == "filament-spec-language" && kind == "dev";
            assert!(
                root_test_dependency
                    || !dependencies.iter().any(|dependency| dependency == "xtask"),
                "{} has a {kind} dependency on off-DAG xtask",
                package.name
            );
        }
    }
}

/// FR-401-AC-2: the crates whose shipped code names a `filament-eval` item
/// (`fsl-eval`, `fsl-replay`) list it in `[dependencies]`; the lower layers
/// name it in no table, so the leaf stays below them in the dependency
/// direction.
/// The FR-401-AC-2 violations over `packages`, each naming the crate and the
/// table: a crate that must name `filament-eval` in `[dependencies]` and does
/// not, and a lower layer that names it in any table.
fn filament_eval_placement_violations(packages: &[PackageDependencies]) -> Vec<String> {
    let mut violations = Vec::new();
    for named in ["fsl-eval", "fsl-replay"] {
        match packages.iter().find(|package| package.name == named) {
            None => violations.push(format!("cargo metadata lists no {named}")),
            Some(package) if !package.normal.iter().any(|d| d == "filament-eval") => {
                violations.push(format!(
                    "{named} does not name filament-eval in [dependencies]"
                ));
            }
            Some(_) => {}
        }
    }
    for lower in [
        "fsl-foundation",
        "fsl-cst",
        "fsl-source",
        "fsl-forms",
        "fsl-semantics",
        "fsl-package",
        "fsl-route",
    ] {
        let Some(package) = packages.iter().find(|package| package.name == lower) else {
            violations.push(format!("cargo metadata lists no {lower}"));
            continue;
        };
        for (table, names) in [
            ("[dependencies]", &package.normal),
            ("[dev-dependencies]", &package.dev),
            ("[build-dependencies]", &package.build),
        ] {
            if names.iter().any(|dependency| dependency == "filament-eval") {
                violations.push(format!(
                    "{lower} names filament-eval in {table}, which sits above it"
                ));
            }
        }
    }
    violations
}

#[trace("FR-401-AC-2", "TC-390")]
#[test]
fn only_the_evaluation_layers_depend_on_filament_eval() {
    let violations = filament_eval_placement_violations(&workspace_dependencies());
    assert!(violations.is_empty(), "{violations:#?}");
}

/// FR-401-AC-2: a fixture in which `fsl-semantics` depends on `filament-eval`
/// fails the check, naming the crate and the table.
#[trace("FR-401-AC-2", "TC-390")]
#[test]
fn a_lower_layer_naming_filament_eval_fails_the_placement_check() {
    let package = |name: &str, normal: &[&str]| PackageDependencies {
        name: name.to_owned(),
        normal: normal.iter().map(|d| (*d).to_owned()).collect(),
        dev: Vec::new(),
        build: Vec::new(),
        shipped_features: Vec::new(),
        construct_tables: Vec::new(),
        default_features: Vec::new(),
        features: Vec::new(),
    };
    let mut fixture: Vec<_> = [
        "fsl-foundation",
        "fsl-cst",
        "fsl-source",
        "fsl-forms",
        "fsl-package",
        "fsl-route",
    ]
    .into_iter()
    .map(|name| package(name, &[]))
    .collect();
    fixture.push(package("fsl-eval", &["filament-eval"]));
    fixture.push(package("fsl-replay", &["filament-eval"]));
    fixture.push(package("fsl-semantics", &[]));
    assert!(filament_eval_placement_violations(&fixture).is_empty());
    fixture
        .last_mut()
        .unwrap()
        .normal
        .push("filament-eval".to_owned());
    assert_eq!(
        filament_eval_placement_violations(&fixture),
        ["fsl-semantics names filament-eval in [dependencies], which sits above it"]
    );
}

/// Every package that enables `filament-checked`'s `construct` feature, named
/// with the table, or through one of its own features that forwards to it.
fn construct_enablers(packages: &[PackageDependencies]) -> Vec<String> {
    let mut found = Vec::new();
    for package in packages {
        for table in &package.construct_tables {
            found.push(format!("{} in {table}", package.name));
        }
        for (feature, values) in &package.features {
            for value in values {
                if value.starts_with("filament-checked/") && value.ends_with("construct")
                    || value.starts_with("filament-checked?/") && value.ends_with("construct")
                {
                    found.push(format!("{} through its feature `{feature}`", package.name));
                }
            }
        }
    }
    found
}

/// FR-400-AC-1: `fsl-semantics` is the only crate that enables
/// `filament-checked`'s `construct` feature, in any table.
#[trace("FR-400-AC-1", "TC-390")]
#[test]
fn only_fsl_semantics_enables_the_construct_feature() {
    let enablers = construct_enablers(&workspace_dependencies());
    assert_eq!(enablers, ["fsl-semantics in [dependencies]"]);
}

/// FR-400-AC-1: a fixture manifest in which `fsl-eval` enables `construct`
/// fails the check, naming the crate and the table.
#[trace("FR-400-AC-1", "TC-390")]
#[test]
fn a_crate_other_than_fsl_semantics_enabling_construct_is_named() {
    let package = |name: &str, construct_tables: Vec<&'static str>| PackageDependencies {
        name: name.to_owned(),
        normal: Vec::new(),
        dev: Vec::new(),
        build: Vec::new(),
        shipped_features: Vec::new(),
        construct_tables,
        default_features: Vec::new(),
        features: Vec::new(),
    };
    let fixture = vec![
        package("fsl-semantics", vec!["[dependencies]"]),
        package("fsl-eval", vec!["[dev-dependencies]"]),
    ];
    assert_eq!(
        construct_enablers(&fixture),
        [
            "fsl-semantics in [dependencies]",
            "fsl-eval in [dev-dependencies]"
        ]
    );
}

/// The filesystem request owner sits above the core; core dependency edges
/// must never import its ambient reads into a qualified binary.
#[trace("TC-771", "FR-287-AC-10")]
#[test]
fn command_intake_is_a_non_core_leaf_consumer() {
    let packages = workspace_dependencies();
    let command = packages
        .iter()
        .find(|package| package.name == "fsl-command")
        .expect("the command owner is a workspace package");
    assert!(command.normal.iter().any(|name| name == "fsl-replay"));
    assert!(!command
        .normal
        .iter()
        .any(|name| name == "filament-spec-language"));
    for core in QUALIFIED_CORE_CRATES {
        let package = packages
            .iter()
            .find(|package| package.name == core)
            .unwrap_or_else(|| panic!("cargo metadata lists no {core}"));
        for (kind, dependencies) in [("normal", &package.normal), ("build", &package.build)] {
            assert!(
                !dependencies.iter().any(|name| name == "fsl-command"),
                "{core} has a {kind} dependency on non-core filesystem intake"
            );
        }
    }
    let root = std::fs::read_to_string(workspace_root().join("src/lib.rs")).unwrap();
    let parsed = syn::parse_file(&root).unwrap();
    for item in parsed.items {
        if let syn::Item::Mod(module) = item {
            assert!(
                !["command", "formal_source"]
                    .iter()
                    .any(|name| module.ident == *name),
                "root defines the moved {} module",
                module.ident
            );
        }
    }
}
