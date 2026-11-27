// SPDX-License-Identifier: AGPL-3.0-or-later
//! What a file's non-test code names (FR-1051 "Names and detection"): its
//! `use` trees, its qualified paths, the bare names its imports bind and the
//! path token runs inside macro arguments. Method calls, operators and macro
//! expansions are not seen.

use std::collections::{BTreeSet, HashMap};

use ix_trace_rs::implements;
use syn::{punctuated::Punctuated, visit::Visit};

use super::index::{is_test_only, line_of, Binding, Index, Kind, Resolved};
use crate::api_surface::{flatten_tokens, use_leaves, LocatedToken, Token};

/// One naming of an item.
#[derive(Clone, Debug)]
pub(super) struct Reference {
    /// A type, a function or constant, or an external path.
    pub(super) target: Resolved,
    pub(super) module: usize,
    pub(super) line: usize,
    /// The named items the site is inside, outermost first: `function`,
    /// `Type` or `Type::method`; empty at module level.
    pub(super) within: String,
}

#[derive(Default)]
struct BlockScope {
    names: HashMap<String, Resolved>,
    globs: Vec<Resolved>,
}

struct Scanner<'a> {
    index: &'a Index,
    module: usize,
    within: String,
    self_type: Option<usize>,
    blocks: Vec<BlockScope>,
    found: Vec<Reference>,
}

impl Scanner<'_> {
    fn record(&mut self, target: &Resolved, line: usize) {
        if matches!(
            target,
            Resolved::Type(_) | Resolved::Value(_) | Resolved::External(_)
        ) {
            self.found.push(Reference {
                target: target.clone(),
                module: self.module,
                line,
                within: self.within.clone(),
            });
        }
    }

    /// What `name` stands for here: a block's own `use`, then the module.
    fn first(&self, name: &str) -> Option<(Resolved, Binding)> {
        for scope in self.blocks.iter().rev() {
            if let Some(found) = scope.names.get(name) {
                return Some((found.clone(), Binding::Imported));
            }
        }
        if let Some(found) = self.index.lookup(self.module, name) {
            return Some(found);
        }
        for scope in self.blocks.iter().rev() {
            for glob in &scope.globs {
                let found = self.index.step(glob.clone(), name);
                if found != Resolved::Unknown && found != *glob {
                    return Some((found, Binding::Imported));
                }
            }
        }
        None
    }

    /// Record what the path `segments` names, and every type it passes
    /// through (`T::f` names both `T` and `f`).
    fn path(&mut self, leading_colon: bool, segments: &[String], line: usize) {
        let Some((head, rest)) = segments.split_first() else {
            return;
        };
        if rest.is_empty() && !leading_colon {
            // A bare name counts only where an import binds it.
            if head != "Self" {
                if let Some((found, Binding::Imported)) = self.first(head) {
                    let found = self.index.dealias(found);
                    self.record(&found, line);
                }
            }
            return;
        }
        let keyword = matches!(
            head.as_str(),
            "crate" | "$crate" | "self" | "super" | "Self"
        );
        let start = match self.first(head) {
            Some((found, _)) if !keyword && !leading_colon => found,
            _ => self.index.resolve(
                self.module,
                leading_colon,
                std::slice::from_ref(head),
                self.self_type,
            ),
        };
        self.walk(start, rest, line);
    }

    fn walk(&mut self, start: Resolved, rest: &[String], line: usize) {
        let mut current = start;
        let mut parent = None;
        for name in rest {
            if matches!(current, Resolved::Type(_)) {
                let passed = self.index.dealias(current.clone());
                self.record(&passed, line);
            }
            if let Resolved::Module(module) = current {
                parent = Some((module, name));
            }
            current = self.index.step(current, name);
        }
        let end = self.index.dealias(current);
        if let (Resolved::Module(_), Some((module, name))) = (&end, parent) {
            self.namesake(module, name, line);
        }
        self.record(&end, line);
    }

    /// Record the item a module path's last name also stands for.
    fn namesake(&mut self, module: usize, name: &str, line: usize) {
        if let Some(found) = self.index.namesake(module, name) {
            let found = self.index.dealias(found);
            self.record(&found, line);
        }
    }

    /// Record the function paths a `serde` attribute writes as strings.
    fn serde_paths(&mut self, meta: &syn::Meta) {
        let syn::Meta::List(list) = meta else {
            return;
        };
        let Ok(members) =
            list.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        else {
            return;
        };
        if list.path.is_ident("cfg_attr") {
            for member in &members {
                self.serde_paths(member);
            }
            return;
        }
        if !list.path.is_ident("serde") {
            return;
        }
        for member in &members {
            let syn::Meta::NameValue(pair) = member else {
                continue;
            };
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) = &pair.value
            else {
                continue;
            };
            let suffixes: &[&str] = if pair.path.is_ident("with") {
                &["serialize", "deserialize"]
            } else if [
                "default",
                "serialize_with",
                "deserialize_with",
                "skip_serializing_if",
            ]
            .iter()
            .any(|key| pair.path.is_ident(key))
            {
                &[""]
            } else {
                continue;
            };
            let Ok(path) = text.parse::<syn::Path>() else {
                continue;
            };
            for suffix in suffixes {
                let mut segments: Vec<String> = path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect();
                if !suffix.is_empty() {
                    segments.push((*suffix).to_owned());
                }
                // A one-segment path names an item of this module.
                if let [name] = segments.as_slice() {
                    if let Some((found, _)) = self.first(name) {
                        let found = self.index.dealias(found);
                        self.record(&found, line_of(text));
                    }
                } else {
                    self.path(path.leading_colon.is_some(), &segments, line_of(text));
                }
            }
        }
    }

    fn qualified(&mut self, qself: Option<&syn::QSelf>, path: &syn::Path) {
        let line = line_of(path);
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        match qself {
            // `<Type>::item`.
            Some(qself) if qself.position == 0 => {
                if let syn::Type::Path(ty) = qself.ty.as_ref() {
                    let start = self
                        .index
                        .resolve_path(self.module, &ty.path, self.self_type);
                    self.walk(start, &segments, line);
                }
            }
            _ => self.path(path.leading_colon.is_some(), &segments, line),
        }
    }

    fn use_target(&self, leading_colon: bool, segments: &[String]) -> Resolved {
        self.index
            .resolve(self.module, leading_colon, segments, self.self_type)
    }

    fn tokens(&mut self, stream: proc_macro2::TokenStream) {
        let mut tokens = Vec::new();
        flatten_tokens(stream, &mut tokens);
        let punct = |tokens: &[LocatedToken], at: usize, wanted: char| {
            tokens
                .get(at)
                .is_some_and(|token| token.token == Token::Punct(wanted))
        };
        let mut position = 0;
        while position < tokens.len() {
            let Token::Ident(first) = &tokens[position].token else {
                position += 1;
                continue;
            };
            let line = tokens[position].line;
            let before = |back: usize| position.checked_sub(back);
            // `.name` is a field or a method; `..name` is a range end.
            let member = before(1).is_some_and(|at| punct(&tokens, at, '.'))
                && !before(2).is_some_and(|at| punct(&tokens, at, '.'));
            let rooted = before(1).is_some_and(|at| punct(&tokens, at, ':'))
                && before(2).is_some_and(|at| punct(&tokens, at, ':'))
                && !before(3).is_some_and(|at| {
                    matches!(tokens[at].token, Token::Ident(_) | Token::Punct('>'))
                });
            let mut segments = vec![first.clone()];
            let mut end = position + 1;
            while punct(&tokens, end, ':') && punct(&tokens, end + 1, ':') {
                let Some(Token::Ident(next)) = tokens.get(end + 2).map(|token| &token.token) else {
                    break;
                };
                segments.push(next.clone());
                end += 3;
            }
            position = end;
            if member {
                continue;
            }
            // `name!` is a macro and `name:` a field or a binding.
            let labelled = segments.len() == 1
                && (punct(&tokens, end, '!')
                    || punct(&tokens, end, ':') && !punct(&tokens, end + 1, ':'));
            if !labelled {
                self.path(rooted, &segments, line);
            }
        }
    }

    fn named(&mut self, name: &str, self_type: Option<usize>, visit: impl FnOnce(&mut Self)) {
        let previous = self.within.clone();
        let previous_self = self.self_type;
        // Items inside a function body belong to that function.
        if previous.is_empty() {
            self.within = name.to_owned();
        }
        if self_type.is_some() {
            self.self_type = self_type;
        }
        visit(self);
        self.within = previous;
        self.self_type = previous_self;
    }

    fn type_here(&self, ident: &syn::Ident) -> Option<usize> {
        match self.index.lookup(self.module, &ident.to_string()) {
            Some((Resolved::Type(id), _)) => Some(id),
            _ => None,
        }
    }
}

fn item_attributes(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(node) => &node.attrs,
        syn::Item::Enum(node) => &node.attrs,
        syn::Item::ExternCrate(node) => &node.attrs,
        syn::Item::Fn(node) => &node.attrs,
        syn::Item::ForeignMod(node) => &node.attrs,
        syn::Item::Impl(node) => &node.attrs,
        syn::Item::Macro(node) => &node.attrs,
        syn::Item::Mod(node) => &node.attrs,
        syn::Item::Static(node) => &node.attrs,
        syn::Item::Struct(node) => &node.attrs,
        syn::Item::Trait(node) => &node.attrs,
        syn::Item::TraitAlias(node) => &node.attrs,
        syn::Item::Type(node) => &node.attrs,
        syn::Item::Union(node) => &node.attrs,
        syn::Item::Use(node) => &node.attrs,
        _ => &[],
    }
}

macro_rules! unless_test_only {
    ($visit:ident, $node:ty) => {
        fn $visit(&mut self, node: &'ast $node) {
            if !is_test_only(&node.attrs) {
                syn::visit::$visit(self, node);
            }
        }
    };
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    unless_test_only!(visit_local, syn::Local);
    unless_test_only!(visit_field, syn::Field);
    unless_test_only!(visit_variant, syn::Variant);
    unless_test_only!(visit_arm, syn::Arm);
    unless_test_only!(visit_expr_block, syn::ExprBlock);

    // Of attribute and visibility paths, FR-1051 reads only the function
    // paths of `serde` attributes.
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        self.serde_paths(&node.meta);
    }
    fn visit_visibility(&mut self, _: &'ast syn::Visibility) {}

    fn visit_item(&mut self, item: &'ast syn::Item) {
        if is_test_only(item_attributes(item)) {
            return;
        }
        match item {
            syn::Item::Fn(node) => {
                let name = node.sig.ident.to_string();
                self.named(&name, None, |scanner| {
                    syn::visit::visit_item(scanner, item);
                });
            }
            syn::Item::Struct(syn::ItemStruct { ident, .. })
            | syn::Item::Enum(syn::ItemEnum { ident, .. })
            | syn::Item::Trait(syn::ItemTrait { ident, .. })
            | syn::Item::Type(syn::ItemType { ident, .. }) => {
                let self_type = self.type_here(ident);
                self.named(&ident.to_string(), self_type, |scanner| {
                    syn::visit::visit_item(scanner, item);
                });
            }
            _ => syn::visit::visit_item(self, item),
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        // An out-of-line module is scanned from its own file.
        let Some((_, items)) = &node.content else {
            return;
        };
        if let Some(child) = self
            .index
            .module_named(self.module, &node.ident.to_string())
        {
            let previous = std::mem::replace(&mut self.module, child);
            for item in items {
                self.visit_item(item);
            }
            self.module = previous;
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let (name, self_type) = match node.self_ty.as_ref() {
            syn::Type::Path(path) => {
                let name = path
                    .path
                    .segments
                    .last()
                    .map(|segment| segment.ident.to_string())
                    .unwrap_or_default();
                let self_type = match self.index.resolve_path(self.module, &path.path, None) {
                    Resolved::Type(id) => Some(id),
                    _ => None,
                };
                (name, self_type)
            }
            _ => (String::new(), None),
        };
        if let Some((_, path, _)) = &node.trait_ {
            self.within = name.clone();
            self.qualified(None, path);
            self.within.clear();
        }
        let previous = std::mem::replace(&mut self.self_type, self_type);
        let outer = std::mem::replace(&mut self.within, name.clone());
        self.visit_generics(&node.generics);
        self.visit_type(&node.self_ty);
        for member in &node.items {
            let (attributes, member_name) = match member {
                syn::ImplItem::Fn(method) => (&method.attrs, method.sig.ident.to_string()),
                syn::ImplItem::Const(constant) => (&constant.attrs, constant.ident.to_string()),
                syn::ImplItem::Type(alias) => (&alias.attrs, alias.ident.to_string()),
                _ => {
                    self.visit_impl_item(member);
                    continue;
                }
            };
            if !is_test_only(attributes) {
                self.within = format!("{name}::{member_name}");
                self.visit_impl_item(member);
                self.within = name.clone();
            }
        }
        self.within = outer;
        self.self_type = previous;
    }

    fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
        let attributes: &[syn::Attribute] = match node {
            syn::TraitItem::Fn(method) => &method.attrs,
            syn::TraitItem::Const(constant) => &constant.attrs,
            syn::TraitItem::Type(alias) => &alias.attrs,
            _ => &[],
        };
        if !is_test_only(attributes) {
            syn::visit::visit_trait_item(self, node);
        }
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        let mut scope = BlockScope::default();
        for statement in &node.stmts {
            let syn::Stmt::Item(syn::Item::Use(declaration)) = statement else {
                continue;
            };
            if is_test_only(&declaration.attrs) {
                continue;
            }
            let mut leaves = Vec::new();
            use_leaves(&declaration.tree, &mut Vec::new(), &mut leaves);
            for (mut segments, bound, _) in leaves {
                if segments.last().is_some_and(|last| last == "self") {
                    segments.pop();
                }
                let glob = bound == "*";
                if glob {
                    segments.pop();
                }
                let target = self.use_target(declaration.leading_colon.is_some(), &segments);
                if glob {
                    scope.globs.push(target);
                } else if target != Resolved::Unknown {
                    let name = if bound == "self" {
                        segments.last().cloned().unwrap_or_default()
                    } else {
                        bound
                    };
                    scope.names.insert(name, target);
                }
            }
        }
        self.blocks.push(scope);
        syn::visit::visit_block(self, node);
        self.blocks.pop();
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let mut leaves = Vec::new();
        use_leaves(&node.tree, &mut Vec::new(), &mut leaves);
        for (mut segments, bound, line) in leaves {
            if segments.last().is_some_and(|last| last == "self") {
                segments.pop();
            }
            let glob = bound == "*";
            if glob {
                segments.pop();
            }
            let target = self.use_target(node.leading_colon.is_some(), &segments);
            // A `use` of a module also brings in its namesake item.
            if let (Resolved::Module(_), false, Some((last, prefix))) =
                (&target, glob, segments.split_last())
            {
                if let Resolved::Module(parent) =
                    self.use_target(node.leading_colon.is_some(), prefix)
                {
                    self.namesake(parent, last, line);
                }
            }
            match target {
                // A glob over another crate brings in names this check cannot
                // enumerate, so the glob itself is what is named.
                Resolved::External(path) if glob => {
                    self.record(&Resolved::External(format!("{path}::*")), line);
                }
                // An imported crate or external module is judged by what is
                // named through it.
                Resolved::External(path) if !path.contains("::") => {}
                other => self.record(&other, line),
            }
        }
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        // A `macro_rules!` body is not read.
        if node.ident.is_none() {
            self.visit_macro(&node.mac);
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.tokens(node.tokens.clone());
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.qualified(None, node);
        for segment in &node.segments {
            self.visit_path_arguments(&segment.arguments);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        match &node.qself {
            Some(qself) => {
                self.visit_type(&qself.ty);
                self.qualified(Some(qself), &node.path);
            }
            None => self.visit_path(&node.path),
        }
    }

    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        match &node.qself {
            Some(qself) => {
                self.visit_type(&qself.ty);
                self.qualified(Some(qself), &node.path);
            }
            None => self.visit_path(&node.path),
        }
    }

    // A bare constant or unit variant in a pattern parses as a binding.
    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if node.by_ref.is_none() && node.mutability.is_none() && node.subpat.is_none() {
            if let Some((Resolved::Value(id), Binding::Imported)) =
                self.first(&node.ident.to_string())
            {
                if self.index.values[id].item.kind == Kind::Const {
                    self.record(&Resolved::Value(id), line_of(&node.ident));
                }
            }
        }
        syn::visit::visit_pat_ident(self, node);
    }
}

/// Every naming in the files `wanted` accepts (by module of the file's root).
#[implements("FR-1051-AC-5")]
pub(super) fn references(index: &Index, wanted: impl Fn(usize) -> bool) -> Vec<Reference> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    // A file's root module precedes its inline modules in the index.
    for (module, record) in index.modules.iter().enumerate() {
        if !seen.insert(record.file) || !wanted(module) {
            continue;
        }
        let mut scanner = Scanner {
            index,
            module,
            within: String::new(),
            self_type: None,
            blocks: Vec::new(),
            found: Vec::new(),
        };
        for item in &index.files[record.file].syntax.items {
            scanner.visit_item(item);
        }
        found.append(&mut scanner.found);
    }
    found
}
