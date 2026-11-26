// SPDX-License-Identifier: AGPL-3.0-or-later
//! `arch-lint trusted-base` (FR-1051): each certificate checker names only its
//! trusted base, shares no other function with the producer, is named by the
//! producer only through check entries, codec functions and types, and takes
//! no producer-built value. Every sharing that stands is an entry of
//! [`data::FSL`] naming its FR-1051 row.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Write as _,
    path::Path,
};

use ix_trace_rs::implements;

pub(crate) mod data;
mod index;
mod scan;

#[cfg(test)]
mod spec;
#[cfg(test)]
mod tests;

use crate::error::Result;
use index::{Index, ItemInfo, Kind, Resolved};

/// Which of the three rules a finding or an exception breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Rule {
    /// Checker code relies on code outside its trusted base.
    SharedSemantics,
    /// Producer code names checker code other than an entry, codec or type.
    ProducerCall,
    /// A check entry takes a producer-built value.
    ProducerObject,
}

impl Rule {
    /// The rule's name in FR-1051's exception table.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::SharedSemantics => "Shared semantics",
            Self::ProducerCall => "Producer call into checker",
            Self::ProducerObject => "Producer object",
        }
    }
}

/// How much of a base row's modules checker code may name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Extent {
    /// Every item of the crate.
    WholeCrate,
    /// Every item of the module and its submodules, except the parts the
    /// listed exception rows concern.
    Module,
    /// The module's types, and the functions the row records.
    TypeItems,
    /// Only the items the row records.
    Recorded,
    /// The recorded items, which are data only.
    DataItems,
}

/// A row of FR-1051's "Trusted base" table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BaseRow {
    pub(crate) id: &'static str,
    /// The crates and modules as the table writes them.
    pub(crate) modules: &'static [&'static str],
    /// The canonical prefix of `modules`.
    pub(crate) prefix: &'static str,
    pub(crate) extent: Extent,
    pub(crate) except: &'static [&'static str],
    /// The items recorded under the row, canonical.
    pub(crate) recorded: &'static [&'static str],
}

/// A row of FR-1051's "Recorded exceptions" table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExceptionRow {
    pub(crate) id: &'static str,
    pub(crate) rule: Rule,
    /// Each name of the table's "Code it concerns" cell, with what it covers:
    /// a canonical module or item path, or a source file (`dir/**` for every
    /// file under a directory).
    pub(crate) concerns: &'static [(&'static str, &'static [&'static str])],
    /// The row may have no entry yet.
    pub(crate) planned: bool,
}

/// One recorded sharing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub(crate) row: &'static str,
    pub(crate) rule: Rule,
    /// The canonical item; for [`Rule::ProducerObject`], the check entry.
    pub(crate) item: &'static str,
    /// For [`Rule::ProducerObject`], the producer-built type taken.
    pub(crate) takes: &'static str,
}

/// The classes of code (FR-1051 "Classes of code"). A crate is named by its
/// package name or its directory (`dir/**` for every crate under it); a file
/// of the by-file package relative to its `src`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Classes {
    pub(crate) by_file: &'static str,
    pub(crate) checker_files: &'static [&'static str],
    /// Checker items inside producer files: `(file, item)`.
    pub(crate) checker_items: &'static [(&'static str, &'static str)],
    pub(crate) checker_crates: &'static [&'static str],
    pub(crate) settling_files: &'static [&'static str],
    pub(crate) out_of_scope_files: &'static [&'static str],
    pub(crate) trusted_crates: &'static [&'static str],
    pub(crate) producer_crates: &'static [&'static str],
    pub(crate) tool_crates: &'static [&'static str],
}

/// Everything the check reads besides the source.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Policy {
    pub(crate) classes: Classes,
    pub(crate) base: &'static [BaseRow],
    pub(crate) exceptions: &'static [ExceptionRow],
    pub(crate) routes: &'static [&'static str],
    /// The producer-built types of "No producer objects".
    pub(crate) producer_types: &'static [&'static str],
    /// Each check entry with the routes it serves.
    pub(crate) check_entries: &'static [(&'static str, &'static [&'static str])],
    /// Each codec function with its codec class.
    pub(crate) codec: &'static [(&'static str, &'static str)],
    pub(crate) codec_classes: &'static [&'static str],
    pub(crate) entries: &'static [Entry],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Class {
    Checker,
    Settling,
    OutOfScope,
    TrustedBase,
    Producer,
    Tools,
}

/// What the check found.
#[derive(Clone, Debug, Default)]
pub(crate) struct Outcome {
    /// Sharings no entry covers.
    pub(crate) findings: Vec<String>,
    /// Wrong data and wrongly classed code.
    pub(crate) errors: Vec<String>,
    /// How many findings entries cover.
    pub(crate) recorded: usize,
}

impl Outcome {
    pub(crate) fn passed(&self) -> bool {
        self.findings.is_empty() && self.errors.is_empty()
    }

    pub(crate) fn report(&self) -> String {
        let mut report = String::from("FR-1051 certificate checker trusted base\n");
        for line in self.errors.iter().chain(&self.findings) {
            let _ = writeln!(report, "  {line}");
        }
        let _ = writeln!(
            report,
            "  {}: {} uncovered, {} recorded, {} data or class errors",
            if self.passed() { "PASS" } else { "FAIL" },
            self.findings.len(),
            self.recorded,
            self.errors.len()
        );
        report
    }
}

/// Whether `pattern` (a file, or `dir/**`) matches `path`.
fn matches_file(pattern: &str, path: &str) -> bool {
    match pattern.strip_suffix("/**") {
        Some(directory) => path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/')),
        None => pattern == path,
    }
}

fn under(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
}

fn names_crate(tokens: &[&str], package: &index::Package) -> bool {
    tokens
        .iter()
        .any(|token| *token == package.name || matches_file(token, &package.dir))
}

struct Check<'a> {
    policy: &'a Policy,
    index: Index,
    items: HashMap<String, Vec<ItemInfo>>,
    entries: BTreeSet<&'a str>,
    codec: BTreeSet<&'a str>,
}

/// One item as a finding names it.
struct Named {
    path: String,
    kind: Option<Kind>,
    item: Option<ItemInfo>,
}

impl Check<'_> {
    fn crate_classes(&self, package: &index::Package) -> Vec<Class> {
        let classes = &self.policy.classes;
        [
            (classes.checker_crates, Class::Checker),
            (classes.trusted_crates, Class::TrustedBase),
            (classes.producer_crates, Class::Producer),
            (classes.tool_crates, Class::Tools),
        ]
        .into_iter()
        .filter(|(tokens, _)| names_crate(tokens, package))
        .map(|(_, class)| class)
        .collect()
    }

    /// The file's path below the by-file package's `src`, and whether it is
    /// there at all.
    fn by_file_path<'f>(&self, module: usize, file: &'f str) -> (&'f str, bool) {
        let package = self.index.package_of(module);
        let source = format!("{}/src/", package.dir);
        match file.strip_prefix(source.as_str()) {
            Some(relative) => (relative, true),
            None => (file, false),
        }
    }

    /// The classes of the file holding `module`.
    fn file_classes(&self, module: usize) -> Vec<Class> {
        let package = self.index.package_of(module);
        if package.name != self.policy.classes.by_file {
            return self.crate_classes(package);
        }
        let classes = &self.policy.classes;
        let (file, in_source) = self.by_file_path(module, self.index.file_of(module));
        let mut found: Vec<Class> = [
            (classes.checker_files, Class::Checker),
            (classes.settling_files, Class::Settling),
            (classes.out_of_scope_files, Class::OutOfScope),
        ]
        .into_iter()
        .filter(|(patterns, _)| {
            in_source && patterns.iter().any(|pattern| matches_file(pattern, file))
        })
        .map(|(_, class)| class)
        .collect();
        if found.is_empty() && in_source {
            found.push(Class::Producer);
        }
        found.extend(self.crate_classes(package));
        found
    }

    /// The class of code at a site or definition: `within` names the items
    /// around it.
    fn class(&self, module: usize, within: &str) -> Option<Class> {
        let (file, _) = self.by_file_path(module, self.index.file_of(module));
        let checker_item = self.index.package_of(module).name == self.policy.classes.by_file
            && self
                .policy
                .classes
                .checker_items
                .iter()
                .any(|(at, item)| *at == file && under(within, item));
        if checker_item {
            Some(Class::Checker)
        } else {
            self.file_classes(module).first().copied()
        }
    }

    /// The item's name below its defining module (`Type::method`).
    fn local_name<'i>(&self, item: &'i ItemInfo) -> &'i str {
        let mut parts = item.path.rsplitn(3, "::");
        let last = parts.next().unwrap_or_default();
        match parts.next() {
            Some(owner) if owner.starts_with(char::is_uppercase) => {
                &item.path[item.path.len() - last.len() - owner.len() - 2..]
            }
            _ => &item.path[item.path.len() - last.len()..],
        }
    }

    fn item_class(&self, item: &ItemInfo) -> Option<Class> {
        self.class(item.module, self.local_name(item))
    }

    #[implements("FR-1051-AC-6")]
    fn class_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let describe = |subject: String, count: usize| match count {
            0 => Some(format!("{subject} is in no class")),
            1 => None,
            _ => Some(format!("{subject} is in two classes")),
        };
        for package in &self.index.packages {
            if package.name != self.policy.classes.by_file {
                let count = self.crate_classes(package).len();
                errors.extend(describe(format!("crate {}", package.name), count));
            }
        }
        let mut files = BTreeSet::new();
        for (module, record) in self.index.modules.iter().enumerate() {
            let package = self.index.package_of(module);
            if package.name == self.policy.classes.by_file && files.insert(record.file) {
                let count = self.file_classes(module).len();
                errors.extend(describe(
                    format!("file {}", self.index.file_of(module)),
                    count,
                ));
            }
        }
        errors
    }

    fn named(&self, target: &Resolved) -> Option<Named> {
        let item = match target {
            Resolved::Type(id) => &self.index.types[*id].item,
            Resolved::Value(id) => &self.index.values[*id].item,
            Resolved::External(path) => {
                return Some(Named {
                    path: path.clone(),
                    kind: None,
                    item: None,
                })
            }
            Resolved::Module(_) | Resolved::Unknown => return None,
        };
        Some(Named {
            path: item.path.clone(),
            kind: Some(item.kind),
            item: Some(item.clone()),
        })
    }

    /// Whether `selector` (a canonical path, or a file) covers `item`.
    fn covers(&self, selector: &str, item: &ItemInfo) -> bool {
        if selector.contains('/') {
            matches_file(selector, self.index.file_of(item.module))
        } else {
            under(&item.path, selector) || under(&self.index.modules[item.module].path, selector)
        }
    }

    fn row_concerns(&self, row: &str, item: &ItemInfo) -> bool {
        self.policy
            .exceptions
            .iter()
            .filter(|candidate| candidate.id == row)
            .flat_map(|candidate| candidate.concerns)
            .flat_map(|(_, selectors)| selectors.iter())
            .any(|selector| self.covers(selector, item))
    }

    /// Whether `row` of the trusted base reaches `item`'s module.
    fn row_holds(&self, row: &BaseRow, item: &ItemInfo) -> bool {
        let module = &self.index.modules[item.module].path;
        row.modules.iter().any(|name| match row.extent {
            Extent::WholeCrate => under(module, &name.replace('-', "_")),
            _ => under(module, &format!("{}{}", row.prefix, name.replace('-', "_"))),
        })
    }

    /// Whether checker code may name `named` (FR-1051 "Trusted base").
    #[implements("FR-1051-AC-1")]
    fn in_base(&self, named: &Named) -> bool {
        let Some(item) = &named.item else {
            let krate = match named.path.split("::").next().unwrap_or_default() {
                // The parts of `std` under their own crate names.
                "core" | "alloc" => "std",
                krate => krate,
            };
            return self
                .policy
                .base
                .iter()
                .any(|row| row.extent == Extent::WholeCrate && row.modules.contains(&krate));
        };
        // A module row nested in another governs its own items, so that its
        // exceptions are not undone by the outer row.
        let nested = self
            .policy
            .base
            .iter()
            .filter(|row| row.extent == Extent::Module && self.row_holds(row, item))
            .max_by_key(|row| row.modules.iter().map(|name| name.len()).max());
        let module_row =
            nested.is_some_and(|row| !row.except.iter().any(|id| self.row_concerns(id, item)));
        module_row
            || self.policy.base.iter().any(|row| {
                row.recorded.contains(&item.path.as_str())
                    || match row.extent {
                        Extent::WholeCrate => self.row_holds(row, item),
                        Extent::TypeItems => item.kind == Kind::Type && self.row_holds(row, item),
                        Extent::Module | Extent::Recorded | Extent::DataItems => false,
                    }
            })
    }

    #[implements("FR-1051-AC-8")]
    fn data_errors(&self) -> Vec<String> {
        let policy = self.policy;
        let mut errors = Vec::new();
        let found = |path: &str| self.items.get(path).map(Vec::as_slice).unwrap_or_default();
        let known_row = |id: &str| policy.exceptions.iter().find(|row| row.id == id);
        for entry in policy.entries {
            let subject = format!("entry {} {}", entry.row, entry.item);
            let Some(row) = known_row(entry.row) else {
                errors.push(format!("{subject}: no row {} in FR-1051", entry.row));
                continue;
            };
            if entry.rule != row.rule {
                errors.push(format!(
                    "{subject}: breaks \"{}\", but row {} is \"{}\"",
                    entry.rule.as_str(),
                    row.id,
                    row.rule.as_str()
                ));
            }
            let items = found(entry.item);
            if items.is_empty() {
                if !row.planned {
                    errors.push(format!("{subject}: the item does not exist"));
                }
            } else if !items.iter().any(|item| self.row_concerns(row.id, item)) {
                errors.push(format!(
                    "{subject}: the item is defined outside the modules row {} concerns",
                    row.id
                ));
            }
        }
        for row in policy.exceptions {
            if !row.planned && !policy.entries.iter().any(|entry| entry.row == row.id) {
                errors.push(format!("row {} has no entry", row.id));
            }
        }
        for (path, routes) in policy.check_entries {
            let items = found(path);
            if items.is_empty() {
                errors.push(format!("check entry {path}: the item does not exist"));
            } else if !items.iter().any(|item| {
                item.public
                    && item.kind == Kind::Function
                    && self.item_class(item) == Some(Class::Checker)
            }) {
                errors.push(format!(
                    "check entry {path}: not a public function of checker code"
                ));
            }
            if routes.is_empty() {
                errors.push(format!("check entry {path}: names no route"));
            }
            for route in *routes {
                if !policy.routes.contains(route) {
                    errors.push(format!("check entry {path}: no route {route} in FR-1051"));
                }
            }
        }
        for route in policy.routes {
            if !policy
                .check_entries
                .iter()
                .any(|(_, routes)| routes.contains(route))
            {
                errors.push(format!("route {route} has no check entry"));
            }
        }
        for (path, class) in policy.codec {
            if found(path).is_empty() {
                errors.push(format!("codec function {path}: the item does not exist"));
            }
            if !policy.codec_classes.contains(class) {
                errors.push(format!(
                    "codec function {path}: no codec class {class} in FR-1051"
                ));
            }
        }
        for row in policy.base {
            for path in row.recorded {
                let items = found(path);
                if items.is_empty() {
                    errors.push(format!("{} item {path}: the item does not exist", row.id));
                } else if !items.iter().any(|item| self.row_holds(row, item)) {
                    errors.push(format!(
                        "{} item {path}: the item is defined outside the row's modules",
                        row.id
                    ));
                }
                if row.extent == Extent::DataItems
                    && items.iter().any(|item| item.kind == Kind::Function)
                {
                    errors.push(format!("{} item {path}: is a function, not data", row.id));
                }
            }
        }
        errors
    }

    /// The producer-built types and closures `ty` carries, each with the
    /// parameter or field that carries it.
    fn taken(&self, walk: &mut Taken, ty: &syn::Type, module: usize, at: &str) {
        match ty {
            syn::Type::Reference(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Slice(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Array(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Ptr(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Paren(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Group(inner) => self.taken(walk, &inner.elem, module, at),
            syn::Type::Tuple(tuple) => {
                for inner in &tuple.elems {
                    self.taken(walk, inner, module, at);
                }
            }
            syn::Type::BareFn(_) => walk.hit("fn pointer", at),
            syn::Type::ImplTrait(inner) => self.bounds(walk, inner.bounds.iter(), module, at),
            syn::Type::TraitObject(inner) => self.bounds(walk, inner.bounds.iter(), module, at),
            syn::Type::Path(path) => {
                let Some(last) = path.path.segments.last() else {
                    return;
                };
                if path.path.segments.len() == 1 {
                    let name = last.ident.to_string();
                    if let Some(bounds) = walk.generics.get(&name).cloned() {
                        if walk.expanding.insert(name.clone()) {
                            self.bounds(walk, bounds.iter(), module, at);
                            walk.expanding.remove(&name);
                        }
                        return;
                    }
                }
                self.arguments(walk, &last.arguments, module, at);
                let resolved = self.index.resolve_path(module, &path.path, walk.self_type);
                let Resolved::Type(id) = resolved else {
                    return;
                };
                let record = &self.index.types[id];
                let name = record.item.path.rsplit("::").next().unwrap_or_default();
                if self.policy.producer_types.contains(&name) {
                    walk.hit(name, at);
                } else if walk.seen.insert(id) {
                    if let Some(alias) = self.index.alias_of(id) {
                        self.taken(walk, alias, record.item.module, at);
                    } else if self.item_class(&record.item) == Some(Class::Checker) {
                        for (field, inner) in &record.fields {
                            let saved = walk.self_type.replace(id);
                            self.taken(walk, inner, record.item.module, &format!("{at}.{field}"));
                            walk.self_type = saved;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn arguments(&self, walk: &mut Taken, arguments: &syn::PathArguments, module: usize, at: &str) {
        if let syn::PathArguments::AngleBracketed(arguments) = arguments {
            for argument in &arguments.args {
                match argument {
                    syn::GenericArgument::Type(inner) => self.taken(walk, inner, module, at),
                    syn::GenericArgument::AssocType(inner) => {
                        self.taken(walk, &inner.ty, module, at);
                    }
                    _ => {}
                }
            }
        }
    }

    fn bounds<'b>(
        &self,
        walk: &mut Taken,
        bounds: impl Iterator<Item = &'b syn::TypeParamBound>,
        module: usize,
        at: &str,
    ) {
        for bound in bounds {
            let syn::TypeParamBound::Trait(bound) = bound else {
                continue;
            };
            let Some(last) = bound.path.segments.last() else {
                continue;
            };
            let name = last.ident.to_string();
            if matches!(name.as_str(), "Fn" | "FnMut" | "FnOnce") {
                walk.hit("closure", at);
            } else if self.policy.producer_types.contains(&name.as_str()) {
                walk.hit(&name, at);
            } else {
                self.arguments(walk, &last.arguments, module, at);
            }
        }
    }

    /// Every `(check entry, producer-built type)` with the parameters and
    /// fields carrying it.
    #[implements("FR-1051-AC-4")]
    fn objects(&self) -> Vec<(String, String, String)> {
        let mut found = Vec::new();
        for (path, _) in self.policy.check_entries {
            for value in self.index.values.iter().filter(|v| v.item.path == *path) {
                let Some(signature) = &value.signature else {
                    continue;
                };
                let mut walk = Taken {
                    generics: generics_of(value.impl_generics.as_ref(), &signature.generics),
                    self_type: value.self_type,
                    seen: BTreeSet::new(),
                    expanding: BTreeSet::new(),
                    found: BTreeMap::new(),
                };
                for input in &signature.inputs {
                    walk.seen.clear();
                    match input {
                        syn::FnArg::Receiver(_) => {
                            if let Some(id) = value.self_type {
                                let name = self.index.types[id].item.path.clone();
                                let name = name.rsplit("::").next().unwrap_or_default().to_owned();
                                let path: syn::Type = syn::parse_quote!(Self);
                                if self.policy.producer_types.contains(&name.as_str()) {
                                    walk.hit(&name, "self");
                                } else {
                                    self.taken(&mut walk, &path, value.item.module, "self");
                                }
                            }
                        }
                        syn::FnArg::Typed(typed) => {
                            let name = match typed.pat.as_ref() {
                                syn::Pat::Ident(pattern) => pattern.ident.to_string(),
                                _ => "_".to_owned(),
                            };
                            self.taken(&mut walk, &typed.ty, value.item.module, &name);
                        }
                    }
                }
                for (taken, at) in walk.found {
                    let at: Vec<String> = at.into_iter().collect();
                    found.push(((*path).to_owned(), taken, at.join(", ")));
                }
            }
        }
        found
    }

    fn site(&self, module: usize, line: usize) -> String {
        format!("{}:{line}", self.index.file_of(module))
    }

    #[implements("FR-1051-AC-2", "FR-1051-AC-3", "FR-1051-AC-7", "FR-1051-AC-10")]
    fn run(&self) -> Outcome {
        let mut outcome = Outcome {
            errors: self.class_errors(),
            ..Outcome::default()
        };
        outcome.errors.extend(self.data_errors());

        // Each item's first site per file in checker, producer and settling
        // code.
        let mut sites: BTreeMap<String, Sites> = BTreeMap::new();
        let wanted = |module: usize| {
            matches!(
                self.file_classes(module).first(),
                Some(Class::Checker | Class::Producer | Class::Settling)
            )
        };
        for reference in scan::references(&self.index, wanted) {
            let Some(named) = self.named(&reference.target) else {
                continue;
            };
            let Some(from) = self.class(reference.module, &reference.within) else {
                continue;
            };
            let site = self.site(reference.module, reference.line);
            let record = sites.entry(named.path.clone()).or_insert_with(|| Sites {
                named,
                checker: BTreeSet::new(),
                producer: BTreeSet::new(),
                settling: BTreeSet::new(),
            });
            match from {
                Class::Checker => record.checker.insert(site),
                Class::Producer => record.producer.insert(site),
                Class::Settling => record.settling.insert(site),
                Class::OutOfScope | Class::TrustedBase | Class::Tools => false,
            };
        }

        let entries = self.policy.entries;
        let mut used = vec![false; entries.len()];
        let mut cover = |rule: Rule, item: &str, takes: &str| {
            let mut covered = false;
            for (at, entry) in entries.iter().enumerate() {
                if entry.rule == rule && entry.item == item && entry.takes == takes {
                    used[at] = true;
                    covered = true;
                }
            }
            covered
        };
        let list = |sites: &BTreeSet<String>| {
            let first = sites.iter().next().cloned().unwrap_or_default();
            match sites.len() {
                0 | 1 => first,
                more => format!("{first} (+{} more)", more - 1),
            }
        };
        for (path, record) in &sites {
            let class = record
                .named
                .item
                .as_ref()
                .and_then(|item| self.item_class(item));
            let function = record.named.kind == Some(Kind::Function);
            let codec = self.codec.contains(path.as_str());
            let entry = self.entries.contains(path.as_str());
            if class == Some(Class::Checker) {
                let value = matches!(record.named.kind, Some(Kind::Function | Kind::Const));
                if !value || codec || entry {
                    continue;
                }
                if !record.producer.is_empty() {
                    if cover(Rule::ProducerCall, path, "") {
                        outcome.recorded += 1;
                    } else {
                        outcome.findings.push(format!(
                            "producer code names checker item {path}, neither a check entry nor \
                             a codec function: {}",
                            list(&record.producer)
                        ));
                    }
                }
                let public = record.named.item.as_ref().is_some_and(|item| item.public);
                if public && !record.settling.is_empty() {
                    outcome.findings.push(format!(
                        "settling code names checker item {path}, neither a check entry nor a \
                         codec function: {}",
                        list(&record.settling)
                    ));
                }
                continue;
            }
            if record.checker.is_empty() || codec || self.in_base(&record.named) {
                continue;
            }
            let covered = cover(Rule::SharedSemantics, path, "");
            if covered {
                outcome.recorded += 1;
            } else if function && !record.producer.is_empty() {
                outcome.findings.push(format!(
                    "function {path} is shared outside the trusted base: checker code at {}, \
                     producer code at {}",
                    list(&record.checker),
                    list(&record.producer)
                ));
            } else {
                outcome.findings.push(format!(
                    "checker code names {path} outside the trusted base: {}",
                    list(&record.checker)
                ));
            }
        }
        for (check_entry, takes, at) in self.objects() {
            if cover(Rule::ProducerObject, &check_entry, &takes) {
                outcome.recorded += 1;
            } else {
                outcome.findings.push(format!(
                    "check entry {check_entry} takes producer-built {takes} through {at}"
                ));
            }
        }
        for (entry, used) in entries.iter().zip(used) {
            let planned = self
                .policy
                .exceptions
                .iter()
                .any(|row| row.id == entry.row && row.planned);
            if !used && !planned {
                outcome.errors.push(format!(
                    "stale entry: row {} {} {} matches no finding",
                    entry.row, entry.item, entry.takes
                ));
            }
        }
        outcome
    }
}

struct Sites {
    named: Named,
    checker: BTreeSet<String>,
    producer: BTreeSet<String>,
    settling: BTreeSet<String>,
}

/// The walk of one check entry's parameter types.
struct Taken {
    generics: HashMap<String, Vec<syn::TypeParamBound>>,
    self_type: Option<usize>,
    seen: BTreeSet<usize>,
    expanding: BTreeSet<String>,
    found: BTreeMap<String, BTreeSet<String>>,
}

impl Taken {
    fn hit(&mut self, taken: &str, at: &str) {
        self.found
            .entry(taken.to_owned())
            .or_default()
            .insert(at.to_owned());
    }
}

/// Each type parameter in scope with every bound on it.
fn generics_of(
    outer: Option<&syn::Generics>,
    inner: &syn::Generics,
) -> HashMap<String, Vec<syn::TypeParamBound>> {
    let mut bounds: HashMap<String, Vec<syn::TypeParamBound>> = HashMap::new();
    for generics in outer.into_iter().chain([inner]) {
        for parameter in generics.type_params() {
            bounds
                .entry(parameter.ident.to_string())
                .or_default()
                .extend(parameter.bounds.iter().cloned());
        }
        for predicate in generics
            .where_clause
            .iter()
            .flat_map(|clause| &clause.predicates)
        {
            let syn::WherePredicate::Type(predicate) = predicate else {
                continue;
            };
            let syn::Type::Path(path) = &predicate.bounded_ty else {
                continue;
            };
            if let Some(name) = path.path.get_ident() {
                bounds
                    .entry(name.to_string())
                    .or_default()
                    .extend(predicate.bounds.iter().cloned());
            }
        }
    }
    bounds
}

/// Run the check over the workspace at `root`.
pub(crate) fn check(root: &Path, policy: &Policy) -> Result<Outcome> {
    let tools = policy.classes.tool_crates;
    let index = Index::load(root, |package| !names_crate(tools, package))?;
    let mut items: HashMap<String, Vec<ItemInfo>> = HashMap::new();
    for item in index.items() {
        items
            .entry(item.path.clone())
            .or_default()
            .push(item.clone());
    }
    let check = Check {
        policy,
        index,
        items,
        entries: policy.check_entries.iter().map(|(path, _)| *path).collect(),
        codec: policy.codec.iter().map(|(path, _)| *path).collect(),
    };
    Ok(check.run())
}
