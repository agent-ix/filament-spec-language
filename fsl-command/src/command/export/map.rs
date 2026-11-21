// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-811 classification and FR-814 name mangling. An obligation whose
//! dependency closure holds a construct the target cannot state is
//! `unrepresented`, with one `target-unrepresentable` cause per such node. An
//! obligation the target can state is lowered by FR-812 and FR-813 when its
//! state and temporal constructs are in the supported subset; an unsupported
//! specified construct stops the export with a typed capability refusal.

use super::super::{Result, RunCause};
use super::lower;
use super::request::{EffectiveLimits, Request, Target};
use super::{Disposition, Mapped, Record, Unlowered};
use filament_contract_model::{CheckedNodeId, CheckedPackageV2};
use filament_exact::NodeKey;
use fsl_semantics::check::wire_id::WireGraph;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// One authored clause of the program an obligation name may resolve to.
pub(super) struct ClauseFact {
    pub(super) name: String,
    /// The clause's node when it is an obligation (an invariant or a
    /// temporal clause); `None` for a precondition, postcondition or
    /// constraint (FR-651 `refused`: not an obligation).
    pub(super) obligation: Option<NodeKey>,
    /// The `node_tag`/`semantic_form` of an obligation's clause node.
    pub(super) construct: &'static str,
    /// Whether a `bound` declaration bounds a population the clause ranges
    /// over (FSpec FR-812 "State and actions").
    pub(super) bounded: bool,
}

/// Resolve export obligations from the admitted V2 package's name rows.
/// No source program is read or compiled on this route.
pub(super) fn package_clauses(package: &CheckedPackageV2) -> Result<Vec<ClauseFact>> {
    let nodes = &package.resolved_graph().nodes;
    let wire = WireGraph::new(nodes.iter().map(|node| &node.node_id))
        .map_err(|_| internal("the package's node identities do not rebuild"))?;
    let mut clauses = Vec::new();
    // `None` means the admitted optional carrier was absent. Strict V2
    // admission rejects a claim-bearing node whose required name row is
    // missing, so it denotes a package with no declarations in that table.
    for row in package.claim_names().unwrap_or_default() {
        let node = nodes
            .iter()
            .find(|node| node.node_id == row.declaration)
            .ok_or_else(|| internal("a claim name has no package node"))?;
        let key = wire
            .key(&row.declaration)
            .map_err(|_| internal("a claim name has no structural key"))?
            .clone();
        let kind = package.node_kind(&row.declaration);
        let (obligation, construct) = match kind {
            Some(filament_contract_model::CheckedNodeKind::State(
                filament_contract_model::StateForm::StateClause,
            )) => {
                let body = serde_json::to_value(&node.body)
                    .map_err(|_| internal("a state clause does not serialize"))?;
                let invariant = body
                    .pointer("/operation/member/clause")
                    .and_then(Value::as_str)
                    == Some("invariant");
                (invariant.then_some(key), "state/state_clause")
            }
            Some(filament_contract_model::CheckedNodeKind::Claim(
                filament_contract_model::ClaimForm::Hyperproperty,
            )) => (Some(key), "temporal/temporal_clause"),
            _ => (None, "claim"),
        };
        clauses.push(ClauseFact {
            name: row.name.to_string(),
            obligation,
            construct,
            bounded: false,
        });
    }
    for row in package.temporal_clause_names().unwrap_or_default() {
        let key = wire
            .key(&row.declaration)
            .map_err(|_| internal("a temporal claim name has no structural key"))?
            .clone();
        clauses.push(ClauseFact {
            name: row.name.to_string(),
            obligation: Some(key),
            construct: "temporal/temporal_clause",
            bounded: false,
        });
    }
    Ok(clauses)
}

/// FSpec FR-814's injective mangling of one authored name under `prefix`:
/// `_` becomes `__`, `::` becomes `_N_`, any other scalar outside ASCII
/// letters and digits becomes `_uXX_` (uppercase hexadecimal).
pub(super) fn mangle(prefix: &str, name: &str) -> String {
    let mut out = String::from(prefix);
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '_' => out.push_str("__"),
            ':' if chars.peek() == Some(&':') => {
                chars.next();
                out.push_str("_N_");
            }
            c if c.is_ascii_alphanumeric() => out.push(c),
            c => out.push_str(&format!("_u{:X}_", u32::from(c))),
        }
    }
    out
}

/// The module name of a package identity.
pub(super) fn module_name(identity: &str) -> String {
    mangle("M_", identity)
}

fn internal(reason: &str) -> RunCause {
    RunCause::ExportMapper {
        construct: "checked package".to_owned(),
        reason: reason.to_owned(),
    }
}

/// The target node of a named checked-term binding on a protocol node.
fn binding_reference(
    graph: &Graph<'_>,
    node: &filament_contract_model::CheckedSemanticNodeV2,
    name: &str,
) -> Option<usize> {
    let body = serde_json::to_value(&node.body).ok()?;
    let target = body
        .get("arguments")?
        .as_array()?
        .iter()
        .find(|argument| {
            argument.get("term").and_then(Value::as_str) == Some("binding")
                && argument.get("name").and_then(Value::as_str) == Some(name)
        })?
        .get("value")?
        .get("target")?;
    let target: CheckedNodeId = serde_json::from_value(target.clone()).ok()?;
    graph.position(&target)
}

/// The selected model object type identity of a reference-typed parameter.
fn parameter_object_type(graph: &Graph<'_>, position: usize) -> Option<String> {
    let parameter = graph.node(position);
    let type_position = graph.position(&parameter.semantic_type)?;
    let body = serde_json::to_value(&graph.node(type_position).body).ok()?;
    body.get("members")?
        .as_array()?
        .first()?
        .get("target")?
        .get("path")?
        .as_array()?
        .get(2)?
        .as_str()
        .map(str::to_owned)
}

/// The type identity named by a protocol's `over` parameter.
fn protocol_over_type(
    graph: &Graph<'_>,
    protocol: &filament_contract_model::CheckedSemanticNodeV2,
) -> Option<String> {
    let over = binding_reference(graph, protocol, "over")?;
    parameter_object_type(graph, over)
}

/// Whether a protocol role is a static binding to an object type. Whole-model
/// roles carry a text literal and are auto-bound by FR-425 instead.
fn role_is_type_bound(node: &filament_contract_model::CheckedSemanticNodeV2) -> Result<bool> {
    let body = serde_json::to_value(&node.body)
        .map_err(|_| internal("a protocol role body does not serialize"))?;
    let value = body
        .get("arguments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|argument| {
            argument.get("term").and_then(Value::as_str) == Some("binding")
                && argument.get("name").and_then(Value::as_str) == Some("model")
        })
        .and_then(|argument| argument.get("value"))
        .ok_or_else(|| internal("a protocol role has no model binding"))?;
    match value.get("term").and_then(Value::as_str) {
        Some("reference") => Ok(true),
        Some("literal") if value.get("value_kind").and_then(Value::as_str) == Some("text") => {
            Ok(false)
        }
        _ => Err(internal(
            "a protocol role model binding has an unreadable shape",
        )),
    }
}

/// The export's FR-255 charges: one node and one work unit per node of the
/// dependency closures read.
struct Meter<'a> {
    limits: &'a EffectiveLimits,
    nodes: u64,
    work: u64,
}

impl Meter<'_> {
    /// Charge one node; the denied setting and its bound when a limit is hit.
    fn charge(&mut self) -> std::result::Result<(), (&'static str, u64)> {
        self.nodes = self.nodes.saturating_add(1);
        self.work = self.work.saturating_add(1);
        if self.nodes > self.limits.nodes {
            return Err(("export.nodes", self.limits.nodes));
        }
        if self.work > self.limits.work_units {
            return Err(("export.work_units", self.limits.work_units));
        }
        Ok(())
    }
}

/// The package as the classification reads it.
pub(super) struct Graph<'a> {
    pub(super) package: &'a CheckedPackageV2,
    by_key: BTreeMap<NodeKey, usize>,
    by_id: BTreeMap<String, usize>,
}

impl<'a> Graph<'a> {
    pub(super) fn new(package: &'a CheckedPackageV2) -> Result<Self> {
        let nodes = &package.resolved_graph().nodes;
        let wire = WireGraph::new(nodes.iter().map(|node| &node.node_id))
            .map_err(|_| internal("the package's node identities do not rebuild"))?;
        let mut by_key = BTreeMap::new();
        let mut by_id = BTreeMap::new();
        for (position, node) in nodes.iter().enumerate() {
            let key = wire
                .key(&node.node_id)
                .map_err(|_| internal("a package node has no structural key"))?;
            by_key.insert(key, position);
            let id = serde_json::to_value(&node.node_id)
                .and_then(|id| serde_json::to_string(&id))
                .map_err(|_| internal("a package node id does not serialize"))?;
            by_id.insert(id, position);
        }
        Ok(Self {
            package,
            by_key,
            by_id,
        })
    }

    pub(super) fn node(&self, position: usize) -> &filament_contract_model::CheckedSemanticNodeV2 {
        &self.package.resolved_graph().nodes[position]
    }

    /// The position of the node a serialized node id names.
    pub(super) fn position_of(&self, id: &Value) -> Option<usize> {
        serde_json::to_string(id)
            .ok()
            .and_then(|id| self.by_id.get(&id).copied())
    }

    pub(super) fn position(&self, id: &CheckedNodeId) -> Option<usize> {
        serde_json::to_value(id)
            .and_then(|id| serde_json::to_string(&id))
            .ok()
            .and_then(|id| self.by_id.get(&id).copied())
    }

    /// The nodes reachable from `root` through dependencies, root included.
    pub(super) fn closure(&self, root: usize) -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(position) = pending.pop() {
            if seen.insert(position) {
                let node = self.node(position);
                pending.extend(
                    node.dependencies
                        .iter()
                        .chain(std::iter::once(&node.semantic_type))
                        .filter_map(|id| self.position(id)),
                );
            }
        }
        seen
    }

    /// The flow-equation nodes (each mode's `rates`) of the hybrid modes in
    /// `closure`, once each, in node order.
    fn flows(&self, closure: &BTreeSet<usize>) -> Result<BTreeSet<usize>> {
        let mut flows = BTreeSet::new();
        for &position in closure {
            let node = self.node(position);
            if &*node.node_tag != "hybrid" || &*node.semantic_form != "hybrid_mode" {
                continue;
            }
            let body = serde_json::to_value(&node.body)
                .map_err(|_| internal("a hybrid mode does not serialize"))?;
            let rates = body["members"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|member| member["name"] == "rates")
                .and_then(|member| member["value"]["members"].as_array())
                .ok_or_else(|| internal("a hybrid mode has no rates"))?;
            for rate in rates {
                let id = serde_json::to_string(&rate["target"])
                    .map_err(|_| internal("a rate does not serialize"))?;
                flows.insert(
                    *self
                        .by_id
                        .get(&id)
                        .ok_or_else(|| internal("a rate names no package node"))?,
                );
            }
        }
        Ok(flows)
    }

    /// The node id and the FR-203 region of the node at `position`.
    fn locate(&self, position: usize) -> Result<(String, Value)> {
        let node = self.node(position);
        let id = fsl_replay::protocol_analysis::node_text(&node.node_id)
            .map_err(|_| internal("a package node id does not spell"))?;
        let locus = self
            .package
            .source_map()
            .iter()
            .filter(|entry| entry.node_id == node.node_id)
            .find_map(|entry| entry.regions.first())
            .and_then(|region| serde_json::to_value(region).ok())
            .map_or(Value::Null, |mut region| {
                region["kind"] = json!("region");
                region
            });
        Ok((id, locus))
    }

    /// The `node_tag/semantic_form` of the node at `position`.
    /// An obligation the mapper does not lower, at the node at `position`.
    fn unlowered(
        &self,
        obligation: &str,
        position: usize,
        construct: Option<&str>,
        detail: &str,
        fault: bool,
    ) -> Result<Unlowered> {
        let (node, locus) = self.locate(position)?;
        Ok(Unlowered {
            obligation: obligation.to_owned(),
            construct: construct.map_or_else(|| self.construct(position), str::to_owned),
            detail: detail.to_owned(),
            node,
            locus,
            fault,
        })
    }

    fn construct(&self, position: usize) -> String {
        let node = self.node(position);
        format!("{}/{}", node.node_tag, node.semantic_form)
    }

    /// The FSpec FR-811 cause for the unrepresentable node at `position`.
    fn cause(&self, target: Target, position: usize) -> Result<Value> {
        let (id, locus) = self.locate(position)?;
        Ok(json!({
            "kind": "target-unrepresentable",
            "target": target.family(),
            "construct": self.construct(position),
            "node": id,
            "locus": locus,
        }))
    }
}

/// Classify the requested obligations and, when none stops the export,
/// assemble the module of the obligations the target can state.
///
/// # Errors
///
/// A mapper failure when the package does not read as the mapper expects.
pub(super) fn run(
    package: &CheckedPackageV2,
    identity: &str,
    clauses: &[ClauseFact],
    models: &[Vec<u8>],
    initial: Option<&Value>,
    request: &Request,
    effective: &EffectiveLimits,
) -> Result<Mapped> {
    let graph = Graph::new(package)?;
    validate_protocol_bindings(&graph, clauses, initial, request)?;
    let mut meter = Meter {
        limits: effective,
        nodes: 0,
        work: 0,
    };
    let mut charged = BTreeSet::new();
    let mut records = Vec::with_capacity(request.obligations.len());
    let mut unlowered = None;
    let mut lowerable: Vec<(String, usize, usize)> = Vec::new();
    for name in &request.obligations {
        let Some((clause, key)) = clauses
            .iter()
            .find(|clause| &clause.name == name)
            .and_then(|clause| clause.obligation.as_ref().map(|key| (clause, key)))
        else {
            records.push(Record {
                obligation: name.clone(),
                disposition: Disposition::Refused,
                conditions: Vec::new(),
                causes: vec![json!({"kind": "not-an-obligation", "name": name})],
                regions: Vec::new(),
            });
            continue;
        };
        let root = *graph
            .by_key
            .get(key)
            .ok_or_else(|| internal("an obligation's clause is not a node of the package"))?;
        let closure = graph.closure(root);
        for &position in &closure {
            if charged.insert(position) {
                if let Err((setting, bound)) = meter.charge() {
                    return Ok(Mapped::Incomplete { setting, bound });
                }
            }
        }
        let flows = graph.flows(&closure)?;
        if flows.is_empty() {
            if matches!(
                clause.construct,
                "state/state_clause" | "temporal/temporal_clause"
            ) {
                if clause.bounded {
                    if unlowered.is_none() {
                        unlowered = Some(graph.unlowered(
                            name,
                            root,
                            Some("model/bound"),
                            "a `bound` declaration bounds the population, and this build does not lower population bounds",
                            false,
                        )?);
                    }
                } else {
                    lowerable.push((name.clone(), root, records.len()));
                }
                records.push(Record {
                    obligation: name.clone(),
                    disposition: Disposition::Preserved,
                    conditions: Vec::new(),
                    causes: Vec::new(),
                    regions: Vec::new(),
                });
            } else {
                if unlowered.is_none() {
                    unlowered =
                        Some(graph.unlowered(name, root, Some(clause.construct), "", false)?);
                }
            }
            continue;
        }
        records.push(Record {
            obligation: name.clone(),
            disposition: Disposition::Unrepresented,
            conditions: Vec::new(),
            causes: flows
                .into_iter()
                .map(|position| graph.cause(request.target, position))
                .collect::<Result<_>>()?,
            regions: Vec::new(),
        });
    }
    let module = module_name(identity);
    let mut lowered = None;
    if let Some((first, _, _)) = lowerable.first() {
        let wanted: Vec<(String, usize)> = lowerable
            .iter()
            .map(|(name, root, _)| (name.clone(), *root))
            .collect();
        match lower::lower(&graph, models, initial, request.target, &module, &wanted) {
            Ok(done) => {
                for (name, _, index) in &lowerable {
                    if let Some(record) = records.get_mut(*index) {
                        record.regions = done.regions.get(name).cloned().unwrap_or_default();
                        if let Some(condition) = done.conditions.get(name) {
                            record.disposition = Disposition::Conditional;
                            record.conditions.extend(condition.iter().cloned());
                        }
                    }
                }
                lowered = Some(done);
            }
            Err(refusal) => {
                if unlowered.is_none() {
                    let obligation = refusal.obligation.unwrap_or_else(|| first.clone());
                    let at = refusal.node.or_else(|| {
                        lowerable
                            .iter()
                            .find(|(name, _, _)| *name == obligation)
                            .map(|(_, root, _)| *root)
                    });
                    let fault = refusal.construct == "checked package";
                    unlowered = Some(graph.unlowered(
                        &obligation,
                        at.unwrap_or_default(),
                        refusal.node.is_none().then_some(refusal.construct.as_str()),
                        &refusal.detail,
                        fault,
                    )?);
                }
            }
        }
    }
    let (text, name_map, regions_written) = match lowered {
        Some(done) => (done.text, done.name_map, done.definitions as u64),
        None => (
            match request.target {
                Target::Tla => format!("---- MODULE {module} ----\n====\n"),
                Target::Lean => format!("namespace {module}\nend {module}\n"),
            },
            Vec::new(),
            0,
        ),
    };
    Ok(Mapped::Assembled {
        text,
        name_map,
        records,
        unlowered,
        work: meter.work,
        regions_written,
    })
}

/// Ensure a protocol-required temporal subject has the static role bindings
/// that the analyzer would require before the exporter classifies it.
fn validate_protocol_bindings(
    graph: &Graph<'_>,
    clauses: &[ClauseFact],
    initial: Option<&Value>,
    request: &Request,
) -> Result<()> {
    let selected: BTreeSet<_> = request.obligations.iter().map(String::as_str).collect();
    if !request.universes.is_empty() {
        return Err(RunCause::ExportRequest {
            path: "/request/export/universes".to_owned(),
            cause: "unsupported-selection",
            reason: "explicit universes are outside the lowered temporal protocol subset"
                .to_owned(),
        });
    }
    let mut required_protocols = BTreeSet::new();
    for clause in clauses
        .iter()
        .filter(|clause| selected.contains(clause.name.as_str()))
    {
        let Some(key) = &clause.obligation else {
            continue;
        };
        let Some(root) = graph.by_key.get(key).copied() else {
            continue;
        };
        if clause.construct != "temporal/temporal_clause" {
            continue;
        }
        let node = graph.node(root);
        let subject = fsl_package::checked_views::temporal_subject(graph.package, &node.node_id)
            .map_err(|_| internal("a selected temporal clause has no readable subject"))?;
        for protocol_id in subject.requirements() {
            let Some(protocol_position) = graph.position(protocol_id) else {
                return Err(internal("a temporal requirement names no package protocol"));
            };
            let protocol_node = graph.node(protocol_position);
            let Ok((_, protocol_name)) = fsl_package::protocol_view::declaration(protocol_node)
            else {
                return Err(internal(
                    "a temporal requirement has no protocol declaration",
                ));
            };
            if protocol_name.is_empty() {
                return Err(internal(
                    "a temporal requirement has an empty protocol name",
                ));
            }
            let protocol_name = protocol_name
                .iter()
                .map(|part| part.as_ref())
                .collect::<Vec<_>>()
                .join("::");
            required_protocols.insert(protocol_name.to_owned());
            let Some((binding_index, binding)) = request
                .protocols
                .iter()
                .enumerate()
                .find(|(_, binding)| binding.protocol == protocol_name)
            else {
                return Err(RunCause::ExportRequest {
                    path: "/request/export/protocols".to_owned(),
                    cause: "wrong-role-mapping",
                    reason: format!("the required protocol `{protocol_name}` has no binding"),
                });
            };
            if binding.over.is_none() {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/over"),
                    cause: "wrong-role-mapping",
                    reason: "the required protocol binding has no `over` selection".to_owned(),
                });
            }
            if !binding.capacities.is_empty() {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/capacities"),
                    cause: "unsupported-selection",
                    reason: "symbolic protocol capacities are outside the origin subset".to_owned(),
                });
            }
            let selected_models: Vec<_> = graph.package.selected_model_packages().collect();
            let [selected_model] = selected_models.as_slice() else {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/roles"),
                    cause: "unsupported-selection",
                    reason: "the origin subset requires exactly one selected model package"
                        .to_owned(),
                });
            };
            let Some(initial) = initial else {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/over"),
                    cause: "unsupported-selection",
                    reason: "the origin subset requires an initial snapshot to validate `over`"
                        .to_owned(),
                });
            };
            if initial.pointer("/model/identity").and_then(Value::as_str) != Some(selected_model) {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/over"),
                    cause: "wrong-role-mapping",
                    reason: "the initial snapshot does not belong to the selected model package"
                        .to_owned(),
                });
            }
            let Some(over) = binding.over.as_ref() else {
                return Err(internal(
                    "the protocol over selection disappeared after validation",
                ));
            };
            let population = initial
                .get("populations")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|row| {
                    row.get("population").and_then(Value::as_str)
                        == Some(over.reference.population.as_str())
                });
            if !population.is_some_and(|row| row.get("complete") == Some(&Value::Bool(true))) {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/over"),
                    cause: "wrong-role-mapping",
                    reason:
                        "the selected `over` population must be complete in the initial snapshot"
                            .to_owned(),
                });
            }
            let selected_object = population
                .and_then(|row| row.get("objects"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|object| {
                    object.get("key").and_then(Value::as_str) == Some(over.reference.key.as_str())
                });
            let protocol_over_type = protocol_over_type(graph, protocol_node);
            if selected_object.is_none_or(|object| {
                object.get("type").and_then(Value::as_str) != protocol_over_type.as_deref()
            }) {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/over"),
                    cause: "wrong-role-mapping",
                    reason: "the selected `over` population, key, and protocol input type must identify one object in the initial snapshot"
                        .to_owned(),
                });
            }
            let view =
                fsl_package::protocol_view::protocol_view(graph.package, &protocol_node.node_id)
                    .map_err(|_| internal("a required protocol has no readable view"))?;
            let mut required_roles = BTreeSet::new();
            for role_id in view.roles() {
                let Some(role_position) = graph.position(role_id) else {
                    return Err(internal("a protocol role names no package node"));
                };
                let role_node = graph.node(role_position);
                let Ok((_, role_name)) = fsl_package::protocol_view::declaration(role_node) else {
                    return Err(internal("a protocol role has no declaration"));
                };
                let Some(role_name) = role_name.last() else {
                    return Err(internal("a protocol role has an empty declaration name"));
                };
                let role_name = role_name.as_ref();
                let role_path = super::request::pointer_token(role_name);
                if fsl_package::protocol_view::role_is_replicated(role_node)
                    .map_err(|_| internal("a protocol role has no readable replication mode"))?
                {
                    return Err(RunCause::ExportRequest {
                        path: format!(
                            "/request/export/protocols/{binding_index}/roles/{role_path}"
                        ),
                        cause: "unsupported-selection",
                        reason: "replicated protocol roles are outside the lowered subset"
                            .to_owned(),
                    });
                }
                let type_bound = role_is_type_bound(role_node)?;
                if type_bound {
                    required_roles.insert(role_name.to_owned());
                }
                let role_binding_is_valid = if type_bound {
                    binding.roles.get(role_name).is_some_and(|role| {
                        role.kind == "model-binding" && role.selection.as_str() == *selected_model
                    })
                } else {
                    !binding.roles.contains_key(role_name)
                };
                if !role_binding_is_valid {
                    return Err(RunCause::ExportRequest {
                        path: format!(
                            "/request/export/protocols/{binding_index}/roles/{role_path}"
                        ),
                        cause: "wrong-role-mapping",
                        reason: format!(
                            "the required static role `{role_name}` has no model binding"
                        ),
                    });
                }
            }
            if binding
                .roles
                .keys()
                .any(|role| !required_roles.contains(role))
            {
                return Err(RunCause::ExportRequest {
                    path: format!("/request/export/protocols/{binding_index}/roles"),
                    cause: "wrong-role-mapping",
                    reason:
                        "the protocol binding names a role that is not declared by the protocol"
                            .to_owned(),
                });
            }
        }
    }
    if request
        .protocols
        .iter()
        .any(|binding| !required_protocols.contains(&binding.protocol))
    {
        return Err(RunCause::ExportRequest {
            path: "/request/export/protocols".to_owned(),
            cause: "wrong-role-mapping",
            reason:
                "the request binds a protocol that is not required by a selected temporal clause"
                    .to_owned(),
        });
    }
    Ok(())
}
