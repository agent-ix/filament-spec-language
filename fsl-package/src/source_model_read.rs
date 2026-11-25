// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740 "Source objects": the source packages of a checked package's
//! source-object identity rows.
//!
//! A `probabilistic` node names the object type and operation it declares on
//! (FR-569). S3 joins it to the source package it built from the unit's
//! source text; the bytes do not carry that package, so a reader rebuilds one
//! per declaring unit from the rows: [`BarePackage::source_model`].
//!
//! A row carries the type's name, supertypes, fields, identity fields and
//! populations, and for each operation its name, whether it is a query and its
//! parameters (a data entry with a type node, or an entry that renders the
//! parameter's type); it carries no effect frame or result. The rebuilt
//! package holds the types, supertypes, operations and parameters, which is
//! what the probabilistic join reads, and nothing else: a consumer that steps
//! the model or reads its fields and populations must not use it.

use std::collections::BTreeMap;

use filament_contract_model::{CheckedNodeId, CheckedNodeOwner};
use filament_exact::{EffectiveId, Integer, IntegerInterval, ValueType};
use fsl_semantics::check::{
    object_types_of_source_rows, package_of_source_rows, source_identity, source_type_node_key,
    SourceRowObject, SourceRowOperation, SourceRowParameter,
};
use fsl_semantics::model::domain_package::DomainPackage;

use crate::BarePackage;

/// A package's source-object rows do not describe a source model.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourceModelFault {
    /// A row's direct supertype is the node of no source-object row. The
    /// identity rows' own read joins every supertype to a row, so rows read
    /// from a package do not give this fault.
    #[error("the supertype of `{object}` is no source object type of the package")]
    UnknownSupertype {
        /// The row's type name.
        object: String,
        /// The row's node.
        node: CheckedNodeId,
    },
    /// A row's operation identity is not `<type>::<operation>`.
    #[error("the operation identity `{identity}` is not `<type>::<operation>`")]
    MalformedOperation {
        /// The operation identity the row states.
        identity: String,
        /// The row's node.
        node: CheckedNodeId,
    },
    /// A row's node is owned by no source unit the package locks. The rows'
    /// own read joins each row to a node of the package, so rows read from a
    /// package do not give this fault.
    #[error("the source object `{object}` is owned by no locked source unit")]
    UnownedObject {
        /// The row's type name.
        object: String,
        /// The row's node.
        node: CheckedNodeId,
    },
    /// A `probabilistic` node names an operation one of whose parameter types
    /// the row does not let a reader rebuild.
    #[error("the type of parameter `{parameter}` of `{operation}` is not readable from its row")]
    UnreadParameter {
        /// The operation's source identity, `<type>::<operation>`.
        operation: String,
        /// The parameter's name.
        parameter: String,
        /// The row's node.
        node: CheckedNodeId,
    },
}

impl SourceModelFault {
    /// The source-object row's node the fault is at.
    pub fn node(&self) -> &CheckedNodeId {
        match self {
            Self::UnknownSupertype { node, .. }
            | Self::MalformedOperation { node, .. }
            | Self::UnownedObject { node, .. }
            | Self::UnreadParameter { node, .. } => node,
        }
    }
}

/// An operation parameter whose type a row spells in a form a reader does not
/// rebuild, so the operation is left out of the source package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnreadSourceParameter {
    /// The declaring object type's name.
    pub object: String,
    /// The declaring object type's node key, as a `probabilistic` node names it.
    pub declaring: filament_exact::NodeKey,
    /// The operation's name.
    pub operation: String,
    /// The parameter's name.
    pub parameter: String,
    /// The declaring row's node.
    pub node: CheckedNodeId,
}

/// The source packages a checked package's source-object rows describe.
#[derive(Clone, Debug, Default)]
pub struct SourceModel {
    /// One source package per declaring unit, in the order S3 builds them
    /// ([`source_identity`]).
    pub packages: Vec<DomainPackage>,
    /// The parameters of operations left out of those packages.
    pub unread: Vec<UnreadSourceParameter>,
}

/// The rows of one declaring unit, with the identity S3 gives its source
/// package.
pub(crate) struct UnitRows {
    pub(crate) identity: String,
    pub(crate) objects: Vec<SourceRowObject>,
    /// The position in the supply's source-object rows of each object, in
    /// `objects` order.
    pub(crate) rows: Vec<usize>,
    pub(crate) nodes: Vec<CheckedNodeId>,
}

impl BarePackage<'_> {
    /// The source objects the supply's identity rows describe, by name and
    /// direct supertypes, grouped by declaring unit in S3's order, each group
    /// under the identity S3 gives that unit's source package. Empty when the
    /// package declares none or holds no identity rows. No operation is read.
    pub(crate) fn source_units(&self) -> Result<Vec<UnitRows>, SourceModelFault> {
        let Ok((supply, _)) = self.witness() else {
            return Ok(Vec::new());
        };
        let rows = supply.identities.source_objects();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let package = self.package();
        let owners: BTreeMap<&CheckedNodeId, &CheckedNodeOwner> = package
            .resolved_graph()
            .nodes
            .iter()
            .filter_map(|node| Some((&node.node_id, node.owner.as_ref()?)))
            .collect();
        let sources = &package.lock().sources;
        let names: BTreeMap<&CheckedNodeId, &str> = rows
            .iter()
            .map(|row| (&row.structural_node, row.source_identity.as_ref()))
            .collect();
        // S3 numbers the units that declare an object, in the order of the
        // package's sources.
        let mut by_unit: BTreeMap<usize, UnitRows> = BTreeMap::new();
        for (index, row) in rows.iter().enumerate() {
            let unowned = || SourceModelFault::UnownedObject {
                object: row.source_identity.to_string(),
                node: row.structural_node.clone(),
            };
            let Some(CheckedNodeOwner::Source {
                authority,
                identity,
            }) = owners.get(&row.structural_node).copied()
            else {
                return Err(unowned());
            };
            let position = sources
                .iter()
                .position(|source| source.authority == *authority && source.identity == *identity)
                .ok_or_else(unowned)?;
            let supertypes = row
                .supertypes
                .iter()
                .map(|node| {
                    names
                        .get(node)
                        .map(|name| (*name).to_owned())
                        .ok_or_else(|| SourceModelFault::UnknownSupertype {
                            object: row.source_identity.to_string(),
                            node: row.structural_node.clone(),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let unit = by_unit.entry(position).or_insert_with(|| UnitRows {
                identity: String::new(),
                objects: Vec::new(),
                rows: Vec::new(),
                nodes: Vec::new(),
            });
            unit.objects.push(SourceRowObject {
                name: row.source_identity.to_string(),
                abstract_type: row.is_abstract,
                supertypes,
                operations: Vec::new(),
            });
            unit.rows.push(index);
            unit.nodes.push(row.structural_node.clone());
        }
        Ok(by_unit
            .into_values()
            .enumerate()
            .map(|(position, mut unit)| {
                unit.identity = source_identity(package.name(), position);
                unit
            })
            .collect())
    }

    /// The admitted source-object node to the effective type identity the
    /// source route builds for its declaring unit.
    pub(crate) fn source_object_effective_ids(
        &self,
    ) -> Result<BTreeMap<CheckedNodeId, EffectiveId>, SourceModelFault> {
        let units = self.source_units()?;
        if self.witness().is_err() {
            return Ok(BTreeMap::new());
        }
        let mut identities = BTreeMap::new();
        for unit in units {
            for (object, node) in unit.objects.iter().zip(unit.nodes) {
                identities.insert(
                    node,
                    source_object_effective_id(&unit.identity, &object.name),
                );
            }
        }
        Ok(identities)
    }

    /// The object types of the source units as declarations of a type
    /// environment, keyed under the same source-unit identities as source
    /// evaluation.
    pub(crate) fn source_object_declarations(
        &self,
    ) -> Result<Vec<filament_semantic_value::declaration::ObjectTypeDeclaration>, SourceModelFault>
    {
        Ok(self
            .source_units()?
            .iter()
            .flat_map(|unit| object_types_of_source_rows(&unit.identity, &unit.objects))
            .collect())
    }

    /// FSL FR-740 "Source objects": the source packages the package's
    /// source-object identity rows describe, one per declaring unit under
    /// S3's identity for it, which the reader joins its `probabilistic` nodes
    /// to as S3 joins them to the packages it built.
    ///
    /// # Errors
    ///
    /// A [`SourceModelFault`] when the rows do not describe such packages.
    pub fn source_model(&self) -> Result<SourceModel, SourceModelFault> {
        let units = self.source_units()?;
        let Ok((supply, _)) = self.witness() else {
            return Ok(SourceModel::default());
        };
        let rows = supply.identities.source_objects();
        let mut model = SourceModel::default();
        for mut unit in units {
            let unit_rows = unit.rows.iter().map(|&index| &rows[index]);
            for (object, row) in unit.objects.iter_mut().zip(unit_rows) {
                let node = &row.structural_node;
                for operation in &row.operations {
                    let name = operation
                        .identity
                        .split_once("::")
                        .map(|(_, name)| name)
                        .ok_or_else(|| SourceModelFault::MalformedOperation {
                            identity: operation.identity.to_string(),
                            node: node.clone(),
                        })?;
                    let mut parameters = Vec::with_capacity(operation.parameters.len());
                    let mut readable = true;
                    for parameter in &operation.parameters {
                        let value_type = match &parameter.type_node {
                            None if operation.clock_parameters.contains(&parameter.name) => {
                                Some(None)
                            }
                            None => parameter
                                .member_type
                                .as_deref()
                                .and_then(|rendering| {
                                    rendered_scalar(rendering, &parameter.collections)
                                })
                                .map(Some),
                            Some(node) => self
                                .exact_type_projections()
                                .get(node)
                                .and_then(|projection| projection.as_ref().ok())
                                .map(|projection| Some(projection.value_type().clone())),
                        };
                        match value_type.zip(multiplicity_of(&parameter.collections)) {
                            Some((value_type, multiplicity)) => {
                                parameters.push(SourceRowParameter {
                                    name: parameter.name.to_string(),
                                    optional: parameter.optional,
                                    multiplicity,
                                    value_type,
                                })
                            }
                            None => {
                                readable = false;
                                model.unread.push(UnreadSourceParameter {
                                    object: object.name.clone(),
                                    declaring: source_type_node_key(
                                        self.package().name(),
                                        &object.name,
                                    )
                                    .ok_or_else(|| SourceModelFault::UnownedObject {
                                        object: object.name.clone(),
                                        node: node.clone(),
                                    })?,
                                    operation: name.to_owned(),
                                    parameter: parameter.name.to_string(),
                                    node: node.clone(),
                                });
                            }
                        }
                    }
                    if readable {
                        object.operations.push(SourceRowOperation {
                            name: name.to_owned(),
                            parameters,
                        });
                    }
                }
            }
            model
                .packages
                .push(package_of_source_rows(&unit.identity, &unit.objects));
        }
        Ok(model)
    }
}

/// The original checked declaration identity of a source object admitted by
/// its source-object row. The synthetic `source/<package>` identity used for
/// rebuilding the source model is a separate intake identity.
pub(crate) fn source_object_effective_id(source_identity: &str, name: &str) -> EffectiveId {
    let original = filament_checked::key::DeclarationKey {
        package: source_identity.to_owned(),
        node: format!("ix://{source_identity}/{name}"),
    };
    fsl_semantics::model::key::effective_id_of(None, &original)
}

/// The value type a source operation's parameter row renders, for the types
/// a reader rebuilds from a rendering: `Boolean`, `Integer`, `Int[lower,
/// upper]`, `Text[min, max; profile]`, `Decimal[lower, upper; min scale, max
/// scale; rounding]` and the unparameterized native types. The rendering is
/// that of the whole parameter type (FR-531, FR-789): a parameter with one
/// collection layer is wrapped in the layer's kind (`Set<Int[1, 3]>`), which
/// this unwraps to the element type. `None` for any other rendering, which
/// the row does not let a reader rebuild.
fn rendered_scalar(
    rendering: &str,
    layers: &[filament_contract_model::SelectedClockCollectionLayer],
) -> Option<ValueType> {
    use filament_exact::{DecimalType, RoundingMode, TextProfile, TextType};
    let rendering = rendering.strip_prefix("resolved:")?;
    let rendering = match layers {
        [] => rendering,
        [layer] => rendering
            .strip_prefix(collection_wrapper(&layer.kind)?)?
            .strip_prefix('<')?
            .strip_suffix('>')?,
        _ => return None,
    };
    let bounds = |text: &str| -> Option<(String, String, Option<String>)> {
        let inner = text.strip_suffix(']')?;
        let (first, rest) = inner.split_once(", ")?;
        match rest.split_once("; ") {
            Some((second, tail)) => {
                Some((first.to_owned(), second.to_owned(), Some(tail.to_owned())))
            }
            None => Some((first.to_owned(), rest.to_owned(), None)),
        }
    };
    let integer = |text: &str| text.parse::<Integer>().ok();
    match rendering {
        "Boolean" => Some(ValueType::Boolean),
        "Integer" => Some(ValueType::Integer),
        "String" => Some(ValueType::String),
        "UUID" => Some(ValueType::Uuid),
        "Timestamp" => Some(ValueType::Timestamp),
        "Bytes" => Some(ValueType::Bytes),
        "JsonObject" => Some(ValueType::JsonObject),
        other => {
            if let Some(rest) = other.strip_prefix("Int[") {
                let (lower, upper, tail) = bounds(rest)?;
                tail.is_none().then_some(())?;
                IntegerInterval::new(integer(&lower)?, integer(&upper)?)
                    .ok()
                    .map(ValueType::Int)
            } else if let Some(rest) = other.strip_prefix("Text[") {
                let (min, max, profile) = bounds(rest)?;
                let profile = match profile?.as_str() {
                    "nfc" => TextProfile::Nfc,
                    "nfd" => TextProfile::Nfd,
                    "nfkc" => TextProfile::Nfkc,
                    "nfkd" => TextProfile::Nfkd,
                    "unicode-scalars" => TextProfile::UnicodeScalars,
                    "binary-utf8" => TextProfile::BinaryUtf8,
                    _ => return None,
                };
                TextType::new(min.parse().ok()?, max.parse().ok()?, profile)
                    .ok()
                    .map(ValueType::Text)
            } else if let Some(rest) = other.strip_prefix("Decimal[") {
                let (lower, upper, tail) = bounds(rest)?;
                let tail = tail?;
                let (scales, rounding) = tail.split_once("; ")?;
                let (min_scale, max_scale) = scales.split_once(", ")?;
                DecimalType::new(
                    integer(&lower)?,
                    integer(&upper)?,
                    min_scale.parse().ok()?,
                    max_scale.parse().ok()?,
                    RoundingMode::from_code(rounding)?,
                )
                .ok()
                .map(ValueType::Decimal)
            } else {
                None
            }
        }
    }
}

impl SourceModel {
    /// The first parameter left out of the source packages whose operation a
    /// `probabilistic` node of `nodes` names: such a node cannot be rebuilt.
    pub fn unread_named(
        &self,
        nodes: &fsl_semantics::check::PackageProbabilistic,
    ) -> Option<&UnreadSourceParameter> {
        self.unread.iter().find(|unread| {
            let names = |declaring: &filament_exact::NodeKey, operation: &str| {
                *declaring == unread.declaring && operation == unread.operation
            };
            nodes
                .random_parameters
                .iter()
                .any(|read| names(&read.declaring, &read.operation))
                || nodes
                    .delays
                    .iter()
                    .any(|read| names(&read.declaring, &read.operation))
                || nodes
                    .rewards
                    .iter()
                    .any(|read| names(&read.declaring, &read.operation))
                || nodes.workloads.iter().any(|read| {
                    read.weights
                        .iter()
                        .any(|weight| names(&weight.declaring, &weight.operation))
                })
        })
    }
}

/// The FR-789 wrapper a collection layer of `kind` renders as.
pub(crate) fn collection_wrapper(kind: &str) -> Option<&'static str> {
    match kind {
        "sequence" => Some("Sequence"),
        "set" => Some("Set"),
        "ordered_set" => Some("OrderedSet"),
        "bag" => Some("Bag"),
        _ => None,
    }
}

/// The multiplicity a parameter row's collection layers spell: one value for
/// no layer, the layer's kind and bounds for one, and `None` for the nested
/// collections and kinds a reader does not rebuild.
fn multiplicity_of(
    layers: &[filament_contract_model::SelectedClockCollectionLayer],
) -> Option<fsl_semantics::model::domain_package::Multiplicity> {
    use fsl_semantics::model::domain_package::Multiplicity;
    let layer = match layers {
        [] => {
            return Some(Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            });
        }
        [layer] => layer,
        _ => return None,
    };
    let (ordered, unique) = match &*layer.kind {
        "sequence" => (true, false),
        "set" => (false, true),
        "ordered_set" => (true, true),
        "bag" => (false, false),
        _ => return None,
    };
    let upper = match &*layer.maximum {
        "unbounded" => None,
        maximum => Some(maximum.parse::<u64>().ok()?),
    };
    Some(Multiplicity {
        lower: layer.minimum.parse().ok()?,
        upper,
        ordered,
        unique,
    })
}
