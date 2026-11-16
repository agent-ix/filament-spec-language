// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: pure reconstruction of a clause-run request after command intake.

use std::collections::BTreeMap;

/// The largest integer RFC 8785 serializes exactly (FSpec's `Count` bound).
const MAX_COUNT: u64 = 9_007_199_254_740_991;

use filament_exact::{CollectionKind, Identifier, ScalarLimits};
use fsl_foundation::serde_object::{deserialize_objects, from_object};
use fsl_semantics::model::observation::{
    collection_kind_of_tag, AnchorKind, ClauseSelection, ClauseSelectionInput, ContextSide,
    DocumentRef, NativeKind, ObservationLimits, PostStateRange, RawNativeKind, SelectedAnchor,
    SelectedObject, SnapshotValue,
};
use quire_canonical::{Node, NodeRef};
use serde_json::value::RawValue;

use super::{
    sample_position, ClauseArgument, ClauseArgumentValue, ClauseRunRequest, ClauseRunSelection,
    ClauseRunSource, DependencyInput, OperationName, OverObject, ProgressAssertion,
    ProgressClosure, ProgressCompleteness, ProgressLedger, ProgressLowerBoundary, SpineLimits,
    TimestampedCover, TimestampedMissingValuation, TimestampedObservation, TimestampedOrderKey,
    TimestampedWindow, TraceBinding, TraceInput,
};

/// A selected observation whose bytes and document identity were admitted by
/// the command path. This decoder reads no filesystem or ambient state.
#[derive(Clone)]
pub struct SuppliedObservation {
    /// The document's identity labels.
    pub reference: DocumentRef,
    /// The declared document wire identifier.
    pub format: String,
    /// A snapshot's declared observation role, if any.
    pub role: Option<String>,
    /// The exact selected document bytes.
    pub bytes: Vec<u8>,
}

impl SuppliedObservation {
    /// Read one FR-106 observation document from its exact bytes: its
    /// declared format, identity labels and role. `key`
    /// names the observation in a refusal's member path.
    #[ix_trace_rs::string_edge]
    pub fn from_document(key: &str, bytes: Vec<u8>) -> Result<Self, ClauseRequestReadError> {
        let parsed = quire_canonical::read(&bytes, u64::MAX)
            .map_err(|error| invalid(key, error.to_string()))?;
        // Reuse the canonical reader's stack-backed tree. Observation values
        // may exceed serde_json's default 128-level recursion ceiling
        // (FR-261).
        let root = parsed.root();
        let text = |path: &str, value: Option<NodeRef<'_>>| match value.map(|node| node.node()) {
            Some(Node::String(value)) => Ok(value.to_owned()),
            _ => Err(invalid(
                format!("observations.{key}.{path}"),
                "expected a string",
            )),
        };
        let format = text("format", root.get("format"))?;
        if format != "filament.state.snapshot/v1" && format != "filament.state.invocation/v1" {
            return Err(invalid(format!("observations.{key}.format"), format));
        }
        let identity = root.get("identity");
        let reference = DocumentRef {
            authority: text(
                "identity.authority",
                identity.and_then(|id| id.get("authority")),
            )?,
            identity: text(
                "identity.identity",
                identity.and_then(|id| id.get("identity")),
            )?,
        };
        let role = root.get("observation").and_then(|node| match node.node() {
            Node::String(value) => Some(value.to_owned()),
            _ => None,
        });
        Ok(Self {
            reference,
            format,
            role,
            bytes,
        })
    }
}

/// The `{population, key}` object a clause selection or argument names.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectReferenceWire {
    /// The population identity.
    pub population: String,
    /// The object's declared key.
    pub key: String,
}

/// A current snapshot's selected anchor.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnchorWire {
    /// An initialization anchor.
    Initialization {
        /// The anchor's name.
        name: String,
    },
    /// A handler anchor.
    Handler {
        /// The anchor's name.
        name: String,
    },
}

/// The `clause` member of a clause-run selection. Role-specific members are
/// checked against the selected document.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClauseSelectionWire {
    /// The clause name.
    pub name: String,
    /// Key in the selected observations table.
    pub observation: String,
    /// The selected self object.
    #[serde(default, rename = "self", deserialize_with = "from_object_opt")]
    pub self_object: Option<ObjectReferenceWire>,
    /// The current snapshot's anchor.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub anchor: Option<AnchorWire>,
    /// The pre snapshot's parameter values.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub parameters: Option<BTreeMap<String, Box<RawValue>>>,
}

/// The `frame` member of a clause-run selection.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameSelectionWire {
    /// The operation `M::T::op`.
    pub operation: String,
    /// Key of the invocation document.
    pub invocation: String,
}

/// One named argument of a `function` selection.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClauseArgumentWire {
    /// The parameter's declared name.
    pub parameter: String,
    /// The canonical integer or object reference.
    pub value: Box<RawValue>,
}

/// The `function` member of a clause-run selection.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSelectionWire {
    /// The function name.
    pub name: String,
    /// The call's arguments.
    #[serde(deserialize_with = "deserialize_objects")]
    pub arguments: Vec<ClauseArgumentWire>,
    /// Key of the current snapshot.
    pub snapshot: String,
}

/// FR-641: one position of a trace: an `observations` key naming a snapshot,
/// or `{"sample": R}`, a record sample of a declared record trace subject.
#[derive(Clone, Debug)]
pub enum PositionWire {
    /// An `observations` key.
    Key(String),
    /// A record sample, as the JSON object text the request wrote.
    Sample(Box<RawValue>),
}

impl<'de> serde::Deserialize<'de> for PositionWire {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PositionWire;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an observations key or a {\"sample\": ...} object")
            }

            fn visit_str<E: serde::de::Error>(self, key: &str) -> Result<PositionWire, E> {
                Ok(PositionWire::Key(key.to_owned()))
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<PositionWire, M::Error> {
                let Some(name) = map.next_key::<String>()? else {
                    return Err(serde::de::Error::missing_field("sample"));
                };
                if name != "sample" {
                    return Err(serde::de::Error::unknown_field(&name, &["sample"]));
                }
                let sample = map.next_value::<Box<RawValue>>()?;
                if let Some(extra) = map.next_key::<String>()? {
                    return Err(serde::de::Error::unknown_field(&extra, &["sample"]));
                }
                Ok(PositionWire::Sample(sample))
            }
        }
        decoder.deserialize_any(Visitor)
    }
}

/// FR-641 `G`: the progress assertion a trace carries.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressWire {
    /// A non-negative inclusive watermark, or a half-open one.
    pub watermark: WatermarkWire,
    /// `{"complete": {"through": N}}` or `"incomplete"`.
    pub completeness: CompletenessWire,
    /// The decision scope's closure.
    pub scope: ClosureWire,
    /// The surrounding execution's closure.
    pub execution: ClosureWire,
}

/// A progress watermark.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum WatermarkWire {
    /// Progress covers every tick up to and including the value.
    Inclusive(u64),
    /// Progress covers every tick strictly below `half_open`.
    HalfOpen {
        /// The exclusive end.
        half_open: u64,
    },
}

/// A completeness assertion.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletenessWire {
    /// Every observation stamped at or before `through` is supplied.
    Complete {
        /// The last tick covered.
        through: u64,
    },
    /// No completeness is asserted.
    Incomplete,
}

/// The closure of a decision scope or of the surrounding execution.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureWire {
    /// Nothing more will be added.
    Closed,
    /// More may still be added.
    Open,
}

/// A trace's lower boundary.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LowerBoundaryWire {
    /// History before the first position is empty.
    AuthoritativeOrigin,
    /// History before the first position was not supplied.
    HistoryCutoff,
}

/// FR-641 `T` (FR-522): the profile, clock and clock configuration a trace
/// asserts.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingWire {
    /// The profile identity.
    pub profile: String,
    /// The clock binding name.
    pub clock: String,
    /// The clock configuration, in FR-640's form.
    pub parameters: fsl_package::ClockConfiguration,
}

/// FR-641 `progressed`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressedWire {
    /// The positions, in order.
    pub positions: Vec<PositionWire>,
    /// The progress assertion.
    pub progress: ProgressWire,
    /// Whether the first position is an authoritative origin.
    pub lower_boundary: LowerBoundaryWire,
    /// The trace binding.
    pub binding: BindingWire,
}

/// FR-641 `O`: one timestamped observation.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationWire {
    /// The position.
    pub position: PositionWire,
    /// The non-negative integer tick.
    pub stamp: u64,
    /// The order key, where the observation has one.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub order_key: Option<OrderKeyWire>,
}

/// An order key comparable within one authority.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderKeyWire {
    /// The authority that assigned the key.
    pub authority: String,
    /// The key.
    pub key: i64,
}

/// FR-641 `C`: how the stamps cover time.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverWire {
    /// Every stamp is exact.
    Exact,
    /// Every stamp is within `uncertainty` ticks.
    Counter {
        /// The uncertainty, in ticks.
        uncertainty: u64,
    },
    /// Every point stamp `t` covers `[t-u, t+u]` (FR-501 "Request-file
    /// spelling").
    Bounded {
        /// The uncertainty, in ticks.
        uncertainty: u64,
    },
}

/// FR-641 `M`: an atom valuation absent from an admitted observation.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissingWire {
    /// The observation's index in `observations`.
    pub position: u64,
    /// The zero-based `holds` formula node.
    pub atom: u64,
}

/// FR-641 `timestamped`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimestampedWire {
    /// The observations, in any order.
    pub observations: Vec<ObservationWire>,
    /// The progress assertion.
    pub progress: ProgressWire,
    /// Whether the first position is an authoritative origin.
    pub lower_boundary: LowerBoundaryWire,
    /// The trace binding.
    pub binding: BindingWire,
    /// How the stamps cover time; `exact` when absent.
    #[serde(default)]
    pub cover: Option<CoverWire>,
    /// Atom valuations absent from an admitted observation.
    #[serde(default)]
    pub missing: Vec<MissingWire>,
}

/// One temporal trace shape in an FR-312 request (FR-641 "Trace kinds").
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalTraceWire {
    /// Finite positions in order.
    #[serde(default, deserialize_with = "present")]
    pub finite: Option<Vec<PositionWire>>,
    /// A lasso's finite prefix and repeating loop.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub lasso: Option<TemporalLassoWire>,
    /// A progressed event-position or fixed-sample trace.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub progressed: Option<ProgressedWire>,
    /// A timestamped finite-window trace.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub timestamped: Option<TimestampedWire>,
    /// A model-arm lasso.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub model: Option<ModelWire>,
    /// A timed trace.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub timed: Option<TimedWire>,
}

/// The prefix and loop of a temporal lasso.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalLassoWire {
    /// Positions before the loop.
    pub prefix: Vec<PositionWire>,
    /// Repeating positions.
    #[serde(rename = "loop")]
    pub repeat: Vec<PositionWire>,
}

/// One stamped position of a timed trace (FR-642).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedPointWire {
    /// The position.
    pub position: PositionWire,
    /// The exact rational stamp.
    pub stamp: String,
}

/// A timed trace (FR-642): finite, or a lasso with a period.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedWire {
    /// Positions before the loop.
    pub prefix: Vec<TimedPointWire>,
    /// Repeating positions.
    #[serde(default, rename = "loop")]
    pub repeat: Vec<TimedPointWire>,
    /// The loop's total delay.
    #[serde(default)]
    pub period: Option<String>,
    /// How far a finite prefix is known complete.
    #[serde(default)]
    pub through: Option<String>,
}

/// A model trace (FR-641, FSpec FR-364's model-arm encoding).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelWire {
    /// The initial-state snapshot's observations key.
    pub initial: String,
    /// The snapshots (observations keys) whose complete populations are the
    /// universes.
    pub universes: Vec<String>,
    /// Steps before the loop.
    pub prefix: Vec<ModelStepWire>,
    /// Repeating steps; the one terminal stutter step closes a terminal loop.
    #[serde(rename = "loop")]
    pub repeat: Vec<ModelStepWire>,
    /// FR-641: the per-expansion candidate bound, a positive integer from 1
    /// to 2^53 - 1; absent is 1,000,000. Kept as read so the reader names the
    /// member for a wrong type.
    #[serde(default)]
    pub max_candidates: Option<serde_json::Value>,
}

/// One step of a model-arm behaviour.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelStepWire {
    /// The step's transition identity.
    pub transition: ModelTransitionWire,
    /// The state key of the step's post-state.
    pub post_state: PostStateWire,
}

/// A post-state key, base64url of its canonical bytes.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostStateWire {
    /// The key text.
    pub key: String,
}

/// A transition identity: `{"type": "transition", operation, receiver,
/// arguments}` or `{"type": "terminal-stutter"}`.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTransitionWire {
    /// `transition` or `terminal-stutter`.
    #[serde(rename = "type")]
    pub kind: String,
    /// The operation's typed value.
    #[serde(default)]
    pub operation: Option<Box<RawValue>>,
    /// The receiver's typed value.
    #[serde(default)]
    pub receiver: Option<Box<RawValue>>,
    /// The arguments' typed values.
    #[serde(default)]
    pub arguments: Option<Vec<Box<RawValue>>>,
}

/// The temporal member of an FR-312 request.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalSelectionWire {
    /// The declared temporal clause name.
    pub name: String,
    /// The supplied trace.
    pub trace: TemporalTraceWire,
    /// The object binding for the clause's `over` parameter: present exactly
    /// when the trace's positions are snapshot keys.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub over: Option<TemporalOverWire>,
    /// The relative path of a `filament.progress-ledger/1` file (FR-641).
    #[serde(default, deserialize_with = "present")]
    pub ledger: Option<String>,
}

fn present<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(decoder).map(Some)
}

/// A temporal clause's object binding.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalOverWire {
    /// The object reference.
    pub reference: TemporalObjectWire,
}

/// A temporal clause's population and object key.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalObjectWire {
    /// The population declaration identity.
    pub population: String,
    /// The object's key.
    pub key: String,
}

/// FR-312's closed clause-run selection: exactly one of its members.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClauseRunSelectionWire {
    /// A state clause.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub clause: Option<ClauseSelectionWire>,
    /// An operation frame.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub frame: Option<FrameSelectionWire>,
    /// A function run as a claim.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub function: Option<FunctionSelectionWire>,
    /// A temporal clause over an observed trace.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub temporal: Option<TemporalSelectionWire>,
    /// A state-machine context on one side of an invocation.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub context: Option<ContextSelectionWire>,
}

/// The context selection's exact FR-652 member set.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSelectionWire {
    /// The state-machine invariant name.
    pub name: String,
    /// Key of the invocation document.
    pub invocation: String,
    /// The selected invocation side, `pre` or `post`.
    pub side: String,
}

fn from_object_opt<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    from_object(decoder).map(Some)
}

impl ClauseRunSelectionWire {
    /// FR-641: the relative path of the `temporal` selection's progress ledger
    /// file, when it names one.
    pub fn ledger_path(&self) -> Option<&str> {
        self.temporal
            .as_ref()
            .and_then(|temporal| temporal.ledger.as_deref())
    }

    /// The closed selection this member set names.
    pub fn into_parts(&self) -> Result<ClauseSelectionPartsKind, ClauseRequestReadError> {
        let count = usize::from(self.clause.is_some())
            + usize::from(self.frame.is_some())
            + usize::from(self.function.is_some());
        let count = count + usize::from(self.temporal.is_some());
        let count = count + usize::from(self.context.is_some());
        if count != 1 {
            return Err(invalid(
                "selection",
                "select exactly one clause, frame, function, temporal clause or context",
            ));
        }
        if let Some(clause) = &self.clause {
            let anchor = clause.anchor.as_ref().map(|anchor| {
                let (kind, name) = match anchor {
                    AnchorWire::Initialization { name } => (AnchorKind::Initialization, name),
                    AnchorWire::Handler { name } => (AnchorKind::Handler, name),
                };
                SelectedAnchor {
                    kind,
                    name: name.clone(),
                }
            });
            return Ok(ClauseSelectionPartsKind::Clause(ClauseSelectionParts {
                name: clause.name.clone(),
                observation: clause.observation.clone(),
                self_object: clause.self_object.as_ref().map(|object| SelectedObject {
                    population: object.population.clone(),
                    key: object.key.clone(),
                }),
                anchor,
                parameters: clause.parameters.clone(),
            }));
        }
        if let Some(frame) = &self.frame {
            return Ok(ClauseSelectionPartsKind::Frame {
                operation: frame.operation.clone(),
                invocation: frame.invocation.clone(),
            });
        }
        if let Some(temporal) = &self.temporal {
            return temporal.selection_parts();
        }
        if let Some(context) = &self.context {
            let side = match context.side.as_str() {
                "pre" => ContextSide::Pre,
                "post" => ContextSide::Post,
                _ => return Err(invalid("side", "expected `pre` or `post`")),
            };
            return Ok(ClauseSelectionPartsKind::Context {
                name: context.name.clone(),
                invocation: context.invocation.clone(),
                side,
            });
        }
        let function = self.function.as_ref().expect("exactly one variant");
        Ok(ClauseSelectionPartsKind::Function {
            name: function.name.clone(),
            arguments: function
                .arguments
                .iter()
                .map(|argument| (argument.parameter.clone(), argument.value.clone()))
                .collect(),
            snapshot: function.snapshot.clone(),
        })
    }
}

/// FR-106 observation limit overrides of a clause run.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLimitsWire {
    /// The document byte ceiling.
    pub document_bytes: Option<u64>,
    /// The objects-per-document ceiling.
    pub objects_per_document: Option<u64>,
    /// The values-per-document ceiling.
    pub values_per_document: Option<u64>,
    /// The post-state range policy, `refuse` or `witness`.
    pub post_state: Option<String>,
}

impl ObservationLimitsWire {
    /// The default observation limits with each stated override applied.
    #[ix_trace_rs::string_edge]
    pub fn resolve(overrides: Option<&Self>) -> Result<ObservationLimits, ClauseRequestReadError> {
        let mut limits = ObservationLimits::default();
        if let Some(overrides) = overrides {
            if let Some(value) = overrides.document_bytes {
                limits.document_bytes = value;
            }
            if let Some(value) = overrides.objects_per_document {
                limits.objects_per_document = value;
            }
            if let Some(value) = overrides.values_per_document {
                limits.values_per_document = value;
            }
            if let Some(value) = &overrides.post_state {
                limits.post_state = match value.as_str() {
                    "refuse" => PostStateRange::Refuse,
                    "witness" => PostStateRange::Witness,
                    _ => return Err(invalid("observation_limits.post_state", "unknown policy")),
                };
            }
        }
        Ok(limits)
    }
}

/// Context members supplied for one clause selection.
pub struct ClauseSelectionParts {
    /// The clause name.
    pub name: String,
    /// Key in the selected observations table.
    pub observation: String,
    /// The current or pre snapshot's selected self object.
    pub self_object: Option<SelectedObject>,
    /// The current snapshot's selected anchor.
    pub anchor: Option<SelectedAnchor>,
    /// The pre snapshot's parameter values, in canonical typed wire form.
    pub parameters: Option<BTreeMap<String, Box<RawValue>>>,
}

/// One closed request selection before role-specific reconstruction.
pub enum ClauseSelectionPartsKind {
    /// A state clause over one observation.
    Clause(ClauseSelectionParts),
    /// A frame over an invocation document.
    Frame {
        /// Qualified operation name.
        operation: String,
        /// Key of the invocation document.
        invocation: String,
    },
    /// A Boolean function over a current snapshot.
    Function {
        /// Function name.
        name: String,
        /// Named arguments, each as a canonical integer or object reference.
        arguments: Vec<(String, Box<RawValue>)>,
        /// Key of the current snapshot.
        snapshot: String,
    },
    /// A temporal clause over a supplied trace.
    Temporal {
        /// Clause name.
        name: String,
        /// Trace shape and its positions.
        trace: TemporalTraceParts,
        /// Population and object key for the clause's `over` binding: present
        /// exactly when the trace's positions are snapshot keys.
        over: Option<(String, String)>,
    },
    /// A state-machine invariant over an invocation's pre or post context.
    Context {
        /// The invariant name.
        name: String,
        /// Key of its invocation document.
        invocation: String,
        /// Which invocation snapshot the context reads.
        side: ContextSide,
    },
}

/// One position of a trace before observation lookup (FR-641).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalPosition {
    /// An `observations` key.
    Key(String),
    /// A record sample, as its canonical JSON object text.
    Sample(String),
}

impl PartialEq<&str> for TemporalPosition {
    fn eq(&self, key: &&str) -> bool {
        matches!(self, Self::Key(own) if own == key)
    }
}

/// An FR-641 `progressed` trace before observation lookup.
pub struct ProgressedParts {
    /// The positions, in order.
    pub positions: Vec<TemporalPosition>,
    /// The progress assertion.
    pub progress: ProgressWire,
    /// The lower boundary.
    pub lower_boundary: LowerBoundaryWire,
    /// The trace binding.
    pub binding: BindingWire,
}

/// An FR-641 `timestamped` trace before observation lookup.
pub struct TimestampedParts {
    /// Each observation's position, stamp and order key.
    pub observations: Vec<(TemporalPosition, u64, Option<OrderKeyWire>)>,
    /// The progress assertion.
    pub progress: ProgressWire,
    /// The lower boundary.
    pub lower_boundary: LowerBoundaryWire,
    /// The trace binding.
    pub binding: BindingWire,
    /// How the stamps cover time.
    pub cover: Option<CoverWire>,
    /// Atom valuations absent from an admitted observation.
    pub missing: Vec<MissingWire>,
}

/// An FR-641 trace before observation lookup.
pub enum TemporalTraceParts {
    /// Finite positions in order.
    Finite(Vec<TemporalPosition>),
    /// Prefix and repeating loop.
    Lasso {
        prefix: Vec<TemporalPosition>,
        repeat: Vec<TemporalPosition>,
    },
    /// A progressed event-position or fixed-sample trace.
    Progressed(ProgressedParts),
    /// A timestamped finite-window trace.
    Timestamped(TimestampedParts),
    /// A model-arm behaviour (FR-641).
    Model(ModelParts),
    /// A timed trace (FR-642): read, not yet run.
    Timed(TimedParts),
}

/// A model-arm behaviour read from the wire.
pub struct ModelParts {
    /// The initial-state snapshot's observations key.
    pub initial: String,
    /// The universe snapshots' observations keys.
    pub universes: Vec<String>,
    /// The steps before the loop.
    pub prefix: Vec<crate::ModelStep>,
    /// The loop's steps, a terminal stutter step excluded.
    pub repeat: Vec<crate::ModelStep>,
    /// Whether the loop is the one terminal stutter step.
    pub terminal_stutter: bool,
    /// The candidate bound.
    pub max_candidates: u64,
}

/// A timed trace's positions with their rational stamps.
pub struct TimedParts {
    /// Stamped positions before the loop.
    pub prefix: Vec<(TemporalPosition, String)>,
    /// Stamped repeating positions.
    pub repeat: Vec<(TemporalPosition, String)>,
    /// The loop's total delay.
    pub period: Option<String>,
    /// The prefix's known extent.
    pub through: Option<String>,
}

/// A sample's canonical JSON object text.
fn canonical_sample(raw: &RawValue, member: &str) -> Result<String, ClauseRequestReadError> {
    let document = quire_canonical::read(raw.get().as_bytes(), u64::MAX)
        .map_err(|error| invalid(member, error.to_string()))?;
    if !matches!(document.root().node(), Node::Object(_)) {
        return Err(invalid(member, "a sample is a JSON object"));
    }
    let bytes = quire_canonical::to_vec(&document.root(), quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| invalid(member, error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| invalid(member, error.to_string()))
}

/// The canonical JSON text of the typed value `raw`.
fn canonical_text(raw: &RawValue, member: &str) -> Result<String, ClauseRequestReadError> {
    let document = quire_canonical::read(raw.get().as_bytes(), u64::MAX)
        .map_err(|error| invalid(member, error.to_string()))?;
    let bytes = quire_canonical::to_vec(&document.root(), quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| invalid(member, error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| invalid(member, error.to_string()))
}

/// One step of a model-arm behaviour; `None` for the terminal stutter step.
fn model_step(
    step: &ModelStepWire,
    member: &str,
) -> Result<Option<crate::ModelStep>, ClauseRequestReadError> {
    use base64::Engine;
    let transition = &step.transition;
    let post_state = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(step.post_state.key.as_bytes())
        .map_err(|error| invalid(format!("{member}.post_state.key"), error.to_string()))?;
    let identity = match transition.kind.as_str() {
        "terminal-stutter" => return Ok(None),
        "transition" => {
            let operation = transition.operation.as_deref().ok_or_else(|| {
                invalid(
                    format!("{member}.transition.operation"),
                    "an operation is required",
                )
            })?;
            let at = format!("{member}.transition");
            filament_semantic_value::model_transition::ModelTransition::new(
                canonical_text(operation, &format!("{at}.operation"))?,
                transition
                    .receiver
                    .as_deref()
                    .map(|receiver| canonical_text(receiver, &format!("{at}.receiver")))
                    .transpose()?,
                transition
                    .arguments
                    .iter()
                    .flatten()
                    .enumerate()
                    .map(|(index, argument)| {
                        canonical_text(argument, &format!("{at}.arguments[{index}]"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                quire_canonical::Limits::new(u64::MAX),
            )
            .map_err(|error| invalid(&at, error.to_string()))?
        }
        _ => {
            return Err(invalid(
                format!("{member}.transition.type"),
                "a transition or a terminal-stutter",
            ))
        }
    };
    Ok(Some(crate::ModelStep {
        transition: identity,
        post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(post_state),
        delay: None,
    }))
}

/// FR-641 `model`: the initial state, universes, and the prefix and loop
/// steps of a model-arm behaviour.
fn model_parts(model: &ModelWire) -> Result<ModelParts, ClauseRequestReadError> {
    let steps = |wire: &[ModelStepWire], member: &str| {
        wire.iter()
            .enumerate()
            .map(|(index, step)| model_step(step, &format!("{member}[{index}]")))
            .collect::<Result<Vec<_>, _>>()
    };
    let prefix = steps(&model.prefix, "trace.model.prefix")?;
    let looped = steps(&model.repeat, "trace.model.loop")?;
    if prefix.iter().any(Option::is_none) {
        return Err(invalid(
            "trace.model.prefix",
            "a terminal stutter step closes a loop, never a prefix",
        ));
    }
    let stutters = looped.iter().filter(|step| step.is_none()).count();
    if stutters > 1 || (stutters == 1 && looped.len() != 1) {
        return Err(invalid(
            "trace.model.loop",
            "a terminal stutter loop is the one terminal stutter step",
        ));
    }
    let max_candidates = match &model.max_candidates {
        None => crate::spine::DEFAULT_MODEL_MAX_CANDIDATES,
        Some(value) => value
            .as_u64()
            .filter(|bound| (1..=MAX_COUNT).contains(bound))
            .ok_or_else(|| {
                invalid(
                    "trace.model.max_candidates",
                    "expected a positive integer from 1 to 2^53 - 1",
                )
            })?,
    };
    Ok(ModelParts {
        initial: model.initial.clone(),
        universes: model.universes.clone(),
        prefix: prefix.into_iter().flatten().collect(),
        repeat: looped.into_iter().flatten().collect(),
        terminal_stutter: stutters == 1,
        max_candidates,
    })
}

fn position_part(
    position: &PositionWire,
    member: &str,
) -> Result<TemporalPosition, ClauseRequestReadError> {
    Ok(match position {
        PositionWire::Key(key) => TemporalPosition::Key(key.clone()),
        PositionWire::Sample(sample) => TemporalPosition::Sample(canonical_sample(sample, member)?),
    })
}

fn position_parts(
    positions: &[PositionWire],
    member: &str,
) -> Result<Vec<TemporalPosition>, ClauseRequestReadError> {
    positions
        .iter()
        .enumerate()
        .map(|(index, position)| position_part(position, &format!("{member}[{index}]")))
        .collect()
}

impl TemporalSelectionWire {
    /// FR-641 "Behavior": the closed `temporal` selection this member set
    /// names.
    fn selection_parts(&self) -> Result<ClauseSelectionPartsKind, ClauseRequestReadError> {
        let trace = &self.trace;
        let members = usize::from(trace.finite.is_some())
            + usize::from(trace.lasso.is_some())
            + usize::from(trace.progressed.is_some())
            + usize::from(trace.timestamped.is_some())
            + usize::from(trace.model.is_some())
            + usize::from(trace.timed.is_some());
        if members != 1 {
            return Err(invalid(
                "trace",
                "select exactly one finite, lasso, model, progressed, timestamped or timed trace",
            ));
        }
        let parts = if let Some(positions) = &trace.finite {
            TemporalTraceParts::Finite(position_parts(positions, "trace.finite")?)
        } else if let Some(lasso) = &trace.lasso {
            TemporalTraceParts::Lasso {
                prefix: position_parts(&lasso.prefix, "trace.lasso.prefix")?,
                repeat: position_parts(&lasso.repeat, "trace.lasso.loop")?,
            }
        } else if let Some(progressed) = &trace.progressed {
            TemporalTraceParts::Progressed(ProgressedParts {
                positions: position_parts(&progressed.positions, "trace.progressed.positions")?,
                progress: progressed.progress.clone(),
                lower_boundary: progressed.lower_boundary,
                binding: progressed.binding.clone(),
            })
        } else if let Some(timestamped) = &trace.timestamped {
            let observations = timestamped
                .observations
                .iter()
                .enumerate()
                .map(|(index, observation)| {
                    Ok((
                        position_part(
                            &observation.position,
                            &format!("trace.timestamped.observations[{index}].position"),
                        )?,
                        observation.stamp,
                        observation.order_key.clone(),
                    ))
                })
                .collect::<Result<Vec<_>, ClauseRequestReadError>>()?;
            TemporalTraceParts::Timestamped(TimestampedParts {
                observations,
                progress: timestamped.progress.clone(),
                lower_boundary: timestamped.lower_boundary,
                binding: timestamped.binding.clone(),
                cover: timestamped.cover,
                missing: timestamped.missing.clone(),
            })
        } else if let Some(model) = &trace.model {
            TemporalTraceParts::Model(model_parts(model)?)
        } else if let Some(timed) = &trace.timed {
            let points = |points: &[TimedPointWire], member: &str| {
                points
                    .iter()
                    .enumerate()
                    .map(|(index, point)| {
                        Ok((
                            position_part(&point.position, &format!("{member}[{index}].position"))?,
                            point.stamp.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>, ClauseRequestReadError>>()
            };
            TemporalTraceParts::Timed(TimedParts {
                prefix: points(&timed.prefix, "trace.timed.prefix")?,
                repeat: points(&timed.repeat, "trace.timed.loop")?,
                period: timed.period.clone(),
                through: timed.through.clone(),
            })
        } else {
            return Err(invalid("trace", "select a trace"));
        };
        let positions = parts.positions();
        let keys = positions
            .iter()
            .filter(|position| matches!(position, TemporalPosition::Key(_)))
            .count();
        if keys != 0 && keys != positions.len() {
            return Err(invalid(
                "trace",
                "every position of a trace is an observations key or every one is a sample",
            ));
        }
        if matches!(parts, TemporalTraceParts::Model(_)) && self.over.is_none() {
            return Err(invalid("over", "a model trace needs an over"));
        }
        // A trace with no position is judged by the run, whichever form it
        // would have had: `over` decides it.
        match (&self.over, keys != 0) {
            (None, true) => return Err(invalid("over", "a trace of snapshot keys needs an over")),
            (Some(_), false) if !positions.is_empty() => {
                return Err(invalid("over", "a trace of record samples takes no over"))
            }
            _ => {}
        }
        Ok(ClauseSelectionPartsKind::Temporal {
            name: self.name.clone(),
            trace: parts,
            over: self.over.as_ref().map(|over| {
                (
                    over.reference.population.clone(),
                    over.reference.key.clone(),
                )
            }),
        })
    }
}

impl TemporalTraceParts {
    /// Every position, in the order the trace lists them.
    fn positions(&self) -> Vec<&TemporalPosition> {
        match self {
            Self::Finite(positions) => positions.iter().collect(),
            Self::Lasso { prefix, repeat } => prefix.iter().chain(repeat).collect(),
            Self::Progressed(parts) => parts.positions.iter().collect(),
            Self::Timestamped(parts) => parts
                .observations
                .iter()
                .map(|(position, ..)| position)
                .collect(),
            Self::Model(_) => Vec::new(),
            Self::Timed(parts) => parts
                .prefix
                .iter()
                .chain(&parts.repeat)
                .map(|(position, _)| position)
                .collect(),
        }
    }
}

/// The typed parts supplied by the path and byte-budget adapter.
pub struct ClauseRequestParts {
    /// The exact program or extracted source.
    pub source: ClauseRunSource,
    /// The package manifest's top-level name.
    pub package_name: String,
    /// Canonical domain package input.
    pub packages: fsl_semantics::model::intake::PackageInput,
    /// Supplied library dependencies.
    pub dependencies: DependencyInput,
    /// Named observation documents, each already read once.
    pub observations: BTreeMap<String, SuppliedObservation>,
    /// The wire selection reconstructed against the named documents.
    pub selection: ClauseSelectionPartsKind,
    /// Spine stage limits.
    pub limits: SpineLimits,
    /// Observation admission limits.
    pub observation_limits: ObservationLimits,
    /// Evaluation meter limits.
    pub accounting: ScalarLimits,
    /// FR-640: the request's clock configurations by clock name.
    pub clocks: super::RequestClocks,
    /// FR-641: the progress ledger the `temporal` selection's `ledger` file
    /// holds, read once by the adapter.
    pub ledger: Option<ProgressLedger>,
}

/// A closed clause request member that cannot be reconstructed.
#[derive(Debug, thiserror::Error)]
#[error("{member}: {reason}")]
pub struct ClauseRequestReadError {
    /// The member or selected observation key at fault.
    pub member: String,
    /// The typed reason for refusal.
    pub reason: String,
}

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> ClauseRequestReadError {
    ClauseRequestReadError {
        member: member.into(),
        reason: reason.into(),
    }
}

fn named<'a>(
    observations: &'a BTreeMap<String, SuppliedObservation>,
    key: &str,
) -> Result<&'a SuppliedObservation, ClauseRequestReadError> {
    observations
        .get(key)
        .ok_or_else(|| invalid(format!("observations.{key}"), "unbound observation key"))
}

#[ix_trace_rs::string_edge]
fn snapshot_value(value: NodeRef<'_>) -> Result<SnapshotValue, ClauseRequestReadError> {
    // SnapshotValue itself is arena-backed. Build it bottom-up on a heap
    // worklist so a supplied present/sequence tree has no native stack cap.
    enum Work<'a> {
        Read(NodeRef<'a>),
        Present,
        Collection(CollectionKind, usize),
    }
    let mut work = vec![Work::Read(value)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Work::Present => {
                let inner = values
                    .pop()
                    .ok_or_else(|| invalid("parameters", "missing present value"))?;
                values.push(SnapshotValue::present(inner));
            }
            Work::Collection(kind, len) => {
                if values.len() < len {
                    return Err(invalid("parameters.collection", "missing collection value"));
                }
                let items = values.split_off(values.len() - len);
                values.push(SnapshotValue::collection(kind, items));
            }
            Work::Read(value) => {
                let Node::Object(mut object) = value.node() else {
                    return Err(invalid("parameters", "expected a value object"));
                };
                // A native payload object (String, UUID, Timestamp) carries
                // its kind as a member beside the payload; every defect is
                // held for admission to judge in FR-106's native order.
                if let Some(kind) = value.get("kind") {
                    let kind = match kind.node() {
                        Node::String("string") => RawNativeKind::Known(NativeKind::String),
                        Node::String("uuid") => RawNativeKind::Known(NativeKind::Uuid),
                        Node::String("timestamp") => RawNativeKind::Known(NativeKind::Timestamp),
                        Node::String("text") => RawNativeKind::Known(NativeKind::Text),
                        Node::String(_) => RawNativeKind::Unknown,
                        _ => RawNativeKind::NotText,
                    };
                    let members = object
                        .filter(|(name, _)| *name != "kind")
                        .map(|(name, member)| {
                            let text = match member.node() {
                                Node::String(text) => Some(text.to_owned()),
                                _ => None,
                            };
                            (name.to_owned(), text)
                        })
                        .collect();
                    values.push(SnapshotValue::native(kind, members));
                    continue;
                }
                if object.len() != 1 {
                    return Err(invalid("parameters", "expected one value form"));
                }
                let (kind, content) = object.next().expect("one member");
                match kind {
                    "boolean" => values.push(SnapshotValue::boolean(match content.node() {
                        Node::Bool(value) => value,
                        _ => return Err(invalid("parameters.boolean", "expected Boolean")),
                    })),
                    "integer" => values.push(SnapshotValue::integer(match content.node() {
                        Node::String(value) => value,
                        _ => return Err(invalid("parameters.integer", "expected decimal string")),
                    })),
                    "absent" if matches!(content.node(), Node::Object(items) if items.len() == 0) =>
                    {
                        values.push(SnapshotValue::absent());
                    }
                    "present" => {
                        work.push(Work::Present);
                        work.push(Work::Read(content));
                    }
                    "reference" => {
                        let Node::Object(reference) = content.node() else {
                            return Err(invalid("parameters.reference", "expected object"));
                        };
                        if reference.len() != 2 {
                            return Err(invalid(
                                "parameters.reference",
                                "expected population and key",
                            ));
                        }
                        let population = match content.get("population").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid(
                                    "parameters.reference.population",
                                    "expected string",
                                ))
                            }
                        };
                        let key = match content.get("key").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid("parameters.reference.key", "expected string"))
                            }
                        };
                        values.push(SnapshotValue::reference(SelectedObject {
                            population: population.to_owned(),
                            key: key.to_owned(),
                        }));
                    }
                    "variant" => {
                        let Node::Object(payload) = content.node() else {
                            return Err(invalid("parameters.variant", "expected object"));
                        };
                        let (mut type_identity, mut case, mut extra_member) = (None, None, false);
                        for (name, member) in payload {
                            let text = |member: NodeRef<'_>, field: &str| match member.node() {
                                Node::String(text) => Ok(text.to_owned()),
                                _ => Err(invalid(
                                    format!("parameters.variant.{field}"),
                                    "expected string",
                                )),
                            };
                            match name {
                                "type" => type_identity = Some(text(member, "type")?),
                                "case" => case = Some(text(member, "case")?),
                                _ => extra_member = true,
                            }
                        }
                        values.push(SnapshotValue::variant_payload(
                            type_identity,
                            case,
                            extra_member,
                        ));
                    }
                    "sequence" | "set" | "bag" | "ordered-set" => {
                        let Some(collection) = collection_kind_of_tag(kind) else {
                            return Err(invalid(
                                format!("parameters.{kind}"),
                                "unknown value form",
                            ));
                        };
                        let Node::Array(items) = content.node() else {
                            return Err(invalid(format!("parameters.{kind}"), "expected array"));
                        };
                        work.push(Work::Collection(collection, items.len()));
                        work.extend(items.collect::<Vec<_>>().into_iter().rev().map(Work::Read));
                    }
                    _ => return Err(invalid(format!("parameters.{kind}"), "unknown value form")),
                }
            }
        }
    }
    values
        .pop()
        .ok_or_else(|| invalid("parameters", "missing value"))
}

#[ix_trace_rs::string_edge]
fn selection(
    selected: ClauseSelectionPartsKind,
    observations: &BTreeMap<String, SuppliedObservation>,
    ledger: Option<ProgressLedger>,
) -> Result<ClauseRunSelection, ClauseRequestReadError> {
    match selected {
        ClauseSelectionPartsKind::Clause(clause) => {
            let document = named(observations, &clause.observation)?;
            let input = match (document.format.as_str(), document.role.as_deref()) {
                ("filament.state.snapshot/v1", Some("current")) => {
                    if clause.parameters.is_some() {
                        return Err(invalid("parameters", "forbidden for current snapshot"));
                    }
                    ClauseSelectionInput::Current {
                        snapshot: document.reference.clone(),
                        self_object: clause.self_object,
                        anchor: clause
                            .anchor
                            .ok_or_else(|| invalid("anchor", "required for current snapshot"))?,
                    }
                }
                ("filament.state.snapshot/v1", Some("pre")) => {
                    if clause.anchor.is_some() {
                        return Err(invalid("anchor", "forbidden for pre snapshot"));
                    }
                    let parameters = clause
                        .parameters
                        .ok_or_else(|| invalid("parameters", "required for pre snapshot"))?
                        .iter()
                        .map(|(name, value)| {
                            let name = Identifier::new(name)
                                .map_err(|error| invalid("parameters", error.to_string()))?;
                            let document = quire_canonical::read(value.get().as_bytes(), u64::MAX)
                                .map_err(|error| invalid("parameters", error.to_string()))?;
                            Ok((name, snapshot_value(document.root())?))
                        })
                        .collect::<Result<BTreeMap<_, _>, ClauseRequestReadError>>()?;
                    ClauseSelectionInput::PreCall {
                        snapshot: document.reference.clone(),
                        self_object: clause.self_object,
                        parameters,
                    }
                }
                ("filament.state.invocation/v1", _) => {
                    if clause.self_object.is_some() {
                        return Err(invalid("self", "forbidden for invocation"));
                    }
                    if clause.anchor.is_some() {
                        return Err(invalid("anchor", "forbidden for invocation"));
                    }
                    if clause.parameters.is_some() {
                        return Err(invalid("parameters", "forbidden for invocation"));
                    }
                    ClauseSelectionInput::Invocation {
                        invocation: document.reference.clone(),
                    }
                }
                _ => return Err(invalid("observation", "unsupported observation role")),
            };
            Ok(ClauseRunSelection::Clause(ClauseSelection {
                name: clause.name,
                input,
            }))
        }
        ClauseSelectionPartsKind::Context {
            name,
            invocation,
            side,
        } => {
            let document = named(observations, &invocation)?;
            if document.format != "filament.state.invocation/v1" {
                return Err(invalid("invocation", "context requires an invocation"));
            }
            Ok(ClauseRunSelection::Clause(ClauseSelection {
                name,
                input: ClauseSelectionInput::Context {
                    invocation: document.reference.clone(),
                    side,
                },
            }))
        }
        ClauseSelectionPartsKind::Frame {
            operation,
            invocation,
        } => {
            let document = named(observations, &invocation)?;
            if document.format != "filament.state.invocation/v1" {
                return Err(invalid("invocation", "frame requires an invocation"));
            }
            let segments = operation.split("::").collect::<Vec<_>>();
            let [model, object, operation] = segments.as_slice() else {
                return Err(invalid("operation", "expected M::T::op"));
            };
            let name = |segment: &str| {
                Identifier::new(segment).map_err(|error| invalid("operation", error.to_string()))
            };
            Ok(ClauseRunSelection::Frame {
                operation: OperationName {
                    model: name(model)?,
                    object: name(object)?,
                    operation: name(operation)?,
                },
                invocation: document.reference.clone(),
            })
        }
        ClauseSelectionPartsKind::Function {
            name,
            arguments,
            snapshot,
        } => {
            let document = named(observations, &snapshot)?;
            if document.format != "filament.state.snapshot/v1"
                || document.role.as_deref() != Some("current")
            {
                return Err(invalid("snapshot", "function requires a current snapshot"));
            }
            let arguments = arguments
                .into_iter()
                .map(|(parameter, value)| {
                    let document = quire_canonical::read(value.get().as_bytes(), u64::MAX)
                        .map_err(|error| invalid("arguments.value", error.to_string()))?;
                    let value = document.root();
                    let parsed_integer = match value.node() {
                        Node::Number(number) => number.text().parse::<i64>().ok(),
                        _ => None,
                    };
                    let value = if let Some(integer) = parsed_integer {
                        ClauseArgumentValue::Integer(integer)
                    } else if let Some(reference) = value.get("reference") {
                        let Node::Object(object) = reference.node() else {
                            return Err(invalid("arguments.value", "expected object reference"));
                        };
                        if object.len() != 2
                            || !matches!(value.node(), Node::Object(members) if members.len() == 1)
                        {
                            return Err(invalid(
                                "arguments.value",
                                "expected one object reference",
                            ));
                        }
                        let population = match reference.get("population").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid(
                                    "arguments.value.population",
                                    "expected string",
                                ))
                            }
                        };
                        let key = match reference.get("key").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => return Err(invalid("arguments.value.key", "expected string")),
                        };
                        ClauseArgumentValue::Reference {
                            population: population.to_owned(),
                            key: key.to_owned(),
                        }
                    } else {
                        return Err(invalid(
                            "arguments.value",
                            "expected integer or object reference",
                        ));
                    };
                    Ok(ClauseArgument { parameter, value })
                })
                .collect::<Result<Vec<_>, ClauseRequestReadError>>()?;
            Ok(ClauseRunSelection::Function {
                name,
                arguments,
                snapshot: document.reference.clone(),
            })
        }
        ClauseSelectionPartsKind::Temporal { name, trace, over } => {
            let document = |key: &str| -> Result<DocumentRef, ClauseRequestReadError> {
                let observation = named(observations, key)?;
                if observation.format != "filament.state.snapshot/v1"
                    || observation.role.as_deref() != Some("current")
                {
                    return Err(invalid(
                        "trace",
                        "temporal trace requires current snapshots",
                    ));
                }
                Ok(observation.reference.clone())
            };
            // A key names a snapshot, a sample is its own position.
            let position = |part: &TemporalPosition| match part {
                TemporalPosition::Key(key) => document(key),
                TemporalPosition::Sample(sample) => Ok(sample_position(sample)),
            };
            let positions = |parts: &[TemporalPosition]| -> Result<Vec<DocumentRef>, _> {
                parts.iter().map(position).collect()
            };
            let trace = match trace {
                TemporalTraceParts::Finite(parts) => TraceInput::Finite(positions(&parts)?),
                TemporalTraceParts::Lasso { prefix, repeat } => TraceInput::Lasso {
                    prefix: positions(&prefix)?,
                    repeat: positions(&repeat)?,
                },
                TemporalTraceParts::Progressed(parts) => TraceInput::Progressed {
                    trace: positions(&parts.positions)?,
                    progress: progress_assertion(&parts.progress)?,
                    lower_boundary: lower_boundary(parts.lower_boundary),
                    binding: trace_binding(parts.binding),
                },
                TemporalTraceParts::Model(parts) => TraceInput::Model {
                    initial: document(&parts.initial)?,
                    universes: parts
                        .universes
                        .iter()
                        .map(|key| document(key))
                        .collect::<Result<Vec<_>, _>>()?,
                    prefix: parts.prefix,
                    repeat: parts.repeat,
                    terminal_stutter: parts.terminal_stutter,
                    max_candidates: parts.max_candidates,
                },
                TemporalTraceParts::Timed(_) => {
                    return Err(invalid(
                        "trace.timed",
                        "a timed trace is read but not yet run: the timed clock kind is not built",
                    ))
                }
                TemporalTraceParts::Timestamped(parts) => {
                    let observations = parts
                        .observations
                        .iter()
                        .map(|(part, stamp, order_key)| {
                            Ok(TimestampedObservation {
                                document: position(part)?,
                                stamp: *stamp,
                                order_key: order_key.as_ref().map(|order| TimestampedOrderKey {
                                    authority: order.authority.clone(),
                                    key: order.key,
                                }),
                            })
                        })
                        .collect::<Result<Vec<_>, ClauseRequestReadError>>()?;
                    TraceInput::Timestamped {
                        observations,
                        progress: progress_assertion(&parts.progress)?,
                        window: Box::new(TimestampedWindow {
                            binding: Some(trace_binding(parts.binding)),
                            authoritative_origin: matches!(
                                parts.lower_boundary,
                                LowerBoundaryWire::AuthoritativeOrigin
                            ),
                            cover: parts.cover.map(|cover| match cover {
                                CoverWire::Exact => TimestampedCover::Exact,
                                CoverWire::Counter { uncertainty } => {
                                    TimestampedCover::Counter { uncertainty }
                                }
                                CoverWire::Bounded { uncertainty } => {
                                    TimestampedCover::Bounded { uncertainty }
                                }
                            }),
                            missing_valuations: parts
                                .missing
                                .iter()
                                .map(|missing| TimestampedMissingValuation {
                                    position: missing.position,
                                    atom: missing.atom,
                                })
                                .collect(),
                        }),
                    }
                }
            };
            Ok(match over {
                Some((population, key)) => ClauseRunSelection::Temporal {
                    name,
                    trace,
                    over: OverObject { population, key },
                    ledger,
                },
                None => ClauseRunSelection::SampledTemporal {
                    name,
                    trace,
                    ledger,
                },
            })
        }
    }
}

/// FR-641 `G` as the progress assertion the spine reads.
fn progress_assertion(wire: &ProgressWire) -> Result<ProgressAssertion, ClauseRequestReadError> {
    let watermark = match wire.watermark {
        WatermarkWire::Inclusive(tick) => tick,
        // FR-501 reads a half-open watermark as the inclusive `end - 1`.
        WatermarkWire::HalfOpen { half_open } => half_open.checked_sub(1).ok_or_else(|| {
            invalid(
                "trace.progress.watermark.half_open",
                "a half-open watermark of 0 covers no tick",
            )
        })?,
    };
    let closure = |closure: ClosureWire| match closure {
        ClosureWire::Closed => ProgressClosure::Closed,
        ClosureWire::Open => ProgressClosure::Open,
    };
    Ok(ProgressAssertion {
        watermark,
        completeness: match wire.completeness {
            CompletenessWire::Complete { through } => ProgressCompleteness::Complete { through },
            CompletenessWire::Incomplete => ProgressCompleteness::Incomplete,
        },
        scope: closure(wire.scope),
        execution: closure(wire.execution),
    })
}

fn lower_boundary(wire: LowerBoundaryWire) -> ProgressLowerBoundary {
    match wire {
        LowerBoundaryWire::AuthoritativeOrigin => ProgressLowerBoundary::AuthoritativeOrigin,
        LowerBoundaryWire::HistoryCutoff => ProgressLowerBoundary::HistoryCutoff,
    }
}

fn trace_binding(wire: BindingWire) -> TraceBinding {
    TraceBinding {
        profile: wire.profile,
        clock_name: wire.clock,
        parameters: wire.parameters,
    }
}

/// Reconstruct the FR-109 request from file-admitted, typed parts. The root
/// command adapter owns paths, byte ceilings and selected-file reads; this
/// function owns the FR-106 role and selection mapping and reads no path.
#[ix_trace_rs::string_edge]
pub fn read_clause_run_request(
    parts: ClauseRequestParts,
) -> Result<ClauseRunRequest, ClauseRequestReadError> {
    let selection = selection(parts.selection, &parts.observations, parts.ledger)?;
    let mut snapshots = BTreeMap::new();
    let mut invocations = BTreeMap::new();
    for observation in parts.observations.into_values() {
        match observation.format.as_str() {
            "filament.state.snapshot/v1" => {
                snapshots.insert(observation.reference, observation.bytes);
            }
            "filament.state.invocation/v1" => {
                invocations.insert(observation.reference, observation.bytes);
            }
            _ => return Err(invalid("observation", "unsupported document format")),
        }
    }
    Ok(ClauseRunRequest {
        source: parts.source,
        package_name: parts.package_name,
        packages: parts.packages,
        dependencies: parts.dependencies,
        snapshots,
        invocations,
        selection,
        limits: parts.limits,
        observation_limits: parts.observation_limits,
        accounting: parts.accounting,
        clocks: parts.clocks,
    })
}

/// FR-641 `G` as the progress assertion the spine reads.
///
/// # Errors
///
/// A half-open watermark of 0, which covers no tick.
pub fn read_progress_assertion(
    wire: &ProgressWire,
) -> Result<ProgressAssertion, ClauseRequestReadError> {
    progress_assertion(wire)
}

/// FR-641's lower boundary as the spine reads it.
pub fn read_lower_boundary(wire: LowerBoundaryWire) -> ProgressLowerBoundary {
    lower_boundary(wire)
}

/// FR-641 `T` as the trace binding the spine authenticates.
pub fn read_trace_binding(wire: BindingWire) -> TraceBinding {
    trace_binding(wire)
}

/// FR-641 `C` as the cover the spine reads.
pub fn read_cover(wire: CoverWire) -> TimestampedCover {
    match wire {
        CoverWire::Exact => TimestampedCover::Exact,
        CoverWire::Counter { uncertainty } => TimestampedCover::Counter { uncertainty },
        CoverWire::Bounded { uncertainty } => TimestampedCover::Bounded { uncertainty },
    }
}

/// A record sample's canonical JSON object text, the identity of its position.
///
/// # Errors
///
/// A sample that is not one JSON object.
pub fn read_sample_text(raw: &RawValue, member: &str) -> Result<String, ClauseRequestReadError> {
    canonical_sample(raw, member)
}

#[cfg(test)]
mod tests {
    use super::snapshot_value;
    use super::{
        selection, ClauseRunSelection, ClauseRunSelectionWire, ClauseSelectionParts,
        ClauseSelectionPartsKind, SuppliedObservation, TemporalTraceParts,
    };
    use filament_exact::CollectionKind;
    use fsl_semantics::model::observation::{ClauseSelectionInput, DocumentRef, SelectedObject};
    use fsl_semantics::model::observation::{NativeKind, RawForm, RawNativeKind};
    use std::collections::BTreeMap;

    #[test]
    fn temporal_wire_selection_reads_finite_and_lasso_key_order() {
        let finite: ClauseRunSelectionWire = serde_json::from_str(
            r#"{"temporal":{"name":"Reaches","trace":{"finite":["s0","s1"]},"over":{"reference":{"population":"P","key":"c1"}}}}"#,
        )
        .unwrap();
        let ClauseSelectionPartsKind::Temporal {
            name,
            trace: TemporalTraceParts::Finite(keys),
            over,
        } = finite.into_parts().unwrap()
        else {
            panic!("expected finite temporal selection");
        };
        assert_eq!(name, "Reaches");
        assert_eq!(keys, ["s0", "s1"]);
        assert_eq!(over, Some(("P".to_owned(), "c1".to_owned())));

        let lasso: ClauseRunSelectionWire = serde_json::from_str(
            r#"{"temporal":{"name":"Reaches","trace":{"lasso":{"prefix":["s0"],"loop":["s1","s2"]}},"over":{"reference":{"population":"P","key":"c1"}}}}"#,
        )
        .unwrap();
        let ClauseSelectionPartsKind::Temporal {
            trace: TemporalTraceParts::Lasso { prefix, repeat },
            ..
        } = lasso.into_parts().unwrap()
        else {
            panic!("expected lasso temporal selection");
        };
        assert_eq!(prefix, ["s0"]);
        assert_eq!(repeat, ["s1", "s2"]);
    }

    /// FR-641 (TC-1662, TC-1663): the `temporal` selection decodes every
    /// trace kind, record-sample positions and the `ledger` file member.
    #[test]
    #[ix_trace_rs::trace("TC-1663", "FR-641-AC-4")]
    fn temporal_wire_selection_reads_every_trace_kind_and_the_ledger() {
        let binding = r#"{"profile":"filament.temporal.event-position.false-extension/v1","clock":"events","parameters":{"kind":"event_position","sequence_authority":"events"}}"#;
        let progress = r#"{"watermark":3,"completeness":{"complete":{"through":3}},"scope":"open","execution":"open"}"#;
        let sample = r#"{"sample":{"level":{"integer":"1"}}}"#;
        for selection in [
            format!(r#"{{"temporal":{{"name":"E","trace":{{"finite":[{sample},{sample}]}}}}}}"#),
            format!(
                r#"{{"temporal":{{"name":"E","trace":{{"progressed":{{"positions":["s0","s1"],"progress":{progress},"lower_boundary":"authoritative-origin","binding":{binding}}}}},"over":{{"reference":{{"population":"P","key":"c1"}}}},"ledger":"ledger.json"}}}}"#
            ),
            format!(
                r#"{{"temporal":{{"name":"T","trace":{{"timestamped":{{"observations":[{{"position":"s0","stamp":0,"order_key":{{"authority":"a","key":1}}}},{{"position":"s1","stamp":2}}],"progress":{progress},"lower_boundary":"history-cutoff","binding":{binding},"cover":{{"counter":{{"uncertainty":1}}}},"missing":[{{"position":1,"atom":0}}]}}}},"over":{{"reference":{{"population":"P","key":"c1"}}}}}}}}"#
            ),
            format!(
                r#"{{"temporal":{{"name":"Ack","trace":{{"timed":{{"prefix":[{{"position":{sample},"stamp":"0"}}],"loop":[{{"position":{sample},"stamp":"3/2"}}],"period":"5"}}}}}}}}"#
            ),
        ] {
            let wire: Result<ClauseRunSelectionWire, _> = serde_json::from_str(&selection);
            let wire = wire.unwrap_or_else(|error| panic!("{selection}: {error}"));
            assert!(
                wire.into_parts().is_ok(),
                "{selection} names one trace kind of FR-641"
            );
        }
    }

    /// FR-641 (TC-1663), FSpec FR-364: the `model` trace reads its initial
    /// state, universes and steps; a terminal stutter step closes a loop and
    /// never a prefix.
    #[test]
    #[ix_trace_rs::trace("TC-1663", "FR-641-AC-4")]
    fn a_model_trace_reads_its_steps_and_refuses_a_misplaced_stutter() {
        let step = r#"{"transition":{"type":"transition","operation":"inc","receiver":null,"arguments":[]},"post_state":{"key":"W10"}}"#;
        let stutter = r#"{"transition":{"type":"terminal-stutter"},"post_state":{"key":"W10"}}"#;
        let model = |prefix: &str, looped: &str| {
            format!(
                r#"{{"temporal":{{"name":"E","trace":{{"model":{{"initial":"s0","universes":["s0"],"prefix":[{prefix}],"loop":[{looped}]}}}},"over":{{"reference":{{"population":"P","key":"c1"}}}}}}}}"#
            )
        };
        let read = |text: String| {
            let wire: ClauseRunSelectionWire = serde_json::from_str(&text).unwrap();
            wire.into_parts()
        };
        assert!(read(model(step, step)).is_ok());
        assert!(read(model(step, stutter)).is_ok());
        assert!(
            read(model(stutter, step)).is_err(),
            "a prefix has no stutter"
        );
        assert!(read(model(step, &format!("{stutter},{step}"))).is_err());
        let no_over =
            model(step, step).replace(r#","over":{"reference":{"population":"P","key":"c1"}}"#, "");
        assert!(read(no_over).is_err(), "a model trace needs an over");
    }

    /// FR-641-AC-7: a `model` trace's `max_candidates` is a positive integer
    /// from 1 to 2^53 - 1, 1,000,000 when absent; 0, a wrong type and a
    /// value past the bound refuse naming the member.
    #[test]
    #[ix_trace_rs::trace("TC-2246", "FR-641-AC-7")]
    fn a_model_trace_reads_its_candidate_bound() {
        let model = |bound: &str| {
            format!(
                r#"{{"temporal":{{"name":"E","trace":{{"model":{{"initial":"s0","universes":[],"prefix":[],"loop":[],{bound}}}}},"over":{{"reference":{{"population":"P","key":"c1"}}}}}}}}"#
            )
        };
        let bound = |text: String| -> Result<u64, super::ClauseRequestReadError> {
            let wire: ClauseRunSelectionWire = serde_json::from_str(&text).unwrap();
            match wire.into_parts()? {
                ClauseSelectionPartsKind::Temporal {
                    trace: TemporalTraceParts::Model(parts),
                    ..
                } => Ok(parts.max_candidates),
                _ => panic!("a model trace"),
            }
        };
        assert_eq!(bound(model(r#""max_candidates":7"#)).unwrap(), 7);
        assert_eq!(
            bound(model(r#""max_candidates":9007199254740991"#)).unwrap(),
            9_007_199_254_740_991
        );
        let absent = model("").replace(",}}", "}}");
        assert_eq!(bound(absent).unwrap(), 1_000_000);
        for refused in ["0", "9007199254740992", "\"7\"", "1.5", "-1", "true"] {
            let error = bound(model(&format!(r#""max_candidates":{refused}"#)))
                .expect_err("the bound is refused");
            assert_eq!(error.member, "trace.model.max_candidates", "{refused}");
        }
    }

    #[test]
    fn parameter_value_reader_uses_heap_stack_past_128_levels() {
        let mut value = String::from("{\"integer\":\"7\"}");
        for _ in 0..160 {
            value = format!("{{\"present\":{value}}}");
        }
        let document = quire_canonical::read(value.as_bytes(), u64::MAX).unwrap();
        let decoded = snapshot_value(document.root()).unwrap();
        let mut form = decoded.form();
        for _ in 0..160 {
            form = match form {
                RawForm::Present(inner) => inner.form(),
                other => panic!("expected present, got {other:?}"),
            };
        }
        assert!(matches!(form, RawForm::Integer("7")));
    }

    /// FR-106: a native payload object parameter keeps its `kind` and every
    /// other member as read, so admission judges a defect in the native
    /// payload order rather than the request reader refusing it.
    #[test]
    fn native_parameter_keeps_its_kind_and_members_as_read() {
        let read = |text: &str| {
            let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
            snapshot_value(document.root()).unwrap()
        };
        let uuid = read(r#"{"kind":"uuid","text":"t","extra":1}"#);
        let RawForm::Native(native) = uuid.form() else {
            panic!("expected a native object");
        };
        assert_eq!(native.kind(), RawNativeKind::Known(NativeKind::Uuid));
        assert_eq!(
            native.members().collect::<Vec<_>>(),
            [("text", Some("t")), ("extra", None)]
        );
        for (text, kind) in [
            (
                r#"{"kind":"string","text":""}"#,
                RawNativeKind::Known(NativeKind::String),
            ),
            (
                r#"{"kind":"timestamp","nanoseconds":"0"}"#,
                RawNativeKind::Known(NativeKind::Timestamp),
            ),
            (r#"{"kind":"String"}"#, RawNativeKind::Unknown),
            (r#"{"kind":1}"#, RawNativeKind::NotText),
        ] {
            let value = read(text);
            let RawForm::Native(native) = value.form() else {
                panic!("{text}: expected a native object");
            };
            assert_eq!(native.kind(), kind, "{text}");
        }
    }

    /// FR-106: a `set`, `bag` or `ordered-set` parameter keeps its kind and its
    /// items in document order, as a `sequence` does, so admission judges the
    /// kind against the declared one and decides duplicates itself.
    #[test]
    fn parameter_value_reader_keeps_the_kind_and_items_of_a_collection() {
        for (tag, kind) in [
            ("sequence", CollectionKind::Sequence),
            ("set", CollectionKind::Set),
            ("bag", CollectionKind::Bag),
            ("ordered-set", CollectionKind::OrderedSet),
        ] {
            let text =
                format!(r#"{{"{tag}":[{{"integer":"2"}},{{"integer":"1"}},{{"integer":"2"}}]}}"#);
            let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
            let decoded = snapshot_value(document.root()).unwrap();
            let RawForm::Collection(read_kind, items) = decoded.form() else {
                panic!("{tag}: a collection, not {:?}", decoded.form());
            };
            assert_eq!(read_kind, kind, "{tag}");
            let spellings: Vec<_> = items
                .iter()
                .map(|item| match item.form() {
                    RawForm::Integer(spelling) => spelling,
                    other => panic!("{tag}: an integer, not {other:?}"),
                })
                .collect();
            assert_eq!(spellings, ["2", "1", "2"], "{tag}");
        }
    }

    /// FR-360 item 9: a `variant` parameter keeps the members it was read
    /// with, so admission can refuse a missing or an extra one in its own
    /// order.
    #[test]
    fn parameter_value_reader_keeps_the_members_of_a_variant() {
        for (text, type_identity, case, extra_member) in [
            (
                r#"{"variant":{"type":"ix://p/Tier","case":"Gold"}}"#,
                Some("ix://p/Tier"),
                Some("Gold"),
                false,
            ),
            (
                r#"{"variant":{"type":"ix://p/Tier"}}"#,
                Some("ix://p/Tier"),
                None,
                false,
            ),
            (r#"{"variant":{"case":"Gold"}}"#, None, Some("Gold"), false),
            (
                r#"{"variant":{"type":"ix://p/Tier","case":"Gold","x":1}}"#,
                Some("ix://p/Tier"),
                Some("Gold"),
                true,
            ),
        ] {
            let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
            let decoded = snapshot_value(document.root()).unwrap();
            let RawForm::Variant(variant) = decoded.form() else {
                panic!("a variant, not {:?}", decoded.form());
            };
            assert_eq!(variant.type_identity(), type_identity, "{text}");
            assert_eq!(variant.case(), case, "{text}");
            assert_eq!(variant.has_extra_member(), extra_member, "{text}");
        }
    }

    fn pre_snapshot() -> BTreeMap<String, SuppliedObservation> {
        let reference = DocumentRef {
            authority: "a".into(),
            identity: "i".into(),
        };
        BTreeMap::from([(
            "pre".to_owned(),
            SuppliedObservation {
                reference,
                format: "filament.state.snapshot/v1".into(),
                role: Some("pre".into()),
                bytes: Vec::new(),
            },
        )])
    }

    fn pre_call_self(self_object: Option<SelectedObject>) -> Option<SelectedObject> {
        let parts = ClauseSelectionParts {
            name: "Machine::op::requires".into(),
            observation: "pre".into(),
            self_object,
            anchor: None,
            parameters: Some(BTreeMap::new()),
        };
        let Ok(ClauseRunSelection::Clause(clause)) = selection(
            ClauseSelectionPartsKind::Clause(parts),
            &pre_snapshot(),
            None,
        ) else {
            panic!("a pre snapshot selection reads");
        };
        let ClauseSelectionInput::PreCall { self_object, .. } = clause.input else {
            panic!("a pre snapshot selects PreCall");
        };
        self_object
    }

    /// FR-106: a state machine's precondition has no object identity, so its
    /// pre-call selection reads with no `self`; an object receiver's `self`
    /// is carried through for admission to judge against the receiver.
    #[test]
    fn pre_call_selection_reads_with_and_without_self() {
        assert_eq!(pre_call_self(None), None);
        let object = SelectedObject {
            population: "p".into(),
            key: "k".into(),
        };
        assert_eq!(pre_call_self(Some(object.clone())), Some(object));
    }
}
