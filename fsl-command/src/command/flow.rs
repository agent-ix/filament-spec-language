// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-821: lower `quire.flow-report/v1` facts into a generated native model,
//! compile generated source through `native-analyze/1`, and return its normal
//! `filament-outcome/1` result.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use filament_exact::Cancel;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{analyze, RunResult};
use fsl_analyze::analyze::document::Negotiate;
#[cfg(test)]
use fsl_route::CandidateOutcome;

const PACKAGE: &str = "filament/flow-report";
const NATIVE_STRING: &str = "ix://filament/native/String";
const GENERATOR: &str = "ix://filament-spec-language/FR-821";
const MAX_GENERATED_ITEMS: usize = 4096;
static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

/// A malformed report or a failure while preparing the generated native request.
#[derive(Debug, thiserror::Error)]
pub enum FlowAnalyzeError {
    /// The generated temporary workspace could not be created or written.
    #[error("cannot prepare generated flow analysis: {0}")]
    Io(#[from] std::io::Error),
    /// The generated native request failed before producing its outcome.
    #[error("native flow analysis failed: {0}")]
    Native(Box<super::RunError>),
}

fn invalid_report(reason: impl Into<String>) -> FlowAnalyzeError {
    FlowAnalyzeError::Native(Box::new(super::RunError {
        cause: super::RunCause::AnalyzeRequest {
            member: "flow-report".to_owned(),
            reason: reason.into(),
        },
        sources: crate::Sources::default(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema: String,
    rules: Vec<Rule>,
    vocabularies: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    facts: BTreeMap<String, Vec<Fact>>,
    // The denial list is deliberately discarded without deserializing its shape.
    #[serde(rename = "denials")]
    _denials: Vec<serde::de::IgnoredAny>,
    #[serde(rename = "remediations")]
    _remediations: Vec<Value>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    name: String,
    subject: Vec<String>,
    #[serde(rename = "match")]
    matches: Vec<Condition>,
    unless_any: Vec<Condition>,
    check: String,
    #[serde(rename = "remediations")]
    _remediations: Vec<String>,
    _rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Condition {
    #[serde(default)]
    path: Option<Vec<Step>>,
    #[serde(default)]
    any_path: Option<Vec<Vec<Step>>>,
    field: String,
    #[serde(default)]
    any_of: Option<Vec<String>>,
    #[serde(default)]
    absent: Option<bool>,
    #[serde(default)]
    present: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    edge: String,
    direction: String,
    to: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    id: String,
    #[serde(default)]
    repo: Option<String>,
    path: String,
    fields: BTreeMap<String, Value>,
    edges: Vec<FactEdge>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactEdge {
    edge: String,
    direction: String,
    target: FactRef,
    archetype: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactRef {
    id: String,
    #[serde(default)]
    repo: Option<String>,
}

/// Analyze a Quire flow report through the source-driven native analyze path.
///
/// The generated semantic model contains the report's facts and resolved
/// edges. Claims are generated from rules, vocabularies, and facts only; the
/// report's `denials` member is ignored. Unsupported/malformed syntax refuses
/// the generated request as a whole, while native per-item limits and outcomes
/// are preserved by the existing analyze command.
pub fn analyze_flow_report(
    report_bytes: &[u8],
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<RunResult, FlowAnalyzeError> {
    let report: Report =
        serde_json::from_slice(report_bytes).map_err(|error| invalid_report(error.to_string()))?;
    if report.schema != "quire.flow-report/v1" {
        return Err(invalid_report(format!(
            "schema must be quire.flow-report/v1, got {:?}",
            report.schema
        )));
    }
    validate_report(&report)?;
    let generated = Generated::new(&report)?;
    run_generated(&generated, negotiate, cancel)
}

fn run_generated(
    generated: &Generated,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<RunResult, FlowAnalyzeError> {
    let workspace = Workspace::new()?;
    workspace.write("flow.fsl", generated.source.as_bytes())?;
    workspace.write("model.json", generated.model.as_bytes())?;
    workspace.write("snapshot.json", generated.snapshot.as_bytes())?;
    workspace.write("analyze.json", generated.request.as_bytes())?;
    let mut result = analyze(&workspace.path.join("analyze.json"), negotiate, cancel)
        .map_err(FlowAnalyzeError::Native)?;
    if !generated.incomplete_items.is_empty() {
        result
            .value
            .mark_flow_items_incomplete(&generated.incomplete_items);
        result.exit_code = 22;
    }
    Ok(result)
}

fn validate_report(report: &Report) -> Result<(), FlowAnalyzeError> {
    let mut names = BTreeSet::new();
    let mut expected_archetypes = BTreeSet::new();
    for rule in &report.rules {
        if rule.name.trim().is_empty() || !names.insert(rule.name.clone()) {
            return Err(invalid_report(format!(
                "rule name {:?} is blank or duplicated",
                rule.name
            )));
        }
        if rule.subject.is_empty() || rule.matches.is_empty() || rule.check.trim().is_empty() {
            return Err(invalid_report(format!(
                "rule {:?} has an empty subject, match, or check",
                rule.name
            )));
        }
        for archetype in &rule.subject {
            expected_archetypes.insert(archetype.clone());
            if !report.facts.contains_key(archetype) {
                return Err(invalid_report(format!(
                    "rule {:?} names subject archetype {:?} without facts",
                    rule.name, archetype
                )));
            }
        }
        for condition in rule.matches.iter().chain(&rule.unless_any) {
            let paths = condition.path.as_ref().map(|path| vec![path.as_slice()]);
            let any_paths = condition
                .any_path
                .as_ref()
                .map(|paths| paths.iter().map(Vec::as_slice).collect::<Vec<_>>());
            if paths.is_some() == any_paths.is_some() {
                return Err(invalid_report(format!(
                    "rule {:?} condition must have exactly one of path and any_path",
                    rule.name
                )));
            }
            let tests = usize::from(condition.any_of.is_some())
                + usize::from(condition.absent == Some(true))
                + usize::from(condition.present == Some(true));
            if tests != 1 {
                return Err(invalid_report(format!(
                    "rule {:?} condition must have exactly one predicate",
                    rule.name
                )));
            }
            let path_refs = paths
                .as_ref()
                .into_iter()
                .flatten()
                .chain(any_paths.as_ref().into_iter().flatten())
                .copied()
                .collect::<Vec<_>>();
            for path in &path_refs {
                for step in *path {
                    if !matches!(step.direction.as_str(), "outgoing" | "incoming")
                        || step.edge.is_empty()
                        || step.to.is_empty()
                    {
                        return Err(invalid_report(format!(
                            "rule {:?} contains an invalid path step",
                            rule.name
                        )));
                    }
                    for archetype in &step.to {
                        expected_archetypes.insert(archetype.clone());
                        if !report.facts.contains_key(archetype) {
                            return Err(invalid_report(format!(
                                "rule {:?} path names target archetype {:?} without facts",
                                rule.name, archetype
                            )));
                        }
                    }
                }
            }
            if condition.any_path.as_ref().is_some_and(Vec::is_empty)
                || condition.field.trim().is_empty()
            {
                return Err(invalid_report(format!(
                    "rule {:?} contains an empty any_path or field",
                    rule.name
                )));
            }
            if let Some(values) = &condition.any_of {
                let mut endpoints = BTreeSet::new();
                for path in &path_refs {
                    if let Some(last) = path.last() {
                        endpoints.extend(last.to.iter().cloned());
                    } else {
                        endpoints.extend(rule.subject.iter().cloned());
                    }
                }
                for archetype in endpoints {
                    if let Some(vocabulary) = report
                        .vocabularies
                        .get(&archetype)
                        .and_then(|fields| fields.get(&condition.field))
                    {
                        let declared = vocabulary
                            .iter()
                            .map(|value| fold(value))
                            .collect::<BTreeSet<_>>();
                        if values.iter().any(|value| !declared.contains(&fold(value))) {
                            return Err(invalid_report(format!(
                                "rule {:?} selects a value outside the closed vocabulary for {:?}.{:?}",
                                rule.name, archetype, condition.field
                            )));
                        }
                    }
                }
            }
        }
    }
    for (archetype, facts) in &report.facts {
        for fact in facts {
            for edge in &fact.edges {
                if !matches!(edge.direction.as_str(), "outgoing" | "incoming")
                    || !report.facts.contains_key(&edge.archetype)
                {
                    return Err(invalid_report(format!(
                        "fact {:?} in {:?} contains an invalid edge",
                        fact.id, archetype
                    )));
                }
                let target_present = report.facts.get(&edge.archetype).is_some_and(|targets| {
                    targets.iter().any(|target| {
                        target.id == edge.target.id && target.repo == edge.target.repo
                    })
                });
                if !target_present {
                    return Err(invalid_report(format!(
                        "fact {:?} edge target {:?} is absent from {:?} facts",
                        fact.id, edge.target.id, edge.archetype
                    )));
                }
            }
        }
    }
    let actual_archetypes = report.facts.keys().cloned().collect::<BTreeSet<_>>();
    if actual_archetypes != expected_archetypes {
        return Err(invalid_report(format!(
            "facts archetypes {actual_archetypes:?} do not match the rule-declared set {expected_archetypes:?}"
        )));
    }
    Ok(())
}

struct Generated {
    source: String,
    model: String,
    snapshot: String,
    request: String,
    incomplete_items: BTreeSet<String>,
}

impl Generated {
    fn new(report: &Report) -> Result<Self, FlowAnalyzeError> {
        Self::with_item_limit(report, MAX_GENERATED_ITEMS)
    }

    fn with_item_limit(report: &Report, item_limit: usize) -> Result<Self, FlowAnalyzeError> {
        let rule_pairs = report
            .rules
            .len()
            .checked_mul(report.rules.len().saturating_sub(1))
            .ok_or_else(|| invalid_report("rule implication count overflows"))?;
        let implication_count = rule_pairs;
        if implication_count > item_limit {
            return Err(invalid_report(format!(
                "flow analysis exceeds the {item_limit} generated-item limit"
            )));
        }
        let flow_item_limit = item_limit - implication_count;
        let mut source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             profile t0 = \"filament.temporal.infinite-trace/v1\";\n\
             model M = {PACKAGE:?};\n"
        );
        let facts = flatten_facts(&report.facts);
        let rels = relationship_names(report);
        let key_map = flat_key_map(&facts);
        let mut flow_items = Vec::new();
        let mut incomplete_items = BTreeSet::new();
        let group_count = report.rules.iter().try_fold(0usize, |count, rule| {
            count.checked_add(subject_groups(rule, &facts).len())
        });
        let Some(group_count) = group_count else {
            return Err(invalid_report("rule-subject group count overflows"));
        };
        if group_count > flow_item_limit {
            return Err(invalid_report(format!(
                "flow analysis has {group_count} rule-subject groups but only {flow_item_limit} generated items are available"
            )));
        }
        let mut groups_left = group_count;
        for (index, rule) in report.rules.iter().enumerate() {
            for subject in subject_groups(rule, &facts) {
                let remaining = flow_item_limit - flow_items.len();
                let quota = remaining / groups_left;
                let tuples = rule_tuples(rule, subject, &facts, &key_map, quota)?;
                for tuple in tuples {
                    let name = rule_tuple_claim(index, rule, &tuple);
                    let witnesses = if tuple.incomplete {
                        None
                    } else {
                        Some(tuple.matches.as_slice())
                    };
                    let denied = rule_expression_scoped(
                        rule,
                        &rule.subject,
                        &rels,
                        tuple.subject.as_ref().map(|subject| subject.key.as_str()),
                        witnesses,
                    );
                    source.push_str(&format!(
                        "temporal {name} using t0 over (x: M::Fact) clock \"model-steps\" on origin {{ always holds(not ({denied})) }}\n"
                    ));
                    flow_items.push(name.clone());
                    if tuple.incomplete {
                        // This native per-item ceiling turns the bounded
                        // generation marker into an explicit non-conclusive
                        // result while allowing the other items to settle.
                        incomplete_items.insert(name);
                    }
                }
                groups_left -= 1;
            }
        }
        let mut implication_items = Vec::new();
        for left in 0..report.rules.len() {
            for right in (left + 1)..report.rules.len() {
                let a = &report.rules[left];
                let b = &report.rules[right];
                if a.subject.iter().all(|kind| !b.subject.contains(kind)) {
                    continue;
                }
                for (from, to, suffix) in [(a, b, "AtoB"), (b, a, "BtoA")] {
                    let shared = from
                        .subject
                        .iter()
                        .filter(|kind| to.subject.contains(kind))
                        .cloned()
                        .collect::<Vec<_>>();
                    let name = format!(
                        "FlowRedundancy{left}_{right}_{suffix}_{}_{}",
                        hex(a.name.as_bytes()),
                        hex(b.name.as_bytes())
                    );
                    let from_deny = rule_expression_scoped(from, &shared, &rels, None, None);
                    let to_deny = rule_expression_scoped(to, &shared, &rels, None, None);
                    source.push_str(&format!(
                        "temporal {name} using t0 over (x: M::Fact) clock \"model-steps\" on origin {{ always holds((not ({from_deny})) or ({to_deny})) }}\n"
                    ));
                    implication_items.push(name);
                }
            }
        }
        let model = model_document(&facts, report)?;
        let snapshot = snapshot_document(&facts, &rels, &report.facts)?;
        let mut items = flow_items
            .into_iter()
            .map(|claim| json!({"claim": claim}))
            .collect::<Vec<_>>();
        items.extend(
            implication_items
                .into_iter()
                .map(|claim| json!({"claim": claim})),
        );
        let request = json!({
            "format": "native-analyze/1",
            "request": {
                "program": {"source": {"file": "flow.fsl", "authority": "fsl-flow", "identity": "generated-flow", "document": "Flow", "formal_revision": 1}},
                "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "model.json", "authority": "fsl-flow", "identity": "generated-model", "document": "FlowModel", "formal_revision": 1}}],
                "documents": [{"file": "snapshot.json"}],
                "items": items
            }
        });
        Ok(Self {
            source,
            model,
            snapshot: snapshot.to_string(),
            request: request.to_string(),
            incomplete_items,
        })
    }
}

fn rule_expression_scoped(
    rule: &Rule,
    subject_types: &[String],
    rels: &BTreeMap<(String, String), String>,
    subject_key: Option<&str>,
    match_witnesses: Option<&[Option<ClaimWitness>]>,
) -> String {
    let mut subject_terms = subject_types
        .iter()
        .map(|kind| format!("x.archetype = {}", q(kind)))
        .collect::<Vec<_>>();
    if let Some(subject_key) = subject_key {
        subject_terms.push(format!("x.key = {}", q(subject_key)));
    }
    let subject = or(subject_terms.into_iter());
    let matches = and(rule.matches.iter().enumerate().map(|(index, condition)| {
        condition_expression(
            "x",
            condition,
            rels,
            match_witnesses
                .and_then(|witnesses| witnesses.get(index))
                .and_then(Option::as_ref),
        )
    }));
    let protected = if rule.unless_any.is_empty() {
        "false".to_owned()
    } else {
        or(rule
            .unless_any
            .iter()
            .map(|condition| condition_expression("x", condition, rels, None)))
    };
    format!("({subject}) and ({matches}) and not ({protected})")
}

fn condition_expression(
    root: &str,
    condition: &Condition,
    rels: &BTreeMap<(String, String), String>,
    witness: Option<&ClaimWitness>,
) -> String {
    let paths = condition
        .path
        .as_ref()
        .map(|path| vec![path])
        .or_else(|| {
            condition
                .any_path
                .as_ref()
                .map(|paths| paths.iter().collect())
        })
        .unwrap_or_default();
    let test = if let Some(values) = &condition.any_of {
        let accepted = or(values
            .iter()
            .map(|value| format!("v.value = {}", q(&fold(value)))));
        format!("v.kind = \"string\" and ({accepted})")
    } else {
        String::new()
    };
    let present = condition.present == Some(true);
    let absent = condition.absent == Some(true);
    if let Some(ClaimWitness::Unreachable) = witness {
        return "false".to_owned();
    }
    let scoped_path = match witness {
        Some(ClaimWitness::Path { path_index, keys }) => {
            paths.get(*path_index).map(|path| (*path, keys.as_slice()))
        }
        _ => None,
    };
    if condition.field == "id" && (present || absent) {
        let exists_endpoint = if let Some((path, keys)) = scoped_path {
            path_exists_direct_scoped(root, path, "true", rels, Some(keys))
        } else {
            or(paths
                .iter()
                .map(|path| path_exists_direct_scoped(root, path, "true", rels, None)))
        };
        return if absent {
            format!("not ({exists_endpoint})")
        } else {
            exists_endpoint
        };
    }
    let exists_any = if let Some((path, keys)) = scoped_path {
        let endpoint = if absent || present {
            format!("v.field = {}", q(&condition.field))
        } else {
            format!("v.field = {} and ({test})", q(&condition.field))
        };
        path_exists_scoped(root, path, &endpoint, rels, Some(keys))
    } else {
        or(paths.iter().map(|path| {
            let endpoint = if absent || present {
                format!("v.field = {}", q(&condition.field))
            } else {
                format!("v.field = {} and ({test})", q(&condition.field))
            };
            path_exists_scoped(root, path, &endpoint, rels, None)
        }))
    };
    if absent {
        format!("not ({exists_any})")
    } else {
        exists_any
    }
}

fn path_exists_scoped(
    root: &str,
    path: &[Step],
    endpoint: &str,
    rels: &BTreeMap<(String, String), String>,
    witness_keys: Option<&[String]>,
) -> String {
    let mut current = root.to_owned();
    let mut prefixes = Vec::new();
    for (index, step) in path.iter().enumerate() {
        let next = format!("n{index}");
        let role = rels
            .get(&(step.edge.clone(), step.direction.clone()))
            .cloned()
            .unwrap_or_else(|| "missing_edge".to_owned());
        let allowed = or(step
            .to
            .iter()
            .map(|kind| format!("{next}.archetype = {}", q(kind))));
        let selected = witness_keys
            .and_then(|keys| keys.get(index))
            .map(|key| format!(" and {next}.key = {}", q(key)))
            .unwrap_or_default();
        prefixes.push((
            next.clone(),
            format!("reaches({current}, {next}, {role}) and ({allowed}){selected}"),
        ));
        current = next;
    }
    let value_cond = format!(
        "exists(v in allInstances<M::Fact>(M::values) : reaches({current}, v, hasValue) and ({endpoint}))"
    );
    let mut nested = value_cond;
    for (variable, prefix) in prefixes.into_iter().rev() {
        nested = format!(
            "exists({variable} in allInstances<M::Fact>(M::facts) : ({prefix}) and ({nested}))"
        );
    }
    if path.is_empty() {
        // The root is already the quantified subject; bind only its values.
        format!(
            "exists(v in allInstances<M::Fact>(M::values) : reaches({root}, v, hasValue) and ({endpoint}))"
        )
    } else {
        nested
    }
}

fn path_exists_direct_scoped(
    root: &str,
    path: &[Step],
    endpoint: &str,
    rels: &BTreeMap<(String, String), String>,
    witness_keys: Option<&[String]>,
) -> String {
    let mut current = root.to_owned();
    let mut prefixes = Vec::new();
    for (index, step) in path.iter().enumerate() {
        let next = format!("n{index}");
        let role = rels
            .get(&(step.edge.clone(), step.direction.clone()))
            .cloned()
            .unwrap_or_else(|| "missing_edge".to_owned());
        let allowed = or(step
            .to
            .iter()
            .map(|kind| format!("{next}.archetype = {}", q(kind))));
        let selected = witness_keys
            .and_then(|keys| keys.get(index))
            .map(|key| format!(" and {next}.key = {}", q(key)))
            .unwrap_or_default();
        prefixes.push((
            next.clone(),
            format!("reaches({current}, {next}, {role}) and ({allowed}){selected}"),
        ));
        current = next;
    }
    let mut nested = endpoint.to_owned();
    for (variable, prefix) in prefixes.into_iter().rev() {
        nested = format!(
            "exists({variable} in allInstances<M::Fact>(M::facts) : ({prefix}) and ({nested}))"
        );
    }
    nested
}

fn model_document(facts: &[FlatFact], report: &Report) -> Result<String, FlowAnalyzeError> {
    let identity = |suffix: &str| format!("ix://{PACKAGE}/{suffix}");
    let origin = |name: &str| json!({"generated":{"generatorIdentity":GENERATOR,"generatorVersion":"1.0.0","inputIdentities":[name]}});
    let constructs = [
        ("object_type", fsl_semantics::model::intake::meaning::OBJECT_TYPE),
        ("population", fsl_semantics::model::intake::meaning::POPULATION),
    ].into_iter().map(|(name, meaning)| json!({"kind":{"module":PACKAGE,"name":name},"moduleVersion":"1.0.0","construct":{"identity":"none","shape":"record","members":{},"meaning":meaning}})).collect::<Vec<_>>();
    let fields = |owner: &str, names: &[&str]| {
        names.iter().map(|name| {
        let id = identity(&format!("{owner}/{name}"));
        json!({"identity":id,"name":name,"typeRef":NATIVE_STRING,"presence":"required","nullable":false,"defaultKind":"none","multiplicity":{"lower":1,"upper":1,"ordered":false,"unique":true},"origin":origin(&id)})
    }).collect::<Vec<_>>()
    };
    let rel_entries = relationship_records(report);
    let fact_relationships = rel_entries
        .iter()
        .map(|(name, role)| relationship_json(&identity, &origin, name, role, "Fact", "Fact"))
        .chain(std::iter::once(relationship_json(
            &identity, &origin, "hasValue", "hasValue", "Fact", "Fact",
        )))
        .collect::<Vec<_>>();
    let object = |name: &str, fs: Vec<Value>, relationships: Vec<Value>| {
        let id = identity(name);
        json!({"identity":id,"displayName":id,"kind":{"module":PACKAGE,"name":"object_type"},"roles":[],"origin":origin(&id),"constraints":[],"extensions":[],"unknownPolicy":"reject","supertypes":[],"fields":fs,"operations":[],"relationships":relationships})
    };
    let types = vec![object(
        "Fact",
        fields(
            "Fact",
            &[
                "key",
                "archetype",
                "id",
                "repo",
                "path",
                "field",
                "kind",
                "value",
            ],
        ),
        fact_relationships,
    )];
    // The closed report vocabularies are verified against the facts' strings;
    // their normalized members are the only accepted values for enum fields.
    for (archetype, fields) in &report.vocabularies {
        if !report.facts.contains_key(archetype) {
            return Err(invalid_report(format!(
                "vocabulary names absent archetype {archetype:?}"
            )));
        }
        for values in fields.values() {
            if values.is_empty() {
                return Err(invalid_report("empty closed vocabulary"));
            }
        }
    }
    let populations = [
        ("facts", "Fact"),
        ("values", "Fact"),
    ].into_iter().map(|(name, ty)| {
        let id = identity(name);
        json!({"identity":id,"displayName":id,"kind":{"module":PACKAGE,"name":"population"},"members":[identity(ty)],"extent":"closed","origin":origin(&id)})
    }).collect::<Vec<_>>();
    let _ = facts;
    Ok(json!({
        "contractVersion":"2.0.0",
        "canonicalization":{"algorithm":"rfc8785-v1"},
        "source":{"identity":identity("spec"),"version":"1.0.0","dialect":"spec-bundle"},
        "package":{"identity":PACKAGE,"version":"1.0.0","mappingVersions":[],"profileVersions":[]},
        "occurrences":[],"extensions":[],"constructs":constructs,"types":types,"populations":populations
    }).to_string())
}

fn relationship_json(
    identity: &impl Fn(&str) -> String,
    _origin: &impl Fn(&str) -> Value,
    name: &str,
    role: &str,
    source_type: &str,
    target_type: &str,
) -> Value {
    let id = identity(&format!("relationship/{name}"));
    // FR-026 admits only exact RFC 8785 integer spellings; this far exceeds
    // the maximum relation count in the bounded native request.
    let end = |ty: &str, role: &str| json!({"type":identity(ty),"role":role,"multiplicity":{"lower":0,"upper":9_007_199_254_740_991_u64,"ordered":false,"unique":true}});
    json!({"identity":id,"sourceEnd":end(source_type,role),"targetEnd":end(target_type,&format!("inverse_{role}")),"category":"structural","composite":false,"direction":"source-to-target","origin":{"source":{"sourceIdentity":identity("spec"),"path":"generated-flow.fsl","startLine":1,"startColumn":1}}})
}

#[derive(Clone)]
struct FlatFact {
    archetype: String,
    fact: Fact,
    key: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ClaimWitness {
    /// One concrete path from the subject to a report fact.
    Path {
        path_index: usize,
        keys: Vec<String>,
    },
    /// No report path can reach a candidate endpoint.
    Unreachable,
}

struct RuleTuple {
    subject: Option<FlatFact>,
    matches: Vec<Option<ClaimWitness>>,
    incomplete: bool,
}

fn flat_key_map(facts: &[FlatFact]) -> BTreeMap<(String, String, Option<String>), String> {
    facts
        .iter()
        .map(|fact| {
            (
                (
                    fact.archetype.clone(),
                    fact.fact.id.clone(),
                    fact.fact.repo.clone(),
                ),
                fact.key.clone(),
            )
        })
        .collect()
}

fn subject_groups(rule: &Rule, facts: &[FlatFact]) -> Vec<Option<FlatFact>> {
    let subjects = facts
        .iter()
        .filter(|fact| rule.subject.contains(&fact.archetype))
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    if subjects.is_empty() {
        vec![None]
    } else {
        subjects
    }
}

fn rule_tuples(
    rule: &Rule,
    subject: Option<FlatFact>,
    facts: &[FlatFact],
    key_map: &BTreeMap<(String, String, Option<String>), String>,
    max_tuples: usize,
) -> Result<Vec<RuleTuple>, FlowAnalyzeError> {
    let fact_by_key = facts
        .iter()
        .map(|fact| (fact.key.as_str(), fact))
        .collect::<BTreeMap<_, _>>();
    let Some(subject) = subject else {
        if max_tuples == 0 {
            return Err(expansion_limit(max_tuples));
        }
        return Ok(vec![RuleTuple {
            subject: None,
            matches: rule.matches.iter().map(|_| None).collect(),
            incomplete: false,
        }]);
    };
    if max_tuples == 0 {
        return Err(expansion_limit(max_tuples));
    }
    let mut witness_truncated = false;
    let witness_limit = max_tuples.saturating_add(1);
    let choices = rule
        .matches
        .iter()
        .map(|condition| {
            if condition.absent == Some(true) {
                return Ok((vec![None], false));
            }
            let mut witnesses = BTreeSet::new();
            let mut condition_truncated = false;
            for (path_index, path) in condition_paths(condition).iter().enumerate() {
                let (paths, path_truncated) =
                    path_targets(&subject, path, key_map, &fact_by_key, witness_limit)?;
                condition_truncated |= path_truncated;
                for keys in paths {
                    witnesses.insert(Some(ClaimWitness::Path { path_index, keys }));
                    if witnesses.len() > witness_limit {
                        witnesses.pop_last();
                        condition_truncated = true;
                        break;
                    }
                }
                if condition_truncated && witnesses.len() >= witness_limit {
                    break;
                }
            }
            witness_truncated |= condition_truncated;
            if witnesses.is_empty() {
                Ok((vec![Some(ClaimWitness::Unreachable)], condition_truncated))
            } else {
                Ok((witnesses.into_iter().collect(), condition_truncated))
            }
        })
        .collect::<Result<Vec<_>, FlowAnalyzeError>>()?;
    let choices = choices
        .into_iter()
        .map(|(choices, _)| choices)
        .collect::<Vec<_>>();
    let tuple_count = choices
        .iter()
        .try_fold(1usize, |count, choice| count.checked_mul(choice.len()));
    let tuple_count = tuple_count.unwrap_or(usize::MAX);
    let incomplete = witness_truncated || tuple_count > max_tuples;
    let concrete_limit = if incomplete {
        max_tuples.saturating_sub(1)
    } else {
        max_tuples
    };
    let mut tuples = product_limited(&choices, concrete_limit)
        .into_iter()
        .map(|matches| RuleTuple {
            subject: Some(subject.clone()),
            matches,
            incomplete: false,
        })
        .collect::<Vec<_>>();
    if incomplete {
        let marker_matches = product_limited(&choices, 1)
            .into_iter()
            .next()
            .unwrap_or_else(|| rule.matches.iter().map(|_| None).collect());
        tuples.push(RuleTuple {
            subject: Some(subject),
            matches: marker_matches,
            incomplete: true,
        });
    }
    Ok(tuples)
}

fn expansion_limit(limit: usize) -> FlowAnalyzeError {
    invalid_report(format!(
        "flow analysis has no remaining generated-item capacity for a group (limit {limit})"
    ))
}

fn condition_paths(condition: &Condition) -> Vec<Vec<Step>> {
    condition
        .path
        .as_ref()
        .map(|path| vec![path.clone()])
        .or_else(|| condition.any_path.clone())
        .unwrap_or_default()
}

fn path_targets(
    subject: &FlatFact,
    path: &[Step],
    key_map: &BTreeMap<(String, String, Option<String>), String>,
    fact_by_key: &BTreeMap<&str, &FlatFact>,
    limit: usize,
) -> Result<(Vec<Vec<String>>, bool), FlowAnalyzeError> {
    let mut current = vec![(subject.key.clone(), Vec::<String>::new())];
    let mut truncated = false;
    for step in path {
        let mut next = BTreeSet::new();
        for (source_key, prefix) in current {
            let Some(source) = fact_by_key.get(source_key.as_str()) else {
                continue;
            };
            for edge in &source.fact.edges {
                if edge.edge != step.edge
                    || edge.direction != step.direction
                    || !step.to.contains(&edge.archetype)
                {
                    continue;
                }
                let Some(target_key) = key_map.get(&(
                    edge.archetype.clone(),
                    edge.target.id.clone(),
                    edge.target.repo.clone(),
                )) else {
                    continue;
                };
                let mut keys = prefix.clone();
                keys.push(target_key.clone());
                next.insert((target_key.clone(), keys));
                if next.len() > limit {
                    next.pop_last();
                    truncated = true;
                }
            }
        }
        current = next.into_iter().collect();
    }
    Ok((
        current.into_iter().map(|(_, keys)| keys).collect(),
        truncated,
    ))
}

fn product_limited(
    choices: &[Vec<Option<ClaimWitness>>],
    limit: usize,
) -> Vec<Vec<Option<ClaimWitness>>> {
    fn append(
        choices: &[Vec<Option<ClaimWitness>>],
        index: usize,
        current: &mut Vec<Option<ClaimWitness>>,
        output: &mut Vec<Vec<Option<ClaimWitness>>>,
        limit: usize,
    ) {
        if output.len() == limit {
            return;
        }
        if index == choices.len() {
            output.push(current.clone());
            return;
        }
        for choice in &choices[index] {
            current.push(choice.clone());
            append(choices, index + 1, current, output, limit);
            current.pop();
        }
    }
    let mut output = Vec::new();
    append(choices, 0, &mut Vec::new(), &mut output, limit);
    output
}

fn rule_tuple_claim(index: usize, rule: &Rule, tuple: &RuleTuple) -> String {
    let mut readable = Vec::new();
    if let Some(subject) = &tuple.subject {
        readable.push(format!(
            "subject_{}_{}",
            symbol_part(&subject.archetype),
            symbol_part(&subject.fact.id)
        ));
    }
    for witness in &tuple.matches {
        match witness {
            Some(ClaimWitness::Path { keys, .. }) => {
                if let Some(endpoint) = keys.last().and_then(|key| key_label(key)) {
                    readable.push(format!("match_{endpoint}"));
                }
            }
            Some(ClaimWitness::Unreachable) => readable.push("match_unreachable".into()),
            None => readable.push("match_absent".into()),
        }
    }
    if tuple.incomplete {
        readable.push("item_limit".to_owned());
    }
    let payload = match &tuple.subject {
        Some(subject) => json!({
            "rule":rule.name,
            "subject":{"archetype":subject.archetype,"id":subject.fact.id,"repo":subject.fact.repo},
            "matched_paths":tuple.matches.iter().map(|witness| match witness {
                Some(ClaimWitness::Path { path_index, keys }) => json!({"path_index":path_index,"fact_keys":keys}),
                Some(ClaimWitness::Unreachable) => json!({"unreachable":true}),
                None => json!({"absent":true})
            }).collect::<Vec<_>>(),
            "incomplete":tuple.incomplete
        }),
        None => json!({"rule":rule.name,"subject":null,"incomplete":tuple.incomplete}),
    };
    let visible = symbol_part(&rule.name);
    format!(
        "FlowRule{index}_{visible}_{}_{}",
        readable.join("_"),
        hex(payload.to_string().as_bytes())
    )
}

fn key_label(key: &str) -> Option<String> {
    let identity: Value = serde_json::from_str(key).ok()?;
    let parts = identity.as_array()?;
    Some(format!(
        "{}_{}",
        symbol_part(parts.get(1)?.as_str()?),
        symbol_part(parts.get(3)?.as_str()?)
    ))
}

fn symbol_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn flatten_facts(grouped: &BTreeMap<String, Vec<Fact>>) -> Vec<FlatFact> {
    let mut out = Vec::new();
    for (archetype, facts) in grouped {
        for fact in facts {
            let identity = json!(["subject", archetype, fact.repo, fact.id]).to_string();
            let key = identity;
            out.push(FlatFact {
                archetype: archetype.clone(),
                fact: fact.clone(),
                key,
            });
        }
    }
    out.sort_by(|left, right| left.key.cmp(&right.key));
    out
}

fn snapshot_document(
    facts: &[FlatFact],
    rels: &BTreeMap<(String, String), String>,
    grouped: &BTreeMap<String, Vec<Fact>>,
) -> Result<Value, FlowAnalyzeError> {
    let mut key_map = BTreeMap::new();
    for fact in facts {
        key_map.insert(
            (
                fact.archetype.clone(),
                fact.fact.id.clone(),
                fact.fact.repo.clone(),
            ),
            fact.key.clone(),
        );
    }
    let mut values = Vec::new();
    for (fact_index, fact) in facts.iter().enumerate() {
        for (field_index, (field, value)) in fact.fact.fields.iter().enumerate() {
            let entries = field_entries(value);
            for (value_index, entry) in entries.iter().enumerate() {
                let (kind, value) = match entry {
                    Value::String(text) => ("string", fold(text)),
                    _ => ("other", String::new()),
                };
                let key = format!("v{fact_index}_{field_index}_{value_index}");
                values.push(json!({"key":key,"type":format!("ix://{PACKAGE}/Fact"),"fields":{"key":string_wire(&key),"archetype":string_wire("__flow_value"),"id":string_wire(&key),"repo":string_wire(""),"path":string_wire(""),"field":string_wire(field),"kind":string_wire(kind),"value":string_wire(&value)}}));
            }
        }
    }
    let fact_objects = facts
        .iter()
        .map(|fact| {
            json!({
                "key":fact.key,
                "type":format!("ix://{PACKAGE}/Fact"),
                "fields":{
                    "key":string_wire(&fact.key),
                    "archetype":string_wire(&fact.archetype),
                    "id":string_wire(&fact.fact.id),
                    "repo":string_wire(fact.fact.repo.as_deref().unwrap_or("")),
                    "path":string_wire(&fact.fact.path),
                    "field":string_wire(""),
                    "kind":string_wire(""),
                    "value":string_wire("")
                }
            })
        })
        .collect::<Vec<_>>();
    let mut instances: BTreeMap<String, Vec<Value>> = rels
        .values()
        .map(|name| (name.clone(), Vec::new()))
        .collect();
    instances.insert("hasValue".into(), Vec::new());
    for (fact_index, fact) in facts.iter().enumerate() {
        for (field_index, (_field, value)) in fact.fact.fields.iter().enumerate() {
            let entries = field_entries(value);
            for (item_index, _) in entries.iter().enumerate() {
                let key = format!("v{fact_index}_{field_index}_{item_index}");
                instances
                    .get_mut("hasValue")
                    .expect("hasValue extent")
                    .push(link(&fact.key, &key, "facts", "values"));
            }
        }
    }
    for fact in facts {
        for edge in &fact.fact.edges {
            let name = rels
                .get(&(edge.edge.clone(), edge.direction.clone()))
                .ok_or_else(|| {
                    invalid_report(format!(
                        "undeclared edge/direction {:?}/{:?}",
                        edge.edge, edge.direction
                    ))
                })?;
            let target = key_map
                .get(&(
                    edge.archetype.clone(),
                    edge.target.id.clone(),
                    edge.target.repo.clone(),
                ))
                .ok_or_else(|| {
                    invalid_report(format!(
                        "edge target {:?} {:?} {:?} is absent from facts",
                        edge.archetype, edge.target.repo, edge.target.id
                    ))
                })?;
            instances
                .get_mut(name)
                .expect("declared relationship extent")
                .push(link(&fact.key, target, "facts", "facts"));
        }
    }
    let mut relationship_extents = Vec::new();
    for (name, links) in instances {
        let relation_identity = format!("ix://{PACKAGE}/relationship/{name}");
        relationship_extents
            .push(json!({"relationship":relation_identity,"complete":true,"instances":links}));
    }
    let mut populations = vec![
        json!({"population":format!("ix://{PACKAGE}/facts"),"complete":true,"objects":fact_objects}),
    ];
    populations.push(
        json!({"population":format!("ix://{PACKAGE}/values"),"complete":true,"objects":values}),
    );
    let _ = grouped;
    Ok(json!({
        "format":"filament.state.snapshot/v1",
        "identity":{"authority":"fsl-flow","identity":"generated-snapshot"},
        "observation":"current",
        "model":{"identity":PACKAGE},
        "anchor":{"kind":"initialization","name":"flow"},
        "populations":populations,
        "relationships":relationship_extents
    }))
}

fn link(source: &str, target: &str, source_pop: &str, target_pop: &str) -> Value {
    json!({"source":{"population":format!("ix://{PACKAGE}/{source_pop}"),"key":source},"target":{"population":format!("ix://{PACKAGE}/{target_pop}"),"key":target}})
}

fn string_wire(value: &str) -> Value {
    json!({"kind":"string","text":value})
}

fn field_entries(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(items) if items.is_empty() => vec![value],
        Value::Array(items) => items.iter().collect(),
        one => vec![one],
    }
}

fn relationship_names(report: &Report) -> BTreeMap<(String, String), String> {
    let mut keys = BTreeSet::new();
    for rule in &report.rules {
        for condition in rule.matches.iter().chain(&rule.unless_any) {
            let paths = condition
                .path
                .iter()
                .cloned()
                .chain(condition.any_path.iter().flatten().cloned());
            for path in paths {
                for step in path {
                    keys.insert((step.edge, step.direction));
                }
            }
        }
    }
    keys.into_iter()
        .enumerate()
        .map(|(index, key)| (key, format!("edge{index}")))
        .collect()
}

fn relationship_records(report: &Report) -> Vec<(String, String)> {
    relationship_names(report)
        .into_values()
        .map(|role| (role.clone(), role))
        .collect()
}

fn fold(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
fn q(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}
fn and(values: impl Iterator<Item = String>) -> String {
    join(values, " and ", "true")
}
fn or(values: impl Iterator<Item = String>) -> String {
    join(values, " or ", "false")
}
fn join(mut values: impl Iterator<Item = String>, delimiter: &str, empty: &str) -> String {
    let Some(first) = values.next() else {
        return empty.to_owned();
    };
    values.fold(first, |text, value| format!("({text}){delimiter}({value})"))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct Workspace {
    path: PathBuf,
}
impl Workspace {
    fn new() -> Result<Self, std::io::Error> {
        let base = std::env::temp_dir();
        for _ in 0..64 {
            let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("fsl-flow-{}-{id}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not reserve a flow workspace",
        ))
    }
    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), std::io::Error> {
        std::fs::write(self.path.join(name), bytes)
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
struct Supported;
#[cfg(test)]
impl Negotiate for Supported {
    fn negotiate(
        &self,
        _item: &fsl_analyze::analyze::ClaimItem,
        candidates: &CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        match candidates {
            CandidateOutcome::Candidates(set) => match set.candidates() {
                [candidate] => (
                    fsl_route::routing::Disposition::Supported(candidate.clone()),
                    None,
                ),
                _ => (fsl_route::routing::Disposition::Unsupported, None),
            },
            CandidateOutcome::UnknownBackend(_) => {
                (fsl_route::routing::Disposition::Unsupported, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn report(protected: bool) -> Vec<u8> {
        let mut connection_edges = vec![
            json!({"edge":"carries","direction":"outgoing","target":{"id":"CLASS-1"},"archetype":"classification"}),
            json!({"edge":"crosses","direction":"outgoing","target":{"id":"BOUNDARY-1"},"archetype":"boundary"}),
        ];
        let mut facts = vec![
            json!({"id":"CONN-1","path":"connections/conn.md","fields":{},"edges":connection_edges}),
            json!({"id":"CLASS-1","path":"classification/class.md","fields":{"regulated_categories":[" Health "]},"edges":[]}),
            json!({"id":"BOUNDARY-1","path":"boundary/b.md","fields":{"trust_level":" Untrusted "},"edges":[]}),
        ];
        if protected {
            facts.push(json!({"id":"CTRL-1","path":"control/c.md","fields":{"safeguards":["encrypt-in-transit"]},"edges":[]}));
            facts[0]["edges"].as_array_mut().expect("edges").push(json!({"edge":"protects","direction":"incoming","target":{"id":"CTRL-1"},"archetype":"control"}));
        }
        connection_edges.clear();
        let mut grouped =
            json!({"connection":[facts[0]],"classification":[facts[1]],"boundary":[facts[2]]});
        if protected {
            grouped["control"] = json!([facts[3]]);
        }
        let mut vocabularies = json!({
            "classification":{"regulated_categories":["health"]},
            "boundary":{"trust_level":["untrusted"]}
        });
        if protected {
            vocabularies["control"] = json!({"safeguards":["encrypt-in-transit"]});
        }
        json!({
            "schema":"quire.flow-report/v1",
            "rules":[{
                "name":"health-across-untrusted-boundary",
                "subject":["connection"],
                "match":[
                    {"path":[{"edge":"carries","direction":"outgoing","to":["classification"]}],"field":"regulated_categories","any_of":["health"]},
                    {"path":[{"edge":"crosses","direction":"outgoing","to":["boundary"]}],"field":"trust_level","any_of":["untrusted"]}
                ],
                "unless_any": if protected { vec![json!({"path":[{"edge":"protects","direction":"incoming","to":["control"]}],"field":"safeguards","any_of":["encrypt-in-transit"]})] } else { vec![] },
                "check":"health-across-untrusted-boundary","remediations":[],"_rationale":"fixture"
            }],
            "vocabularies":vocabularies,
            "facts":grouped,
            "denials":[],"remediations":[]
        }).to_string().into_bytes()
    }

    fn report_with_two_connections() -> Value {
        let mut value: Value = serde_json::from_slice(&report(false)).unwrap();
        let facts = &mut value["facts"];
        facts["connection"][0]["edges"] = json!([
            {"edge":"carries","direction":"outgoing","target":{"id":"CLASS-1"},"archetype":"classification"},
            {"edge":"carries","direction":"outgoing","target":{"id":"CLASS-2"},"archetype":"classification"},
            {"edge":"crosses","direction":"outgoing","target":{"id":"BOUNDARY-1"},"archetype":"boundary"}
        ]);
        facts["connection"].as_array_mut().unwrap().push(json!({
            "id":"CONN-2","path":"connections/conn-2.md","fields":{},"edges":[
                {"edge":"carries","direction":"outgoing","target":{"id":"CLASS-1"},"archetype":"classification"},
                {"edge":"crosses","direction":"outgoing","target":{"id":"BOUNDARY-1"},"archetype":"boundary"}
            ]
        }));
        facts["classification"].as_array_mut().unwrap().push(json!({
            "id":"CLASS-2","path":"classification/class-2.md","fields":{"regulated_categories":["Health"]},"edges":[]
        }));
        value
    }

    #[test]
    #[trace("FR-821-AC-1")]
    #[trace("FR-821-AC-2")]
    #[trace("FR-821-AC-8")]
    fn native_claim_reads_report_facts_and_resolved_edges_not_denials() {
        let cancel = Cancel::new();
        let denied =
            analyze_flow_report(&report(false), &Supported, &cancel).expect("native analysis");
        let denied = denied.value.as_value();
        assert_eq!(denied["format"], "filament-outcome/1");
        assert_eq!(denied["items"][0]["record"]["result"], "refuted");
        let claim_name = denied["items"][0]["claim_name"]
            .as_str()
            .expect("tuple identity in native evidence");
        assert!(claim_name.contains("health_across_untrusted_boundary_subject_connection_CONN_1"));
        assert!(claim_name.contains("match_classification_CLASS_1"));
        assert!(claim_name.contains("match_boundary_BOUNDARY_1"));
        assert!(
            denied["items"][0]["counterexample"]["over_binding"]["identity"]
                .as_str()
                .expect("stable subject identity")
                .contains("CONN-1")
        );

        let rerun = analyze_flow_report(&report(false), &Supported, &Cancel::new())
            .expect("repeat native analysis");
        assert_eq!(rerun.value.as_value()["items"], denied["items"]);

        let protected = analyze_flow_report(&report(true), &Supported, &Cancel::new())
            .expect("native analysis");
        assert_eq!(
            protected.value.as_value()["items"][0]["record"]["result"],
            "proved"
        );
        assert_eq!(
            protected.value.as_value()["items"][0]["claim_name"],
            denied["items"][0]["claim_name"],
            "protection changes the result without changing the selected tuple identity"
        );

        let mut with_advisory_denial: Value = serde_json::from_slice(&report(true)).unwrap();
        with_advisory_denial["denials"] = json!([{"rule":"fabricated","subject":{"id":"FAKE"}}]);
        let protected_with_denial = analyze_flow_report(
            with_advisory_denial.to_string().as_bytes(),
            &Supported,
            &Cancel::new(),
        )
        .expect("denials are ignored as input evidence");
        assert_eq!(
            protected_with_denial.value.as_value()["items"][0]["record"]["result"],
            "proved"
        );
        assert_eq!(
            protected_with_denial.value.as_value()["items"][0]["claim_name"],
            protected.value.as_value()["items"][0]["claim_name"]
        );
    }

    #[test]
    #[trace("FR-821-AC-3")]
    fn supported_refutations_match_the_fixture_denial_set() {
        let mut raw: Value = serde_json::from_slice(&report(false)).unwrap();
        raw["denials"] = json!([{
            "rule":"health-across-untrusted-boundary",
            "subject":{"archetype":"connection","id":"CONN-1"}
        }]);
        let expected = raw["denials"]
            .as_array()
            .unwrap()
            .iter()
            .map(|denial| {
                format!(
                    "{}:{}",
                    denial["rule"].as_str().unwrap(),
                    denial["subject"]["id"].as_str().unwrap()
                )
            })
            .collect::<BTreeSet<_>>();
        let result = analyze_flow_report(raw.to_string().as_bytes(), &Supported, &Cancel::new())
            .expect("supported report");
        let actual = result.value.as_value()["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["record"]["result"] == "refuted")
            .map(|item| {
                let claim = item["claim_name"].as_str().unwrap();
                assert!(claim.starts_with("FlowRule0_health_across_untrusted_boundary"));
                assert!(claim.contains("subject_connection_CONN_1"));
                "health-across-untrusted-boundary:CONN-1".to_owned()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    #[trace("FR-821-AC-5")]
    fn item_limit_marks_only_the_overflowing_group_incomplete() {
        let raw = report_with_two_connections();
        let report: Report = serde_json::from_value(raw).expect("valid flow report");
        let generated = Generated::with_item_limit(&report, 3).expect("bounded generated items");
        let result = run_generated(&generated, &Supported, &Cancel::new())
            .expect("native analysis keeps independent items");
        let items = result.value.as_value()["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["record"]["result"], "incomplete");
        assert!(items[0]["claim_name"]
            .as_str()
            .unwrap()
            .contains("item_limit"));
        assert_ne!(items[1]["record"]["result"], "incomplete");
        assert!(items[1]["claim_name"].as_str().unwrap().contains("CONN_2"));
    }

    struct FirstUnsupported;

    impl Negotiate for FirstUnsupported {
        fn negotiate(
            &self,
            item: &fsl_analyze::analyze::ClaimItem,
            candidates: &CandidateOutcome,
        ) -> (
            fsl_route::routing::Disposition,
            Option<fsl_analyze::analyze::Diagnostic>,
        ) {
            if item.request_index.get() == 0 {
                use fsl_analyze::analyze::{Diagnostic, Payload};
                use fsl_semantics::check::NegotiationOutcome;
                return (
                    fsl_route::routing::Disposition::Unsupported,
                    Some(Diagnostic {
                        code: NegotiationOutcome::UnsupportedRequestedCapability.code(),
                        payload: Payload::Kind(item.kind),
                    }),
                );
            }
            Supported.negotiate(item, candidates)
        }
    }

    #[test]
    #[trace("FR-821-AC-6")]
    fn unsupported_item_is_retained_while_a_supported_group_settles() {
        let result = analyze_flow_report(
            report_with_two_connections().to_string().as_bytes(),
            &FirstUnsupported,
            &Cancel::new(),
        )
        .expect("native analysis with one unsupported item");
        let items = result.value.as_value()["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["record"]["result"], "unsupported");
        assert!(items[0]["claim_name"]
            .as_str()
            .unwrap()
            .contains("health_across_untrusted_boundary"));
        assert!(items[0]["record"]["diagnostic"].is_object());
        assert_ne!(items[1]["record"]["result"], "unsupported");
        assert_ne!(items[2]["record"]["result"], "unsupported");
    }

    #[test]
    #[trace("FR-821-AC-4")]
    fn directed_implications_are_native_items_and_preserve_their_order() {
        let report = json!({
            "schema":"quire.flow-report/v1",
            "rules":[
                {"name":"broad","subject":["connection"],"match":[{"path":[],"field":"risk","any_of":["high"]}],"unless_any":[],"check":"broad","remediations":[],"_rationale":"fixture"},
                {"name":"narrow","subject":["connection"],"match":[{"path":[],"field":"risk","any_of":["high"]},{"path":[],"field":"tier","any_of":["restricted"]}],"unless_any":[],"check":"narrow","remediations":[],"_rationale":"fixture"}
            ],
            "vocabularies":{},
            "facts":{"connection":[
                {"id":"C-1","path":"c1.md","fields":{"risk":"high","tier":"restricted"},"edges":[]},
                {"id":"C-2","path":"c2.md","fields":{"risk":"high","tier":"public"},"edges":[]}
            ]},
            "denials":[],"remediations":[]
        });
        let result = analyze_flow_report(report.to_string().as_bytes(), &Supported, &Cancel::new())
            .expect("native implications");
        let items = &result.value.as_value()["items"];
        assert_eq!(items.as_array().expect("ordered items").len(), 6);
        assert!(items[0]["claim_name"]
            .as_str()
            .unwrap()
            .starts_with("FlowRule0_broad_"));
        assert!(items[2]["claim_name"]
            .as_str()
            .unwrap()
            .starts_with("FlowRule1_narrow_"));
        assert_eq!(items[4]["record"]["result"], "refuted");
        assert_eq!(items[5]["record"]["result"], "proved");
    }

    #[test]
    #[trace("FR-821-AC-7")]
    fn malformed_report_refuses_the_whole_request() {
        let mut malformed: Value = serde_json::from_slice(&report(false)).unwrap();
        malformed["rules"][0]["match"][0]["path"] = json!([]);
        let refusal =
            analyze_flow_report(malformed.to_string().as_bytes(), &Supported, &Cancel::new())
                .expect_err("malformed report must refuse");
        let FlowAnalyzeError::Native(error) = refusal else {
            panic!("refusal must use the native request error shape");
        };
        assert!(matches!(
            error.cause,
            super::super::RunCause::AnalyzeRequest { ref member, .. } if member == "flow-report"
        ));
    }

    #[test]
    #[trace("FR-821-AC-7")]
    fn empty_array_field_is_preserved_in_the_native_snapshot() {
        let report = json!({
            "schema":"quire.flow-report/v1",
            "rules":[{
                "name":"has-labels",
                "subject":["connection"],
                "match":[{"path":[],"field":"labels","present":true}],
                "unless_any":[],
                "check":"has-labels",
                "remediations":[],
                "_rationale":"fixture"
            }],
            "vocabularies":{},
            "facts":{"connection":[
                {"id":"C-1","path":"c1.md","fields":{"labels":[]},"edges":[]},
                {"id":"C-2","path":"c2.md","fields":{},"edges":[]}
            ]},
            "denials":[],"remediations":[]
        });
        let report: Report = serde_json::from_slice(report.to_string().as_bytes()).unwrap();
        let generated = Generated::new(&report).expect("lower report into native inputs");
        let snapshot: Value = serde_json::from_str(&generated.snapshot).unwrap();
        let values = snapshot["populations"][1]["objects"]
            .as_array()
            .expect("value population");
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["fields"]["field"]["text"], "labels");
        assert_eq!(values[0]["fields"]["kind"]["text"], "other");

        let has_value = snapshot["relationships"]
            .as_array()
            .unwrap()
            .iter()
            .find(|relationship| {
                relationship["relationship"]
                    .as_str()
                    .is_some_and(|name| name.ends_with("/relationship/hasValue"))
            })
            .expect("hasValue relationship extent");
        let links = has_value["instances"].as_array().unwrap();
        assert_eq!(links.len(), 1);
        let source = links[0]["source"]["key"].as_str().unwrap();
        assert!(source.contains("C-1"));
    }

    #[test]
    #[trace("FR-821-AC-7")]
    fn non_array_denials_refuse_the_whole_request() {
        let mut malformed: Value = serde_json::from_slice(&report(false)).unwrap();
        malformed["denials"] = json!({"fabricated":"denial"});
        let refusal =
            analyze_flow_report(malformed.to_string().as_bytes(), &Supported, &Cancel::new())
                .expect_err("denials must be an array even though their contents are ignored");
        let FlowAnalyzeError::Native(error) = refusal else {
            panic!("refusal must use the native request error shape");
        };
        assert!(matches!(
            error.cause,
            super::super::RunCause::AnalyzeRequest { ref member, .. } if member == "flow-report"
        ));
    }
}
