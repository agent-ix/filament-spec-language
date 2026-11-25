// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Application result and node type positions": the member
//! operands an application's derived path holds after its operation identity.

use super::*;
use crate::value::member::TypeArgumentAlias;
use filament_exact::PathSegment;

fn key(name: &str) -> NodeKey {
    NodeKey::new("pkg", vec![PathSegment::Text(name.to_owned())], 0).expect("a key")
}

fn node(name: &str) -> PathSegment {
    PathSegment::Node(key(name))
}

fn text(text: &str) -> PathSegment {
    PathSegment::Text(text.to_owned())
}

fn named(name: &str) -> Identifier {
    Identifier::new(name.to_owned()).expect("an identifier")
}

/// The path operands of a `query` expression of type `result` that applies
/// `filament.op.test` with `member` to `receiver`.
fn operands(member: Member) -> Vec<PathSegment> {
    let mut operation = Operation::plain("filament.op.test");
    operation.member = Some(member);
    derived_operands(&NodeContent {
        node_tag: NodeTag::Expression,
        semantic_form: "query",
        semantic_type: Some(NodeRef::Admitted(key("result"))),
        declaration: None,
        owner: None,
        owner_model: None,
        body: BodyTerm::application(
            Operator::Query,
            operation,
            key("result"),
            vec![MemberTerm::Leaf(LeafTerm::reference_node(
                key("receiver").into(),
            ))],
        ),
    })
}

/// The whole path operands around the member's own `member` operands.
fn around(member: Vec<PathSegment>) -> Vec<PathSegment> {
    let mut expected = vec![node("receiver"), text("filament.op.test")];
    expected.extend(member);
    expected.extend([text("query"), node("result"), node("result")]);
    expected
}

#[test]
fn a_history_member_is_its_declaration_and_name() {
    assert_eq!(
        operands(Member::History {
            declaration: key("counter"),
            name: named("seen"),
        }),
        around(vec![node("counter"), text("seen")])
    );
}

/// The destination type is no operand, with or without a population.
#[test]
fn a_relationship_end_member_is_its_declaration_role_and_population() {
    let end = |population: Option<NodeKey>| Member::RelationshipEnd {
        declaration: key("relationship"),
        name: named("owner"),
        destination_type: key("destination"),
        population,
    };
    assert_eq!(
        operands(end(None)),
        around(vec![node("relationship"), text("owner")])
    );
    assert_eq!(
        operands(end(Some(key("population")))),
        around(vec![
            node("relationship"),
            text("owner"),
            node("population")
        ])
    );
}

#[test]
fn a_type_argument_member_is_its_declaration_then_any_alias() {
    let argument = |alias: Option<TypeArgumentAlias>| Member::TypeArgument {
        declaration: key("range"),
        alias,
    };
    assert_eq!(operands(argument(None)), around(vec![node("range")]));
    assert_eq!(
        operands(argument(Some(TypeArgumentAlias {
            package: "pkg".to_owned(),
            node: "ix://pkg/Small".to_owned(),
        }))),
        around(vec![node("range"), text("ix://pkg/Small")])
    );
}

#[test]
fn an_operation_member_is_its_declaration_and_name() {
    assert_eq!(
        operands(Member::Operation {
            declaration: key("counter"),
            name: named("inc"),
        }),
        around(vec![node("counter"), text("inc")])
    );
}

#[test]
fn position_and_element_members_hold_their_declaration() {
    assert_eq!(
        operands(Member::Position {
            declaration: key("pair"),
            position: 1,
        }),
        around(vec![node("pair")])
    );
    assert_eq!(
        operands(Member::Element {
            declaration: key("sequence"),
        }),
        around(vec![node("sequence")])
    );
}

#[test]
fn a_profile_operator_member_is_its_operator_name() {
    assert_eq!(
        operands(Member::ProfileOperator {
            operator: named("transition"),
        }),
        around(vec![text("transition")])
    );
}
