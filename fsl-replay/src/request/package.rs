// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete raw checked-package input retained until native admission.

use super::{PackageMemberCause, ReplayRequestRefusal};
use crate::identity::SourceRefWire;
use filament_contract_model::CheckedRefinementInternal;
use filament_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageIncomplete, CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageV2,
};
use quire_canonical::{Document, Node, NodeRef, ReadError};

/// A complete raw checked-package document, without an admission claim.
///
/// The shared JSON document has flat ownership. Original bytes are retained
/// unchanged so native admission still sees noncanonical numbers and spellings.
#[derive(Clone)]
pub struct ReplayPackage {
    document: Document,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for ReplayPackage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReplayPackage")
            .field("input_bytes", &self.bytes.len())
            .finish()
    }
}

impl PartialEq for ReplayPackage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for ReplayPackage {}

impl ReplayPackage {
    /// Read one complete raw JSON document under the caller's byte ceiling.
    /// This checks JSON syntax only, not the checked-package contract.
    pub fn read(bytes: &[u8], max_input_bytes: u64) -> Result<Self, ReadError> {
        let document = quire_canonical::read(bytes, max_input_bytes)?;
        let mut retained = Vec::new();
        retained
            .try_reserve(bytes.len())
            .map_err(|_| ReadError::Allocation {
                requested: bytes.len(),
            })?;
        retained.extend_from_slice(bytes);
        Ok(Self {
            document,
            bytes: retained,
        })
    }

    /// The entire original document, including every graph and sidecar member.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The original bytes passed to the native checked-package reader.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Admit the complete original document using caller-supplied genuine
    /// evidence and native limits. No partial admitted package is returned.
    pub fn admit(
        &self,
        evidence: &CheckedPackageEvidence,
        limits: CheckedPackageReadLimits,
    ) -> Result<Box<CheckedPackageV2>, ReplayPackageAdmissionRefusal> {
        match read_checked_package(&self.bytes, limits, evidence) {
            CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
            CheckedPackageDispatchResult::Refused(refusal) => {
                Err(ReplayPackageAdmissionRefusal::from_native(refusal))
            }
            CheckedPackageDispatchResult::Incomplete(limit) => {
                Err(ReplayPackageAdmissionRefusal::Incomplete(limit))
            }
            CheckedPackageDispatchResult::CatalogUnavailable(error) => {
                Err(ReplayPackageAdmissionRefusal::CatalogUnavailable(error))
            }
        }
    }

    /// FR-640 "Behavior", FR-640-AC-8: the `clocks` object the package's
    /// `temporal_bindings` rows record, each row's `clock_name` naming a
    /// member whose value is the row's `clock`. A row bound by a model
    /// binding name (`model-steps`, `model-time`) was derived from the model
    /// and is no member. The replay recompile compiles with exactly these
    /// configurations, so it runs under the clocks the proof ran under; the
    /// recompiled package's admission against these bytes judges every row.
    pub(crate) fn clocks(&self) -> Result<crate::spine::RequestClocks, ReplayRequestRefusal> {
        const ROWS: &str = "/package/temporal_bindings";
        let mut members = std::collections::BTreeMap::new();
        if let Some(rows) = member(self.document.root(), "temporal_bindings") {
            for (index, row) in array(rows, ROWS)?.enumerate() {
                let pointer = format!("{ROWS}/{index}");
                let name = string(
                    required(row, "clock_name", &pointer)?,
                    &format!("{pointer}/clock_name"),
                )?;
                if fsl_package::is_model_binding_name(name) {
                    continue;
                }
                let at = format!("{pointer}/clock");
                let clock = serde_json::from_value(json_value(required(row, "clock", &pointer)?))
                    .map_err(|_| field_refusal(&at, PackageMemberCause::WrongKind))?;
                members.entry(name.to_owned()).or_insert(clock);
            }
        }
        crate::spine::RequestClocks::new(members).map_err(|invalid| {
            field_refusal(
                &format!("{ROWS}/{}", invalid.clock),
                PackageMemberCause::WrongKind,
            )
        })
    }

    /// FR-098-AC-34: every `function` node of a package document, in node
    /// order: its position, its node id, the single-segment name it declares
    /// (a declared function; a clause function declares none) and its
    /// recorded `check_mode` spelling. A node id leaves out `check_mode`, so
    /// the same function has the same id in the proving package and in its
    /// recompile.
    pub(crate) fn function_records(bytes: &[u8]) -> Vec<FunctionRecord> {
        let Ok(document) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            return Vec::new();
        };
        let nodes = document["semantic_graph"]["nodes"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node["node_tag"] == "function")
            .map(|(position, node)| FunctionRecord {
                position,
                node_id: node["node_id"].to_string(),
                name: match node["declaration"]["qualified_name"]
                    .as_array()
                    .map(Vec::as_slice)
                {
                    Some([name]) => name.as_str().map(str::to_owned),
                    _ => None,
                },
                mode: node["check_mode"].as_str().map(str::to_owned),
            })
            .collect()
    }

    /// FR-098-AC-34: the recorded mode of each declared function, by name,
    /// which the recompile checks that function's body under. A node whose
    /// member is absent or not `linked`/`kernel` selects nothing: the
    /// native reader refuses it at admission (FR-952-AC-4).
    pub(crate) fn function_modes(
        &self,
    ) -> std::collections::BTreeMap<String, filament_semantic_value::checking::CheckMode> {
        use filament_semantic_value::checking::CheckMode;
        Self::function_records(&self.bytes)
            .into_iter()
            .filter_map(|record| {
                let mode = match record.mode.as_deref()? {
                    "linked" => CheckMode::Linked,
                    "kernel" => CheckMode::Kernel,
                    _ => return None,
                };
                Some((record.name?, mode))
            })
            .collect()
    }

    /// FR-098-AC-34, FR-952: the check mode the replay recompile requests for
    /// function bodies, read from the `check_mode` member of the package's
    /// `function` nodes. A function body checked under `Kernel` records
    /// `kernel`; a declared clause function (`precondition`, `invariant`)
    /// records `linked` whatever mode was requested. The checker applies the
    /// requested mode to bodies only and always checks a declared clause
    /// under `Linked`, so a recompile under the mode a `kernel` node records
    /// reproduces each function's own mode: the bodies `Kernel`, every
    /// declared clause `Linked`. A package with no `kernel` node is
    /// recompiled `Linked`. A node whose member is
    /// absent or not `linked`/`kernel` selects nothing here: the native
    /// reader refuses it at admission (FR-952-AC-4), after the recompile.
    pub(crate) fn check_mode(&self) -> filament_semantic_value::checking::CheckMode {
        use filament_semantic_value::checking::CheckMode;
        let nodes = member(self.document.root(), "semantic_graph")
            .and_then(|graph| member(graph, "nodes"))
            .and_then(|nodes| match nodes.node() {
                Node::Array(items) => Some(items),
                _ => None,
            });
        let kernel = nodes.is_some_and(|nodes| {
            nodes.into_iter().any(|node| {
                matches!(
                    member(node, "node_tag").map(|tag| tag.node()),
                    Some(Node::String("function"))
                ) && matches!(
                    member(node, "check_mode").map(|mode| mode.node()),
                    Some(Node::String("kernel"))
                )
            })
        });
        if kernel {
            CheckMode::Kernel
        } else {
            CheckMode::Linked
        }
    }

    pub(super) fn descriptor(&self) -> Result<Descriptor, ReplayRequestRefusal> {
        let root = self.document.root();
        let name = string(required(root, "name", "/package")?, "/package/name")?.to_owned();
        let lock = required(root, "lock", "/package")?;
        let sources_node = required(lock, "sources", "/package/lock")?;
        let sources = array(sources_node, "/package/lock/sources")?
            .enumerate()
            .map(|(index, source)| {
                let pointer = format!("/package/lock/sources/{index}");
                Ok((
                    string(
                        required(source, "authority", &pointer)?,
                        &format!("{pointer}/authority"),
                    )?
                    .to_owned(),
                    string(
                        required(source, "identity", &pointer)?,
                        &format!("{pointer}/identity"),
                    )?
                    .to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, ReplayRequestRefusal>>()?;
        let refinement_internals = member(lock, "refinement_internals")
            .map(|rows| read_internals(rows, "/package/lock/refinement_internals"))
            .transpose()?;
        Ok(Descriptor {
            name,
            sources,
            refinement_internals,
        })
    }
}

pub(super) struct Descriptor {
    pub(super) name: String,
    pub(super) sources: Vec<SourceRefWire>,
    pub(super) refinement_internals: Option<Vec<CheckedRefinementInternal>>,
}

fn read_internals(
    value: NodeRef<'_>,
    pointer: &str,
) -> Result<Vec<CheckedRefinementInternal>, ReplayRequestRefusal> {
    array(value, pointer)?
        .enumerate()
        .map(|(index, row)| {
            let pointer = format!("{pointer}/{index}");
            let refinement = string(
                required(row, "refinement", &pointer)?,
                &format!("{pointer}/refinement"),
            )?
            .into();
            let internal = array(
                required(row, "internal", &pointer)?,
                &format!("{pointer}/internal"),
            )?
            .enumerate()
            .map(|(index, value)| {
                string(value, &format!("{pointer}/internal/{index}")).map(Into::into)
            })
            .collect::<Result<Vec<_>, _>>()?;
            Ok(CheckedRefinementInternal {
                refinement,
                internal,
            })
        })
        .collect()
}

fn required<'document>(
    value: NodeRef<'document>,
    key: &str,
    pointer: &str,
) -> Result<NodeRef<'document>, ReplayRequestRefusal> {
    if !matches!(value.node(), Node::Object(_)) {
        return Err(field_refusal(pointer, PackageMemberCause::WrongKind));
    }
    member(value, key)
        .ok_or_else(|| field_refusal(&format!("{pointer}/{key}"), PackageMemberCause::Missing))
}

fn string<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<&'document str, ReplayRequestRefusal> {
    match value.node() {
        Node::String(text) => Ok(text),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn array<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<quire_canonical::Items<'document>, ReplayRequestRefusal> {
    match value.node() {
        Node::Array(values) => Ok(values),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn field_refusal(pointer: &str, cause: PackageMemberCause) -> ReplayRequestRefusal {
    ReplayRequestRefusal::PackageMember {
        pointer: pointer.to_owned(),
        cause,
    }
}

/// The native refusal or resource stop while admitting a replay's package.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayPackageAdmissionRefusal {
    /// The native reader rejected the whole package, retaining all native data.
    #[error("replay package refused at {pointer}: {refusal}", pointer = package_pointer(.refusal.loci.first().and_then(|locus| locus.path.as_ref()).map(|path| path.as_str())), refusal = .refusal)]
    Refused {
        /// The complete native refusal.
        refusal: Box<CheckedPackageRefusal>,
        /// The native serde-owned cause spelling, retained without a copied catalog.
        cause: Option<Box<str>>,
    },
    /// The native cause no longer has its published unit-variant wire shape.
    #[error("native checked-package cause is not its catalogued string: {refusal:?}")]
    Catalog {
        /// The original refusal, never replaced by a fabricated native failure.
        refusal: Box<CheckedPackageRefusal>,
    },
    /// The native reader exhausted a caller-selected package-read ceiling.
    #[error("replay package read exhausted {:?} at {} (bound {}, consumed {})", .0.limit_kind, package_pointer(.0.path.as_ref().map(|path| path.as_str())), .0.limit, .0.consumed)]
    Incomplete(CheckedPackageIncomplete),
    /// The native reader's operation catalog is unreadable; nothing was judged.
    #[error("replay package reader catalog unavailable: {0}")]
    CatalogUnavailable(filament_contract_model::CatalogReadError),
}

impl ReplayPackageAdmissionRefusal {
    fn from_native(refusal: CheckedPackageRefusal) -> Self {
        let cause = match refusal.cause {
            None => None,
            Some(cause) => match serde_json::to_value(cause) {
                Ok(serde_json::Value::String(spelling)) => Some(spelling.into_boxed_str()),
                _ => {
                    return Self::Catalog {
                        refusal: Box::new(refusal),
                    }
                }
            },
        };
        Self::Refused {
            refusal: Box::new(refusal),
            cause,
        }
    }

    /// The FSL catalog mapping of the native reader's typed result.
    #[must_use]
    pub fn code(&self) -> fsl_foundation::Code {
        match self {
            Self::Refused { refusal, .. } => {
                fsl_package::checked_package_refusal_code(refusal.code)
            }
            Self::Catalog { .. } | Self::CatalogUnavailable(_) => {
                fsl_foundation::Code::RuntimeInvariant
            }
            Self::Incomplete(_) => fsl_foundation::Code::StageLimitExceeded,
        }
    }

    /// The native reader's cause, without rewriting its vocabulary.
    #[must_use]
    pub fn cause(&self) -> Option<&str> {
        match self {
            Self::Refused { cause, .. } => cause.as_deref(),
            Self::Catalog { .. } | Self::CatalogUnavailable(_) | Self::Incomplete(_) => None,
        }
    }

    /// The primary native pointer prefixed with the request's package member.
    #[must_use]
    pub fn pointer(&self) -> String {
        let path = match self {
            Self::Refused { refusal, .. } | Self::Catalog { refusal } => {
                refusal.loci.first().and_then(|locus| locus.path.as_ref())
            }
            Self::Incomplete(limit) => limit.path.as_ref(),
            Self::CatalogUnavailable(_) => None,
        };
        package_pointer(path.map(|path| path.as_str()))
    }
}

pub(super) fn member<'document>(
    value: NodeRef<'document>,
    name: &str,
) -> Option<NodeRef<'document>> {
    let Node::Object(members) = value.node() else {
        return None;
    };
    members
        .into_iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

fn package_pointer(native: Option<&str>) -> String {
    format!("/package{}", native.unwrap_or_default())
}

/// A clock configuration node as JSON: its objects and strings, which are
/// every member a configuration has. Any other node becomes `null`, which no
/// configuration admits.
fn json_value(value: NodeRef<'_>) -> serde_json::Value {
    match value.node() {
        Node::String(text) => serde_json::Value::String(text.to_owned()),
        Node::Object(members) => serde_json::Value::Object(
            members
                .into_iter()
                .map(|(key, value)| (key.to_owned(), json_value(value)))
                .collect(),
        ),
        Node::Null | Node::Bool(_) | Node::Number(_) | Node::Array(_) => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests;

/// A `function` node of a package document (FR-098-AC-34).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionRecord {
    /// The node's position in `/semantic_graph/nodes`.
    pub(crate) position: usize,
    /// The node id, as its JSON spelling.
    pub(crate) node_id: String,
    /// The name a declared function declares; `None` for a clause function.
    pub(crate) name: Option<String>,
    /// The recorded `check_mode` spelling, when present.
    pub(crate) mode: Option<String>,
}
