// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask limit-literals`: reports production code that puts a type's
//! maximum, an `UNLIMITED` constant or an `unbounded()` constructor where a
//! limit goes. FR-255 gives every limit a published value, so no layer
//! substitutes one of these for it.
//!
//! The gate fails (QSL-3573) when a file holds more findings of a kind than
//! `limit-literals.allow` allows it, or fewer: each row names why the sites
//! may stay, so the list can only shrink. The scan is syntactic. It does not
//! look inside macro invocations, and it skips `#[cfg(test)]` items and `tests` and `*_tests` files.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::implements;
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
        // `tests` and `*_tests` modules are compiled under `cfg(test)`.
        if stem.is_some_and(|stem| stem == "tests" || stem.ends_with("_tests")) {
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

/// The allowlist: one `path<TAB>kind<TAB>count<TAB>reason` row per file and
/// kind, `#` lines and blank lines ignored.
const ALLOWLIST: &str = include_str!("../limit-literals.allow");

/// The ticket the baseline rows were written under. A row naming only this
/// ticket cannot be raised above its `baseline N`.
const BASELINE_TICKET: &str = "QSL-3573";

/// One (path, kind) the gate reads.
type Key = (String, String);

/// One allowlist row: the sites allowed and why.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Row {
    count: usize,
    reason: String,
}

/// The allowed rows of [`ALLOWLIST`].
fn allowed(text: &str) -> BTreeMap<Key, Row> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let (path, kind, count) = (columns.next()?, columns.next()?, columns.next()?);
            let reason = columns.next().unwrap_or_default().to_owned();
            Some((
                (path.to_owned(), kind.to_owned()),
                Row {
                    count: count.parse().ok()?,
                    reason,
                },
            ))
        })
        .collect()
}

/// The ticket ids (`QSL-3573`) a reason names.
#[ix_trace_rs::string_edge]
fn tickets(reason: &str) -> Vec<&str> {
    reason
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|word| {
            word.split_once('-').is_some_and(|(team, number)| {
                team.len() >= 2
                    && team.chars().all(|c| c.is_ascii_uppercase())
                    && !number.is_empty()
                    && number.chars().all(|c| c.is_ascii_digit())
            })
        })
        .collect()
}

/// The count a reason of the form `baseline N ...` gives, if it is one.
#[ix_trace_rs::string_edge]
fn baseline_of(reason: &str) -> Option<usize> {
    reason
        .strip_prefix("baseline ")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Whether `reason` names a ticket other than the baseline one.
fn names_another_ticket(reason: &str) -> bool {
    tickets(reason)
        .iter()
        .any(|ticket| *ticket != BASELINE_TICKET)
}

/// What the allowlist itself gets wrong: a reason without a ticket, and a
/// baseline row raised above its baseline under the baseline ticket alone.
#[implements("FR-255-AC-48")]
fn row_problems(allow: &BTreeMap<Key, Row>) -> Vec<String> {
    let mut problems = Vec::new();
    for ((path, kind), row) in allow {
        if tickets(&row.reason).is_empty() {
            problems.push(format!(
                "{path} {kind}: the reason names no ticket; name the ticket that reviewed the exception"
            ));
        }
        if baseline_of(&row.reason).is_some_and(|baseline| row.count > baseline)
            && !names_another_ticket(&row.reason)
        {
            problems.push(format!(
                "{path} {kind}: raised above its baseline; a raise needs a reason naming a ticket other than {BASELINE_TICKET}"
            ));
        }
    }
    problems
}

/// What `found` (the lines of each key) holds beyond, or short of, what
/// `allowed` permits. A key over its allowance lists each of its
/// `path:line: kind` sites.
#[implements("FR-255-AC-48")]
fn violations(found: &BTreeMap<Key, Vec<usize>>, allowed: &BTreeMap<Key, Row>) -> Vec<String> {
    let mut problems = row_problems(allowed);
    for (key, lines) in found {
        let permitted = allowed.get(key).map_or(0, |row| row.count);
        if lines.len() > permitted {
            problems.push(format!(
                "{} {}: {} found, {permitted} allowed; bound each with the Budget, or name an exception with a reason and a ticket in limit-literals.allow for review",
                key.0,
                key.1,
                lines.len()
            ));
            for line in lines {
                problems.push(format!("  {}:{line}: {}", key.0, key.1));
            }
        }
    }
    for (key, row) in allowed {
        let count = found.get(key).map_or(0, Vec::len);
        if count < row.count {
            problems.push(format!(
                "{} {}: {count} found, {} allowed; lower the row in limit-literals.allow",
                key.0, key.1, row.count
            ));
        }
    }
    problems
}

/// Report every finding of the workspace's production crates, one
/// `path:line: kind` line each, then the count, and fail when they differ
/// from the allowlist.
///
/// # Errors
///
/// A source root that cannot be read or a file that does not parse, or
/// [`Error::LimitLiteralsFound`] when the findings differ from the allowlist.
#[ix_trace_rs::string_edge]
pub fn run(workspace_root: &Path) -> Result<String> {
    let mut report = String::new();
    let mut count = 0_usize;
    let mut found = BTreeMap::<Key, Vec<usize>>::new();
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
                found
                    .entry((shown.display().to_string(), finding.kind.to_owned()))
                    .or_default()
                    .push(finding.line);
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
    let problems = violations(&found, &allowed(ALLOWLIST));
    if problems.is_empty() {
        return Ok(report);
    }
    Err(Error::LimitLiteralsFound {
        summary: problems.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use std::collections::BTreeMap;

    use super::{allowed, scan_source, violations, Finding};

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

    fn row(count: usize, reason: &str) -> super::Row {
        super::Row {
            count,
            reason: reason.to_owned(),
        }
    }

    #[trace("FR-255-AC-48")]
    #[test]
    fn the_gate_fails_on_a_new_finding_and_on_a_stale_allowance() {
        let key = |kind: &str| ("a.rs".to_owned(), kind.to_owned());
        let allow = allowed(
            "# comment\n\na.rs\tUNLIMITED\t2\tQSL-1 why\nb.rs\tunbounded()\t1\tQSL-1 why\n",
        );
        let found = BTreeMap::from([
            (key("UNLIMITED"), vec![3, 9]),
            (key("u64::MAX field"), vec![7]),
        ]);
        let problems = violations(&found, &allow);
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].starts_with("a.rs u64::MAX field: 1 found, 0 allowed"));
        // The failing key's sites are printed as path:line: kind, and the
        // message does not invite adding a row.
        assert_eq!(problems[1], "  a.rs:7: u64::MAX field");
        assert!(problems[0].contains("bound each with the Budget"));
        assert!(!problems[0].contains("add a row"));
        assert!(problems[2].starts_with("b.rs unbounded(): 0 found, 1 allowed"));
        let exact = BTreeMap::from([
            (key("UNLIMITED"), vec![3, 9]),
            (("b.rs".to_owned(), "unbounded()".to_owned()), vec![1]),
        ]);
        assert!(violations(&exact, &allow).is_empty());
    }

    #[trace("FR-255-AC-48")]
    #[test]
    fn a_row_without_a_ticket_or_raised_past_its_baseline_fails() {
        let key = |name: &str| (name.to_owned(), "UNLIMITED".to_owned());
        let allow = BTreeMap::from([
            (key("a.rs"), row(1, "why, but no ticket")),
            (key("b.rs"), row(5, "baseline 4 at QSL-3573: unclassified")),
            (
                key("c.rs"),
                row(5, "baseline 4 at QSL-3573: raised for QSL-4000"),
            ),
            (key("d.rs"), row(4, "baseline 4 at QSL-3573: unclassified")),
        ]);
        let problems = super::row_problems(&allow);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("a.rs UNLIMITED: the reason names no ticket"));
        assert!(problems[1].starts_with("b.rs UNLIMITED: raised above its baseline"));
    }

    #[trace("FR-255-AC-48")]
    #[test]
    fn every_allowlist_row_has_a_count_a_reason_and_a_ticket() {
        let rows = super::ALLOWLIST
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'));
        for row in rows {
            let columns: Vec<_> = row.split('\t').collect();
            assert_eq!(columns.len(), 4, "{row}");
            assert!(
                columns[2].parse::<usize>().is_ok_and(|count| count > 0),
                "{row}"
            );
            assert!(!columns[3].trim().is_empty(), "{row}");
            assert!(!super::tickets(columns[3]).is_empty(), "no ticket: {row}");
        }
        assert!(
            super::row_problems(&allowed(super::ALLOWLIST)).is_empty(),
            "the allowlist breaks its own rules"
        );
    }
}
