// SPDX-License-Identifier: AGPL-3.0-or-later
//! Keep ignored package/IR and seam tests owned by a ticket or a real CI lane.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Lit, Meta, Token};

use crate::definition_scan::{parse_file, source_files};

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

fn owned_reason(reason: &str, ci: &BTreeSet<&str>) -> bool {
    let words: Vec<_> = reason
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && !matches!(character, '-' | '_')
        })
        .filter(|word| !word.is_empty())
        .collect();
    let ticket = words.iter().any(|word| {
        let Some((team, number)) = word.split_once('-') else {
            return false;
        };
        matches!(team, "IR" | "QSL" | "STD")
            && !number.is_empty()
            && !number.starts_with('0')
            && number.bytes().all(|byte| byte.is_ascii_digit())
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

/// Provenance: QSL-2109.
#[test]
fn fixtures_admit_ticket_and_ci_owned_ignore_attributes() {
    let file = syn::parse_file(
        r##"
        #[test] #[ignore = "IR-1321: protocol read-back work remains gated"] fn ir() {}
        #[test] #[ignore = "QSL-2109: restored behavior"] fn qsl() {}
        #[test] #[ignore = "STD-663: metadata contract"] fn std() {}
        #[test] #[ignore = "slow: runs under `make seam-probe` (CI_CHECKS)"] fn lane() {}
        #[test] #[cfg_attr(feature = "slow", ignore = "IR-1321: owned")] fn conditional() {}
        #[test] fn live() { let _text = "#[ignore]"; }
        "##,
    )
    .expect("valid Rust attribute fixtures");
    assert!(rejected(&file, &ci_targets(MAKEFILE)).is_empty());
}

/// Provenance: QSL-2109.
#[test]
fn fixtures_reject_missing_and_unowned_ignore_attributes() {
    let file = syn::parse_file(
        r#"
        mod nested {
            #[test] #[ignore] fn missing() {}
            #[test] #[ignore = ""] fn empty() {}
            #[test] #[ignore = "slow: blocked on reader"] fn prose() {}
            #[test] #[ignore = "FR-726: requirement only"] fn requirement() {}
            #[test] #[ignore = "QSL-2109later"] fn malformed_ticket() {}
            #[test] #[ignore = "make nonexistent"] fn absent_target() {}
            #[test] #[ignore = "make local-only"] fn non_ci_target() {}
            #[test] #[ignore = "make empty"] fn no_recipe() {}
            #[test] #[cfg_attr(test, cfg_attr(feature = "slow", ignore))] fn conditional() {}
        }
        "#,
    )
    .expect("valid Rust attribute fixtures");
    let names: Vec<_> = rejected(&file, &ci_targets(MAKEFILE))
        .into_iter()
        .map(|finding| finding.function)
        .collect();
    assert_eq!(
        names,
        [
            "missing",
            "empty",
            "prose",
            "requirement",
            "malformed_ticket",
            "absent_target",
            "non_ci_target",
            "no_recipe",
            "conditional",
        ],
    );
}

/// Provenance: QSL-2109. Runs in the default xtask library suite in CI.
#[test]
fn package_ir_and_seam_ignored_tests_have_a_ticket_or_ci_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace root");
    let makefile = fs::read_to_string(root.join("Makefile")).expect("read actual CI targets");
    let ci = ci_targets(&makefile);
    let mut files = source_files(root, "fsl-package/src/emit/tests").expect("package tests");
    files.extend(source_files(root, "xtask/src").expect("seam tooling tests"));
    files.push("tests/it/checked_handoff.rs".to_owned());
    let mut findings = Vec::new();
    for path in files {
        let file = parse_file(root, &path).expect("parse the guarded test source");
        findings.extend(
            rejected(&file, &ci)
                .into_iter()
                .map(|finding| format!("{path}:{}: {}", finding.line, finding.function)),
        );
    }
    assert!(
        findings.is_empty(),
        "ignored tests need an IR/QSL/STD ticket or `make <CI_CHECKS target>` with a recipe:\n{}",
        findings.join("\n"),
    );
}
