// SPDX-License-Identifier: AGPL-3.0-or-later
//! The module and item index of a workspace, and the path resolution over it
//! (FR-1051 "Names and detection"): `use` trees with globs, `self`, nested
//! groups and aliases, `extern crate` aliases, qualified paths, `type`
//! aliases and `pub use` re-exports, each resolved to the item it names.
//! Resolution is by module path only: nothing is type-checked.

use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;
use syn::spanned::Spanned;

use crate::{
    api_surface::use_leaves,
    error::{Code, Error, Result},
    metadata,
};

/// A workspace member package.
#[derive(Debug)]
pub(super) struct Package {
    pub(super) name: String,
    /// The manifest directory relative to the workspace root; `""` for the
    /// root package.
    pub(super) dir: String,
}

/// One compiled target of a package: a library, a binary or a build script.
#[derive(Debug)]
pub(super) struct Target {
    /// The first segment of every canonical path of the target's items.
    pub(super) ident: String,
    pub(super) package: usize,
    root_file: PathBuf,
    root: usize,
    /// The crate names this target's code may name, to the crate each names.
    deps: HashMap<String, String>,
}

/// A source file of a scanned target.
#[derive(Clone)]
pub(super) struct SourceFile {
    /// Relative to the workspace root, `/`-separated.
    pub(super) relative: String,
    pub(super) syntax: syn::File,
}

/// One module: a file or an inline `mod`.
#[derive(Debug)]
pub(super) struct Module {
    pub(super) target: usize,
    pub(super) path: String,
    pub(super) file: usize,
    parent: Option<usize>,
    items: BTreeMap<String, Vec<Def>>,
    imports: BTreeMap<String, Vec<Import>>,
    globs: Vec<Import>,
}

#[derive(Clone, Debug)]
struct Import {
    leading_colon: bool,
    segments: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
enum Def {
    Module(usize),
    Type(usize),
    Value(usize),
}

/// What kind of item a path names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Kind {
    /// A struct, enum, union, trait or type alias (an enum variant names its
    /// enum).
    Type,
    /// A function or method.
    Function,
    /// A constant or static, free or associated.
    Const,
}

/// A struct, enum, union, trait or type alias.
#[derive(Clone)]
pub(super) struct TypeItem {
    pub(super) item: ItemInfo,
    variants: BTreeSet<String>,
    assoc: BTreeMap<String, usize>,
    /// The target of a `type` alias.
    alias: Option<syn::Type>,
    /// Every field of a struct or of an enum's variants, with its name.
    pub(super) fields: Vec<(String, syn::Type)>,
}

/// One item: where it is defined and what it is.
#[derive(Clone, Debug)]
pub(super) struct ItemInfo {
    /// The canonical path: target, defining module (or the type for an
    /// associated item) and name.
    pub(super) path: String,
    pub(super) kind: Kind,
    /// The module whose source holds the definition.
    pub(super) module: usize,
    /// Declared `pub`, without a restriction.
    pub(super) public: bool,
}

/// A function, method, constant or static.
#[derive(Clone)]
pub(super) struct ValueItem {
    pub(super) item: ItemInfo,
    /// A function's signature.
    pub(super) signature: Option<syn::Signature>,
    /// The `Self` type of a method, and its impl block's generics.
    pub(super) self_type: Option<usize>,
    pub(super) impl_generics: Option<syn::Generics>,
}

/// What a path resolves to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Resolved {
    Module(usize),
    Type(usize),
    Value(usize),
    /// A path into a crate outside the workspace, canonical from its crate
    /// name.
    External(String),
    Unknown,
}

/// Where a bare name came from, which decides whether a bare path counts as
/// naming it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Binding {
    /// An item of the module itself.
    Local,
    /// A `use` leaf or glob import.
    Imported,
}

/// An `impl` block awaiting its `Self` type.
struct PendingImpl {
    module: usize,
    block: syn::ItemImpl,
}

/// The index of every module and item of a workspace's scanned targets.
#[derive(Default)]
pub(super) struct Index {
    pub(super) packages: Vec<Package>,
    pub(super) targets: Vec<Target>,
    pub(super) files: Vec<SourceFile>,
    pub(super) modules: Vec<Module>,
    pub(super) types: Vec<TypeItem>,
    pub(super) values: Vec<ValueItem>,
    by_ident: HashMap<String, usize>,
    /// Lookups in progress, so cyclic imports end.
    active: RefCell<HashSet<(usize, String)>>,
    cache: RefCell<LookupCache>,
}

const STANDARD_CRATES: [&str; 3] = ["std", "core", "alloc"];

/// What a name looked up in a module stands for.
type LookupCache = HashMap<(usize, String), Option<(Resolved, Binding)>>;

/// Whether a `cfg` predicate holds only in test builds.
fn implies_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test") || path.is_ident("kani"),
        syn::Meta::List(list) => {
            let Ok(members) = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("any") {
                !members.is_empty() && members.iter().all(implies_test)
            } else if list.path.is_ident("all") {
                members.iter().any(implies_test)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// Whether `attrs` gate their item to test builds (FR-1051 "Classes of
/// code": `cfg(test)`, `cfg(kani)`, `cfg(any(test, kani))`).
pub(super) fn is_test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Meta>()
                .is_ok_and(|meta| implies_test(&meta))
    })
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| {
        Error::new(
            Code::InvalidMetadata,
            format!("cargo metadata has no `{key}`"),
        )
    })
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| {
        Error::new(
            Code::InvalidMetadata,
            format!("cargo metadata has no `{key}`"),
        )
    })
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `path` with its `.` and `..` components applied.
fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

impl Index {
    /// Read the workspace at `root`, indexing every package `scanned`
    /// accepts. Every member is listed in `packages`.
    pub(super) fn load(root: &Path, scanned: impl Fn(&Package) -> bool) -> Result<Self> {
        let root = fs::canonicalize(root).map_err(|error| Error::io(root, error))?;
        let document = metadata::load_members(&root.join("Cargo.toml"))?;
        let mut index = Self::default();
        let members = array(&document, "packages")?;
        let mut libraries = HashMap::new();
        for package in members {
            for target in array(package, "targets")? {
                let kinds = array(target, "kind")?;
                if kinds.iter().any(|kind| {
                    matches!(
                        kind.as_str(),
                        Some("lib" | "rlib" | "proc-macro" | "staticlib" | "cdylib")
                    )
                }) {
                    libraries.insert(
                        text(package, "name")?.to_owned(),
                        text(target, "name")?.replace('-', "_"),
                    );
                }
            }
        }
        for package in members {
            let name = text(package, "name")?.to_owned();
            let manifest = PathBuf::from(text(package, "manifest_path")?);
            let directory = manifest.parent().unwrap_or(&root).to_path_buf();
            let record = Package {
                dir: relative_to(&root, &directory),
                name: name.clone(),
            };
            let scan = scanned(&record);
            let package_id = index.packages.len();
            index.packages.push(record);
            if !scan {
                continue;
            }
            let library = libraries.get(&name).cloned();
            let mut deps = HashMap::new();
            for dependency in array(package, "dependencies")? {
                if dependency.get("kind").and_then(Value::as_str) == Some("dev") {
                    continue;
                }
                let real = text(dependency, "name")?;
                let canonical = libraries
                    .get(real)
                    .cloned()
                    .unwrap_or_else(|| real.replace('-', "_"));
                let local = dependency
                    .get("rename")
                    .and_then(Value::as_str)
                    .map_or_else(|| canonical.clone(), |rename| rename.replace('-', "_"));
                deps.insert(local, canonical);
            }
            for target in array(package, "targets")? {
                let kinds: Vec<&str> = array(target, "kind")?
                    .iter()
                    .filter_map(Value::as_str)
                    .collect();
                if kinds
                    .iter()
                    .any(|kind| matches!(*kind, "test" | "bench" | "example"))
                {
                    continue;
                }
                let target_name = text(target, "name")?.replace('-', "_");
                let is_library = library.as_deref() == Some(target_name.as_str())
                    && !kinds.contains(&"bin")
                    && !kinds.contains(&"custom-build");
                let ident = if is_library || library.is_none() && kinds.contains(&"bin") {
                    target_name.clone()
                } else {
                    format!("{}#{target_name}", name.replace('-', "_"))
                };
                let mut target_deps = deps.clone();
                if let Some(library) = library.as_ref().filter(|_| !is_library) {
                    target_deps.insert(library.clone(), library.clone());
                }
                index.targets.push(Target {
                    ident,
                    package: package_id,
                    root_file: PathBuf::from(text(target, "src_path")?),
                    root: 0,
                    deps: target_deps,
                });
            }
        }
        let mut impls = Vec::new();
        for target in 0..index.targets.len() {
            let file = index.targets[target].root_file.clone();
            let ident = index.targets[target].ident.clone();
            index.by_ident.insert(ident.clone(), target);
            let module = index.load_file(&root, target, &file, ident, None, true, &mut impls)?;
            if let Some(module) = module {
                index.targets[target].root = module;
            }
        }
        index.attach_impls(impls);
        Ok(index)
    }

    #[allow(clippy::too_many_arguments)]
    fn load_file(
        &mut self,
        root: &Path,
        target: usize,
        file: &Path,
        path: String,
        parent: Option<usize>,
        is_mod_root: bool,
        impls: &mut Vec<PendingImpl>,
    ) -> Result<Option<usize>> {
        let source = fs::read_to_string(file).map_err(|error| Error::io(file, error))?;
        let syntax = syn::parse_file(&source).map_err(|error| Error::source_parse(file, error))?;
        if is_test_only(&syntax.attrs) {
            return Ok(None);
        }
        let file_id = self.files.len();
        let items = syntax.items.clone();
        self.files.push(SourceFile {
            relative: relative_to(root, file),
            syntax,
        });
        let module = self.add_module(target, path, file_id, parent);
        let directory = file.parent().unwrap_or(root);
        let children = if is_mod_root || file.file_name().is_some_and(|name| name == "mod.rs") {
            directory.to_path_buf()
        } else {
            directory.join(file.file_stem().unwrap_or_default())
        };
        self.add_items(root, module, &items, directory, &children, impls)?;
        Ok(Some(module))
    }

    fn add_module(
        &mut self,
        target: usize,
        path: String,
        file: usize,
        parent: Option<usize>,
    ) -> usize {
        self.modules.push(Module {
            target,
            path,
            file,
            parent,
            items: BTreeMap::new(),
            imports: BTreeMap::new(),
            globs: Vec::new(),
        });
        self.modules.len() - 1
    }

    fn define(&mut self, module: usize, name: String, def: Def) {
        self.modules[module]
            .items
            .entry(name)
            .or_default()
            .push(def);
    }

    #[allow(clippy::too_many_arguments)]
    fn add_type(
        &mut self,
        module: usize,
        ident: &syn::Ident,
        visibility: &syn::Visibility,
        variants: BTreeSet<String>,
        alias: Option<syn::Type>,
        fields: Vec<(String, syn::Type)>,
    ) -> usize {
        let name = ident.to_string();
        let id = self.types.len();
        self.types.push(TypeItem {
            item: ItemInfo {
                path: format!("{}::{name}", self.modules[module].path),
                kind: Kind::Type,
                module,
                public: is_public(visibility),
            },
            variants,
            assoc: BTreeMap::new(),
            alias,
            fields,
        });
        self.define(module, name, Def::Type(id));
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn push_value(
        &mut self,
        module: usize,
        path: String,
        kind: Kind,
        public: bool,
        signature: Option<syn::Signature>,
        self_type: Option<usize>,
        impl_generics: Option<syn::Generics>,
    ) -> usize {
        self.values.push(ValueItem {
            item: ItemInfo {
                path,
                kind,
                module,
                public,
            },
            signature,
            self_type,
            impl_generics,
        });
        self.values.len() - 1
    }

    fn add_free_value(
        &mut self,
        module: usize,
        ident: &syn::Ident,
        kind: Kind,
        visibility: &syn::Visibility,
        signature: Option<syn::Signature>,
    ) {
        let name = ident.to_string();
        let path = format!("{}::{name}", self.modules[module].path);
        let id = self.push_value(
            module,
            path,
            kind,
            is_public(visibility),
            signature,
            None,
            None,
        );
        self.define(module, name, Def::Value(id));
    }

    fn add_use(&mut self, module: usize, leading_colon: bool, tree: &syn::UseTree) {
        let mut leaves = Vec::new();
        use_leaves(tree, &mut Vec::new(), &mut leaves);
        for (mut segments, mut bound, _) in leaves {
            if segments.last().is_some_and(|last| last == "self") {
                segments.pop();
                if bound == "self" {
                    bound = segments.last().cloned().unwrap_or_default();
                }
            }
            if bound == "*" {
                segments.pop();
                self.modules[module].globs.push(Import {
                    leading_colon,
                    segments,
                });
            } else if bound != "_" {
                self.modules[module]
                    .imports
                    .entry(bound)
                    .or_default()
                    .push(Import {
                        leading_colon,
                        segments,
                    });
            }
        }
    }

    fn add_items(
        &mut self,
        root: &Path,
        module: usize,
        items: &[syn::Item],
        file_directory: &Path,
        child_directory: &Path,
        impls: &mut Vec<PendingImpl>,
    ) -> Result<()> {
        for item in items {
            match item {
                syn::Item::Mod(declaration) => self.add_mod(
                    root,
                    module,
                    declaration,
                    file_directory,
                    child_directory,
                    impls,
                )?,
                syn::Item::Use(node) if !is_test_only(&node.attrs) => {
                    self.add_use(module, node.leading_colon.is_some(), &node.tree);
                }
                syn::Item::ExternCrate(node) if !is_test_only(&node.attrs) => {
                    let name = node
                        .rename
                        .as_ref()
                        .map_or_else(|| node.ident.to_string(), |(_, alias)| alias.to_string());
                    self.modules[module]
                        .imports
                        .entry(name)
                        .or_default()
                        .push(Import {
                            leading_colon: true,
                            segments: vec![node.ident.to_string()],
                        });
                }
                syn::Item::Struct(node) if !is_test_only(&node.attrs) => {
                    let fields = fields_of(&node.fields, None);
                    self.add_type(
                        module,
                        &node.ident,
                        &node.vis,
                        BTreeSet::new(),
                        None,
                        fields,
                    );
                }
                syn::Item::Union(node) if !is_test_only(&node.attrs) => {
                    let fields = node
                        .fields
                        .named
                        .iter()
                        .map(|field| {
                            (
                                field
                                    .ident
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_default(),
                                field.ty.clone(),
                            )
                        })
                        .collect();
                    self.add_type(
                        module,
                        &node.ident,
                        &node.vis,
                        BTreeSet::new(),
                        None,
                        fields,
                    );
                }
                syn::Item::Enum(node) if !is_test_only(&node.attrs) => {
                    let variants = node
                        .variants
                        .iter()
                        .map(|variant| variant.ident.to_string())
                        .collect();
                    let fields = node
                        .variants
                        .iter()
                        .filter(|variant| !is_test_only(&variant.attrs))
                        .flat_map(|variant| fields_of(&variant.fields, Some(&variant.ident)))
                        .collect();
                    self.add_type(module, &node.ident, &node.vis, variants, None, fields);
                }
                syn::Item::Type(node) if !is_test_only(&node.attrs) => {
                    self.add_type(
                        module,
                        &node.ident,
                        &node.vis,
                        BTreeSet::new(),
                        Some((*node.ty).clone()),
                        Vec::new(),
                    );
                }
                syn::Item::Trait(node) if !is_test_only(&node.attrs) => {
                    let id = self.add_type(
                        module,
                        &node.ident,
                        &node.vis,
                        BTreeSet::new(),
                        None,
                        Vec::new(),
                    );
                    for member in &node.items {
                        let (ident, kind, signature) = match member {
                            syn::TraitItem::Fn(method) if !is_test_only(&method.attrs) => {
                                (&method.sig.ident, Kind::Function, Some(method.sig.clone()))
                            }
                            syn::TraitItem::Const(constant) if !is_test_only(&constant.attrs) => {
                                (&constant.ident, Kind::Const, None)
                            }
                            _ => continue,
                        };
                        let path = format!("{}::{ident}", self.types[id].item.path);
                        let value = self.push_value(
                            module,
                            path,
                            kind,
                            is_public(&node.vis),
                            signature,
                            Some(id),
                            Some(node.generics.clone()),
                        );
                        self.types[id].assoc.insert(ident.to_string(), value);
                    }
                }
                syn::Item::Fn(node) if !is_test_only(&node.attrs) => self.add_free_value(
                    module,
                    &node.sig.ident,
                    Kind::Function,
                    &node.vis,
                    Some(node.sig.clone()),
                ),
                syn::Item::Const(node) if !is_test_only(&node.attrs) && node.ident != "_" => {
                    self.add_free_value(module, &node.ident, Kind::Const, &node.vis, None);
                }
                syn::Item::Static(node) if !is_test_only(&node.attrs) => {
                    self.add_free_value(module, &node.ident, Kind::Const, &node.vis, None);
                }
                syn::Item::Impl(node) if !is_test_only(&node.attrs) => impls.push(PendingImpl {
                    module,
                    block: node.clone(),
                }),
                _ => {}
            }
        }
        Ok(())
    }

    fn add_mod(
        &mut self,
        root: &Path,
        module: usize,
        declaration: &syn::ItemMod,
        file_directory: &Path,
        child_directory: &Path,
        impls: &mut Vec<PendingImpl>,
    ) -> Result<()> {
        if is_test_only(&declaration.attrs) {
            return Ok(());
        }
        let name = declaration.ident.to_string();
        let path = format!("{}::{name}", self.modules[module].path);
        let target = self.modules[module].target;
        let child = if let Some((_, items)) = &declaration.content {
            let file = self.modules[module].file;
            let child = self.add_module(target, path, file, Some(module));
            let nested = child_directory.join(&name);
            self.add_items(root, child, items, &nested, &nested, impls)?;
            Some(child)
        } else {
            let attribute = declaration.attrs.iter().find_map(|attribute| {
                let syn::Meta::NameValue(pair) = &attribute.meta else {
                    return None;
                };
                match (&pair.value, pair.path.is_ident("path")) {
                    (
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(value),
                            ..
                        }),
                        true,
                    ) => Some(value.value()),
                    _ => None,
                }
            });
            let (file, is_mod_root) = match attribute {
                Some(value) => (normalized(&file_directory.join(value)), true),
                None => {
                    let flat = child_directory.join(format!("{name}.rs"));
                    if flat.is_file() {
                        (flat, false)
                    } else {
                        (child_directory.join(&name).join("mod.rs"), false)
                    }
                }
            };
            let package = &self.packages[self.targets[target].package];
            let package_relative = relative_to(&root.join(&package.dir), &file);
            let in_test_directory = package_relative
                .split('/')
                .next()
                .is_some_and(|first| matches!(first, "tests" | "benches" | "examples"));
            if in_test_directory {
                return Ok(());
            }
            self.load_file(root, target, &file, path, Some(module), is_mod_root, impls)?
        };
        if let Some(child) = child {
            self.define(module, name, Def::Module(child));
        }
        Ok(())
    }

    /// Record every impl block's associated items on its `Self` type.
    fn attach_impls(&mut self, impls: Vec<PendingImpl>) {
        for pending in impls {
            let syn::Type::Path(self_path) = pending.block.self_ty.as_ref() else {
                continue;
            };
            let Resolved::Type(id) = self.resolve_path(pending.module, &self_path.path, None)
            else {
                continue;
            };
            let trait_impl = pending.block.trait_.is_some();
            for member in &pending.block.items {
                let (ident, kind, signature, visibility) = match member {
                    syn::ImplItem::Fn(method) if !is_test_only(&method.attrs) => (
                        &method.sig.ident,
                        Kind::Function,
                        Some(method.sig.clone()),
                        &method.vis,
                    ),
                    syn::ImplItem::Const(constant) if !is_test_only(&constant.attrs) => {
                        (&constant.ident, Kind::Const, None, &constant.vis)
                    }
                    _ => continue,
                };
                let name = ident.to_string();
                // A trait impl's items are reached through the trait's
                // visibility; an inherent item of the same name wins.
                if trait_impl && self.types[id].assoc.contains_key(&name) {
                    continue;
                }
                let path = format!("{}::{name}", self.types[id].item.path);
                let value = self.push_value(
                    pending.module,
                    path,
                    kind,
                    trait_impl || is_public(visibility),
                    signature,
                    Some(id),
                    Some(pending.block.generics.clone()),
                );
                self.types[id].assoc.insert(name, value);
            }
        }
        self.cache.borrow_mut().clear();
    }

    /// The root module of the target named `ident`.
    fn root_of(&self, ident: &str) -> Option<usize> {
        self.by_ident
            .get(ident)
            .map(|&target| self.targets[target].root)
    }

    fn external(&self, name: &str) -> Resolved {
        match self.root_of(name) {
            Some(root) => Resolved::Module(root),
            None => Resolved::External(name.to_owned()),
        }
    }

    /// The crate `name` names from `module`'s target, if it is one.
    fn extern_crate(&self, module: usize, name: &str) -> Option<Resolved> {
        let target = &self.targets[self.modules[module].target];
        if let Some(canonical) = target.deps.get(name) {
            return Some(self.external(canonical));
        }
        STANDARD_CRATES
            .contains(&name)
            .then(|| Resolved::External(name.to_owned()))
    }

    /// What `name` stands for in `module`'s namespace.
    pub(super) fn lookup(&self, module: usize, name: &str) -> Option<(Resolved, Binding)> {
        let key = (module, name.to_owned());
        if let Some(found) = self.cache.borrow().get(&key) {
            return found.clone();
        }
        if !self.active.borrow_mut().insert(key.clone()) {
            return None;
        }
        let found = self.lookup_uncached(module, name);
        self.active.borrow_mut().remove(&key);
        // A failure found while another lookup is open may be only that
        // lookup's cycle guard, so it is not remembered.
        if found.is_some() || self.active.borrow().is_empty() {
            self.cache.borrow_mut().insert(key, found.clone());
        }
        found
    }

    fn lookup_uncached(&self, module: usize, name: &str) -> Option<(Resolved, Binding)> {
        let record = &self.modules[module];
        if let Some(defs) = record.items.get(name) {
            let def = defs
                .iter()
                .find(|def| !matches!(def, Def::Value(_)))
                .or_else(|| defs.first())?;
            return Some((
                match *def {
                    Def::Module(id) => Resolved::Module(id),
                    Def::Type(id) => Resolved::Type(id),
                    Def::Value(id) => Resolved::Value(id),
                },
                Binding::Local,
            ));
        }
        for import in record.imports.get(name).into_iter().flatten() {
            let resolved = self.resolve_import(module, import);
            if resolved != Resolved::Unknown {
                return Some((resolved, Binding::Imported));
            }
        }
        for glob in &record.globs {
            let found = match self.resolve_import(module, glob) {
                Resolved::Module(target) if target != module => {
                    self.lookup(target, name).map(|(resolved, _)| resolved)
                }
                Resolved::Type(id) if self.types[id].variants.contains(name) => {
                    Some(Resolved::Type(id))
                }
                _ => None,
            };
            if let Some(resolved) = found {
                return Some((resolved, Binding::Imported));
            }
        }
        None
    }

    fn resolve_import(&self, module: usize, import: &Import) -> Resolved {
        self.resolve(module, import.leading_colon, &import.segments, None)
    }

    /// Resolve `path` from `module`, with `self_type` as `Self`.
    pub(super) fn resolve_path(
        &self,
        module: usize,
        path: &syn::Path,
        self_type: Option<usize>,
    ) -> Resolved {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        self.resolve(module, path.leading_colon.is_some(), &segments, self_type)
    }

    /// Resolve `segments` from `module`, following a `type` alias at the end
    /// to the item it stands for.
    pub(super) fn resolve(
        &self,
        module: usize,
        leading_colon: bool,
        segments: &[String],
        self_type: Option<usize>,
    ) -> Resolved {
        let Some((first, rest)) = segments.split_first() else {
            return Resolved::Unknown;
        };
        let start = if leading_colon {
            self.external(first)
        } else {
            match first.as_str() {
                "crate" | "$crate" => {
                    Resolved::Module(self.targets[self.modules[module].target].root)
                }
                "self" => Resolved::Module(module),
                "super" => self.modules[module]
                    .parent
                    .map_or(Resolved::Unknown, Resolved::Module),
                "Self" => self_type.map_or(Resolved::Unknown, Resolved::Type),
                name => match self.lookup(module, name) {
                    Some((resolved, _)) => resolved,
                    None => self.extern_crate(module, name).unwrap_or(Resolved::Unknown),
                },
            }
        };
        let end = rest
            .iter()
            .fold(start, |current, name| self.step(current, name));
        self.dealias(end)
    }

    /// One more path segment after `current`.
    pub(super) fn step(&self, current: Resolved, name: &str) -> Resolved {
        match current {
            Resolved::Module(module) => {
                if name == "super" {
                    return self.modules[module]
                        .parent
                        .map_or(Resolved::Unknown, Resolved::Module);
                }
                self.lookup(module, name)
                    .map_or(Resolved::Unknown, |(resolved, _)| resolved)
            }
            Resolved::Type(id) => match self.dealias(Resolved::Type(id)) {
                Resolved::Type(id) => match self.types[id].assoc.get(name) {
                    Some(&value) => Resolved::Value(value),
                    // A variant, or an item a derive or trait provides.
                    None => Resolved::Type(id),
                },
                other => self.step(other, name),
            },
            Resolved::External(path) => Resolved::External(format!("{path}::{name}")),
            Resolved::Value(_) | Resolved::Unknown => Resolved::Unknown,
        }
    }

    /// The item a `type` alias stands for; anything else unchanged.
    pub(super) fn dealias(&self, mut resolved: Resolved) -> Resolved {
        let mut seen = HashSet::new();
        while let Resolved::Type(id) = resolved {
            let record = &self.types[id];
            let Some(syn::Type::Path(target)) = &record.alias else {
                break;
            };
            if !seen.insert(id) || target.qself.is_some() {
                break;
            }
            let segments: Vec<String> = target
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let Some((first, rest)) = segments.split_first() else {
                break;
            };
            let module = record.item.module;
            let start = match first.as_str() {
                "crate" => Resolved::Module(self.targets[self.modules[module].target].root),
                "self" => Resolved::Module(module),
                "super" => self.modules[module]
                    .parent
                    .map_or(Resolved::Unknown, Resolved::Module),
                name => match self.lookup(module, name) {
                    Some((found, _)) => found,
                    None => self.extern_crate(module, name).unwrap_or(Resolved::Unknown),
                },
            };
            let end = rest
                .iter()
                .fold(start, |current, name| self.step(current, name));
            if end == Resolved::Unknown {
                break;
            }
            resolved = end;
        }
        resolved
    }

    /// The `type` alias definition of `id`, if it is an alias.
    pub(super) fn alias_of(&self, id: usize) -> Option<&syn::Type> {
        self.types[id].alias.as_ref()
    }

    /// Every item of the index.
    pub(super) fn items(&self) -> impl Iterator<Item = &ItemInfo> {
        self.types
            .iter()
            .map(|record| &record.item)
            .chain(self.values.iter().map(|record| &record.item))
    }

    pub(super) fn file_of(&self, module: usize) -> &str {
        &self.files[self.modules[module].file].relative
    }

    pub(super) fn package_of(&self, module: usize) -> &Package {
        &self.packages[self.targets[self.modules[module].target].package]
    }

    /// The module named `name` declared directly inside `parent`.
    pub(super) fn module_named(&self, parent: usize, name: &str) -> Option<usize> {
        self.modules[parent]
            .items
            .get(name)?
            .iter()
            .find_map(|def| match def {
                Def::Module(id) => Some(*id),
                _ => None,
            })
    }
}

fn fields_of(fields: &syn::Fields, variant: Option<&syn::Ident>) -> Vec<(String, syn::Type)> {
    fields
        .iter()
        .enumerate()
        .filter(|(_, field)| !is_test_only(&field.attrs))
        .map(|(position, field)| {
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| position.to_string(), ToString::to_string);
            let name = match variant {
                Some(variant) => format!("{variant}.{name}"),
                None => name,
            };
            (name, field.ty.clone())
        })
        .collect()
}

/// The 1-based line a syntax node starts on.
pub(super) fn line_of(node: &impl Spanned) -> usize {
    node.span().start().line
}
