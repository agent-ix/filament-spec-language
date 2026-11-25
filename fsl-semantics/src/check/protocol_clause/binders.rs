// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113's binder no-shadowing rule: a binder shadowing a model/profile
//! alias, a native declaration of the package or an earlier binder of the
//! same protocol.

use std::collections::{BTreeMap, BTreeSet};

use fsl_forms::{BinderKind, ProtocolDeclarationForm, ProtocolNodeKind};
use fsl_foundation::Span;

use super::refusal;
use crate::check::check::SourceScope;
use crate::check::refusal::{AliasKind, CheckRefusal, ProtocolAnchorCause, ShadowedDeclaration};
use crate::check::{Scope, Signatures};

/// FR-113 "Refusals" binder no-shadowing rule: every binder that shadows a
/// model/profile alias, a native declaration of the package or another
/// binder visible where it is declared, in source order.
///
/// Binders are checked in `protocol.binders`' own order (source order,
/// the S2 walk). A later binder shadows an earlier binder only when the
/// earlier binder's scope is active at the later declaration site. Named
/// control scopes are lexical: an enclosing scope remains active in its
/// descendants, while a completed child or sibling scope is no longer
/// active. The finish and compensation-template record binders need their
/// own declaration scope because neither is enclosed by a named control.
/// A package-level alias or native declaration is checked regardless of
/// binder scope.
pub(super) fn shadow_refusals(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Vec<(Span, CheckRefusal)> {
    let mut refusals = Vec::new();
    let mut seen: BTreeMap<String, Vec<(Vec<ScopeSegment>, Span)>> = BTreeMap::new();
    for binder in &protocol.binders {
        let binder_scope = binder_scope(protocol, binder);
        let shadowed = seen
            .get(binder.name.name.as_str())
            .and_then(|earlier| {
                earlier.iter().rev().find_map(|(earlier_scope, span)| {
                    scope_is_prefix(earlier_scope, &binder_scope)
                        .then_some(ShadowedDeclaration::Binder(*span))
                })
            })
            .or_else(|| {
                package_shadow(
                    &binder.name.name,
                    alias_names,
                    native_names,
                    scope.for_declaration(&protocol.name.name),
                    signatures,
                )
            });
        if let Some(shadowed) = shadowed {
            refusals.push((
                binder.name.span,
                refusal(ProtocolAnchorCause::Shadow {
                    name: binder.name.name.clone(),
                    shadowed,
                    span: binder.name.span,
                }),
            ));
        }
        seen.entry(binder.name.name.clone())
            .or_default()
            .push((binder_scope, binder.name.span));
    }
    refusals
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScopeSegment {
    Control(usize),
    Declaration(usize),
}

fn scope_is_prefix(earlier: &[ScopeSegment], later: &[ScopeSegment]) -> bool {
    earlier.len() <= later.len() && earlier.iter().zip(later).all(|(left, right)| left == right)
}

fn binder_scope(
    protocol: &ProtocolDeclarationForm,
    binder: &fsl_forms::BinderForm,
) -> Vec<ScopeSegment> {
    let mut path = Vec::new();
    let mut current = binder.scope;
    while let Some(scope) = current.filter(|_| path.len() < protocol.scopes.len()) {
        let entry = &protocol.scopes[scope.index()];
        path.push(ScopeSegment::Control(scope.index()));
        current = entry.parent;
    }
    path.reverse();

    if binder.kind == BinderKind::RecordBinder {
        if let Some((declaration_index, declaration)) = protocol
            .declarations
            .iter()
            .enumerate()
            .find(|(_, declaration)| {
                declaration.span.start <= binder.name.span.start
                    && binder.name.span.end <= declaration.span.end
                    && matches!(
                        declaration.kind,
                        ProtocolNodeKind::Finish | ProtocolNodeKind::CompensateTemplate
                    )
            })
        {
            match declaration.kind {
                ProtocolNodeKind::Finish => {
                    path.push(ScopeSegment::Declaration(declaration_index));
                }
                ProtocolNodeKind::CompensateTemplate => {
                    if let Some((scope_index, _)) =
                        protocol.scopes.iter().enumerate().find(|(_, entry)| {
                            entry.name.name == declaration.name.name
                                && entry.parent == declaration.scope
                        })
                    {
                        path.push(ScopeSegment::Control(scope_index));
                    }
                }
                _ => {}
            }
        }
    }

    path
}

/// Whether `name` is a model/profile alias or a native declaration of the
/// package (SR-765 FND-003: dimensions and units too), and which kind, for
/// [`shadow_refusals`]'s package-wide half of FR-113's no-shadowing rule.
fn package_shadow(
    name: &str,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: SourceScope<'_>,
    signatures: &Signatures,
) -> Option<ShadowedDeclaration> {
    if let Some(alias) = alias_names.get(name) {
        return Some(match alias {
            AliasKind::Profile => ShadowedDeclaration::ProfileAlias,
            AliasKind::Model => ShadowedDeclaration::ModelAlias,
        });
    }
    if !scope.named_types(name).is_empty() {
        return Some(ShadowedDeclaration::Type);
    }
    if signatures.declares(name) {
        return Some(ShadowedDeclaration::Function);
    }
    if native_names.contains(name) {
        return Some(ShadowedDeclaration::Quantity);
    }
    None
}
