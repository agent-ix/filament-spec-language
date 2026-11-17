// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862, FSpec FR-336 "Replay request file": the one reader of the replay
//! request mapping. `command::run` reads a `native-replay/1` request's
//! `request` member with it, and the FR-350 corpus adapter reads a corpus
//! `replay-request` file with it, so a corpus vector's request and a user's
//! request are decoded by the same code into the same `ReplayRequestWire`.
//!
//! The mapping is closed: a `clocks` member (FSpec FR-715: replay reads its
//! clock configurations from the proving package), any other member outside
//! [`MEMBERS`], and a missing member are refused before the package is read.
//! Package, target and semantic admission stay with the replay request
//! reader (FR-071) and the replay (FR-098).

use std::collections::BTreeMap;

use fsl_foundation::node_wire::{node_key_to_json, node_key_to_text};
use fsl_foundation::SettingName;
use fsl_replay::{
    DeclaredDomain, DomainKey, DomainKind, FiniteBound, Integer, ProofBound, ReplayRequestWire,
    ReplaySource, RequestTargetWire, WitnessValue,
};
use serde_json::{json, Value};

use crate::unique_json::Deep;

/// FSpec FR-336's top-level replay request members, in the order a missing
/// member is reported.
pub const MEMBERS: [&str; 14] = [
    "profile_selections",
    "package",
    "dependencies",
    "replay_target",
    "source",
    "obligation_identity",
    "backend",
    "state_environment",
    "accounting_limits",
    "stage_limits",
    "supplied_limits",
    "object_environment",
    "declared_domains",
    "byte_provision",
];

/// A replay request member that does not decode by FSpec FR-336's mapping,
/// located by its JSON pointer into the request object.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message} at {pointer:?}")]
pub struct RequestDefect {
    pointer: String,
    message: String,
}

impl RequestDefect {
    fn new(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pointer: pointer.into(),
            message: message.into(),
        }
    }

    /// The JSON pointer of the member at fault; empty for the whole request.
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// Why the member is refused.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Read one replay request object `request` (the bytes of FSpec FR-336's
/// mapping) into the typed request FR-071 constructs from.
///
/// `provide` returns the bytes of one `byte_provision` row's `bytes_path`
/// (the second argument is that member's pointer); the caller owns where
/// the path resolves and the ceilings it is read under. `documents` are the
/// definition documents the caller supplies beside the request.
/// `input_bytes` bounds the package document's read.
///
/// # Errors
///
/// A [`RequestDefect`] (converted into `E`) for a member outside the
/// mapping, a missing member or a member value that does not decode, and
/// `provide`'s own error for a byte file it cannot supply.
pub fn read<E: From<RequestDefect>>(
    request: &[u8],
    input_bytes: u64,
    provide: impl FnMut(&str, &str) -> Result<Vec<u8>, E>,
    documents: Vec<Vec<u8>>,
) -> Result<ReplayRequestWire, E> {
    let members: BTreeMap<String, &serde_json::value::RawValue> =
        serde_json::from_slice(request)
            .map_err(|error| RequestDefect::new("", format!("expected a JSON object: {error}")))?;
    closed_members(members.keys().map(String::as_str))?;
    let raw_package = members
        .get("package")
        .ok_or_else(|| RequestDefect::new("/package", "missing member"))?;
    let package = fsl_replay::ReplayPackage::read(raw_package.get().as_bytes(), input_bytes)
        .map_err(|error| RequestDefect::new("/package", error.to_string()))?;
    let value = Deep::new(
        crate::unique_json::read(request).map_err(|message| RequestDefect::new("", message))?,
    );
    decode(&value, provide, package, documents)
}

/// Refuse a `clocks` member, any other member outside [`MEMBERS`], and then
/// the first missing member in [`MEMBERS`] order.
#[ix_trace_rs::string_edge]
fn closed_members<'a>(names: impl Iterator<Item = &'a str> + Clone) -> Result<(), RequestDefect> {
    if names.clone().any(|name| name == "clocks") {
        return Err(RequestDefect::new(
            "/clocks",
            "a replay request carries no clock configuration; replay reads the clocks the proving package records",
        ));
    }
    if let Some(unknown) = names.clone().find(|name| !MEMBERS.contains(name)) {
        return Err(RequestDefect::new(
            format!("/{unknown}"),
            "unknown replay request member",
        ));
    }
    if let Some(missing) = MEMBERS
        .iter()
        .find(|member| !names.clone().any(|name| name == **member))
    {
        return Err(RequestDefect::new(format!("/{missing}"), "missing member"));
    }
    Ok(())
}

fn unsigned(value: &Value, pointer: &str) -> Result<u64, RequestDefect> {
    let text = value
        .as_str()
        .ok_or_else(|| RequestDefect::new(pointer, "expected an unsigned decimal string"))?;
    let number = text
        .parse::<u64>()
        .map_err(|_| RequestDefect::new(pointer, "unsigned decimal is outside u64"))?;
    if number.to_string() != text {
        return Err(RequestDefect::new(pointer, "noncanonical unsigned decimal"));
    }
    Ok(number)
}

fn text<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, RequestDefect> {
    value
        .as_str()
        .ok_or_else(|| RequestDefect::new(pointer, "expected a string"))
}

fn array<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], RequestDefect> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| RequestDefect::new(pointer, "expected an array"))
}

fn object<'a>(
    value: &'a Value,
    pointer: &str,
) -> Result<&'a serde_json::Map<String, Value>, RequestDefect> {
    value
        .as_object()
        .ok_or_else(|| RequestDefect::new(pointer, "expected an object"))
}

fn node(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
) -> Result<fsl_replay::NodeKey, RequestDefect> {
    graph
        .key_of_value(value)
        .map_err(|message| RequestDefect::new(pointer, message))
}

fn source(value: &Value, pointer: &str) -> Result<(String, String), RequestDefect> {
    Ok((
        text(&value["authority"], &format!("{pointer}/authority"))?.to_owned(),
        text(&value["identity"], &format!("{pointer}/identity"))?.to_owned(),
    ))
}

fn sources(value: &Value, pointer: &str) -> Result<Vec<(String, String)>, RequestDefect> {
    array(value, pointer)?
        .iter()
        .enumerate()
        .map(|(index, value)| source(value, &format!("{pointer}/{index}")))
        .collect()
}

fn name(value: &Value, pointer: &str) -> Result<fsl_replay::QualifiedName, RequestDefect> {
    let segments = text(value, pointer)?
        .split('.')
        .map(|segment| {
            fsl_replay::Identifier::new(segment)
                .map_err(|_| RequestDefect::new(pointer, "invalid qualified name"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    fsl_replay::QualifiedName::new(segments)
        .map_err(|_| RequestDefect::new(pointer, "empty qualified name"))
}

fn optional<T>(
    value: &Value,
    read: impl FnOnce(&Value) -> Result<T, RequestDefect>,
) -> Result<Option<T>, RequestDefect> {
    if value.is_null() {
        Ok(None)
    } else {
        read(value).map(Some)
    }
}

/// Read the closed corpus request encoding into the public request carrier;
/// package, target and semantic admission remain with the native request reader.
#[ix_trace_rs::string_edge]
fn decode<E: From<RequestDefect>>(
    value: &Value,
    mut provide: impl FnMut(&str, &str) -> Result<Vec<u8>, E>,
    raw_package: fsl_replay::ReplayPackage,
    documents: Vec<Vec<u8>>,
) -> Result<ReplayRequestWire, E> {
    let package = &value["package"];
    let graph = fsl_replay::WireGraph::of_package_value(package)
        .map_err(|message| RequestDefect::new("/package", message))?;
    let target = &value["replay_target"];
    let members = &target["members"];
    let mut target_wire = RequestTargetWire {
        members: fsl_replay::ReplayTargetMembers {
            function: members
                .get("function")
                .map(|value| name(value, "/replay_target/members/function"))
                .transpose()?,
            function_owner: members
                .get("function_owner")
                .map(|value| {
                    fsl_replay::LibraryName::new(text(
                        value,
                        "/replay_target/members/function_owner",
                    )?)
                    .map_err(|_| {
                        RequestDefect::new(
                            "/replay_target/members/function_owner",
                            "invalid library identity",
                        )
                    })
                })
                .transpose()?,
            frame_operation: members
                .get("frame_operation")
                .map(|value| {
                    Ok(fsl_replay::FrameOperation {
                        object: name(
                            &value["object"],
                            "/replay_target/members/frame_operation/object",
                        )?,
                        operation: fsl_replay::Identifier::new(text(
                            &value["operation"],
                            "/replay_target/members/frame_operation/operation",
                        )?)
                        .map_err(|_| {
                            RequestDefect::new(
                                "/replay_target/members/frame_operation/operation",
                                "invalid identifier",
                            )
                        })?,
                    })
                })
                .transpose()?,
            state_clause: members
                .get("state_clause")
                .map(|value| name(value, "/replay_target/members/state_clause"))
                .transpose()?,
        },
        arguments: target["arguments"]
            .as_bool()
            .ok_or_else(|| RequestDefect::new("/replay_target/arguments", "expected a Boolean"))?,
        occurrence_key: optional(&target["occurrence_key"], |value| {
            Ok(fsl_replay::OccurrenceKey::new(
                node(
                    &value["node_id"],
                    "/replay_target/occurrence_key/node_id",
                    &graph,
                )?,
                fsl_replay::Origin::new(
                    fsl_replay::Role::new(text(
                        &value["role"],
                        "/replay_target/occurrence_key/role",
                    )?),
                    unsigned(&value["ordinal"], "/replay_target/occurrence_key/ordinal")?,
                ),
            ))
        })?,
        over: optional(&target["over"], |value| {
            Ok(fsl_replay::ReplayOverBinding {
                population: text(&value["population"], "/replay_target/over/population")?
                    .to_owned(),
                key: text(&value["key"], "/replay_target/over/key")?.to_owned(),
            })
        })?,
    };
    // The public request reader, rather than this adapter, decides missing or
    // conflicting target members and whether an occurrence is applicable.
    let source_value = &value["source"];
    let replay_source = match text(&source_value["kind"], "/source/kind")? {
        "witness" => ReplaySource::Witness(
            fsl_replay::Witness::parse(text(&source_value["transcript"], "/source/transcript")?)
                .map_err(|error| RequestDefect::new("/source/transcript", error.to_string()))?,
        ),
        "input" => ReplaySource::Input(
            array(&source_value["assignments"], "/source/assignments")?
                .iter()
                .enumerate()
                .map(|(index, assignment)| {
                    Ok(fsl_replay::CanonicalAssignment {
                        parameter: node(
                            &assignment["parameter"],
                            &format!("/source/assignments/{index}/parameter"),
                            &graph,
                        )?,
                        value: witness(
                            &assignment["value"],
                            &format!("/source/assignments/{index}/value"),
                            &graph,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, RequestDefect>>()?,
        ),
        _ => return Err(RequestDefect::new("/source/kind", "unknown replay source").into()),
    };
    let mut stage_limits = BTreeMap::new();
    for (setting, bound) in object(&value["stage_limits"], "/stage_limits")? {
        let pointer = format!("/stage_limits/{setting}");
        let bound = unsigned(bound, &pointer)?;
        fsl_replay::StageLimits::default()
            .with_named(setting, bound)
            .map_err(|error| RequestDefect::new(&pointer, error.to_string()))?;
        stage_limits.insert(setting.clone(), bound);
    }
    let mut accounting = fsl_replay::AccountingLimits::default();
    for (counter, bound) in object(&value["accounting_limits"], "/accounting_limits")? {
        let Some(SettingName::Accounting(setting)) =
            SettingName::from_name(&format!("accounting.{counter}"))
        else {
            return Err(RequestDefect::new(
                format!("/accounting_limits/{counter}"),
                "unknown counter",
            )
            .into());
        };
        accounting.set_bound(
            setting,
            unsigned(bound, &format!("/accounting_limits/{counter}"))?,
        );
    }
    let supplied_limits = optional(&value["supplied_limits"], |value| {
        let parsed = object(value, "/supplied_limits")?
            .iter()
            .map(|(name, value)| {
                Ok((
                    name.clone(),
                    json!(unsigned(value, &format!("/supplied_limits/{name}"))?),
                ))
            })
            .collect::<Result<serde_json::Map<_, _>, RequestDefect>>()?;
        fsl_replay::read_supplied_limits(&Value::Object(parsed))
            .map_err(|error| RequestDefect::new("/supplied_limits", error.to_string()))
    })?;
    let obligation = &value["obligation_identity"];
    let ordinal = obligation["ordinal"]
        .as_u64()
        .and_then(|ordinal| u32::try_from(ordinal).ok())
        .ok_or_else(|| RequestDefect::new("/obligation_identity/ordinal", "ordinal outside u32"))?;
    let mut byte_provision = Vec::new();
    for (index, provision) in array(&value["byte_provision"], "/byte_provision")?
        .iter()
        .enumerate()
    {
        let pointer = format!("/byte_provision/{index}/bytes_path");
        let path = text(&provision["bytes_path"], &pointer)?;
        let bytes = provide(path, &pointer)?;
        byte_provision.push((
            text(
                &provision["authority"],
                &format!("/byte_provision/{index}/authority"),
            )?
            .to_owned(),
            text(
                &provision["identity"],
                &format!("/byte_provision/{index}/identity"),
            )?
            .to_owned(),
            bytes,
        ));
    }
    let dependencies = array(&value["dependencies"], "/dependencies")?
        .iter()
        .enumerate()
        .map(|(index, dependency)| {
            Ok(fsl_replay::DependencyEntryWire {
                identity: text(
                    &dependency["identity"],
                    &format!("/dependencies/{index}/identity"),
                )?
                .to_owned(),
                sources: sources(
                    &dependency["sources"],
                    &format!("/dependencies/{index}/sources"),
                )?,
                refinement_internals: dependency
                    .get("refinement_internals")
                    .map(|rows| {
                        serde_json::from_value(rows.clone()).map_err(|error| {
                            RequestDefect::new(
                                format!("/dependencies/{index}/refinement_internals"),
                                error.to_string(),
                            )
                        })
                    })
                    .transpose()?,
            })
        })
        .collect::<Result<_, RequestDefect>>()?;
    let state_environment = array(&value["state_environment"], "/state_environment")?
        .iter()
        .enumerate()
        .map(|(index, pair)| {
            let pair = array(pair, &format!("/state_environment/{index}"))?;
            let [name, value] = pair else {
                return Err(RequestDefect::new(
                    format!("/state_environment/{index}"),
                    "expected a string pair",
                ));
            };
            Ok((
                text(name, "/state_environment")?.to_owned(),
                text(value, "/state_environment")?.to_owned(),
            ))
        })
        .collect::<Result<_, RequestDefect>>()?;
    Ok(ReplayRequestWire {
        package: raw_package,
        profile_selections: array(&value["profile_selections"], "/profile_selections")?
            .iter()
            .map(|selection| {
                Ok(fsl_replay::ProfileSelection::new(
                    text(&selection["profile"], "/profile_selections/profile")?.to_owned(),
                    text(&selection["value"], "/profile_selections/value")?.to_owned(),
                ))
            })
            .collect::<Result<_, RequestDefect>>()?,
        dependencies,
        replay_target: std::mem::take(&mut target_wire),
        source: replay_source,
        obligation_identity: fsl_replay::ObligationIdentity::new(
            node(
                &obligation["node_id"],
                "/obligation_identity/node_id",
                &graph,
            )?,
            optional(&obligation["model"], |value| {
                text(value, "/obligation_identity/model").map(fsl_replay::ModelIdentity::new)
            })?,
            ordinal,
        ),
        backend: text(&value["backend"], "/backend")?.to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(state_environment),
        accounting_limits: accounting.0,
        stage_limits,
        supplied_limits,
        object_environment: optional(&value["object_environment"], |value| {
            fsl_replay::read_object_environment(value)
                .map_err(|error| RequestDefect::new("/object_environment", error.to_string()))
        })?,
        declared_domains: array(&value["declared_domains"], "/declared_domains")?
            .iter()
            .enumerate()
            .map(|(index, domain)| {
                declared_domain(domain, &format!("/declared_domains/{index}"), &graph)
            })
            .collect::<Result<_, _>>()?,
        byte_provision,
        documents,
    })
}

/// Decode corpus domain/bound tags directly into native DomainKey, DomainKind
/// and FiniteBound; their owner constructors decide validity.
#[ix_trace_rs::string_edge]
fn declared_domain(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
) -> Result<DeclaredDomain, RequestDefect> {
    let key = &value["domain"];
    let integer32 = |value, pointer: &str| {
        u32::try_from(unsigned(value, pointer)?)
            .map_err(|_| RequestDefect::new(pointer, "value outside u32"))
    };
    let domain = match text(&key["kind"], pointer)? {
        "node" => DomainKey::Node {
            node: node(&key["node"], pointer, graph)?,
            path: array(&key["path"], pointer)?
                .iter()
                .map(|value| integer32(value, pointer))
                .collect::<Result<_, _>>()?,
        },
        "population" => DomainKey::Population {
            member_type: node(&key["member_type"], pointer, graph)?,
            ordinal: integer32(&key["ordinal"], pointer)?,
        },
        _ => return Err(RequestDefect::new(pointer, "unknown domain key")),
    };
    let kind = optional(&value["kind"], |kind| {
        DomainKind::from_wire(&text(&kind["kind"], pointer)?.replace('_', "-"))
            .ok_or_else(|| RequestDefect::new(pointer, "unknown domain kind"))
    })?;
    let bound = &value["bound"];
    let bound = match text(&bound["kind"], pointer)? {
        "cardinality" => FiniteBound::cardinality(unsigned(&bound["maximum"], pointer)?),
        "depth" => FiniteBound::depth(unsigned(&bound["maximum"], pointer)?)
            .map_err(|error| RequestDefect::new(pointer, error.to_string()))?,
        "integer_range" => FiniteBound::integer_range(
            integer(&bound["lower"], pointer)?,
            integer(&bound["upper"], pointer)?,
        )
        .map_err(|error| RequestDefect::new(pointer, error.to_string()))?,
        "variants" => FiniteBound::variants(
            array(&bound["members"], pointer)?
                .iter()
                .map(|member| text(member, pointer).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?,
        )
        .map_err(|error| RequestDefect::new(pointer, error.to_string()))?,
        _ => return Err(RequestDefect::new(pointer, "unknown finite bound")),
    };
    ProofBound::new(domain, kind, bound)
        .map(DeclaredDomain::new)
        .map_err(|error| RequestDefect::new(pointer, error.to_string()))
}

fn integer(value: &Value, pointer: &str) -> Result<Integer, RequestDefect> {
    text(value, pointer)?
        .parse()
        .map_err(|error| RequestDefect::new(pointer, format!("{error}")))
}

/// Transient corpus grammar tag. It carries no semantic value or identity;
/// the owner codec admits the completed native wire after iterative assembly.
#[derive(Clone, Copy, Eq, PartialEq)]
enum WitnessForm {
    Boolean,
    Integer,
    ExactInteger,
    Enum,
    Text,
    String,
    Uuid,
    Timestamp,
    Rational,
    Decimal,
    Float32,
    Float64,
    Reference,
    Quantity,
    Option,
    Record,
    Tuple,
    Union,
    Collection(filament_exact::CollectionKind),
}

/// Read the corpus witness discriminator once. Subsequent traversal and
/// encoding use this typed parser state, then the native owner codec.
#[ix_trace_rs::string_edge]
fn witness_form(value: &Value, pointer: &str) -> Result<WitnessForm, RequestDefect> {
    use filament_exact::CollectionKind;
    Ok(match text(&value["kind"], pointer)? {
        "boolean" => WitnessForm::Boolean,
        "integer" => WitnessForm::Integer,
        "exact_integer" => WitnessForm::ExactInteger,
        "enum" => WitnessForm::Enum,
        "text" => WitnessForm::Text,
        "string" => WitnessForm::String,
        "uuid" => WitnessForm::Uuid,
        "timestamp" => WitnessForm::Timestamp,
        "rational" => WitnessForm::Rational,
        "decimal" => WitnessForm::Decimal,
        "float32" => WitnessForm::Float32,
        "float64" => WitnessForm::Float64,
        "reference" => WitnessForm::Reference,
        "quantity" => WitnessForm::Quantity,
        "option" => WitnessForm::Option,
        "record" => WitnessForm::Record,
        "tuple" => WitnessForm::Tuple,
        "union" => WitnessForm::Union,
        "sequence" => WitnessForm::Collection(CollectionKind::Sequence),
        "ordered_set" => WitnessForm::Collection(CollectionKind::OrderedSet),
        "set" => WitnessForm::Collection(CollectionKind::Set),
        "bag" => WitnessForm::Collection(CollectionKind::Bag),
        _ => return Err(RequestDefect::new(pointer, "unknown witness value")),
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SlotForm {
    Present,
    Absent,
    Null,
}

/// Read a record slot's corpus tag once, retaining it until its child has
/// converted; semantic presence still belongs to the native witness reader.
#[ix_trace_rs::string_edge]
fn slot_form(value: &Value, pointer: &str) -> Result<SlotForm, RequestDefect> {
    Ok(match text(&value["kind"], pointer)? {
        "present" => SlotForm::Present,
        "absent" => SlotForm::Absent,
        "null" => SlotForm::Null,
        _ => return Err(RequestDefect::new(pointer, "unknown slot")),
    })
}

fn witness(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
) -> Result<WitnessValue, RequestDefect> {
    let form = witness_form(value, pointer)?;
    match form {
        WitnessForm::Boolean => {
            Ok(WitnessValue::Boolean(value["value"].as_bool().ok_or_else(
                || RequestDefect::new(pointer, "expected a Boolean"),
            )?))
        }
        WitnessForm::Integer => Ok(WitnessValue::Integer(
            text(&value["decimal"], pointer)?
                .parse()
                .map_err(|_| RequestDefect::new(pointer, "integer outside i128"))?,
        )),
        WitnessForm::ExactInteger => Ok(WitnessValue::ExactInteger(integer(
            &value["decimal"],
            pointer,
        )?)),
        _ => {
            let typed = witness_json(value, pointer, graph, form)?;
            let bytes = quire_canonical::to_vec(&*typed, quire_canonical::Limits::new(u64::MAX))
                .map_err(|error| RequestDefect::new(pointer, error.to_string()))?;
            WitnessValue::from_wire_bytes(&bytes)
                .map_err(|error| RequestDefect::new(pointer, format!("{error:?}")))
        }
    }
}

fn node_text(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
) -> Result<String, RequestDefect> {
    node_key_to_text(&node(value, pointer, graph)?)
        .map_err(|error| RequestDefect::new(pointer, error.to_string()))
}

fn wire_object<const N: usize>(members: [(&str, Value); N]) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn witness_json(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
    form: WitnessForm,
) -> Result<Deep, RequestDefect> {
    enum Task<'a> {
        Read(&'a Value),
        Parsed(&'a Value, WitnessForm),
        Finish(&'a Value, usize, Option<String>, WitnessForm, Vec<SlotForm>),
    }
    let mut tasks = vec![Task::Parsed(value, form)];
    let mut values = Vec::<Deep>::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Read(value) => {
                tasks.push(Task::Parsed(value, witness_form(value, pointer)?));
                continue;
            }
            Task::Parsed(value, form) => {
                let mut slots = Vec::new();
                // Tuple/union identity was read before their components by
                // the owner adapter. Preserve that refusal order while
                // moving the recursive walk onto the explicit stack.
                let declaration = if matches!(form, WitnessForm::Tuple | WitnessForm::Union) {
                    Some(node_text(&value["declaration"], pointer, graph)?)
                } else {
                    None
                };
                let children = match form {
                    WitnessForm::Quantity => vec![&value["magnitude"]],
                    WitnessForm::Option => {
                        if value["value"].is_null() {
                            Vec::new()
                        } else {
                            vec![&value["value"]]
                        }
                    }
                    WitnessForm::Record => {
                        let mut children = Vec::new();
                        for field in array(&value["fields"], pointer)? {
                            let slot = &field["slot"];
                            let form = slot_form(slot, pointer)?;
                            if form == SlotForm::Present {
                                children.push(&slot["value"]);
                            }
                            slots.push(form);
                        }
                        children
                    }
                    WitnessForm::Tuple | WitnessForm::Union => {
                        array(&value["components"], pointer)?.iter().collect()
                    }
                    WitnessForm::Collection(_) => {
                        array(&value["elements"], pointer)?.iter().collect()
                    }
                    _ => {
                        values.push(Deep::new(witness_leaf(value, pointer, graph, form)?));
                        continue;
                    }
                };
                tasks.push(Task::Finish(
                    value,
                    children.len(),
                    declaration,
                    form,
                    slots,
                ));
                tasks.extend(children.into_iter().rev().map(Task::Read));
            }
            Task::Finish(value, count, declaration, form, slots) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| RequestDefect::new(pointer, "missing witness children"))?;
                let mut children = values.split_off(start).into_iter();
                let next = |children: &mut std::vec::IntoIter<Deep>| {
                    children
                        .next()
                        .map(Deep::into_inner)
                        .ok_or_else(|| RequestDefect::new(pointer, "missing witness child"))
                };
                let converted = match form {
                    WitnessForm::Quantity => {
                        let mut magnitude = Deep::new(next(&mut children)?);
                        let unit = format!(
                            "{{\"declared\":{}}}",
                            node_text(&value["unit"], pointer, graph)?
                        );
                        magnitude
                            .as_object_mut()
                            .ok_or_else(|| RequestDefect::new(pointer, "invalid magnitude"))?
                            .insert("unit".to_owned(), Value::String(unit));
                        magnitude.into_inner()
                    }
                    WitnessForm::Option => wire_object([
                        ("type", json!("option")),
                        (
                            "value",
                            if count == 0 {
                                Value::Null
                            } else {
                                next(&mut children)?
                            },
                        ),
                    ]),
                    WitnessForm::Record => {
                        let name = node_text(&value["declaration"], pointer, graph)?;
                        let mut fields = Vec::new();
                        for (field, slot) in array(&value["fields"], pointer)?.iter().zip(slots) {
                            let converted = match slot {
                                SlotForm::Present => next(&mut children)?,
                                SlotForm::Absent => Value::Null,
                                SlotForm::Null => json!({"type":"null"}),
                            };
                            fields.push(Deep::new(wire_object([
                                ("name", field["name"].clone()),
                                ("value", converted),
                            ])));
                        }
                        wire_object([
                            ("type", json!("record")),
                            ("name", Value::String(name)),
                            (
                                "fields",
                                Value::Array(fields.into_iter().map(Deep::into_inner).collect()),
                            ),
                        ])
                    }
                    WitnessForm::Tuple | WitnessForm::Union => {
                        let name = declaration.ok_or_else(|| {
                            RequestDefect::new(pointer, "missing witness declaration")
                        })?;
                        let components = Value::Array(children.map(Deep::into_inner).collect());
                        let mut members = serde_json::Map::new();
                        members.insert(
                            "type".into(),
                            json!(if form == WitnessForm::Union {
                                "union"
                            } else {
                                "tuple"
                            }),
                        );
                        members.insert("name".into(), Value::String(name));
                        if form == WitnessForm::Union {
                            members.insert("member".into(), value["member"].clone());
                        }
                        members.insert("components".into(), components);
                        Value::Object(members)
                    }
                    WitnessForm::Collection(kind) => wire_object([
                        (
                            "type",
                            json!(match kind {
                                filament_exact::CollectionKind::Sequence => "sequence",
                                filament_exact::CollectionKind::OrderedSet => "ordered-set",
                                filament_exact::CollectionKind::Set => "set",
                                filament_exact::CollectionKind::Bag => "bag",
                            }),
                        ),
                        (
                            "elements",
                            Value::Array(children.map(Deep::into_inner).collect()),
                        ),
                    ]),
                    _ => {
                        return Err(RequestDefect::new(
                            pointer,
                            "unknown composite witness value",
                        ))
                    }
                };
                values.push(Deep::new(converted));
            }
        }
    }
    values
        .pop()
        .ok_or_else(|| RequestDefect::new(pointer, "missing witness value"))
}

fn witness_leaf(
    value: &Value,
    pointer: &str,
    graph: &fsl_replay::WireGraph,
    form: WitnessForm,
) -> Result<Value, RequestDefect> {
    Ok(match form {
        WitnessForm::Boolean => json!({"type":"boolean", "value":value["value"]}),
        WitnessForm::Integer | WitnessForm::ExactInteger => {
            json!({"type":"integer", "value":value["decimal"]})
        }
        WitnessForm::Enum => {
            json!({"type":"enum", "name":node_text(&value["declaration"], pointer, graph)?, "member":value["member"]})
        }
        WitnessForm::Text => json!({"type":"text", "value":value["value"]}),
        WitnessForm::String | WitnessForm::Uuid => {
            json!({"kind":if form == WitnessForm::String { "string" } else { "uuid" },"text":value["value"]})
        }
        WitnessForm::Timestamp => json!({"kind":"timestamp","nanoseconds":value["decimal"]}),
        WitnessForm::Rational => {
            json!({"type":"rational","numerator":value["numerator"],"denominator":value["denominator"]})
        }
        WitnessForm::Decimal => {
            json!({"type":"decimal","coefficient":value["coefficient"],"scale":value["scale"]})
        }
        WitnessForm::Float32 => {
            json!({"type":"float32", "bits":format!("{:08x}", u32::try_from(unsigned(&value["bits"], pointer)?).map_err(|_| RequestDefect::new(pointer,"float bits outside u32"))?)})
        }
        WitnessForm::Float64 => {
            json!({"type":"float64", "bits":format!("{:016x}", unsigned(&value["bits"], pointer)?)})
        }
        WitnessForm::Reference => {
            let type_node = node(&value["object_type"], pointer, graph)?;
            let path = Deep::new(
                node_key_to_json(&type_node)
                    .map_err(|error| RequestDefect::new(pointer, error.to_string()))?,
            );
            let original = path["path"]
                .as_array()
                .and_then(|path| path.last())
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    RequestDefect::new(pointer, "reference type is not a declared type")
                })?;
            json!({"type":"reference", "universe":node_text(&value["universe"], pointer, graph)?, "object_type":serde_json::to_string(&json!([{"package":type_node.package(),"node":original}])).map_err(|error| RequestDefect::new(pointer,error.to_string()))?, "identity":value["identity"]})
        }
        _ => return Err(RequestDefect::new(pointer, "unknown witness value")),
    })
}
mod tests {
    use ix_trace_rs::trace;

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn typed_wire_states_retain_slots_collections_and_declaration_refusal_order() {
        use fsl_replay::{WitnessSlot, WitnessValue};
        use serde_json::json;
        // A structural codec fixture, not a claim about a checked declaration.
        // The executor still owns declaration-kind and value-type admission.
        let key = filament_exact::NodeKey::new(
            "fixture",
            vec![filament_exact::PathSegment::Text("shape".into())],
            0,
        )
        .unwrap();
        let id = fsl_semantics::check::wire_id::structural_id(&key);
        let graph = fsl_replay::WireGraph::new([&id]).unwrap();
        let declaration = graph.wire_value(&key).unwrap();
        let record = json!({"kind":"record", "declaration":declaration, "fields":[
            {"name":"held", "slot":{"kind":"present", "value":{"kind":"boolean", "value":true}}},
            {"name":"omitted", "slot":{"kind":"absent"}},
            {"name":"nullish", "slot":{"kind":"null"}}
        ]});
        let native = super::witness(&record, "/value", &graph)
            .unwrap_or_else(|failure| panic!("{}", failure.message));
        let WitnessValue::Record {
            declaration: actual,
            fields,
        } = &native
        else {
            panic!("record form is retained")
        };
        assert_eq!(actual, &key);
        assert_eq!(
            fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["held", "omitted", "nullish"]
        );
        assert!(matches!(
            fields[0].slot,
            WitnessSlot::Present(WitnessValue::Boolean(true))
        ));
        assert!(matches!(fields[1].slot, WitnessSlot::Absent));
        assert!(matches!(fields[2].slot, WitnessSlot::Null));
        for (tag, expected) in [
            (
                "sequence",
                WitnessValue::Sequence(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "ordered_set",
                WitnessValue::OrderedSet(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "set",
                WitnessValue::Set(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "bag",
                WitnessValue::Bag(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
        ] {
            let value = json!({"kind":tag,"elements":[{"kind":"boolean","value":false},{"kind":"boolean","value":true}]});
            let actual = super::witness(&value, "/value", &graph)
                .unwrap_or_else(|failure| panic!("{}", failure.message));
            assert!(
                actual == expected,
                "the native collection form and order survive"
            );
        }
        let unknown_declaration = serde_json::Value::Null;
        let expected = super::node(&unknown_declaration, "/value", &graph)
            .err()
            .unwrap();
        for value in [
            json!({"kind":"tuple", "declaration":unknown_declaration, "components":[{"kind":"unknown"}]}),
            json!({"kind":"union", "declaration":unknown_declaration, "member":"Arm", "components":[{"kind":"unknown"}]}),
        ] {
            let refused = super::witness(&value, "/value", &graph).err().unwrap();
            assert_eq!(
                refused.message, expected.message,
                "tuple and union declarations still refuse before child intake"
            );
        }
        let mut record = record;
        record["declaration"] = unknown_declaration;
        record["fields"][0]["slot"]["value"] = json!({"kind":"unknown"});
        let refused = super::witness(&record, "/value", &graph).err().unwrap();
        assert_eq!(
            refused.message, "unknown witness value",
            "record declaration remains after its children"
        );
    }

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn a_deep_corpus_witness_converts_and_releases_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let depth = 1_000;
                let mut bytes = br#"{"kind":"sequence","elements":["#.repeat(depth);
                bytes
                    .extend_from_slice(br#"{"kind":"exact_integer","decimal":"9007199254740993"}"#);
                bytes.extend_from_slice(&b"]}".repeat(depth));
                let value = super::Deep::new(
                    crate::unique_json::read(&bytes)
                        .expect("the byte-bounded witness reads past 128 levels"),
                );
                let graph = fsl_replay::WireGraph::new([]).expect("an unused, empty key lookup");
                let witness = super::witness(&value, "/source/assignments/0/value", &graph)
                    .unwrap_or_else(|_| panic!("the owner admits the complete typed witness"));
                let mut current = &witness;
                for _ in 0..depth {
                    let fsl_replay::WitnessValue::Sequence(elements) = current else {
                        panic!("every sequence layer is retained");
                    };
                    assert_eq!(elements.len(), 1);
                    current = &elements[0];
                }
                let fsl_replay::WitnessValue::ExactInteger(value) = current else {
                    panic!("the leaf remains an exact integer");
                };
                assert_eq!(value.to_string(), "9007199254740993");
                // Both the corpus JSON guard and the native witness release
                // their complete deep trees on this same small stack.
                drop(witness);
            })
            .expect("spawn the bounded-stack qualification control")
            .join()
            .expect("deep witness conversion and release do not overflow");
    }
}
