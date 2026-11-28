// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dependency, feature and license-grant inventories over the workspace.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;
use syn::visit::Visit;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level below the workspace")
        .to_path_buf()
}

fn cargo(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO"))
        .current_dir(root())
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("run cargo {arguments:?}: {error}"))
}

/// Cargo's resolution of the locked manifests, with every feature enabled so
/// that optional dependencies appear.
fn metadata() -> Value {
    let output = cargo(&[
        "metadata",
        "--locked",
        "--offline",
        "--all-features",
        "--format-version",
        "1",
    ]);
    assert!(
        output.status.success(),
        "cargo metadata: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata JSON")
}

fn array<'value>(value: &'value Value, name: &str) -> &'value [Value] {
    value[name]
        .as_array()
        .unwrap_or_else(|| panic!("metadata array `{name}`"))
}

fn text<'value>(value: &'value Value, name: &str) -> &'value str {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("metadata string `{name}`"))
}

fn package<'value>(metadata: &'value Value, id: &str) -> &'value Value {
    array(metadata, "packages")
        .iter()
        .find(|package| package["id"] == id)
        .unwrap_or_else(|| panic!("metadata package {id}"))
}

/// The packages `id` depends on directly, by the crate name its source uses.
fn direct_dependencies<'value>(
    metadata: &'value Value,
    id: &str,
) -> BTreeMap<&'value str, &'value str> {
    let node = array(&metadata["resolve"], "nodes")
        .iter()
        .find(|node| node["id"] == id)
        .unwrap_or_else(|| panic!("resolve node {id}"));
    array(node, "deps")
        .iter()
        .map(|dependency| (text(dependency, "name"), text(dependency, "pkg")))
        .collect()
}

/// The crate roots a set of source files imports, and the names those files
/// declare themselves.
#[derive(Default)]
struct Imports {
    roots: BTreeSet<String>,
    declared: BTreeSet<String>,
}

impl Imports {
    fn read(&mut self, path: &str, source: &str) {
        let file = syn::parse_file(source).unwrap_or_else(|error| panic!("{path}: {error}"));
        self.visit_file(&file);
    }

    /// The imported roots that are neither declared dependencies, names the
    /// sources declare, the sysroot, nor path keywords. A root starting with
    /// an upper-case letter is a type in scope, never a crate.
    fn undeclared(&self, dependencies: &BTreeSet<String>) -> Vec<&str> {
        const BUILT_IN: [&str; 8] = [
            "alloc",
            "core",
            "crate",
            "proc_macro",
            "self",
            "std",
            "super",
            "test",
        ];
        self.roots
            .iter()
            .filter(|root| {
                !root.starts_with(|first: char| first.is_ascii_uppercase())
                    && !BUILT_IN.contains(&root.as_str())
                    && !dependencies.contains(root.as_str())
                    && !self.declared.contains(root.as_str())
            })
            .map(String::as_str)
            .collect()
    }

    /// Record the first segment of each path of one `use` tree.
    fn use_roots(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                self.roots.insert(path.ident.to_string());
                self.use_names(&path.tree);
            }
            syn::UseTree::Name(name) => {
                self.roots.insert(name.ident.to_string());
            }
            syn::UseTree::Rename(rename) => {
                self.roots.insert(rename.ident.to_string());
                self.declared.insert(rename.rename.to_string());
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.use_roots(tree);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }

    /// Visit the types of a qualified path, whose own first segment is an
    /// item of the qualifying type.
    fn visit_qualified(&mut self, qualified: &syn::QSelf, path: &syn::Path) {
        self.visit_type(&qualified.ty);
        for segment in &path.segments {
            self.visit_path_arguments(&segment.arguments);
        }
    }

    /// Record the names one `use` tree brings into scope below its root.
    fn use_names(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => self.use_names(&path.tree),
            syn::UseTree::Name(name) => {
                self.declared.insert(name.ident.to_string());
            }
            syn::UseTree::Rename(rename) => {
                self.declared.insert(rename.rename.to_string());
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.use_names(tree);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }
}

impl<'ast> Visit<'ast> for Imports {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.use_roots(&item.tree);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.roots.insert(item.ident.to_string());
        if let Some((_, rename)) = &item.rename {
            self.declared.insert(rename.to_string());
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.declared.insert(item.ident.to_string());
        syn::visit::visit_item_mod(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if let Some(name) = &item.ident {
            self.declared.insert(name.to_string());
        }
    }

    /// The path of `<Type>::item` starts with `::` but names no crate.
    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        match &expression.qself {
            Some(qualified) => self.visit_qualified(qualified, &expression.path),
            None => syn::visit::visit_expr_path(self, expression),
        }
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        match &path.qself {
            Some(qualified) => self.visit_qualified(qualified, &path.path),
            None => syn::visit::visit_type_path(self, path),
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path.leading_colon.is_some() {
            if let Some(first) = path.segments.first() {
                self.roots.insert(first.ident.to_string());
            }
        }
        syn::visit::visit_path(self, path);
    }
}

/// The Rust sources of the package in `directory`: its target directories and
/// build script, without the packages nested inside them.
fn package_sources(directory: &Path) -> Vec<PathBuf> {
    fn walk(path: &Path, found: &mut Vec<PathBuf>) {
        if path.is_file() {
            if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path.to_path_buf());
            }
        } else if path.is_dir() && !path.join("Cargo.toml").is_file() {
            for entry in
                fs::read_dir(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            {
                walk(&entry.expect("directory entry").path(), found);
            }
        }
    }
    let mut found = Vec::new();
    for part in ["src", "tests", "benches", "examples", "build.rs"] {
        walk(&directory.join(part), &mut found);
    }
    found
}

/// Every crate a workspace member's sources import, in any target and under
/// any `cfg`, is a direct dependency that its manifest declares and the
/// lockfile resolves.
#[test]
#[trace("NFR-002-M-1")]
fn every_imported_crate_is_a_declared_and_locked_dependency() {
    let metadata = metadata();
    let members = array(&metadata, "workspace_members");
    assert!(!members.is_empty(), "the workspace has members");
    let mut findings = Vec::new();
    for member in members {
        let id = member.as_str().expect("member id");
        let package = package(&metadata, id);
        let mut dependencies: BTreeSet<String> = direct_dependencies(&metadata, id)
            .keys()
            .map(|name| (*name).to_owned())
            .collect();
        // A package's own library is in scope in its other targets.
        dependencies.extend(
            array(package, "targets")
                .iter()
                .map(|target| text(target, "name").replace('-', "_")),
        );
        let directory = Path::new(text(package, "manifest_path"))
            .parent()
            .expect("manifest directory");
        let mut imports = Imports::default();
        let sources = package_sources(directory);
        assert!(!sources.is_empty(), "{} has sources", text(package, "name"));
        for source in sources {
            let content = fs::read_to_string(&source)
                .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
            imports.read(&source.display().to_string(), &content);
        }
        findings.extend(
            imports
                .undeclared(&dependencies)
                .into_iter()
                .map(|root| format!("{}: `{root}`", text(package, "name"))),
        );
    }
    assert!(
        findings.is_empty(),
        "imported crates that no manifest declares: {findings:#?}"
    );
}

#[test]
fn the_import_scan_reports_only_undeclared_crates() {
    let mut imports = Imports::default();
    imports.read(
        "fixture.rs",
        "use serde::Serialize;
         use missing_use::Thing;
         extern crate missing_extern;
         mod local;
         use local::item;
         use std::fs;
         use Kind::First;
         fn qualified() { let _ = <[u8]>::len(&[]); let _: <u8>::Item = <u8>::try_from(1); }
         #[cfg(feature = \"never\")]
         fn hidden() { use missing_cfg::{a, b}; let _ = ::missing_path::value(); }",
    );
    assert_eq!(
        imports.undeclared(&BTreeSet::from(["serde".to_owned()])),
        [
            "missing_cfg",
            "missing_extern",
            "missing_path",
            "missing_use"
        ]
    );
}

/// The grant and notices of two third-party packages xtask depends on, at
/// whatever versions the lockfile resolves, are the reviewed ones.
#[test]
#[trace("NFR-004-M-2")]
fn locked_registry_dependencies_retain_their_grants_and_notices() {
    let metadata = metadata();
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(root().join("Cargo.lock")).expect("read lockfile"))
            .expect("parse lockfile");
    let xtask = array(&metadata, "workspace_members")
        .iter()
        .filter_map(Value::as_str)
        .find(|id| package(&metadata, id)["name"] == "xtask")
        .expect("xtask is a workspace member");
    let dependencies = direct_dependencies(&metadata, xtask);
    for (name, license) in [
        ("toml", "MIT OR Apache-2.0"),
        ("serde", "MIT OR Apache-2.0"),
    ] {
        let id = dependencies
            .get(name)
            .unwrap_or_else(|| panic!("xtask depends on {name}"));
        let resolved = package(&metadata, id);
        let version = text(resolved, "version");
        let locked = lock["package"]
            .as_array()
            .expect("lock packages")
            .iter()
            .find(|package| {
                package["name"].as_str() == Some(name)
                    && package["version"].as_str() == Some(version)
            })
            .unwrap_or_else(|| panic!("{name} {version} is locked"));
        assert!(locked["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("registry+")));
        assert!(locked["checksum"]
            .as_str()
            .is_some_and(|checksum| !checksum.is_empty()));
        assert_eq!(
            resolved["license"].as_str(),
            Some(license),
            "{name} {version} retains its reviewed grant"
        );
        let directory = Path::new(text(resolved, "manifest_path"))
            .parent()
            .expect("crate directory");
        for notice in ["LICENSE-MIT", "LICENSE-APACHE"] {
            let notice = directory.join(notice);
            let contents = fs::read_to_string(&notice)
                .unwrap_or_else(|error| panic!("{}: {error}", notice.display()));
            assert!(!contents.trim().is_empty(), "{} is empty", notice.display());
        }
    }
}

/// The workspace, and each member that has default features, builds with every
/// default feature off and no feature named.
#[test]
#[ignore = "runs in make test-deep-measurements (part of make ci)"]
#[trace("NFR-002-M-3")]
fn the_native_build_requires_no_optional_feature() {
    let metadata = metadata();
    let members = array(&metadata, "workspace_members");
    let with_defaults: Vec<&str> = members
        .iter()
        .filter_map(Value::as_str)
        .map(|id| package(&metadata, id))
        .filter(|package| {
            package["features"]["default"]
                .as_array()
                .is_some_and(|features| !features.is_empty())
        })
        .map(|package| text(package, "name"))
        .collect();
    assert!(
        !with_defaults.is_empty(),
        "the inventory found the members with default features"
    );
    let selections = std::iter::once(vec!["--workspace"])
        .chain(with_defaults.iter().map(|name| vec!["--package", *name]));
    for selection in selections {
        let mut arguments = vec!["build", "--locked", "--offline", "--no-default-features"];
        arguments.extend(selection);
        let output = cargo(&arguments);
        assert!(
            output.status.success(),
            "cargo {arguments:?} needs a feature: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Every tracked implementation file records NFR-004's selected grant,
/// AGPL-3.0-or-later, on its first line (after a script's interpreter line).
/// The inventory is every tracked Rust source, shell script and Makefile; all
/// are first-party, and the repository tracks no generated or third-party one.
#[test]
#[trace("NFR-004-M-1")]
fn every_tracked_implementation_file_records_its_selected_grant() {
    let root = root();
    let output = Command::new("git")
        .current_dir(&root)
        .args(["ls-files", "-z"])
        .output()
        .expect("list the tracked files");
    assert!(
        output.status.success(),
        "git ls-files: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tracked = String::from_utf8(output.stdout).expect("UTF-8 tracked paths");
    let mut inventoried = BTreeSet::new();
    let mut missing = Vec::new();
    for path in tracked.split_terminator('\0') {
        let file = Path::new(path);
        let comment = match file.extension().and_then(|extension| extension.to_str()) {
            Some("rs") => "//",
            Some("sh" | "mk") => "#",
            _ if file.file_name().is_some_and(|name| name == "Makefile") => "#",
            _ => continue,
        };
        let content =
            fs::read_to_string(root.join(file)).unwrap_or_else(|error| panic!("{path}: {error}"));
        let first = content.lines().find(|line| !line.starts_with("#!/"));
        if first
            != Some(&format!(
                "{comment} SPDX-License-Identifier: AGPL-3.0-or-later"
            ))
        {
            missing.push(path);
        }
        inventoried.insert(path);
    }
    for expected in [
        "Makefile",
        "src/lib.rs",
        "fsl-semantics/src/model/dispatch.rs",
        "tools/check-no-committed-binaries.sh",
    ] {
        assert!(inventoried.contains(expected), "{expected} is inventoried");
    }
    assert!(
        missing.is_empty(),
        "implementation files without the selected grant: {missing:#?}"
    );
}
