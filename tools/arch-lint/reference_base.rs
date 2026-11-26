// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051-AC-18: the definitional reference cannot name producer model
//! stepping or a producer reward evaluator.

use std::{
    fs,
    path::{Path, PathBuf},
};

use syn::visit::Visit;

use crate::error::{Error, Result};

/// One forbidden producer path in reference source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Finding {
    /// Source file, relative to the FSL root.
    pub file: PathBuf,
    /// One-based source line.
    pub line: usize,
    /// Stable name of the forbidden construct.
    pub construct: &'static str,
}

#[derive(Default)]
struct Paths(Vec<Finding>);

impl<'ast> Visit<'ast> for Paths {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let names: Vec<_> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if names.windows(4).any(|window| {
            window[0] == "fsl_eval"
                && window[1] == "simulation"
                && window[2] == "model"
                && window[3] == "ModelSystem"
        }) {
            self.0.push(Finding {
                file: PathBuf::new(),
                line: path.segments[0].ident.span().start().line,
                construct: "fsl_eval::simulation::model::ModelSystem",
            });
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if let Some((_, trait_path, _)) = &item.trait_ {
            let names: Vec<_> = trait_path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            if names.first().is_some_and(|name| name == "fsl_eval")
                && names.last().is_some_and(|name| name == "RewardEvaluator")
            {
                self.0.push(Finding {
                    file: PathBuf::new(),
                    line: trait_path.segments[0].ident.span().start().line,
                    construct: "fsl_eval RewardEvaluator implementation",
                });
            }
        }
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        fn leaves(tree: &syn::UseTree, prefix: &mut Vec<String>, rows: &mut Vec<Vec<String>>) {
            match tree {
                syn::UseTree::Path(path) => {
                    prefix.push(path.ident.to_string());
                    leaves(&path.tree, prefix, rows);
                    prefix.pop();
                }
                syn::UseTree::Name(name) => {
                    let mut names = prefix.clone();
                    names.push(name.ident.to_string());
                    rows.push(names);
                }
                syn::UseTree::Rename(rename) => {
                    let mut names = prefix.clone();
                    names.push(rename.ident.to_string());
                    rows.push(names);
                }
                syn::UseTree::Group(group) => {
                    for tree in &group.items {
                        leaves(tree, prefix, rows);
                    }
                }
                syn::UseTree::Glob(_) => {}
            }
        }
        let mut rows = Vec::new();
        leaves(&item.tree, &mut Vec::new(), &mut rows);
        for names in rows {
            if names.windows(4).any(|window| {
                window[0] == "fsl_eval"
                    && window[1] == "simulation"
                    && window[2] == "model"
                    && window[3] == "ModelSystem"
            }) {
                self.0.push(Finding {
                    file: PathBuf::new(),
                    line: item.use_token.span.start().line,
                    construct: "fsl_eval::simulation::model::ModelSystem",
                });
            }
        }
        syn::visit::visit_item_use(self, item);
    }
}

fn visit_rs_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory).map_err(|error| Error::io(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(directory, error))?;
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Scan the reference's non-test source for forbidden producer paths.
pub(crate) fn scan(fsl_root: &Path) -> Result<Vec<Finding>> {
    let source_root = fsl_root.join("fsl-definitional-reference/src");
    if !source_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    visit_rs_files(&source_root, &mut files)?;
    files.sort();
    let mut findings = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
        let syntax = syn::parse_file(&text).map_err(|source| Error::source_parse(&path, source))?;
        let mut paths = Paths::default();
        paths.visit_file(&syntax);
        let relative = path.strip_prefix(fsl_root).unwrap_or(&path).to_path_buf();
        findings.extend(paths.0.into_iter().map(|mut finding| {
            finding.file = relative.clone();
            finding
        }));
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::scan;
    use ix_trace_rs::trace;
    use std::{fs, path::Path};

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture has parent"))
            .expect("create fixture dirs");
        fs::write(path, source).expect("write fixture source");
    }

    #[trace("FR-1051-AC-18")]
    #[test]
    fn reference_scan_rejects_model_system_and_reward_evaluator_paths() {
        let fixture = tempfile::tempdir().expect("temporary fixture");
        let root = fixture.path();
        write(
            root,
            "fsl-definitional-reference/src/lib.rs",
            "use fsl_eval::simulation::model::ModelSystem;\n\
             struct R;\n\
             impl fsl_eval::value::RewardEvaluator for R { fn evaluate(&self) {} }\n",
        );

        let findings = scan(root).expect("the reference fixture parses");
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings.iter().any(|finding| {
            finding.construct == "fsl_eval::simulation::model::ModelSystem" && finding.line == 1
        }));
        assert!(findings.iter().any(|finding| {
            finding.construct == "fsl_eval RewardEvaluator implementation" && finding.line == 3
        }));
    }
}
