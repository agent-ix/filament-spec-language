// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keep every ignored test in the workspace owned by a ticket or a real CI lane. Ticket state is checked in the tracker, not here.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Lit, Meta, Token};

fn ci_targets(makefile: &str) -> BTreeSet<&str> {
    let lines: Vec<_> = makefile.lines().collect();
    let declared = lines
        .iter()
        .filter_map(|line| line.strip_prefix("CI_CHECKS :="))
        .flat_map(|line| line.split_whitespace());
    declared
        .filter(|target| {
            lines.iter().enumerate().any(|(index, line)| {
                line.strip_prefix(*target).is_some_and(|rest| rest == ":")
                    && lines[index + 1..]
                        .iter()
                        .take_while(|line| {
                            line.starts_with('\t')
                                || line.trim().is_empty()
                                || line.starts_with('#')
                        })
                        .any(|line| line.starts_with('\t') && !line.trim().is_empty())
            })
        })
        .collect()
}

/// A reason is owned when it starts with a ticket id of a known team followed
/// by a one-line reason, or names a `make` target that is a real CI lane.
/// Whether the ticket is still open is checked in the tracker, not here.
fn owned_reason(reason: &str, ci: &BTreeSet<&str>) -> bool {
    let words: Vec<_> = reason
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_')
        })
        .filter(|word| !word.is_empty())
        .collect();
    let ticket = reason
        .split_once(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_')
        })
        .is_some_and(|(id, rest)| {
            let Some((team, number)) = id.split_once('-') else {
                return false;
            };
            matches!(team, "IR" | "QSL" | "STD")
                && !number.is_empty()
                && !number.starts_with('0')
                && number.bytes().all(|byte| byte.is_ascii_digit())
                && !rest.trim().is_empty()
        });
    ticket
        || words
            .windows(2)
            .any(|words| words[0] == "make" && ci.contains(words[1]))
}

#[derive(Debug, Eq, PartialEq)]
struct UnownedIgnore {
    function: String,
    line: usize,
}

struct Scanner<'a> {
    ci: &'a BTreeSet<&'a str>,
    rejected: Vec<UnownedIgnore>,
}

impl Scanner<'_> {
    fn inspect(&mut self, function: &syn::Ident, meta: &Meta) {
        if meta.path().is_ident("cfg_attr") {
            if let Meta::List(list) = meta {
                let attributes = list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                    .expect("a cfg_attr contains a condition and Rust attributes");
                for attribute in attributes.iter().skip(1) {
                    self.inspect(function, attribute);
                }
            }
        } else if meta.path().is_ident("ignore") {
            let owned = match meta {
                Meta::NameValue(value) => match &value.value {
                    Expr::Lit(value) => match &value.lit {
                        Lit::Str(reason) => owned_reason(&reason.value(), self.ci),
                        _ => false,
                    },
                    _ => false,
                },
                _ => false,
            };
            if !owned {
                self.rejected.push(UnownedIgnore {
                    function: function.to_string(),
                    line: meta.span().start().line,
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        for attribute in &function.attrs {
            self.inspect(&function.sig.ident, &attribute.meta);
        }
        syn::visit::visit_item_fn(self, function);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        for attribute in &function.attrs {
            self.inspect(&function.sig.ident, &attribute.meta);
        }
        syn::visit::visit_impl_item_fn(self, function);
    }
}

fn rejected(file: &syn::File, ci: &BTreeSet<&str>) -> Vec<UnownedIgnore> {
    let mut scanner = Scanner {
        ci,
        rejected: Vec::new(),
    };
    scanner.visit_file(file);
    scanner.rejected
}

const MAKEFILE: &str = "CI_CHECKS := seam-probe empty\n\
seam-probe:\n\tcargo run --package xtask -- seam-probe\n\
local-only:\n\tcargo test\n\
empty:\n";

fn rejected_names(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("valid Rust attribute fixtures");
    rejected(&file, &ci_targets(MAKEFILE))
        .into_iter()
        .map(|finding| finding.function)
        .collect()
}

/// Provenance: QSL-2852.
#[test]
fn fixtures_admit_ticket_and_ci_owned_ignore_attributes() {
    let names = rejected_names(
        r##"
        #[test] #[ignore = "IR-1321: protocol read-back work remains gated"] fn ir() {}
        #[test] #[ignore = "QSL-2109: restored behavior"] fn qsl() {}
        #[test] #[ignore = "STD-663: metadata contract"] fn std() {}
        #[test] #[ignore = "IR-1320 (STD-654): composite leaves"] fn parenthesised() {}
        #[test] #[ignore = "slow: runs under `make seam-probe` (CI_CHECKS)"] fn lane() {}
        #[test] #[cfg_attr(feature = "slow", ignore = "IR-1321: owned")] fn conditional() {}
        struct Holder;
        impl Holder { #[test] #[ignore = "QSL-2109: method"] fn method() {} }
        #[test] fn live() { let _text = "#[ignore]"; }
        "##,
    );
    assert!(names.is_empty(), "{names:?}");
}

/// Provenance: QSL-2109.
#[test]
fn fixtures_reject_missing_and_unowned_ignore_attributes() {
    let names = rejected_names(
        r#"
        mod nested {
            #[test] #[ignore] fn missing() {}
            #[test] #[ignore = ""] fn empty() {}
            #[test] #[ignore = "slow: blocked on reader"] fn prose() {}
            #[test] #[ignore = "FR-726: requirement only"] fn requirement() {}
            #[test] #[ignore = "QSL-2109later"] fn malformed_ticket() {}
            #[test] #[ignore = "see QSL-2109: not leading"] fn ticket_not_first() {}
            #[test] #[ignore = "QSL-2109"] fn bare_ticket() {}
            #[test] #[ignore = "ZZZ-2109: unknown team"] fn unknown_team() {}
            #[test] #[ignore = "make nonexistent"] fn absent_target() {}
            #[test] #[ignore = "make local-only"] fn non_ci_target() {}
            #[test] #[ignore = "make empty"] fn no_recipe() {}
            #[test] #[cfg_attr(test, cfg_attr(feature = "slow", ignore))] fn conditional() {}
        }
        "#,
    );
    assert_eq!(
        names,
        [
            "missing",
            "empty",
            "prose",
            "requirement",
            "malformed_ticket",
            "ticket_not_first",
            "bare_ticket",
            "unknown_team",
            "absent_target",
            "non_ci_target",
            "no_recipe",
            "conditional",
        ],
    );
}

/// Every `.rs` file in the workspace outside build output, VCS state and
/// vendored trees, relative to the workspace root.
fn workspace_rust_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read a workspace directory") {
            let path = entry.expect("read a directory entry").path();
            let name = path.file_name().map(|name| name.to_string_lossy());
            if path.is_dir() {
                if !name.is_some_and(|name| {
                    matches!(
                        name.as_ref(),
                        "target" | ".git" | ".worktrees" | "node_modules" | "fuzz"
                    )
                }) {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path.strip_prefix(root).expect("a path under the root");
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.sort();
    files
}

/// Provenance: QSL-2852. Runs in the default xtask library suite in CI.
#[test]
fn every_ignored_test_in_the_workspace_cites_a_ticket_or_ci_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace root");
    let makefile = fs::read_to_string(root.join("Makefile")).expect("read actual CI targets");
    let ci = ci_targets(&makefile);
    let mut findings = Vec::new();
    for path in workspace_rust_files(root) {
        let text = fs::read_to_string(root.join(&path)).expect("read a workspace source file");
        if !text.contains("ignore") {
            continue;
        }
        match syn::parse_file(&text) {
            Ok(file) => findings.extend(
                rejected(&file, &ci)
                    .into_iter()
                    .map(|finding| format!("{path}:{}: {}", finding.line, finding.function)),
            ),
            // A file that cannot be parsed cannot be audited: it may hold no
            // ignore at all, so only a literal attribute is a finding.
            Err(_) if text.contains("#[ignore") => {
                findings.push(format!("{path}: unparsable source with an ignore attribute"))
            }
            Err(_) => {}
        }
    }
    assert!(
        findings.is_empty(),
        "ignored tests need a leading IR/QSL/STD ticket id and reason, or `make <CI_CHECKS target>` with a recipe:\n{}",
        findings.join("\n"),
    );
}
