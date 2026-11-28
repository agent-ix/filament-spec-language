// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051-AC-18: the definitional reference cannot name producer model
//! stepping or a producer reward evaluator.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use ix_trace_rs::implements;
use syn::visit::Visit;

use crate::error::{Code, Error, Result};

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
struct Imports {
    model_aliases: BTreeSet<String>,
    value_aliases: BTreeSet<String>,
    model_glob: bool,
    value_glob: bool,
}

fn path_is(path: &[String], expected: &[&str]) -> bool {
    path.len() == expected.len()
        && path
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == expected)
}

impl Imports {
    fn record(
        &mut self,
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        findings: &mut Vec<(usize, &'static str)>,
        line: usize,
    ) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.record(&path.tree, prefix, findings, line);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let mut path = prefix.clone();
                path.push(name.ident.to_string());
                self.record_leaf(&path, None, findings, line);
            }
            syn::UseTree::Rename(rename) => {
                let mut path = prefix.clone();
                path.push(rename.ident.to_string());
                self.record_leaf(&path, Some(rename.rename.to_string()), findings, line);
            }
            syn::UseTree::Group(group) => {
                for member in &group.items {
                    self.record(member, prefix, findings, line);
                }
            }
            syn::UseTree::Glob(_) => {
                if path_is(prefix, &["fsl_eval", "simulation", "model"]) {
                    self.model_glob = true;
                }
                if path_is(prefix, &["fsl_eval", "value"]) {
                    self.value_glob = true;
                }
            }
        }
    }

    fn record_leaf(
        &mut self,
        path: &[String],
        alias: Option<String>,
        findings: &mut Vec<(usize, &'static str)>,
        line: usize,
    ) {
        if path_is(path, &["fsl_eval", "simulation", "model", "ModelSystem"]) {
            findings.push((line, "fsl_eval::simulation::model::ModelSystem"));
        }
        if path_is(path, &["fsl_eval", "value", "S6aRewardEvaluator"]) {
            findings.push((line, "fsl_eval::value::S6aRewardEvaluator"));
        }
        if path_is(path, &["fsl_eval", "simulation", "model"]) {
            if let Some(alias) = &alias {
                self.model_aliases.insert(alias.clone());
            }
        }
        if path_is(path, &["fsl_eval", "value"]) {
            if let Some(alias) = &alias {
                self.value_aliases.insert(alias.clone());
            }
        }
    }

    fn from_file(file: &syn::File) -> (Self, Vec<(usize, &'static str)>) {
        struct Uses {
            imports: Imports,
            findings: Vec<(usize, &'static str)>,
        }
        impl<'ast> Visit<'ast> for Uses {
            fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
                self.imports.record(
                    &item.tree,
                    &mut Vec::new(),
                    &mut self.findings,
                    item.use_token.span.start().line,
                );
                syn::visit::visit_item_use(self, item);
            }
        }
        let mut uses = Uses {
            imports: Self::default(),
            findings: Vec::new(),
        };
        uses.visit_file(file);
        (uses.imports, uses.findings)
    }
}

struct Paths<'a> {
    imports: &'a Imports,
    findings: Vec<(usize, &'static str)>,
}

impl<'ast> Visit<'ast> for Paths<'_> {
    fn visit_item_use(&mut self, _item: &'ast syn::ItemUse) {}

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let names: Vec<_> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let line = path.segments[0].ident.span().start().line;
        let direct_model = names.windows(4).any(|window| {
            window[0] == "fsl_eval"
                && window[1] == "simulation"
                && window[2] == "model"
                && window[3] == "ModelSystem"
        });
        let alias_model = names.len() >= 2
            && self.imports.model_aliases.contains(&names[0])
            && names[1] == "ModelSystem";
        let glob_model = self.imports.model_glob && path_is(&names, &["ModelSystem"]);
        if direct_model || alias_model || glob_model {
            self.findings
                .push((line, "fsl_eval::simulation::model::ModelSystem"));
        }

        let direct_reward = names.windows(3).any(|window| {
            window[0] == "fsl_eval" && window[1] == "value" && window[2] == "S6aRewardEvaluator"
        });
        let alias_reward = names.len() >= 2
            && self.imports.value_aliases.contains(&names[0])
            && names[1] == "S6aRewardEvaluator";
        let glob_reward = self.imports.value_glob && path_is(&names, &["S6aRewardEvaluator"]);
        if direct_reward || alias_reward || glob_reward {
            self.findings
                .push((line, "fsl_eval::value::S6aRewardEvaluator"));
        }
        syn::visit::visit_path(self, path);
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

/// Scan the reference source for evaluator and model stepping paths.
#[implements("FR-1051-AC-18")]
pub(crate) fn scan(fsl_root: &Path) -> Result<Vec<Finding>> {
    let source_root = fsl_root.join("fsl-definitional-reference/src");
    if !source_root.is_dir() {
        return Err(Error::new(
            Code::Usage,
            format!(
                "definitional reference source root does not exist: {}",
                source_root.display()
            ),
        ));
    }
    let mut files = Vec::new();
    visit_rs_files(&source_root, &mut files)?;
    files.sort();
    let mut findings = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
        let syntax = syn::parse_file(&text).map_err(|source| Error::source_parse(&path, source))?;
        let (imports, mut paths) = Imports::from_file(&syntax);
        let mut visitor = Paths {
            imports: &imports,
            findings: Vec::new(),
        };
        visitor.visit_file(&syntax);
        paths.extend(visitor.findings);
        let relative = path.strip_prefix(fsl_root).unwrap_or(&path).to_path_buf();
        findings.extend(paths.into_iter().map(|(line, construct)| Finding {
            file: relative.clone(),
            line,
            construct,
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
    fn reference_scan_rejects_evaluator_alias_and_glob_imports() {
        let fixture = tempfile::tempdir().expect("temporary fixture");
        let root = fixture.path();
        write(
            root,
            "fsl-definitional-reference/src/evaluator.rs",
            "use fsl_eval::value::S6aRewardEvaluator;\n",
        );
        write(
            root,
            "fsl-definitional-reference/src/alias.rs",
            "use fsl_eval::simulation::model as stepping;\n\
             fn f() { let _: Option<stepping::ModelSystem> = None; }\n",
        );
        write(
            root,
            "fsl-definitional-reference/src/glob.rs",
            "use fsl_eval::simulation::model::*;\n\
             fn f() { let _: Option<ModelSystem> = None; }\n",
        );

        let findings = scan(root).expect("the reference fixture parses");
        assert_eq!(findings.len(), 3, "{findings:?}");
        assert!(findings.iter().any(|finding| {
            finding.construct == "fsl_eval::value::S6aRewardEvaluator"
                && finding.file.ends_with("evaluator.rs")
        }));
        assert!(findings.iter().any(|finding| {
            finding.construct == "fsl_eval::simulation::model::ModelSystem"
                && finding.file.ends_with("alias.rs")
        }));
        assert!(findings.iter().any(|finding| {
            finding.construct == "fsl_eval::simulation::model::ModelSystem"
                && finding.file.ends_with("glob.rs")
        }));
    }

    #[trace("FR-1051-AC-18")]
    #[test]
    fn reference_scan_fails_when_reference_source_is_missing() {
        let fixture = tempfile::tempdir().expect("temporary fixture");
        assert!(scan(fixture.path()).is_err());
    }
}
