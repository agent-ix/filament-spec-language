// SPDX-License-Identifier: AGPL-3.0-or-later
//! The binders an expression introduces itself (FR-390 "Selection and supply
//! admission", rule 3).
//!
//! A `let`, a collection query (`map`, `filter`, `forall`, `exists`, `count`,
//! `sum`) and a `case` arm each open a scope. Within that scope the binder they
//! name hides every other binder of the same name, so a `value`/`parameter`
//! node reached under it is the introduced binder and is not a source of the
//! slot. This module walks the admitted terms with that scoping, and the slot's
//! source discovery and the reader's binder choice both use the one walk.

use std::collections::BTreeSet;
use std::rc::Rc;

use filament_contract_model::{CheckedNodeId, CheckedSemanticNodeV2, CheckedTerm};

/// The binders in force at a point of a walk: the names `let` and collection
/// queries bound, and the nodes `case` arms bound.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Shadow {
    names: BTreeSet<String>,
    nodes: BTreeSet<CheckedNodeId>,
}

impl Shadow {
    fn with_name(&self, name: &str) -> Self {
        let mut next = self.clone();
        next.names.insert(name.to_owned());
        next
    }

    fn with_nodes(&self, nodes: impl IntoIterator<Item = CheckedNodeId>) -> Self {
        let mut next = self.clone();
        next.nodes.extend(nodes);
        next
    }

    fn hides(&self, node: &CheckedSemanticNodeV2) -> bool {
        self.nodes.contains(&node.node_id)
            || crate::reader_protocol_projection::parameter_name(&node.body)
                .is_some_and(|name| self.names.contains(name))
    }
}

/// The scopes an application opens.
enum Opening<'t> {
    /// `let`: the bound value is outside the scope, the body inside.
    Let {
        name: &'t str,
        binder: Option<&'t CheckedNodeId>,
        value: &'t CheckedTerm,
        body: &'t CheckedTerm,
    },
    /// A collection query: the source is outside the scope, the body inside.
    Query {
        source: &'t CheckedTerm,
        name: &'t str,
        body: &'t CheckedTerm,
    },
    /// `seq`: the two bounds are outside the scope, the body inside.
    Seq {
        bounds: [&'t CheckedTerm; 2],
        name: &'t str,
        body: &'t CheckedTerm,
    },
    /// `case`: the scrutinee is outside, each arm's binders and body inside.
    Case(&'t [CheckedTerm]),
    /// No scope.
    None,
}

/// Whether `name` is the checker's reserved name of the operand binders an
/// IEEE ordering lowers to.
///
/// `#[string_edge]`: binder names are the admitted wire's fixed vocabulary.
#[ix_trace_rs::string_edge]
pub(crate) fn is_ieee_operand(name: &str) -> bool {
    name == "#ieee_operand"
}

/// `#[string_edge]`: operation identities are the admitted wire's fixed
/// operation vocabulary.
#[ix_trace_rs::string_edge]
fn opening<'t>(identity: Option<&str>, arguments: &'t [CheckedTerm]) -> Opening<'t> {
    match (identity, arguments) {
        (
            Some("filament.op.control.let"),
            [CheckedTerm::Binding {
                name,
                binder,
                value,
            }, body],
        ) => Opening::Let {
            name: name.as_ref(),
            binder: binder.as_ref(),
            value,
            body,
        },
        (
            Some(
                "filament.op.collection.map"
                | "filament.op.collection.filter"
                | "filament.op.collection.forall"
                | "filament.op.collection.exists"
                | "filament.op.collection.count",
            ),
            [source, CheckedTerm::Binding { name, value, .. }],
        ) => Opening::Query {
            source,
            name: name.as_ref(),
            body: value,
        },
        (Some(identity), [source, CheckedTerm::Binding { name, value, .. }])
            if is_sum_identity(identity) =>
        {
            Opening::Query {
                source,
                name: name.as_ref(),
                body: value,
            }
        }
        (
            Some("filament.op.collection.seq"),
            [low, high, CheckedTerm::Binding { name, value, .. }],
        ) => Opening::Seq {
            bounds: [low, high],
            name: name.as_ref(),
            body: value,
        },
        (Some("filament.op.control.case"), [_, ..]) => Opening::Case(arguments),
        _ => Opening::None,
    }
}

/// Whether `identity` is one of the seven `filament.op.collection.sum.*`
/// identities, one per numeric family a `sum<N>` totals (FR-145), the carried
/// quantity included (FR-322 "Carried-quantity sum").
pub(crate) fn is_sum_identity(identity: &str) -> bool {
    matches!(
        identity,
        "filament.op.collection.sum.integer"
            | "filament.op.collection.sum.rational"
            | "filament.op.collection.sum.decimal"
            | "filament.op.collection.sum.float32"
            | "filament.op.collection.sum.float64"
            | "filament.op.collection.sum.quantity"
            | "filament.op.collection.sum.carried_quantity"
    )
}

/// What a walk reports: each parameter node reached outside every scope that
/// hides it.
pub(crate) type OnParameter<'a, 'w> = dyn FnMut(&'w CheckedSemanticNodeV2) + 'a;

/// The lookup of a node by id.
pub(crate) type Lookup<'a, 'w> = dyn Fn(&CheckedNodeId) -> Option<&'w CheckedSemanticNodeV2> + 'a;

struct Walk<'a, 'w> {
    /// The one binder name the walk looks for; a scope that binds any other
    /// name hides nothing the walk reports, so it opens no scope.
    watch: Option<&'a str>,
    lookup: &'a Lookup<'a, 'w>,
    on_parameter: &'a mut OnParameter<'a, 'w>,
    seen: BTreeSet<(CheckedNodeId, Shadow)>,
}

/// Walks `start` and every `expression`/`value` node it reaches, reporting
/// each parameter node that no scope of the walk hides. The walk stops at a
/// parameter and at any node that is not an expression or a value (a
/// function, a type), as the dependency closure of FR-390 rule 3 does. With
/// `watch`, only the parameters of that name are of interest, and the walk
/// opens only the scopes that bind it.
pub(crate) fn reach_term<'a, 'w>(
    lookup: &'a Lookup<'a, 'w>,
    start: &CheckedTerm,
    watch: Option<&'a str>,
    on_parameter: &'a mut OnParameter<'a, 'w>,
) {
    let mut walk = Walk {
        watch,
        lookup,
        on_parameter,
        seen: BTreeSet::new(),
    };
    // Every node a term references waits here with the scopes it is reached
    // in; no walk recurses in proportion to the nesting of its input
    // (ADR-030 D-1), so the stack is one entry per reference, all bounded by
    // the admitted graph's I2 edge ceiling.
    let mut pending = Vec::new();
    walk.term(start, &Rc::default(), &mut BTreeSet::new(), &mut pending);
    while let Some((id, shadow)) = pending.pop() {
        walk.node(&id, &shadow, &mut pending);
    }
}

/// A term still to be read, and the scopes it is read in.
enum Reading<'t> {
    /// Any term.
    Term(&'t CheckedTerm),
    /// A `case` arm: `Binding{member, Aggregate[Aggregate[binders], body]}`.
    Arm(&'t CheckedTerm),
}

impl<'w> Walk<'_, 'w> {
    /// Read `start` and every term nested in it. `covered` collects every
    /// node the term references, so a dependency the term does not reference
    /// can be told from one it does; each referenced node is pushed on
    /// `pending` under the scopes it is reached in.
    fn term(
        &mut self,
        start: &CheckedTerm,
        shadow: &Rc<Shadow>,
        covered: &mut BTreeSet<CheckedNodeId>,
        pending: &mut Vec<(CheckedNodeId, Rc<Shadow>)>,
    ) {
        let mut reading = vec![(Reading::Term(start), Rc::clone(shadow))];
        while let Some((next, shadow)) = reading.pop() {
            match next {
                Reading::Term(CheckedTerm::Reference { target }) => {
                    covered.insert(target.clone());
                    pending.push((target.clone(), shadow));
                }
                Reading::Term(CheckedTerm::Application {
                    operation,
                    arguments,
                    ..
                }) => match opening(operation.identity(), arguments) {
                    Opening::Let {
                        name,
                        binder,
                        value,
                        body,
                    } => {
                        if let Some(binder) = binder {
                            // The binder link declares a parameter; it does
                            // not read that parameter in the outer scope.
                            covered.insert(binder.clone());
                        }
                        let inner = self.scoped(&shadow, name);
                        reading.push((Reading::Term(body), inner));
                        reading.push((Reading::Term(value), shadow));
                    }
                    Opening::Query { source, name, body } => {
                        let inner = self.scoped(&shadow, name);
                        reading.push((Reading::Term(body), inner));
                        reading.push((Reading::Term(source), shadow));
                    }
                    Opening::Seq { bounds, name, body } => {
                        let inner = self.scoped(&shadow, name);
                        reading.push((Reading::Term(body), inner));
                        reading.extend(
                            bounds
                                .into_iter()
                                .rev()
                                .map(|bound| (Reading::Term(bound), Rc::clone(&shadow))),
                        );
                    }
                    Opening::Case(arms) => {
                        if let Some((scrutinee, arms)) = arms.split_first() {
                            reading.extend(
                                arms.iter()
                                    .rev()
                                    .map(|arm| (Reading::Arm(arm), Rc::clone(&shadow))),
                            );
                            reading.push((Reading::Term(scrutinee), shadow));
                        }
                    }
                    Opening::None => reading.extend(
                        arguments
                            .iter()
                            .rev()
                            .map(|argument| (Reading::Term(argument), Rc::clone(&shadow))),
                    ),
                },
                Reading::Term(CheckedTerm::Aggregate { members, .. }) => reading.extend(
                    members
                        .iter()
                        .rev()
                        .map(|member| (Reading::Term(member), Rc::clone(&shadow))),
                ),
                Reading::Term(CheckedTerm::Binding { value, .. }) => {
                    reading.push((Reading::Term(value), shadow));
                }
                Reading::Term(_) => {}
                Reading::Arm(arm) => {
                    let CheckedTerm::Binding { value, .. } = arm else {
                        reading.push((Reading::Term(arm), shadow));
                        continue;
                    };
                    let CheckedTerm::Aggregate { members, .. } = value.as_ref() else {
                        reading.push((Reading::Term(value), shadow));
                        continue;
                    };
                    let [CheckedTerm::Aggregate {
                        members: binders, ..
                    }, body] = members.as_slice()
                    else {
                        reading.push((Reading::Term(value), shadow));
                        continue;
                    };
                    let mut introduced = Vec::with_capacity(binders.len());
                    for binder in binders {
                        if let CheckedTerm::Reference { target } = binder {
                            covered.insert(target.clone());
                            introduced.push(target.clone());
                        }
                    }
                    reading.push((Reading::Term(body), Rc::new(shadow.with_nodes(introduced))));
                }
            }
        }
    }

    /// `shadow` with the scope of a binder named `name` opened.
    fn scoped(&self, shadow: &Rc<Shadow>, name: &str) -> Rc<Shadow> {
        // The checker nests IEEE operand binders under one reserved name and
        // tells them apart by level, so when the walk looks for them no scope
        // of that name hides another.
        if is_ieee_operand(name) && self.watch.is_some_and(is_ieee_operand) {
            return Rc::clone(shadow);
        }
        match self.watch {
            Some(watched) if watched != name => Rc::clone(shadow),
            _ => Rc::new(shadow.with_name(name)),
        }
    }

    fn node(
        &mut self,
        id: &CheckedNodeId,
        shadow: &Rc<Shadow>,
        pending: &mut Vec<(CheckedNodeId, Rc<Shadow>)>,
    ) {
        let Some(node) = (self.lookup)(id) else {
            return;
        };
        match (&*node.node_tag, &*node.semantic_form) {
            ("value", "parameter") => {
                if !shadow.hides(node) {
                    (self.on_parameter)(node);
                }
            }
            ("expression", _) | ("value", _) => {
                if !self.seen.insert((id.clone(), (**shadow).clone())) {
                    return;
                }
                let mut covered = BTreeSet::new();
                self.term(&node.body, shadow, &mut covered, pending);
                // A dependency the body does not reference is still part of
                // the closure, under the scopes the node is reached in.
                for dependency in &node.dependencies {
                    if !covered.contains(dependency) {
                        pending.push((dependency.clone(), Rc::clone(shadow)));
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::*;

    fn id(name: &str) -> Value {
        json!({"ordinal": 0, "package": "p", "path": [name]})
    }

    fn node(name: &str, tag: &str, form: &str, dependencies: &[&str], body: Value) -> Value {
        json!({
            "node_id": id(name),
            "schema_version": "filament.checked-semantic-graph/v2",
            "node_tag": tag,
            "semantic_form": form,
            "semantic_type": id("type"),
            "dependencies": dependencies.iter().map(|d| id(d)).collect::<Vec<_>>(),
            "occurrences": [],
            "body": body,
        })
    }

    fn parameter(name: &str, text: &str) -> Value {
        let literal = |kind: &str, value: Value| json!({"term": "literal", "type": id("type"), "value": value, "value_kind": kind});
        node(
            name,
            "value",
            "parameter",
            &[],
            json!({"term": "aggregate", "members": [
                {"term": "binding", "name": "name", "value": literal("text", json!(text))},
                {"term": "binding", "name": "level", "value": literal("integer", json!("0"))},
            ]}),
        )
    }

    fn reference(name: &str) -> Value {
        json!({"term": "reference", "target": id(name)})
    }

    /// The parameter nodes a walk from `root` reaches outside every scope.
    fn sources(nodes: Vec<Value>, root: &str) -> Vec<String> {
        let nodes: BTreeMap<CheckedNodeId, CheckedSemanticNodeV2> = nodes
            .into_iter()
            .map(|value| {
                let node: CheckedSemanticNodeV2 = serde_json::from_value(value).expect("a node");
                (node.node_id.clone(), node)
            })
            .collect();
        let lookup = |id: &CheckedNodeId| nodes.get(id);
        let mut found = Vec::new();
        let start: CheckedTerm = serde_json::from_value(reference(root)).expect("a term");
        reach_term(&lookup, &start, None, &mut |node| {
            found.push(
                crate::reader_protocol_projection::parameter_name(&node.body)
                    .unwrap_or_default()
                    .to_owned(),
            );
        });
        found.sort();
        found.dedup();
        found
    }

    /// FR-390 rule 3 and FSpec FR-322 `seq`: the binder of `seq` scopes its body
    /// only. The binder `i` is no source, and the bounds and the body's read
    /// of `outer` stay sources.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn a_seq_binder_is_not_a_source() {
        let seq = node(
            "seq",
            "expression",
            "collection",
            &["lo", "hi", "body"],
            json!({
                "term": "application",
                "operator": "collection",
                "operation": {"identity": "filament.op.collection.seq", "laws": [], "leaves": [], "member": null, "mode": null},
                "arguments": [
                    reference("lo"),
                    reference("hi"),
                    {"term": "binding", "name": "i", "value": reference("body")},
                ],
                "result_type": id("type"),
            }),
        );
        let body = node(
            "body",
            "expression",
            "call",
            &["i", "outer"],
            json!({"term": "aggregate", "members": [reference("i"), reference("outer")]}),
        );
        let found = sources(
            vec![
                seq,
                body,
                parameter("lo", "lo"),
                parameter("hi", "hi"),
                parameter("i", "i"),
                parameter("outer", "outer"),
            ],
            "seq",
        );
        assert_eq!(found, vec!["hi", "lo", "outer"]);
    }

    /// FR-390 rule 3: the binders of a `case` arm are not sources. The package
    /// reader admits no arm binder in a protocol slot today, so this walks the
    /// term directly. The scrutinee `s` and the body's read of `outer` stay
    /// sources.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn a_case_arm_binder_is_not_a_source() {
        let arm = json!({"term": "binding", "name": "Circle", "value": {
            "term": "aggregate",
            "members": [
                {"term": "aggregate", "members": [reference("r")]},
                {"term": "aggregate", "members": [reference("r"), reference("outer")]},
            ],
        }});
        let case = node(
            "case",
            "expression",
            "case",
            &["s", "r", "outer"],
            json!({
                "term": "application",
                "operator": "case",
                "operation": {"identity": "filament.op.control.case", "laws": [], "leaves": [], "member": null, "mode": null},
                "arguments": [reference("s"), arm],
                "result_type": id("type"),
            }),
        );
        let found = sources(
            vec![
                case,
                parameter("s", "s"),
                parameter("r", "r"),
                parameter("outer", "outer"),
            ],
            "case",
        );
        assert_eq!(found, vec!["outer", "s"]);
    }
    fn operand(name: &str, level: u64) -> Value {
        let mut node = parameter(name, "#ieee_operand");
        node["body"]["members"][1]["value"]["value"] = json!(level.to_string());
        node
    }

    fn let_of(name: &str, binder: &str, value: Value, body: Value) -> Value {
        json!({
            "term": "application",
            "operator": "let",
            "operation": {"identity": "filament.op.control.let", "laws": [], "leaves": [], "member": null, "mode": null},
            "arguments": [{"term": "binding", "name": name, "binder": id(binder), "value": value}, body],
            "result_type": id("type"),
        })
    }

    /// The nodes of an IEEE ordering `-x < -y` over captures `x` and `y`: two
    /// nested `#ieee_operand` lets, one explicit binder link per level, read by
    /// `lt`. Their dependency edges are metadata, not free reads in the outer
    /// scope.
    fn ordering_nodes(root_form: &str) -> Vec<Value> {
        let inner = let_of("#ieee_operand", "operand1", reference("y"), reference("lt"));
        let outer = let_of("#ieee_operand", "operand0", reference("x"), inner);
        vec![
            node(
                "ordering",
                "expression",
                root_form,
                &["x", "y", "lt", "operand0", "operand1"],
                outer,
            ),
            node(
                "lt",
                "expression",
                "call",
                &["operand0", "operand1"],
                json!({
                    "term": "aggregate",
                    "members": [reference("operand0"), reference("operand1")],
                }),
            ),
            parameter("x", "x"),
            parameter("y", "y"),
            operand("operand0", 0),
            operand("operand1", 1),
        ]
    }

    /// A walk with no watched name, which a protocol slot's sources and a
    /// temporal clause's binders make, reports the captures `x` and `y` and
    /// neither of the two bound IEEE operand binders.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn two_bound_ieee_operands_are_no_slot_source() {
        let found = sources(ordering_nodes("let"), "ordering");
        assert_eq!(found, vec!["x", "y"]);
    }

    /// A temporal clause term's free binders, read the same way, hold the
    /// captures only when its formula holds an IEEE ordering.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn two_bound_ieee_operands_are_no_temporal_clause_binder() {
        let mut nodes = ordering_nodes("application");
        let clause = node(
            "clause",
            "expression",
            "call",
            &["ordering"],
            json!({"term": "aggregate", "members": [reference("ordering")]}),
        );
        nodes.push(clause);
        assert_eq!(sources(nodes, "clause"), vec!["x", "y"]);
    }

    /// The reader's binder lookup watches `#ieee_operand` and reaches both
    /// operands of the nested lets by level.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn a_watched_ieee_operand_name_reaches_every_level() {
        let nodes: BTreeMap<CheckedNodeId, CheckedSemanticNodeV2> = ordering_nodes("let")
            .into_iter()
            .map(|value| {
                let node: CheckedSemanticNodeV2 = serde_json::from_value(value).expect("a node");
                (node.node_id.clone(), node)
            })
            .collect();
        let lookup = |id: &CheckedNodeId| nodes.get(id);
        let start: CheckedTerm = serde_json::from_value(reference("ordering")).expect("a term");
        let mut found = Vec::new();
        reach_term(&lookup, &start, Some("#ieee_operand"), &mut |node| {
            if crate::reader_protocol_projection::parameter_name(&node.body)
                == Some("#ieee_operand")
            {
                found.push(node.node_id.clone());
            }
        });
        assert_eq!(
            found.len(),
            2,
            "both operand binders are reached: {found:?}"
        );
    }
}
