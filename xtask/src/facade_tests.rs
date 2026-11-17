// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask facade-tests`: require a live outside-crate test for every
//! consumer fact in the pinned FSpec inventory.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    error::{Error, Result},
    Outcome,
};
use serde::Deserialize;
use syn::{Attribute, Item, ItemMod};

const INVENTORY: &str = "proposals/consumer-facades/consumer-facts.json";
const DEFAULT_TESTS: &str = "tests/fixtures/facade-contract-consumer/tests/it";

#[derive(Deserialize)]
struct Inventory {
    consumers: Vec<Consumer>,
}
#[derive(Deserialize)]
struct Consumer {
    facts: Vec<Fact>,
}
#[derive(Deserialize)]
struct Fact {
    id: String,
}

#[derive(Debug)]
struct ContractFunction {
    name: String,
    path: PathBuf,
    live: bool,
    reason: Option<&'static str>,
}

fn has_attribute(attributes: &[Attribute], name: &str) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident(name))
}

fn disabled(attributes: &[Attribute]) -> bool {
    has_attribute(attributes, "ignore")
        || has_attribute(attributes, "cfg")
        || has_attribute(attributes, "cfg_attr")
}

fn module_file(module: &ItemMod, source_file: &Path) -> Option<PathBuf> {
    if let Some(attribute) = module
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("path"))
    {
        let path: syn::LitStr = attribute.parse_args().ok()?;
        return Some(source_file.parent()?.join(path.value()));
    }
    let parent = source_file.parent()?;
    let stem = source_file.file_stem()?.to_str()?;
    let directory = if matches!(stem, "main" | "lib" | "mod") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };
    let name = module.ident.to_string();
    let flat = directory.join(format!("{name}.rs"));
    Some(if flat.is_file() {
        flat
    } else {
        directory.join(name).join("mod.rs")
    })
}

fn collect_items(
    items: &[Item],
    source_file: &Path,
    test_root: &Path,
    module_disabled: bool,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    functions: &mut Vec<ContractFunction>,
) -> Result<()> {
    for item in items {
        match item {
            Item::Fn(function) => {
                let Some(name) = function
                    .sig
                    .ident
                    .to_string()
                    .strip_prefix("cf_")
                    .map(str::to_owned)
                else {
                    continue;
                };
                let has_test = has_attribute(&function.attrs, "test");
                let reason = if module_disabled {
                    Some("enclosing module is conditional")
                } else if has_attribute(&function.attrs, "ignore") {
                    Some("function is ignored")
                } else if has_attribute(&function.attrs, "cfg")
                    || has_attribute(&function.attrs, "cfg_attr")
                {
                    Some("function is conditional")
                } else if !has_test {
                    Some("function has no #[test]")
                } else {
                    None
                };
                functions.push(ContractFunction {
                    name,
                    path: source_file.to_path_buf(),
                    live: reason.is_none(),
                    reason,
                });
            }
            Item::Mod(module) => {
                let child_disabled = module_disabled || disabled(&module.attrs);
                if let Some((_, nested)) = &module.content {
                    collect_items(
                        nested,
                        source_file,
                        test_root,
                        child_disabled,
                        seen,
                        functions,
                    )?;
                } else if let Some(path) = module_file(module, source_file) {
                    let canonical = path
                        .canonicalize()
                        .map_err(|error| Error::io(&path, error))?;
                    if !canonical.starts_with(test_root) {
                        return Err(Error::QualificationRefusal(format!(
                            "facade-tests: module {} escapes test root {}",
                            canonical.display(),
                            test_root.display()
                        )));
                    }
                    if seen.insert(canonical.clone()) {
                        collect_file(&canonical, test_root, child_disabled, seen, functions)?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn fact_id(name: &str) -> Option<String> {
    let parts: Vec<_> = name.split('_').collect();
    if parts.len() < 4
        || parts[0] != "cf"
        || parts[1].is_empty()
        || parts[2].len() != 2
        || !parts[2].bytes().all(|byte| byte.is_ascii_digit())
        || parts[3..].iter().any(|part| part.is_empty())
    {
        return None;
    }
    Some(format!("CF-{}-{}", parts[1].to_ascii_uppercase(), parts[2]))
}

fn collect_file(
    path: &Path,
    test_root: &Path,
    module_disabled: bool,
    seen: &mut std::collections::BTreeSet<PathBuf>,
    functions: &mut Vec<ContractFunction>,
) -> Result<()> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let file = syn::parse_file(&source).map_err(|error| {
        Error::QualificationRefusal(format!(
            "facade-tests: cannot parse {}: {error}",
            path.display()
        ))
    })?;
    collect_items(
        &file.items,
        path,
        test_root,
        module_disabled,
        seen,
        functions,
    )
}

fn source_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(directory).map_err(|error| Error::io(directory, error))?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(directory, error))?;
        let path = entry.path();
        if path.is_dir() {
            paths.extend(source_paths(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn scan_sources(directory: &Path) -> Result<Vec<ContractFunction>> {
    let root = directory
        .canonicalize()
        .map_err(|error| Error::io(directory, error))?;
    let mut paths = source_paths(&root)?;
    paths.sort();
    let mut seen = std::collections::BTreeSet::new();
    let mut functions = Vec::new();
    if let Some(main) = paths
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name == "main.rs"))
    {
        let main = main
            .canonicalize()
            .map_err(|error| Error::io(main, error))?;
        seen.insert(main.clone());
        collect_file(&main, &root, false, &mut seen, &mut functions)?;
    }
    for path in paths {
        let path = path
            .canonicalize()
            .map_err(|error| Error::io(&path, error))?;
        if seen.insert(path.clone()) {
            collect_file(&path, &root, false, &mut seen, &mut functions)?;
        }
    }
    Ok(functions)
}

fn inventory_ids(override_path: Option<&Path>, workspace_root: &Path) -> Result<Vec<String>> {
    let bytes = match override_path {
        Some(path) => fs::read(path).map_err(|error| Error::io(path, error))?,
        None => filament_specification::published_file(INVENTORY)
            .ok_or_else(|| {
                Error::QualificationRefusal(format!(
                    "facade-tests: pinned inventory {INVENTORY} is absent"
                ))
            })?
            .as_bytes()
            .to_vec(),
    };
    let inventory: Inventory = serde_json::from_slice(&bytes).map_err(|error| {
        Error::QualificationRefusal(format!("facade-tests: invalid inventory: {error}"))
    })?;
    Ok(inventory
        .consumers
        .into_iter()
        .flat_map(|consumer| consumer.facts)
        .map(|fact| fact.id)
        .collect())
}

/// Run the facade-contract completeness gate.
pub fn run(workspace_root: &Path, arguments: &[OsString]) -> Result<Outcome> {
    let mut tests = workspace_root.join(DEFAULT_TESTS);
    let mut inventory = None;
    let mut operands = arguments.iter();
    while let Some(argument) = operands.next() {
        match argument.to_str() {
            Some("--tests") => {
                let value = operands.next().ok_or(Error::Usage(
                    "cargo xtask facade-tests [--tests <dir>] [--inventory <file>]",
                ))?;
                tests = PathBuf::from(value);
            }
            Some("--inventory") => {
                let value = operands.next().ok_or(Error::Usage(
                    "cargo xtask facade-tests [--tests <dir>] [--inventory <file>]",
                ))?;
                inventory = Some(PathBuf::from(value));
            }
            _ => {
                return Err(Error::Usage(
                    "cargo xtask facade-tests [--tests <dir>] [--inventory <file>]",
                ))
            }
        }
    }
    if !tests.is_absolute() {
        tests = workspace_root.join(tests);
    }
    if let Some(path) = inventory.as_mut() {
        if !path.is_absolute() {
            *path = workspace_root.join(&*path);
        }
    }
    let known: std::collections::BTreeSet<_> = inventory_ids(inventory.as_deref(), workspace_root)?
        .into_iter()
        .collect();
    let functions = scan_sources(&tests)?;
    let mut seen = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut failures = Vec::new();
    for function in &functions {
        let Some(id) = fact_id(&function.name) else {
            failures.push(format!(
                "{}: malformed contract test name {}",
                function.path.display(),
                function.name
            ));
            continue;
        };
        if !known.contains(&id) {
            failures.push(format!(
                "{}: {} names unknown inventory fact {id}",
                function.path.display(),
                function.name
            ));
        } else if function.live {
            seen.entry(id).or_default().push(function.name.clone());
        } else {
            failures.push(format!(
                "{}: {} for {id} is not live: {}",
                function.path.display(),
                function.name,
                function.reason.unwrap_or("unknown reason")
            ));
        }
    }
    let mut output = String::new();
    for id in &known {
        if let Some(names) = seen.get(id) {
            output.push_str(&format!("{id}: {}\n", names.join(", ")));
        } else {
            failures.push(format!("{id}: no live contract test"));
        }
    }
    if failures.is_empty() {
        Ok(Outcome::success(output))
    } else {
        failures.sort();
        output.push_str(
            &failures
                .into_iter()
                .map(|failure| format!("FAIL: {failure}\n"))
                .collect::<String>(),
        );
        Ok(Outcome {
            stdout: output,
            code: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tempdir;

    #[test]
    fn test_prefix_is_the_lowercase_fact_id() {
        assert_eq!(
            fact_id("pr_03_reads_endpoints").as_deref(),
            Some("CF-PR-03")
        );
        assert_eq!(fact_id("x_03_bad"), None);
    }

    #[test]
    fn live_test_must_have_test_and_no_disable_attributes() {
        let root = tempdir();
        let source = "#[test] fn cf_pr_01_live() {} #[test] #[ignore] fn cf_pr_02_ignored() {} #[cfg(test)] mod hidden { #[test] fn cf_pr_03_hidden() {} }";
        fs::write(root.path().join("contract.rs"), source).unwrap();
        let functions = scan_sources(root.path()).unwrap();
        assert_eq!(functions.len(), 3);
        assert!(
            functions
                .iter()
                .find(|item| item.name == "pr_01_live")
                .unwrap()
                .live
        );
        assert!(
            !functions
                .iter()
                .find(|item| item.name == "pr_02_ignored")
                .unwrap()
                .live
        );
        assert!(
            !functions
                .iter()
                .find(|item| item.name == "pr_03_hidden")
                .unwrap()
                .live
        );
    }
}
