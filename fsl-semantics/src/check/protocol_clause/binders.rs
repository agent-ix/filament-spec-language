// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113's binder no-shadowing rule: a binder shadowing a model/profile
//! alias, a native declaration of the package or an earlier binder of the
//! same protocol.

use std::collections::{BTreeMap, BTreeSet};

use fsl_forms::{BinderKind, ProtocolDeclarationForm, ProtocolNodeKind};
use fsl_foundation::Span;
use ix_trace_rs::implements;

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
#[implements("FR-513-AC-12")]
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
                    scopes_overlap_at_declaration(protocol, earlier_scope, &binder_scope)
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

/// Whether `earlier` is still active where `later` is declared. A binder
/// inherited from an enclosing scope remains active in its descendants; a
/// binder in one parallel branch can also be active while another branch is
/// checked.
fn scopes_overlap_at_declaration(
    protocol: &ProtocolDeclarationForm,
    earlier: &[ScopeSegment],
    later: &[ScopeSegment],
) -> bool {
    if scope_is_prefix(earlier, later) {
        return true;
    }

    let shared = earlier
        .iter()
        .zip(later)
        .take_while(|(left, right)| left == right)
        .count();
    let (Some(ScopeSegment::Control(earlier)), Some(ScopeSegment::Control(later))) =
        (earlier.get(shared), later.get(shared))
    else {
        return false;
    };

    parallel_branch_siblings(protocol, *earlier, *later)
}

/// Whether these distinct control scopes are sibling branches of one
/// `parallel` control.
fn parallel_branch_siblings(
    protocol: &ProtocolDeclarationForm,
    earlier: usize,
    later: usize,
) -> bool {
    let (Some(earlier_scope), Some(later_scope)) =
        (protocol.scopes.get(earlier), protocol.scopes.get(later))
    else {
        return false;
    };
    let Some(parent) = earlier_scope.parent else {
        return false;
    };
    if later_scope.parent != Some(parent) || earlier == later {
        return false;
    }

    let Some(parent_scope) = protocol.scopes.get(parent.index()) else {
        return false;
    };
    let parent_is_parallel = protocol.parallels.iter().any(|parallel| {
        protocol
            .declarations
            .get(parallel.declaration)
            .is_some_and(|declaration| declaration.name.span == parent_scope.name.span)
    });
    if !parent_is_parallel {
        return false;
    }

    [earlier_scope, later_scope]
        .into_iter()
        .all(|branch_scope| {
            protocol.declarations.iter().any(|declaration| {
                declaration.kind == ProtocolNodeKind::Branch
                    && declaration.name.span == branch_scope.name.span
                    && declaration.scope == Some(parent)
            })
        })
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
