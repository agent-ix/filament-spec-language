// SPDX-License-Identifier: AGPL-3.0-or-later
//! Relation bindings have grammatical separators and leave name uniqueness to S3.

use fsl_cst::Production;
use ix_trace_rs::trace;

use super::claim_syntax;

const VALID: &str = "relation SameResult using v over (left: M::Job::step, right: M::Job::step) { left.output = right.output }";

fn admitted(fragment: &str) {
    let text = claim_syntax::unit(fragment);
    let parsed = claim_syntax::admitted(&text);
    assert_eq!(
        claim_syntax::node_texts(&text, &parsed, Production::RelationClause),
        [fragment]
    );
    let bindings = claim_syntax::node_texts(&text, &parsed, Production::ExecutionBinding);
    assert_eq!(bindings, ["left: M::Job::step", "right: M::Job::step"]);
}

#[trace("FR-002-AC-6")]
#[test]
fn relation_requires_over_before_its_execution_bindings() {
    admitted(VALID);
    claim_syntax::refused(
        "relation SameResult using v (left: M::Job::step, right: M::Job::step) { left.output = right.output }",
        "(left:", "(", "expected `over`",
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn execution_binding_requires_a_target_after_its_colon() {
    admitted(VALID);
    claim_syntax::refused(
        "relation SameResult using v over (left: , right: M::Job::step) { left.output = right.output }",
        ", right", ",", "expected identifier",
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn execution_binding_requires_the_colon_before_its_qualified_target() {
    admitted(VALID);
    claim_syntax::refused(
        "relation SameResult using v over (left M::Job::step, right: M::Job::step) { left.output = right.output }",
        "M::Job", "M", "expected `:`",
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn relation_requires_a_comma_between_its_first_two_bindings() {
    admitted(VALID);
    claim_syntax::refused(
        "relation SameResult using v over (left: M::Job::step right: M::Job::step) { left.output = right.output }",
        "right:", "right", "expected `,`",
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn relation_requires_its_block_closer_rather_than_a_declaration_semicolon() {
    admitted(VALID);
    claim_syntax::refused(
        "relation SameResult using v over (left: M::Job::step, right: M::Job::step) { left.output = right.output",
        "", "", "expected `}`",
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn repeated_execution_names_remain_syntax_before_semantic_uniqueness_checking() {
    let fragment = "relation Alias using v over (left: M::Job::alpha, left: M::Job::beta) { true }";
    let text = claim_syntax::unit(fragment);
    let parsed = claim_syntax::admitted(&text);
    assert_eq!(
        claim_syntax::node_texts(&text, &parsed, Production::ExecutionBinding),
        ["left: M::Job::alpha", "left: M::Job::beta"]
    );
}
