// SPDX-License-Identifier: AGPL-3.0-or-later
//! Path-based, name-filtered merge-gate recipes for FSL changes.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

const LARGE_EMIT_FILTER: &str =
    "emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack";
const EMIT_LARGE_SKIP: &str = "a_function_body_100000_deep_projects_on_a_small_stack";
const ENGINE_FILTERS: [&str; 4] = ["monitor_", "timed_", "hyper", "probabilistic"];
const USAGE: &str = "usage: cargo xtask merge-gate-recipe <changed-path>...";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TestTarget {
    Library,
    Integration(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TestSelector {
    package: String,
    target: TestTarget,
    filter: String,
    exact: bool,
    skip: Option<String>,
    timed: bool,
}

#[derive(Default)]
struct Recipe {
    selectors: BTreeSet<TestSelector>,
    notes: BTreeSet<String>,
}

impl TestSelector {
    fn command(&self, list_only: bool) -> Vec<String> {
        let mut arguments = vec![
            "test".to_owned(),
            "--locked".to_owned(),
            "-p".to_owned(),
            self.package.clone(),
        ];
        match &self.target {
            TestTarget::Library => arguments.push("--lib".to_owned()),
            TestTarget::Integration(name) => {
                arguments.push("--test".to_owned());
                arguments.push(name.clone());
            }
        }
        arguments.push("--".to_owned());
        arguments.push(self.filter.clone());
        if self.exact {
            arguments.push("--exact".to_owned());
        }
        if let Some(skip) = &self.skip {
            arguments.push("--skip".to_owned());
            arguments.push(skip.clone());
        }
        if self.timed {
            arguments.push("--test-threads=1".to_owned());
        }
        if list_only {
            arguments.push("--list".to_owned());
        }
        arguments
    }

    fn display(&self) -> String {
        let cargo = self.command(false).join(" ");
        if self.timed {
            format!("/usr/bin/time -v cargo {cargo}")
        } else {
            format!("cargo {cargo}")
        }
    }
}

/// Print compile-first and name-filtered test commands for changed paths.
///
/// Each filter is preflighted with a filtered `--list` invocation. A filter
/// that selects no tests fails the command instead of printing a false gate.
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<String> {
    if operands.is_empty() {
        return Err(Error::Usage(USAGE));
    }
    let paths = operands
        .iter()
        .map(|operand| {
            operand
                .to_str()
                .map(normalize_path)
                .ok_or(Error::Usage(USAGE))
        })
        .collect::<Result<Vec<_>>>()?;
    if paths.iter().all(|path| path.starts_with("spec/")) {
        return Ok("NO CARGO: spec-only change.\n".to_owned());
    }
    require_main_merged(workspace_root)?;
    let test_only = paths.iter().all(|path| is_test_path(path));
    let mut recipe = plan_paths(workspace_root, &paths, test_only)?;
    for path in &paths {
        if ignore_attribute_changed(workspace_root, path)? {
            add_library(&mut recipe, "xtask", "ignored_reason_tests::".to_owned());
        }
    }

    let mut output = String::new();
    for selector in &recipe.selectors {
        preflight(workspace_root, selector)?;
    }
    for target in compile_targets(&recipe) {
        output.push_str(&format!("{}\n", no_run_command(&target)));
    }
    for selector in &recipe.selectors {
        output.push_str(&format!("{}\n", selector.display()));
    }
    for note in &recipe.notes {
        output.push_str(&format!("NOTE: {note}\n"));
    }
    output.push_str("cargo check --locked --workspace --all-targets\n");
    Ok(output)
}

fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .to_owned()
}

#[cfg(test)]
fn plan(paths: &[String]) -> Result<Recipe> {
    if paths.iter().all(|path| path.starts_with("spec/")) {
        return Ok(Recipe::default());
    }
    plan_paths(
        Path::new("."),
        paths,
        paths.iter().all(|path| is_test_path(path)),
    )
}

fn plan_paths(workspace_root: &Path, paths: &[String], test_only: bool) -> Result<Recipe> {
    let mut recipe = Recipe::default();
    for path in paths {
        if path.is_empty()
            || Path::new(path).is_absolute()
            || path.split('/').any(|part| part == "..")
        {
            return Err(Error::MergeGateRecipe(format!(
                "invalid changed path {path:?}"
            )));
        }
        if path.starts_with("spec/") {
            continue;
        }
        if test_only {
            plan_test_path(workspace_root, path, &mut recipe)?;
        } else {
            plan_path(path, &mut recipe)?;
        }
    }
    if recipe.selectors.is_empty() {
        if test_only {
            return Err(Error::MergeGateRecipe(
                "test-only paths selected no test modules".to_owned(),
            ));
        }
    }
    Ok(recipe)
}

fn plan_path(path: &str, recipe: &mut Recipe) -> Result<()> {
    if path == "Cargo.toml" || path == "Cargo.lock" || path.ends_with("/Cargo.toml") {
        add_emitter_consumers(recipe);
        return Ok(());
    }
    if path.starts_with("fsl-package/src/emit/") || path == "fsl-package/src/emit.rs" {
        add_emitter_recipe(recipe);
        return Ok(());
    }
    if path == "fsl-package/src/reader_protocol_projection.rs"
        || path == "fsl-package/src/checked_v2.rs"
    {
        add_library(recipe, "fsl-package", source_module_filter(path)?);
        add_large_input(recipe);
        return Ok(());
    }
    if path == "fsl-forms/src/spans.rs" {
        add_library(recipe, "fsl-forms", source_module_filter(path)?);
        add_large_input(recipe);
        return Ok(());
    }
    if path.starts_with("fsl-replay/src/") {
        add_library(recipe, "fsl-replay", source_module_filter(path)?);
        for filter in [
            "dependency_handoff",
            "run_requirement_items",
            "run_text_witness",
        ] {
            add_integration(recipe, "fsl-command", filter);
        }
        for filter in [
            "probabilistic_replay",
            "bounded_probabilistic_replay",
            "package_probabilistic",
        ] {
            add_integration(recipe, "fsl-analyze", filter);
        }
        return Ok(());
    }
    if path.starts_with("fsl-inspect/src/") {
        add_library(recipe, "fsl-inspect", "render::tests::".to_owned());
        add_integration(recipe, "fsl-command", "run_text_witness");
        return Ok(());
    }
    if path.starts_with("fsl-command/src/") {
        add_command_source_controls(path, recipe)?;
        return Ok(());
    }
    if path.starts_with("fsl-analyze/src/") {
        add_library(recipe, "fsl-analyze", source_module_filter(path)?);
        for filter in analyze_engine_filters(path) {
            add_integration(recipe, "fsl-analyze", filter);
        }
        return Ok(());
    }
    if path.starts_with("fsl-analyze/tests/it/") {
        add_integration(recipe, "fsl-analyze", integration_module_filter(path)?);
        return Ok(());
    }
    if path == "fsl-semantics/src/check/abstraction.rs" {
        add_integration(recipe, "fsl-analyze", "timed_");
        add_integration(
            recipe,
            "filament-spec-language",
            "abstraction_relation_inert",
        );
        return Ok(());
    }
    if path.starts_with("fsl-eval/src/") || path.starts_with("fsl-semantics/src/") {
        let package = path.split('/').next().unwrap_or_default();
        add_library(recipe, package, source_module_filter(path)?);
        add_integration(recipe, "fsl-analyze", "timed_");
        add_root_source_consumers(path, recipe);
        return Ok(());
    }
    if path.starts_with("xtask/src/") {
        if path.ends_with("/main.rs") {
            add_library(recipe, "xtask", "merge_gate_recipe::".to_owned());
        } else {
            add_library(recipe, "xtask", source_module_filter(path)?);
        }
        return Ok(());
    }
    if path.starts_with("fsl-analyze/tests/it/main.rs") {
        for filter in ENGINE_FILTERS {
            add_integration(recipe, "fsl-analyze", filter);
        }
        return Ok(());
    }
    if path.starts_with("fsl-command/tests/it/main.rs") {
        for filter in [
            "run_requirement_items",
            "run_text_witness",
            "dependency_handoff",
        ] {
            add_integration(recipe, "fsl-command", filter);
        }
        return Ok(());
    }
    if path.starts_with("fsl-command/tests/it/") {
        add_integration(recipe, "fsl-command", integration_module_filter(path)?);
        return Ok(());
    }
    if path.starts_with("tests/it/") {
        add_integration(
            recipe,
            "filament-spec-language",
            integration_module_filter(path)?,
        );
        return Ok(());
    }
    if let Some(package) = package_from_path(path) {
        if path.starts_with(&format!("{package}/src/")) {
            add_library(recipe, package, source_module_filter(path)?);
            return Ok(());
        }
        if path.starts_with(&format!("{package}/tests/it/")) {
            add_integration(recipe, package, integration_module_filter(path)?);
            return Ok(());
        }
    }
    Err(Error::MergeGateRecipe(format!(
        "no path-to-test rule for {path:?}"
    )))
}

fn plan_test_path(workspace_root: &Path, path: &str, recipe: &mut Recipe) -> Result<()> {
    if path == "tests/it/main.rs" || path.ends_with("/tests/it/main.rs") {
        let file = workspace_root.join(path);
        let source = fs::read_to_string(&file).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot read {}: {error}", file.display()))
        })?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot parse {}: {error}", file.display()))
        })?;
        let modules = syntax
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(module) if module.content.is_none() => {
                    Some(module.ident.to_string())
                }
                _ => None,
            })
            .filter(|module| {
                let module_file = file
                    .parent()
                    .unwrap_or(workspace_root)
                    .join(format!("{module}.rs"));
                let Ok(module_source) = fs::read_to_string(module_file) else {
                    return false;
                };
                let Ok(module_syntax) = syn::parse_file(&module_source) else {
                    return false;
                };
                let mut test_names = Vec::new();
                collect_test_names(&module_syntax.items, &mut Vec::new(), &mut test_names);
                !test_names.is_empty()
            })
            .collect::<Vec<_>>();
        if modules.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "changed test root {path:?} declares no test modules"
            )));
        }
        let package = if path.starts_with("tests/") {
            "filament-spec-language"
        } else {
            path.split('/').next().unwrap_or_default()
        };
        for module in modules {
            add_integration(recipe, package, module);
        }
        return Ok(());
    }
    if path.starts_with("tests/it/") {
        add_integration(
            recipe,
            "filament-spec-language",
            integration_module_filter(path)?,
        );
        return Ok(());
    }
    if path.starts_with("xtask/tests/") {
        let file = workspace_root.join(path);
        let source = fs::read_to_string(&file).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot read {}: {error}", file.display()))
        })?;
        let syntax = syn::parse_file(&source).map_err(|error| {
            Error::MergeGateRecipe(format!("cannot parse {}: {error}", file.display()))
        })?;
        let mut names = Vec::new();
        collect_test_names(&syntax.items, &mut Vec::new(), &mut names);
        if names.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "changed test file {path:?} declares no named tests"
            )));
        }
        let target = path
            .rsplit('/')
            .next()
            .and_then(|file| file.strip_suffix(".rs"))
            .ok_or_else(|| {
                Error::MergeGateRecipe(format!("changed test path {path:?} is not Rust"))
            })?;
        for name in names {
            add_target_filter(
                recipe,
                "xtask",
                TestTarget::Integration(target.to_owned()),
                name,
                true,
                None,
                false,
            );
        }
        return Ok(());
    }
    if path.starts_with("xtask/src/") {
        add_library(recipe, "xtask", source_module_filter(path)?);
        return Ok(());
    }
    if path.starts_with("fsl-analyze/tests/it/")
        || path.starts_with("fsl-command/tests/it/")
        || path.starts_with("fsl-replay/tests/it/")
        || path.starts_with("fsl-inspect/tests/it/")
    {
        let package = path.split('/').next().unwrap_or_default();
        add_integration(recipe, package, integration_module_filter(path)?);
        return Ok(());
    }
    if let Some(package) = package_from_path(path) {
        add_library(recipe, package, source_module_filter(path)?);
        return Ok(());
    }
    Err(Error::MergeGateRecipe(format!(
        "no test-module rule for {path:?}"
    )))
}

fn is_test_path(path: &str) -> bool {
    path.split('/').any(|component| component == "tests")
        || path.ends_with("_tests.rs")
        || path.ends_with("/tests.rs")
}

fn add_emitter_recipe(recipe: &mut Recipe) {
    add_target_filter(
        recipe,
        "fsl-package",
        TestTarget::Library,
        "emit::",
        false,
        Some(EMIT_LARGE_SKIP),
        false,
    );
    add_emitter_consumers(recipe);
    add_large_input(recipe);
}

fn add_emitter_consumers(recipe: &mut Recipe) {
    for filter in ENGINE_FILTERS {
        add_integration(recipe, "fsl-analyze", filter);
    }
    add_integration(recipe, "fsl-command", "run_requirement_items");
}

fn add_large_input(recipe: &mut Recipe) {
    add_target_filter(
        recipe,
        "fsl-package",
        TestTarget::Library,
        LARGE_EMIT_FILTER,
        true,
        None,
        true,
    );
}

fn add_command_source_controls(path: &str, recipe: &mut Recipe) -> Result<()> {
    recipe.notes.insert(
        "driver pin move: run driver targets frontend_cli, binary_process, cli_contract".to_owned(),
    );
    let lower = path.to_ascii_lowercase();
    let mut filters = BTreeSet::new();
    if lower.contains("package_run") || lower.contains("requirements") || lower.contains("items") {
        filters.insert("run_requirement_items".to_owned());
    }
    if lower.contains("package_run") {
        filters.insert("package_run".to_owned());
    }
    if lower.contains("package_run") || lower.contains("dependency") {
        filters.insert("dependency_handoff".to_owned());
    }
    if lower.contains("output") || lower.contains("run") || lower.contains("settle") {
        filters.insert("run_text_witness".to_owned());
    }
    if lower.contains("monitor") {
        filters.insert("cli_contract::monitor".to_owned());
    }
    if filters.is_empty() {
        let filter = source_module_filter(path)?;
        let stem = filter
            .trim_end_matches("::")
            .rsplit("::")
            .next()
            .unwrap_or_default();
        filters.insert(
            match stem {
                "analyze" | "clause" | "compile" | "export" | "monitor" | "run" => "cli_contract",
                "package_run" => "run_requirement_items",
                _ => stem,
            }
            .to_owned(),
        );
    }
    for filter in filters {
        add_integration(recipe, "fsl-command", filter);
    }
    Ok(())
}

fn add_root_source_consumers(path: &str, recipe: &mut Recipe) {
    add_integration(recipe, "filament-spec-language", root_test_filter(path));
}

fn root_test_filter(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.contains("bare") || lower.contains("expression") {
        "bare_".to_owned()
    } else if lower.contains("abstraction") {
        "abstraction_relation_inert".to_owned()
    } else if lower.contains("protocol") {
        "protocol_".to_owned()
    } else {
        lower
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".rs")
            .to_owned()
    }
}

fn add_target_filter(
    recipe: &mut Recipe,
    package: &str,
    target: TestTarget,
    filter: impl Into<String>,
    exact: bool,
    skip: Option<&str>,
    timed: bool,
) {
    recipe.selectors.insert(TestSelector {
        package: package.to_owned(),
        target,
        filter: filter.into(),
        exact,
        skip: skip.map(str::to_owned),
        timed,
    });
}

fn compile_targets(recipe: &Recipe) -> BTreeSet<(String, TestTarget)> {
    recipe
        .selectors
        .iter()
        .map(|selector| (selector.package.clone(), selector.target.clone()))
        .collect()
}

fn no_run_command(target: &(String, TestTarget)) -> String {
    let mut command = format!("cargo test --locked -p {}", target.0);
    match &target.1 {
        TestTarget::Library => command.push_str(" --lib"),
        TestTarget::Integration(name) => command.push_str(&format!(" --test {name}")),
    }
    command.push_str(" --no-run");
    command
}

fn require_main_merged(workspace_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(workspace_root)
        .args(["merge-base", "--is-ancestor", "origin/main", "HEAD"])
        .output()
        .map_err(|error| {
            Error::MergeGateRecipe(format!("cannot check origin/main merge: {error}"))
        })?;
    if !output.status.success() {
        return Err(Error::MergeGateRecipe(
            "merge origin/main into the PR head before generating the merge recipe".to_owned(),
        ));
    }
    Ok(())
}

fn ignore_attribute_changed(workspace_root: &Path, path: &str) -> Result<bool> {
    let mut changed = false;
    for args in [
        vec![
            "diff",
            "--no-ext-diff",
            "--unified=0",
            "origin/main...HEAD",
            "--",
            path,
        ],
        vec![
            "diff",
            "--no-ext-diff",
            "--unified=0",
            "--cached",
            "--",
            path,
        ],
        vec!["diff", "--no-ext-diff", "--unified=0", "--", path],
    ] {
        let output = Command::new("git")
            .current_dir(workspace_root)
            .args(&args)
            .output()
            .map_err(|error| {
                Error::MergeGateRecipe(format!("cannot inspect diff for {path:?}: {error}"))
            })?;
        if !output.status.success() {
            return Err(Error::MergeGateRecipe(format!(
                "git diff failed for {path:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        changed |= String::from_utf8_lossy(&output.stdout).lines().any(|line| {
            (line.starts_with('+') || line.starts_with('-'))
                && !line.starts_with("+++")
                && !line.starts_with("---")
                && line.contains("#[ignore")
        });
    }
    Ok(changed)
}

fn package_from_path(path: &str) -> Option<&str> {
    let package = path.split('/').next()?;
    (package.starts_with("fsl-") && package.len() > 4).then_some(package)
}

fn source_module_filter(path: &str) -> Result<String> {
    module_filter(path, "/src/")
}

fn integration_module_filter(path: &str) -> Result<String> {
    if let Some(relative) = path.strip_prefix("tests/it/") {
        return module_name_filter(relative, path)
            .map(|filter| filter.trim_end_matches("::").to_owned());
    }
    module_filter(path, "/tests/it/").map(|filter| filter.trim_end_matches("::").to_owned())
}

fn module_filter(path: &str, marker: &str) -> Result<String> {
    let (_, relative) = path
        .split_once(marker)
        .ok_or_else(|| Error::MergeGateRecipe(format!("no module path in {path:?}")))?;
    module_name_filter(relative, path)
}

fn module_name_filter(relative: &str, path: &str) -> Result<String> {
    let mut modules = relative.split('/').map(str::to_owned).collect::<Vec<_>>();
    let Some(file) = modules.pop() else {
        return Err(Error::MergeGateRecipe(format!(
            "no module file in {path:?}"
        )));
    };
    if file == "mod.rs" {
        if modules.is_empty() {
            return Err(Error::MergeGateRecipe(format!(
                "crate-root module {path:?} needs an explicit named test area"
            )));
        }
    } else if let Some(stem) = file.strip_suffix(".rs") {
        if stem == "lib" || stem == "main" {
            return Err(Error::MergeGateRecipe(format!(
                "crate-root module {path:?} needs an explicit named test area"
            )));
        }
        modules.push(stem.to_owned());
    } else {
        return Err(Error::MergeGateRecipe(format!(
            "changed path {path:?} is not a Rust module"
        )));
    }
    if modules.is_empty() {
        return Err(Error::MergeGateRecipe(format!(
            "changed path {path:?} does not identify a test module"
        )));
    }
    Ok(format!("{}::", modules.join("::")))
}

fn analyze_engine_filters(path: &str) -> Vec<&'static str> {
    let lower = path.to_ascii_lowercase();
    let mut filters = Vec::new();
    if lower.contains("monitor") {
        filters.push("monitor_");
    }
    if lower.contains("timed") || lower.contains("schedulability") || lower.contains("zone_check") {
        filters.push("timed_");
    }
    if lower.contains("hyper") {
        filters.push("hyper");
    }
    if lower.contains("probabilistic") || lower.contains("statistical") {
        filters.push("probabilistic");
    }
    if filters.is_empty() && (lower.ends_with("/outcome.rs") || lower.contains("/outcome/")) {
        return ENGINE_FILTERS.to_vec();
    }
    filters
}

fn collect_test_names(items: &[syn::Item], modules: &mut Vec<String>, names: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Fn(function)
                if function.attrs.iter().any(|attribute| {
                    attribute
                        .path()
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == "test")
                }) =>
            {
                let mut path = modules.clone();
                path.push(function.sig.ident.to_string());
                names.push(path.join("::"));
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    modules.push(module.ident.to_string());
                    collect_test_names(nested, modules, names);
                    modules.pop();
                }
            }
            _ => {}
        }
    }
}

fn add_library(recipe: &mut Recipe, package: &str, filter: String) {
    add_target_filter(
        recipe,
        package,
        TestTarget::Library,
        filter,
        false,
        None,
        false,
    );
}

fn add_integration(recipe: &mut Recipe, package: &str, filter: impl Into<String>) {
    add_target_filter(
        recipe,
        package,
        TestTarget::Integration("it".to_owned()),
        filter,
        false,
        None,
        false,
    );
}

fn preflight(workspace_root: &Path, selector: &TestSelector) -> Result<usize> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .current_dir(workspace_root)
        .args(selector.command(true))
        .output()
        .map_err(|error| {
            Error::MergeGateRecipe(format!(
                "cannot list tests for {:?}: {error}",
                selector.display()
            ))
        })?;
    if !output.status.success() {
        return Err(Error::MergeGateRecipe(format!(
            "test-filter preflight failed for {:?}: {}{}",
            selector.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let listed = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.trim_end().ends_with(": test"))
        .count();
    if listed == 0 {
        return Err(Error::MergeGateRecipe(format!(
            "test filter {:?} matched no tests; refusing to print a green recipe",
            selector.display()
        )));
    }
    Ok(listed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitter_paths_add_unit_consumers_and_isolated_large_input() {
        let recipe = plan(&["fsl-package/src/emit.rs".to_owned()]).expect("known path");
        assert!(recipe.selectors.contains(&TestSelector {
            package: "fsl-package".to_owned(),
            target: TestTarget::Library,
            filter: "emit::".to_owned(),
            exact: false,
            skip: Some(EMIT_LARGE_SKIP.to_owned()),
            timed: false,
        }));
        for filter in ENGINE_FILTERS {
            assert!(recipe.selectors.iter().any(|selector| {
                selector.package == "fsl-analyze" && selector.filter == filter
            }));
        }
        assert!(recipe.selectors.iter().any(|selector| {
            selector.package == "fsl-command" && selector.filter == "run_requirement_items"
        }));
        let large = recipe
            .selectors
            .iter()
            .find(|selector| selector.filter == LARGE_EMIT_FILTER)
            .expect("large input selected");
        assert!(large.exact && large.timed && large.skip.is_none());
    }

    #[test]
    fn engine_paths_select_their_consumer_modules() {
        let timed = plan(&["fsl-analyze/src/zone_check/outcome.rs".to_owned()])
            .expect("engine outcome path");
        assert!(
            timed
                .selectors
                .iter()
                .any(|selector| selector.filter == "timed_")
        );
        assert_eq!(timed.selectors.len(), 2);
        let hyper =
            plan(&["fsl-analyze/src/model_check/hyper/closed.rs".to_owned()]).expect("hyper path");
        assert!(
            hyper
                .selectors
                .iter()
                .any(|selector| selector.filter == "hyper")
        );
    }

    #[test]
    fn replay_inspect_and_command_changes_route_consumer_controls() {
        let replay = plan(&["fsl-replay/src/planner/outcomes.rs".to_owned()]).expect("replay path");
        for filter in [
            "dependency_handoff",
            "run_requirement_items",
            "run_text_witness",
        ] {
            assert!(
                replay
                    .selectors
                    .iter()
                    .any(|selector| selector.filter == filter)
            );
        }
        for filter in [
            "probabilistic_replay",
            "bounded_probabilistic_replay",
            "package_probabilistic",
        ] {
            assert!(
                replay
                    .selectors
                    .iter()
                    .any(|selector| selector.filter == filter)
            );
        }
        let inspect = plan(&["fsl-inspect/src/render.rs".to_owned()]).expect("inspect path");
        assert!(
            inspect
                .selectors
                .iter()
                .any(|selector| selector.filter == "render::tests::")
        );
        assert!(
            inspect
                .selectors
                .iter()
                .any(|selector| selector.filter == "run_text_witness")
        );
        let command =
            plan(&["fsl-command/src/command/package_run.rs".to_owned()]).expect("command path");
        assert!(
            command
                .notes
                .iter()
                .any(|note| note.contains("frontend_cli, binary_process, cli_contract"))
        );
    }

    #[test]
    fn special_large_input_paths_select_the_timed_control() {
        for path in [
            "fsl-package/src/reader_protocol_projection.rs",
            "fsl-package/src/checked_v2.rs",
            "fsl-forms/src/spans.rs",
        ] {
            let recipe = plan(&[path.to_owned()]).expect("large-input rule");
            assert!(recipe.selectors.iter().any(|selector| {
                selector.filter == LARGE_EMIT_FILTER && selector.timed && selector.exact
            }));
        }
    }

    #[test]
    fn specs_only_changes_do_not_emit_cargo_controls() {
        let recipe = plan(&["spec/requirements.md".to_owned()]).expect("spec path");
        assert!(recipe.selectors.is_empty());
    }

    #[test]
    fn test_name_extraction_keeps_module_paths_and_qualified_test_attributes() {
        let syntax = syn::parse_file(
            "mod nested { #[tokio::test] async fn sample() {} } #[test] fn root_case() {}",
        )
        .expect("syntactically valid Rust");
        let mut names = Vec::new();
        collect_test_names(&syntax.items, &mut Vec::new(), &mut names);
        assert_eq!(names, ["nested::sample", "root_case"]);
    }

    #[test]
    fn integration_test_paths_keep_their_named_module_filter() {
        let recipe = plan(&["fsl-analyze/tests/it/analyze_package_value.rs".to_owned()])
            .expect("integration test path");
        assert!(recipe.selectors.iter().any(|selector| {
            selector.package == "fsl-analyze"
                && selector.target == TestTarget::Integration("it".to_owned())
                && selector.filter == "analyze_package_value"
        }));
    }

    #[test]
    fn crate_root_and_unsafe_paths_fail_closed() {
        assert!(plan(&["fsl-analyze/src/lib.rs".to_owned()]).is_err());
        assert!(plan(&["../outside.rs".to_owned()]).is_err());
    }

    #[test]
    fn rendered_large_input_command_is_exact_and_serial() {
        let selector = TestSelector {
            package: "fsl-package".to_owned(),
            target: TestTarget::Library,
            filter: LARGE_EMIT_FILTER.to_owned(),
            exact: true,
            skip: None,
            timed: true,
        };
        assert_eq!(
            selector.display(),
            "/usr/bin/time -v cargo test --locked -p fsl-package --lib -- emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack --exact --test-threads=1"
        );
    }

    #[test]
    fn pr_3495_paths_keep_the_regression_controls_in_the_recipe() {
        let paths = [
            "Cargo.lock",
            "Cargo.toml",
            "fsl-analyze/tests/it/analysis_view.rs",
            "fsl-analyze/tests/it/analyze_package_value.rs",
            "fsl-command/src/command/package_run.rs",
            "fsl-command/tests/it/run_requirement_items.rs",
            "fsl-eval/src/value/expression/bare.rs",
            "fsl-package/src/emit.rs",
            "fsl-package/src/emit/tests.rs",
            "fsl-package/src/emit/tests/admission_corpus.rs",
            "fsl-package/src/emit/value_validity_requirements.rs",
            "fsl-package/src/emit/value_validity_requirements/tests.rs",
            "fsl-package/src/reader_protocol_projection.rs",
            "fsl-semantics/src/check/abstraction.rs",
        ]
        .map(str::to_owned);
        let recipe = plan(&paths).expect("PR #3495 paths have a recipe");
        assert!(recipe.selectors.iter().any(|selector| {
            selector.package == "fsl-analyze"
                && selector.target == TestTarget::Integration("it".to_owned())
                && selector.filter == "analyze_package_value"
        }));
        assert!(recipe.selectors.iter().any(|selector| {
            selector.package == "fsl-command" && selector.filter == "run_requirement_items"
        }));
        assert!(
            recipe
                .selectors
                .iter()
                .any(|selector| selector.filter == "emit::")
        );
        assert!(
            recipe
                .selectors
                .iter()
                .any(|selector| selector.filter == LARGE_EMIT_FILTER)
        );
    }
}
