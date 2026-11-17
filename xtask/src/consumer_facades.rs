// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-761: bind each standard consumer fact to a public FSL API path.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use syn::{Item, UseTree, Visibility};

use crate::error::{Error, Result};
use crate::Outcome;

const USAGE: &str = "usage: cargo xtask consumer-facades [--bindings <file>] [--inventory <file>]";
const INVENTORY_PATH: &str = "proposals/consumer-facades/consumer-facts.json";

#[derive(Default)]
struct PublicApi {
    paths: BTreeSet<String>,
    members: BTreeSet<(String, String)>,
    wildcard_reexports: Vec<(String, String)>,
}

/// Run the facade contract check over the committed bindings and pinned inventory.
pub fn run(workspace_root: &Path, arguments: &[OsString]) -> Result<Outcome> {
    let mut bindings_path = workspace_root.join("docs/consumer-facades.json");
    let mut inventory_path = None;
    let mut args = arguments.iter();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(Error::Usage(USAGE))?;
        match flag.to_str() {
            Some("--bindings")
                if bindings_path == workspace_root.join("docs/consumer-facades.json") =>
            {
                bindings_path = PathBuf::from(value);
            }
            Some("--inventory") if inventory_path.is_none() => {
                inventory_path = Some(PathBuf::from(value));
            }
            _ => return Err(Error::Usage(USAGE)),
        }
    }

    let inventory_bytes = if let Some(path) = inventory_path {
        fs::read(&path).map_err(|source| Error::ConsumerFacadesRead {
            path: path.display().to_string(),
            source,
        })?
    } else {
        filament_specification::published_file(INVENTORY_PATH)
            .ok_or(Error::ConsumerFacadesInventoryMissing(INVENTORY_PATH))?
            .to_vec()
    };
    let bindings_bytes = fs::read(&bindings_path).map_err(|source| Error::ConsumerFacadesRead {
        path: bindings_path.display().to_string(),
        source,
    })?;
    let inventory: Value = serde_json::from_slice(&inventory_bytes)
        .map_err(|source| Error::ConsumerFacadesJson { source })?;
    let bindings: Value = serde_json::from_slice(&bindings_bytes)
        .map_err(|source| Error::ConsumerFacadesJson { source })?;

    let mut failures = Vec::new();
    validate_object(
        &inventory,
        &["inventory", "consumers"],
        "inventory",
        &mut failures,
    );
    if inventory.get("inventory").and_then(Value::as_str) != Some("filament.consumer-facts/v1") {
        failures.push("inventory: unsupported or missing format identifier".to_owned());
    }
    validate_object(
        &bindings,
        &["bindings", "consumers"],
        "bindings",
        &mut failures,
    );
    if bindings.get("bindings").and_then(Value::as_str) != Some("fsl.consumer-facade-bindings/v1") {
        failures.push("bindings: unsupported or missing format identifier".to_owned());
    }
    let inventory_facts = facts_by_consumer(&inventory, &mut failures);
    let binding_facts = bindings_by_consumer(&bindings, &mut failures);
    validate_binding_order(&inventory, &bindings, &mut failures);
    let crate_roots = crate_roots(workspace_root)?;
    let mut api_cache = BTreeMap::<String, PublicApi>::new();
    let mut report = Vec::new();

    for (consumer, facts) in &inventory_facts {
        let Some(binding) = binding_facts.get(consumer) else {
            for fact in facts {
                failures.push(format!("{fact}: unbound (consumer {consumer})"));
            }
            continue;
        };
        for fact in facts {
            let Some(accessors) = binding.facts.get(fact) else {
                failures.push(format!("{fact}: unbound"));
                continue;
            };
            let (crate_name, modules) = binding
                .module
                .split_once("::")
                .unwrap_or((binding.module.as_str(), ""));
            let Some(root) = crate_roots.get(crate_name) else {
                failures.push(format!("{fact}: unknown facade crate {}", binding.module));
                continue;
            };
            let api = api_cache
                .entry(crate_name.to_owned())
                .or_insert_with(|| public_api(crate_name, root));
            let module_path = if modules.is_empty() {
                crate_name.to_owned()
            } else {
                format!("{crate_name}::{modules}")
            };
            if !api.paths.contains(&module_path) {
                failures.push(format!("{fact}: facade module {module_path} is not public"));
                continue;
            }
            let mut resolved = Vec::new();
            for accessor in accessors {
                let path = format!("{module_path}::{accessor}");
                if accessor_resolves(api, &module_path, accessor) {
                    resolved.push(path);
                } else {
                    failures.push(format!("{fact}: unresolved public accessor {path}"));
                }
            }
            if !resolved.is_empty() {
                report.push(format!("{fact}: {}", resolved.join(", ")));
            }
        }
    }

    for (consumer, facts) in &binding_facts {
        let known: BTreeSet<_> = inventory_facts
            .get(consumer)
            .into_iter()
            .flat_map(|facts| facts.iter().map(String::as_str))
            .collect();
        for fact in facts.facts.keys() {
            if !known.contains(fact.as_str()) {
                failures.push(format!("{fact}: binding has no inventory fact"));
            }
        }
        if !inventory_facts.contains_key(consumer) {
            failures.push(format!(
                "{}: binding has no inventory consumer",
                facts.module
            ));
        }
    }

    failures.sort();
    report.sort();
    if failures.is_empty() {
        Ok(Outcome::success(report.join("\n") + "\n"))
    } else {
        let mut output = failures;
        output.extend(report);
        Ok(Outcome {
            stdout: output.join("\n") + "\n",
            code: 1,
        })
    }
}

struct ConsumerBinding {
    module: String,
    facts: BTreeMap<String, Vec<String>>,
}

fn validate_object(value: &Value, allowed: &[&str], name: &str, failures: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        failures.push(format!("{name}: expected an object"));
        return;
    };
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            failures.push(format!("{name}: unexpected field {key}"));
        }
    }
}

fn validate_binding_order(inventory: &Value, bindings: &Value, failures: &mut Vec<String>) {
    let Some(inventory_consumers) = inventory.get("consumers").and_then(Value::as_array) else {
        return;
    };
    let Some(binding_consumers) = bindings.get("consumers").and_then(Value::as_array) else {
        return;
    };
    let inventory_names: Vec<_> = inventory_consumers
        .iter()
        .filter_map(|consumer| consumer.get("consumer").and_then(Value::as_str))
        .collect();
    let binding_names: Vec<_> = binding_consumers
        .iter()
        .filter_map(|consumer| consumer.get("consumer").and_then(Value::as_str))
        .collect();
    if inventory_names != binding_names {
        if let Some(fact) = inventory_consumers
            .iter()
            .filter_map(|consumer| consumer.get("facts").and_then(Value::as_array))
            .flatten()
            .find_map(|fact| fact.get("id").and_then(Value::as_str))
        {
            failures.push(format!("{fact}: consumers are not in inventory order"));
        }
    }

    for (inventory_consumer, binding_consumer) in inventory_consumers.iter().zip(binding_consumers)
    {
        let name = inventory_consumer
            .get("consumer")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let inventory_facts: Vec<_> = inventory_consumer
            .get("facts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|fact| fact.get("id").and_then(Value::as_str))
            .collect();
        let binding_facts: Vec<_> = binding_consumer
            .get("facts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|fact| fact.get("fact").and_then(Value::as_str))
            .collect();
        if inventory_facts != binding_facts {
            let max_len = inventory_facts.len().max(binding_facts.len());
            for index in 0..max_len {
                let expected = inventory_facts.get(index).copied();
                let actual = binding_facts.get(index).copied();
                if expected != actual {
                    if let Some(fact) = expected.or(actual) {
                        failures.push(format!(
                            "{fact}: bindings {name} facts are not in inventory order"
                        ));
                    }
                }
            }
        }
    }
}

fn facts_by_consumer(
    inventory: &Value,
    failures: &mut Vec<String>,
) -> BTreeMap<String, Vec<String>> {
    let mut result = BTreeMap::new();
    let Some(consumers) = inventory.get("consumers").and_then(Value::as_array) else {
        failures.push("inventory: consumers is not an array".to_owned());
        return result;
    };
    for consumer in consumers {
        validate_object(
            consumer,
            &["consumer", "role", "facts"],
            "inventory consumer",
            failures,
        );
        let Some(name) = consumer.get("consumer").and_then(Value::as_str) else {
            failures.push("inventory: consumer has no name".to_owned());
            continue;
        };
        let Some(facts) = consumer.get("facts").and_then(Value::as_array) else {
            failures.push(format!("inventory {name}: facts is not an array"));
            continue;
        };
        let mut entries = Vec::new();
        for fact in facts {
            validate_object(
                fact,
                &["id", "fact", "requirement"],
                "inventory fact",
                failures,
            );
            let Some(id) = fact.get("id").and_then(Value::as_str) else {
                failures.push(format!("inventory {name}: fact has no id"));
                continue;
            };
            entries.push(id.to_owned());
        }
        if result.insert(name.to_owned(), entries).is_some() {
            failures.push(format!("inventory: duplicate consumer {name}"));
        }
    }
    result
}

fn bindings_by_consumer(
    bindings: &Value,
    failures: &mut Vec<String>,
) -> BTreeMap<String, ConsumerBinding> {
    let mut result = BTreeMap::new();
    let Some(consumers) = bindings.get("consumers").and_then(Value::as_array) else {
        failures.push("bindings: consumers is not an array".to_owned());
        return result;
    };
    for consumer in consumers {
        validate_object(
            consumer,
            &["consumer", "module", "facts"],
            "binding consumer",
            failures,
        );
        let Some(name) = consumer.get("consumer").and_then(Value::as_str) else {
            failures.push("bindings: consumer has no name".to_owned());
            continue;
        };
        let Some(module) = consumer.get("module").and_then(Value::as_str) else {
            failures.push(format!("bindings {name}: module is missing"));
            continue;
        };
        let Some(facts) = consumer.get("facts").and_then(Value::as_array) else {
            failures.push(format!("bindings {name}: facts is not an array"));
            continue;
        };
        let mut fact_map = BTreeMap::new();
        for fact in facts {
            validate_object(fact, &["fact", "accessors"], "binding fact", failures);
            let Some(id) = fact.get("fact").and_then(Value::as_str) else {
                failures.push(format!("bindings {name}: fact has no id"));
                continue;
            };
            let accessors = match fact.get("accessors").and_then(Value::as_array) {
                Some(values) => {
                    if values.iter().any(|value| !value.is_string()) {
                        failures.push(format!("{id}: accessor entries must be strings"));
                    }
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                }
                None => {
                    failures.push(format!("{id}: accessors is not an array"));
                    Vec::new()
                }
            };
            if accessors.is_empty() {
                failures.push(format!("{id}: accessors must be non-empty"));
            }
            if fact_map.insert(id.to_owned(), accessors).is_some() {
                failures.push(format!("{id}: duplicate binding"));
            }
        }
        if result
            .insert(
                name.to_owned(),
                ConsumerBinding {
                    module: module.to_owned(),
                    facts: fact_map,
                },
            )
            .is_some()
        {
            failures.push(format!("bindings: duplicate consumer {name}"));
        }
    }
    result
}

fn crate_roots(workspace_root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .current_dir(workspace_root)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|source| Error::ConsumerFacadesMetadataSpawn { source })?;
    if !output.status.success() {
        return Err(Error::ConsumerFacadesMetadataFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|source| Error::ConsumerFacadesMetadataJson { source })?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or(Error::ConsumerFacadesMetadataMissing("packages"))?;
    let mut roots = BTreeMap::new();
    for package in packages {
        let (Some(name), Some(manifest)) =
            (package["name"].as_str(), package["manifest_path"].as_str())
        else {
            continue;
        };
        let crate_name = package["targets"]
            .as_array()
            .and_then(|targets| {
                targets.iter().find(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kind| kind.iter().any(|item| item == "lib"))
                })
            })
            .and_then(|target| target["src_path"].as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(manifest)
                    .parent()
                    .expect("manifest parent")
                    .join("src/lib.rs")
            });
        roots.insert(name.replace('-', "_"), crate_name);
    }
    Ok(roots)
}

fn public_api(crate_name: &str, root: &Path) -> PublicApi {
    let mut api = PublicApi::default();
    let mut visited = BTreeSet::new();
    scan_module(
        crate_name,
        root,
        String::new(),
        true,
        &mut api,
        &mut visited,
    );
    let reexports = api.wildcard_reexports.clone();
    for (destination, source) in reexports {
        let prefix = format!("{crate_name}::{source}::");
        let additions: Vec<_> = api
            .paths
            .iter()
            .filter_map(|path| {
                path.strip_prefix(&prefix)
                    .map(|tail| format!("{crate_name}::{destination}{tail}"))
            })
            .collect();
        api.paths.extend(additions);
    }
    api
}

fn scan_module(
    crate_name: &str,
    file: &Path,
    module: String,
    visible: bool,
    api: &mut PublicApi,
    visited: &mut BTreeSet<PathBuf>,
) {
    let Ok(file) = file.canonicalize() else {
        return;
    };
    if !visited.insert(file.clone()) {
        return;
    }
    let Ok(source) = fs::read_to_string(&file) else {
        return;
    };
    let Ok(parsed) = syn::parse_file(&source) else {
        return;
    };
    let module_path = if module.is_empty() {
        crate_name.to_owned()
    } else {
        format!("{crate_name}::{module}")
    };
    if visible {
        api.paths.insert(module_path.clone());
    }
    for item in parsed.items {
        match item {
            Item::Mod(item) => {
                let name = item.ident.to_string();
                let child = if module.is_empty() {
                    name.clone()
                } else {
                    format!("{module}::{name}")
                };
                let child_visible = visible && is_public(&item.vis);
                if child_visible {
                    api.paths.insert(format!("{crate_name}::{child}"));
                }
                if let Some((_, items)) = item.content {
                    scan_items(crate_name, &file, child, child_visible, items, api, visited);
                } else if let Some(path) = module_file(&file, &name, &item.attrs) {
                    scan_module(crate_name, &path, child, child_visible, api, visited);
                }
            }
            other => scan_item(crate_name, &module_path, visible, other, api),
        }
    }
}

fn scan_items(
    crate_name: &str,
    file: &Path,
    module: String,
    visible: bool,
    items: Vec<Item>,
    api: &mut PublicApi,
    visited: &mut BTreeSet<PathBuf>,
) {
    let module_path = format!("{crate_name}::{module}");
    for item in items {
        match item {
            Item::Mod(item) => {
                let name = item.ident.to_string();
                let child = format!("{module}::{name}");
                let child_visible = visible && is_public(&item.vis);
                if child_visible {
                    api.paths.insert(format!("{crate_name}::{child}"));
                }
                if let Some((_, items)) = item.content {
                    scan_items(crate_name, file, child, child_visible, items, api, visited);
                } else if let Some(path) = module_file(file, &name, &item.attrs) {
                    scan_module(crate_name, &path, child, child_visible, api, visited);
                }
            }
            other => scan_item(crate_name, &module_path, visible, other, api),
        }
    }
}

fn scan_item(crate_name: &str, module: &str, visible: bool, item: Item, api: &mut PublicApi) {
    match item {
        Item::Fn(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.sig.ident.to_string())
        }
        Item::Struct(item) if is_public(&item.vis) => {
            let name = item.ident.to_string();
            if visible {
                add_path(api, module, &name);
            }
            for field in item.fields {
                if is_public(&field.vis) {
                    if let Some(ident) = field.ident {
                        api.members.insert((name.clone(), ident.to_string()));
                    }
                }
            }
        }
        Item::Enum(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.ident.to_string())
        }
        Item::Type(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.ident.to_string())
        }
        Item::Const(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.ident.to_string())
        }
        Item::Static(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.ident.to_string())
        }
        Item::Trait(item) if visible && is_public(&item.vis) => {
            add_path(api, module, &item.ident.to_string())
        }
        Item::Use(item) if visible && is_public(&item.vis) => {
            let mut exported = Vec::new();
            flatten_use(&item.tree, String::new(), &mut exported);
            for (source, alias) in exported {
                if source.ends_with("::*") {
                    let source_module =
                        source.trim_end_matches("::*").trim_start_matches("crate::");
                    let destination = if module == crate_name {
                        alias
                    } else {
                        format!("{module}::{alias}")
                    };
                    api.wildcard_reexports
                        .push((destination, source_module.to_owned()));
                } else {
                    let destination = if module == crate_name {
                        alias
                    } else {
                        format!("{module}::{alias}")
                    };
                    if destination.starts_with(&format!("{crate_name}::")) {
                        api.paths.insert(destination);
                    } else {
                        add_path(api, crate_name, &destination);
                    }
                }
            }
        }
        Item::Impl(item) => {
            let syn::Type::Path(type_path) = &*item.self_ty else {
                return;
            };
            let Some(type_name) = type_path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
            else {
                return;
            };
            for member in item.items {
                match member {
                    syn::ImplItem::Fn(method) if is_public(&method.vis) => {
                        api.members
                            .insert((type_name.clone(), method.sig.ident.to_string()));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn flatten_use(tree: &UseTree, prefix: String, out: &mut Vec<(String, String)>) {
    match tree {
        UseTree::Path(path) => {
            let next = if prefix.is_empty() {
                path.ident.to_string()
            } else {
                format!("{prefix}::{}", path.ident)
            };
            flatten_use(&path.tree, next, out);
        }
        UseTree::Name(name) => {
            let ident = name.ident.to_string();
            let alias = if ident == "self" {
                prefix.rsplit("::").next().unwrap_or(&prefix).to_owned()
            } else {
                ident.clone()
            };
            let source = if ident == "self" {
                prefix
            } else if prefix.is_empty() {
                ident
            } else {
                format!("{prefix}::{ident}")
            };
            out.push((source, alias));
        }
        UseTree::Rename(rename) => {
            let ident = rename.ident.to_string();
            let source = if prefix.is_empty() {
                ident
            } else {
                format!("{prefix}::{ident}")
            };
            out.push((source, rename.rename.to_string()));
        }
        UseTree::Glob(_) => out.push((
            format!("{prefix}::*"),
            prefix.rsplit("::").next().unwrap_or(&prefix).to_owned(),
        )),
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use(item, prefix.clone(), out);
            }
        }
    }
}

fn module_file(parent: &Path, name: &str, attributes: &[syn::Attribute]) -> Option<PathBuf> {
    for attribute in attributes {
        if attribute.path().is_ident("path") {
            if let Ok(path) = attribute.parse_args::<syn::LitStr>() {
                return Some(parent.parent()?.join(path.value()));
            }
        }
    }
    let directory = parent.parent()?;
    let parent_name = parent.file_name().and_then(|name| name.to_str())?;
    let base = if parent_name == "lib.rs" || parent_name == "mod.rs" {
        directory.to_path_buf()
    } else {
        directory.join(parent.file_stem()?)
    };
    let file = base.join(format!("{name}.rs"));
    if file.is_file() {
        return Some(file);
    }
    let nested = base.join(name).join("mod.rs");
    nested.is_file().then_some(nested)
}

fn add_path(api: &mut PublicApi, crate_name: &str, item: &str) {
    api.paths.insert(format!("{crate_name}::{item}"));
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

fn accessor_resolves(api: &PublicApi, module: &str, accessor: &str) -> bool {
    if let Some((type_name, member)) = accessor.split_once("::") {
        api.paths.contains(&format!("{module}::{type_name}"))
            && api
                .members
                .contains(&(type_name.to_owned(), member.to_owned()))
    } else {
        api.paths.contains(&format!("{module}::{accessor}"))
    }
}
