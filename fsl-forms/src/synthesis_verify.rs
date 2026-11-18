// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2 dispatch for declarations whose detailed check/run forms are owned by
//! FR-753 and FR-754. Keep their source identity and complete spelling intact.

use fsl_cst::TokenKind;

use crate::dispatch::{Construct, FormsRefusal};
use crate::syntax::{
    DeclarationForm, DeclaredName, SynthesisForm, UsingAlias, VerificationPlanForm,
};
use crate::value::{significant_tokens, text, unexpected};

/// Build the source-bearing S2 form for `synthesis` (FR-754).
pub(crate) fn synthesis(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let (name, profile, source, span) = identity(construct)?;
    Ok(DeclarationForm::Synthesis(Box::new(SynthesisForm {
        name,
        profile,
        source,
        span,
    })))
}
/// Build the source-bearing S2 form for `verify` (FR-753).
pub(crate) fn verification_plan(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let (name, profile, source, span) = identity(construct)?;
    Ok(DeclarationForm::VerificationPlan(Box::new(
        VerificationPlanForm {
            name,
            profile,
            source,
            span,
        },
    )))
}

fn identity(
    construct: Construct<'_>,
) -> Result<(DeclaredName, UsingAlias, String, fsl_foundation::Span), FormsRefusal> {
    let tokens = significant_tokens(construct.cst, construct.node);
    let node = construct.node;
    let name_token = tokens
        .iter()
        .skip(1)
        .find(|token| token.kind() == TokenKind::Identifier)
        .copied()
        .ok_or_else(|| unexpected(node))?;
    let using_index = tokens
        .iter()
        .position(|token| token.spelling() == b"using")
        .ok_or_else(|| unexpected(node))?;
    let profile_token = tokens
        .get(using_index + 1)
        .filter(|token| token.kind() == TokenKind::Identifier)
        .copied()
        .ok_or_else(|| unexpected(node))?;
    let name = DeclaredName {
        name: text(name_token, node)?,
        span: name_token.span(),
    };
    let profile = UsingAlias {
        alias: text(profile_token, node)?,
        span: profile_token.span(),
    };
    let source = construct
        .cst
        .source()
        .text()
        .get(node.span().start..node.span().end)
        .ok_or_else(|| unexpected(node))?
        .to_owned();
    Ok((name, profile, source, node.span()))
}
