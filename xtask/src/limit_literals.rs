// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask limit-literals`: reports production code that puts a type's
//! maximum, an `UNLIMITED` constant or an `unbounded()` constructor where a
//! limit goes. FR-255 gives every limit a published value, so no layer
//! substitutes one of these for it.
//!
//! The report is advisory: it never fails, and a person reads each line.
//! The scan is syntactic. It does not look inside macro invocations, and it
//! skips `#[cfg(test)]` items and `tests` files.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::definition_scan::has_cfg_test;
use crate::error::{Error, Result};
use crate::string_edge::crate_roots;

/// One reported use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// 1-based source line.
    pub line: usize,
    /// How the value is used.
    pub kind: &'static str,
}

/// Methods whose `u64::MAX` argument saturates arithmetic and sets no limit.
const ARITHMETIC: &[&str] = &[
    "unwrap_or",
    "map_or",
    "min",
    "max",
    "saturating_sub",
    "saturating_add",
    "checked_sub",
    "checked_add",
    "checked_div",
];

#[ix_trace_rs::string_edge]
fn is_maximum(expr: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = expr else {
        return false;
    };
    let mut segments = path.path.segments.iter().rev();
    segments.next().is_some_and(|last| last.ident == "MAX")
        && segments.next().is_some_and(|owner| owner.ident == "u64")
}

#[derive(Default)]
struct Scan {
    findings: Vec<Finding>,
}

impl Scan {
    fn report(&mut self, at: &impl Spanned, kind: &'static str) {
        self.findings.push(Finding {
            line: at.span().start().line,
            kind,
        });
    }

    fn arguments<'a>(&mut self, arguments: impl Iterator<Item = &'a syn::Expr>) {
        for argument in arguments.filter(|argument| is_maximum(argument)) {
            self.report(argument, "u64::MAX argument");
        }
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_fn(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_impl_item_fn(self, node);
        }
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        if is_maximum(&node.expr) {
            self.report(&node.expr, "u64::MAX constant");
        }
        syn::visit::visit_item_const(self, node);
    }

    fn visit_field_value(&mut self, node: &'ast syn::FieldValue) {
        if is_maximum(&node.expr) {
            self.report(&node.expr, "u64::MAX field");
        }
        syn::visit::visit_field_value(self, node);
    }

    #[ix_trace_rs::string_edge]
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|last| last.ident == "UNLIMITED")
        {
            self.report(node, "UNLIMITED");
        }
        syn::visit::visit_expr_path(self, node);
    }

    #[ix_trace_rs::string_edge]
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = node.func.as_ref() {
            let name = function.path.segments.last();
            if name.is_some_and(|last| last.ident == "unbounded") {
                self.report(node, "unbounded()");
            }
        }
        self.arguments(node.args.iter());
        syn::visit::visit_expr_call(self, node);
    }

    #[ix_trace_rs::string_edge]
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "unbounded" {
            self.report(node, "unbounded()");
        }
        if !ARITHMETIC.iter().any(|name| node.method == name) {
            self.arguments(node.args.iter());
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// The findings of one source text, in line order.
///
/// # Errors
///
/// The `syn` error of a text that does not parse.
pub fn scan_source(source: &str) -> std::result::Result<Vec<Finding>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut scan = Scan::default();
    scan.visit_file(&file);
    scan.findings.sort_by_key(|finding| finding.line);
    Ok(scan.findings)
}

#[ix_trace_rs::string_edge]
fn production_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let path = entry.map_err(|source| Error::io(dir, source))?.path();
        let stem = path.file_stem().and_then(std::ffi::OsStr::to_str);
        if stem == Some("tests") {
            continue;
        }
        if path.is_dir() {
            production_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

/// Report every finding of the workspace's production crates, one
/// `path:line: kind` line each, then the count.
///
/// # Errors
///
/// A source root that cannot be read or a file that does not parse.
#[ix_trace_rs::string_edge]
pub fn run(workspace_root: &Path) -> Result<String> {
    let mut report = String::new();
    let mut count = 0_usize;
    for root in crate_roots() {
        if root.starts_with("tests") {
            continue;
        }
        let mut files = Vec::new();
        production_files(&workspace_root.join(&root), &mut files)?;
        for path in files {
            let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
            let findings = scan_source(&source).map_err(|source| Error::ImportGraphParse {
                path: path.clone(),
                source,
            })?;
            let shown = path.strip_prefix(workspace_root).unwrap_or(&path);
            for finding in findings {
                count += 1;
                let _ = writeln!(
                    report,
                    "{}:{}: {}",
                    shown.display(),
                    finding.line,
                    finding.kind
                );
            }
        }
    }
    let _ = writeln!(report, "limit-literals: {count} to read");
    Ok(report)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{scan_source, Finding};

    #[trace("FR-255")]
    #[test]
    fn a_literal_limit_is_reported_and_saturating_arithmetic_is_not() {
        let source = r#"
const CEILING: u64 = u64::MAX;
fn production(read: Option<u64>) {
    let limits = ExpansionLimits { max_candidates: u64::MAX };
    let reader = Reader::new(u64::MAX);
    let meter = Meter::new(ScalarLimits::UNLIMITED);
    let all = AnalyzeLimits::unbounded();
    let count = read.unwrap_or(u64::MAX).min(u64::MAX);
}
#[cfg(test)]
mod tests {
    fn fixture() {
        let limits = ExpansionLimits { max_candidates: u64::MAX };
    }
}
"#;
        let found = scan_source(source).expect("the text parses");
        let expected = [
            (2, "u64::MAX constant"),
            (4, "u64::MAX field"),
            (5, "u64::MAX argument"),
            (6, "UNLIMITED"),
            (7, "unbounded()"),
        ]
        .map(|(line, kind)| Finding { line, kind });
        assert_eq!(found, expected);
    }
}
