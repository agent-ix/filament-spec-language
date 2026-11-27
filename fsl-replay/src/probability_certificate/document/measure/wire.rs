// SPDX-License-Identifier: AGPL-3.0-or-later
//! The finite-horizon measure document's one declared wire type.
//!
//! `WireDocument` and its member structs are declared once. The strict reader
//! deserializes them with owned leaves (`ReadDocument`); the writer fills the same
//! structs with borrowed views over the domain document and emits them member by
//! member, so rows are never materialised and every string and integer still
//! passes through `writer::Text` for cancellation and storage admission.

use super::*;
use quire_canonical::{Encode, Error, Sink, Writer};
use serde::Deserialize;
use serde_json::value::RawValue;

/// `kind`, `window`, `reward`, `unit`, `comparison`, `threshold`, `level`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireMeasure<Str, Rw, U, Rat> {
    pub(super) kind: Str,
    pub(super) window: u64,
    pub(super) reward: Option<Rw>,
    pub(super) unit: U,
    pub(super) comparison: Option<Str>,
    pub(super) threshold: Option<Rat>,
    pub(super) level: Option<Rat>,
}
impl<Str: Emit, Rw: Emit, U: Emit, Rat: Emit> Emit for WireMeasure<Str, Rw, U, Rat> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        member(writer, "kind", &self.kind)?;
        member(writer, "window", &self.window)?;
        member(writer, "reward", &self.reward)?;
        member(writer, "unit", &self.unit)?;
        member(writer, "comparison", &self.comparison)?;
        member(writer, "threshold", &self.threshold)?;
        member(writer, "level", &self.level)?;
        writer.end_object()
    }
}

/// `profile`, `payoff`, `measure`, `optimization`, `fairness`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireObjective<Str, M, F> {
    pub(super) profile: Str,
    pub(super) payoff: Str,
    pub(super) measure: M,
    pub(super) optimization: Str,
    pub(super) fairness: F,
}
impl<Str: Emit, M: Emit, F: Emit> Emit for WireObjective<Str, M, F> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        member(writer, "profile", &self.profile)?;
        member(writer, "payoff", &self.payoff)?;
        member(writer, "measure", &self.measure)?;
        member(writer, "optimization", &self.optimization)?;
        member(writer, "fairness", &self.fairness)?;
        writer.end_object()
    }
}

/// `mode` and, for a ratio, its `numerator` and `denominator` parts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConditional<Str, D> {
    pub(super) mode: Str,
    pub(super) numerator: Option<D>,
    pub(super) denominator: Option<D>,
}
impl<Str: Emit, D: Emit> Emit for WireConditional<Str, D> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        member(writer, "mode", &self.mode)?;
        if let Some(numerator) = &self.numerator {
            member(writer, "numerator", numerator)?;
        }
        if let Some(denominator) = &self.denominator {
            member(writer, "denominator", denominator)?;
        }
        writer.end_object()
    }
}

/// `identity`, `objective`, `kind`, `values`, `ranking` and, for a conditional
/// document, `conditional`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireDocument<Str, I, O, V, R, C> {
    pub(super) identity: I,
    pub(super) objective: O,
    pub(super) kind: Str,
    pub(super) values: V,
    pub(super) ranking: R,
    pub(super) conditional: Option<C>,
}
impl<Str: Emit, I: Emit, O: Emit, V: Emit, R: Emit, C: Emit> Emit
    for WireDocument<Str, I, O, V, R, C>
{
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        member(writer, "identity", &self.identity)?;
        member(writer, "objective", &self.objective)?;
        member(writer, "kind", &self.kind)?;
        member(writer, "values", &self.values)?;
        member(writer, "ranking", &self.ranking)?;
        if let Some(conditional) = &self.conditional {
            member(writer, "conditional", conditional)?;
        }
        writer.end_object()
    }
}

/// The reader's owned instantiation of the declared type.
pub(super) type ReadObjective = WireObjective<
    String,
    WireMeasure<String, (Box<RawValue>, String), Box<RawValue>, WireRational>,
    Vec<Box<RawValue>>,
>;
pub(super) type ReadBody = WireDocument<
    String,
    WireIdentity,
    ReadObjective,
    Vec<WireValue>,
    Vec<WireRank>,
    WireConditional<String, Box<ReadDocument>>,
>;
#[derive(Deserialize)]
#[serde(transparent)]
pub(super) struct ReadDocument(pub(super) ReadBody);

/// Borrowed leaves: each emits one member value through `Text` admission.
type Text<'a, 't> = &'a writer::Text<'t>;
pub(super) struct Node<'a>(&'a filament_exact::NodeKey);
impl Emit for Node<'_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        fsl_foundation::node_wire::encode_node_key(self.0, writer)
    }
}
struct Int<'a, 't>(Text<'a, 't>, &'a Integer);
impl Emit for Int<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.0.integer(self.1, writer)
    }
}
struct Rat<'a, 't>(Text<'a, 't>, &'a Rational);
impl Emit for Rat<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        WireRational {
            numerator: Int(self.0, self.1.numerator()),
            denominator: Int(self.0, self.1.denominator()),
        }
        .emit(writer)
    }
}
struct State<'a, 't>(Text<'a, 't>, &'a ProbabilityProductKey, bool);
impl Emit for State<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.0.key(self.1, writer, self.2)
    }
}
struct Unit<'a, 't>(Text<'a, 't>, Option<&'a filament_exact::UnitId>);
impl Emit for Unit<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.0.unit(self.1, writer)
    }
}
struct Fairness<'a, 't>(Text<'a, 't>, &'a [fairness::Record]);
impl Emit for Fairness<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        for constraint in self.1 {
            self.0.poll()?;
            constraint.encode_into(writer)?;
        }
        writer.end_array()
    }
}
struct Values<'a, 't>(Text<'a, 't>, &'a [ExactProbabilityValue]);
impl Emit for Values<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        for (index, row) in self.1.iter().enumerate() {
            WireValue {
                state: State(self.0, &row.state, index != 0),
                value: Rat(self.0, &row.value),
            }
            .emit(writer)?;
        }
        writer.end_array()
    }
}
struct Ranks<'a, 't>(Text<'a, 't>, &'a [(ProbabilityProductKey, u64)]);
impl Emit for Ranks<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        for (index, (state, rank)) in self.1.iter().enumerate() {
            WireRank {
                state: State(self.0, state, index != 0),
                rank: *rank,
            }
            .emit(writer)?;
        }
        writer.end_array()
    }
}

/// A nested part of a ratio, written with its own `Text` over the same limits.
pub(super) struct Events<'d, 'c> {
    pub(super) document: &'d MeasureProbabilityDocument,
    pub(super) text: writer::Text<'c>,
}
impl Emit for Events<'_, '_> {
    fn emit<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        let document = self.document;
        let text = &self.text;
        let objective = &document.objective;
        fn nested<'d, 'c>(
            part: &'d MeasureProbabilityDocument,
            text: &writer::Text<'c>,
        ) -> Events<'d, 'c> {
            Events {
                document: part,
                text: writer::Text::new(text.limits(), text.cancel()),
            }
        }
        WireDocument {
            identity: WireIdentity {
                node_id: Node(document.identity.node()),
                model: document.identity.model().map(|model| model.as_str()),
                ordinal: document.identity.ordinal(),
            },
            objective: WireObjective {
                profile: PROFILE,
                payoff: document.payoff.text(),
                measure: WireMeasure {
                    kind: objective.kind,
                    window: objective.window,
                    reward: objective
                        .reward
                        .as_ref()
                        .map(|reward| (Node(&reward.model), reward.name.as_str())),
                    unit: Unit(
                        text,
                        objective
                            .reward
                            .as_ref()
                            .and_then(|reward| reward.unit.as_ref()),
                    ),
                    comparison: objective.comparison.map(|comparison| match comparison {
                        MeasureComparison::Le => "le",
                        MeasureComparison::Lt => "lt",
                        MeasureComparison::Ge => "ge",
                    }),
                    threshold: objective.threshold.as_ref().map(|value| Rat(text, value)),
                    level: objective.level.as_ref().map(|value| Rat(text, value)),
                },
                optimization: match objective.optimization {
                    ProbabilityObjective::Minimum => "minimum",
                    ProbabilityObjective::Maximum => "maximum",
                    ProbabilityObjective::Workload => "workload",
                    ProbabilityObjective::WorkloadMinimum => "workload-minimum",
                    ProbabilityObjective::WorkloadMaximum => "workload-maximum",
                },
                fairness: Fairness(text, &objective.fairness),
            },
            kind: if document.conditional.is_some() {
                "conditional"
            } else {
                "exact"
            },
            values: Values(text, &document.values),
            ranking: Ranks(text, &document.ranking),
            conditional: document
                .conditional
                .as_deref()
                .map(|conditional| match conditional {
                    Conditional::Vacuous => WireConditional {
                        mode: "vacuous",
                        numerator: None,
                        denominator: None,
                    },
                    Conditional::Ratio(numerator, denominator) => WireConditional {
                        mode: "ratio",
                        numerator: Some(nested(numerator, text)),
                        denominator: Some(nested(denominator, text)),
                    },
                }),
        }
        .emit(writer)
    }
}
impl Encode for Events<'_, '_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.emit(writer)
    }
}
