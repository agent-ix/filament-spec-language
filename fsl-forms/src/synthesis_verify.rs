// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2 forms for verification plans and synthesis declarations (FR-753/FR-754).

use fsl_cst::{LosslessCst, Production, TokenKind};

use crate::dispatch::{Construct, FormsRefusal};
use crate::syntax::{
    DeclarationForm, DeclaredName, ExpressionForm, NameForm, SynthesisForm, SynthesisObjectiveForm,
    UsingAlias, VerificationPlanForm,
};
use crate::value::{
    declared_name, items, name_form, nodes_of, only, production_node, significant_tokens, text,
    tokens_of, unexpected,
};

/// Build the source-bearing S2 form for `synthesis` (FR-754).
pub(crate) fn synthesis(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    let identifiers = tokens_of(&declaration_items, TokenKind::Identifier);
    let [_, profile] = identifiers.as_slice() else {
        return Err(unexpected(node));
    };
    let names = nodes_of(&declaration_items, Production::QualifiedName);
    let [grammar, domain] = names.as_slice() else {
        return Err(unexpected(node));
    };
    let objective = match nodes_of(&declaration_items, Production::SynthesisObjective).as_slice() {
        [] => None,
        [objective] => Some(synthesis_objective(cst, objective)?),
        _ => return Err(unexpected(node)),
    };
    let satisfies = only(&declaration_items, Production::Block, node)?;
    let source = source(cst, node)?;
    Ok(DeclarationForm::Synthesis(Box::new(SynthesisForm {
        name: declared_name(&declaration_items, node)?,
        profile: UsingAlias {
            alias: text(profile, node)?,
            span: profile.span(),
        },
        grammar: name_form(cst, grammar)?,
        domain: name_form(cst, domain)?,
        objective,
        satisfies: block_expression(cst, satisfies)?,
        source,
        span: node.span(),
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

fn synthesis_objective(
    cst: &LosslessCst,
    node: &fsl_cst::CstNode,
) -> Result<SynthesisObjectiveForm, FormsRefusal> {
    let identifiers = tokens_of(&items(cst, node), TokenKind::Identifier);
    let value = identifiers
        .last()
        .copied()
        .ok_or_else(|| unexpected(node))?;
    let name = NameForm::new(text(value, node)?, value.span());
    let direction = significant_tokens(cst, node)
        .iter()
        .find(|token| matches!(token.spelling(), b"minimize" | b"maximize"))
        .map(|token| token.spelling())
        .ok_or_else(|| unexpected(node))?;
    match direction {
        b"minimize" => Ok(SynthesisObjectiveForm::Minimize(name)),
        b"maximize" => Ok(SynthesisObjectiveForm::Maximize(name)),
        _ => Err(unexpected(node)),
    }
}

fn block_expression(
    cst: &LosslessCst,
    block: &fsl_cst::CstNode,
) -> Result<ExpressionForm, FormsRefusal> {
    let block_items = items(cst, block);
    let expression_node = only(&block_items, Production::Expression, block)?;
    let (expression, spans) = crate::value::expression(cst, expression_node)?;
    Ok(ExpressionForm { expression, spans })
}

fn source(cst: &LosslessCst, node: &fsl_cst::CstNode) -> Result<String, FormsRefusal> {
    cst.source()
        .text()
        .get(node.span().start..node.span().end)
        .map(str::to_owned)
        .ok_or_else(|| unexpected(node))
}
