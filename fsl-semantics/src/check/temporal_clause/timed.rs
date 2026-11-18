// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-234: a `temporal` clause under the timed profile
//! `filament.temporal.timed/v1` (ADR-026 DF-1 to DF-7).
//!
//! Over a model subject the timed profile is selected by a `model-time`
//! clock binding (ADR-026 CB-1; FSpec FR-250-AC-7), so a clause under it
//! binds `model-time` over a timed model first: FR-233's binding, read as far
//! as FR-234 needs it. `clock "model-time"` over a model with no time source
//! refuses `invalid_timed_model`/`model-time-on-untimed-model`; `clock
//! "model-steps"` contradicts the profile and any other name is no model
//! binding, both refused with the clause's clock (FR-233). Over a record trace
//! subject the clock is a role bound by its `timed` configuration (FR-642,
//! FR-933): the configured unit is the binding unit, and with no configuration
//! the check refuses `missing-clock-selection`; `model-time` and `model-steps`
//! there refuse as FR-642 and FR-610 "Model binding names" state.
//!
//! Each timed interval's bounds are time quantities converted exactly to the
//! binding's unit (FR-233 "Intervals", through FR-230's time units), then
//! validated into a [`TimedInterval`]: a negative lower bound, `a > b` and an
//! open end with `a = b` refuse `invalid_timed_model`/`invalid-interval` at
//! the interval. A step interval `[a, b]` has bounds with no unit and refuses
//! `ill_typed`/`type-mismatch`, `[a, *]` included. A timed interval `[q, *]`
//! has a least distance and no upper bound (FR-943); `[0 ms, *]` is the operator
//! written with no interval.
//!
//! The clause is classified into one [`TimedPropertyForm`], in FR-234's
//! order, and a `TimedInvariant`, `TimedSafety` or `TimedLiveness` clause
//! holding a punctual interval records that interval's span: the request
//! writer disposes its item `Unsupported`, `PunctualInterval` (DF-5).

use filament_contract_model::ClockConfiguration;
use filament_exact::{Integer, Rational, ValueType};
use fsl_forms::{QuantityForm, TimedIntervalForm, TimedUpper};
use fsl_foundation::bound::TimedInterval;
use fsl_foundation::diagnostic::{LimitExceeded, StageFailure};
use fsl_foundation::source::node_table::{
    standalone, DebugTable, DebugWith, NodeKeyDebug, NodeList, With,
};
use fsl_foundation::{Setting, Span};

use super::{
    in_safety_fragment, is_temporal_node, located, Activation, CheckedFormula, CheckedInterval,
    FormulaNode, TemporalDeclarations, UnaryTemporal,
};
use crate::check::lowering::AdmittedModel;
use crate::check::probabilistic::{exact_value, ExactFault};
use crate::check::refusal::{CheckRefusal, ClockSelectionFault, TemporalCause, TimeUnitCause};
use crate::check::time_units::TimeScale;

/// The clock role that binds `model-time` (ADR-026 CB-1).
pub(crate) const MODEL_TIME: &str = "model-time";

/// The clock role that binds a model's steps (FR-233).
const MODEL_STEPS: &str = "model-steps";

/// The kind of a timed model's time source (FR-230 `TimeSource`, FR-233
/// `TimeSourceKind`).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TimeSourceKind {
    /// `time dense unit u`.
    Dense,
    /// `time tick O period p unit u`.
    Tick {
        /// The tick operation `O`, as resolved.
        operation: String,
        /// The period `p`, an exact positive rational in the unit.
        period: Rational,
    },
}

/// The checked time source of one timed model (FR-230's `TimedModel`, as
/// far as a `model-time` binding reads it).
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ModelTimeSource {
    /// The model: the alias of the `model` declaration that selects its
    /// domain package, or the package identity when no alias names it.
    pub model: String,
    /// The complete source `time` declaration, for FR-322 occurrence mapping.
    pub span: Span,
    /// The model node of the selected package's one object type, the state
    /// object: the timed root's and every clock place's `model` (FSpec FR-322
    /// "Timed-model nodes", FR-415-AC-17).
    pub model_node: filament_exact::NodeKey,
    /// The FCD declaration key of a tick operation, used by S4 to build its
    /// checked operation-anchor node.
    pub tick_operation_key: Option<filament_checked::key::DeclarationKey>,
    /// The source span of the qualified tick-operation reference.
    pub tick_operation_span: Option<fsl_foundation::Span>,
    /// The checked `operation_anchor` node for the resolved tick operation.
    pub tick_operation_anchor: Option<filament_exact::NodeKey>,
    /// The source kind.
    pub source: TimeSourceKind,
    /// The time unit `u`, by its declared name in the package's unit graph.
    pub unit: String,
    /// The resolved unit node, retained for checked-package emission.
    pub unit_node: filament_exact::NodeKey,
    /// The exact nominal `Time` dimension node selected by `unit` (FR-230-AC-33).
    pub time_dimension: filament_exact::NodeKey,
    /// Nominal dimension and unit nodes needed by the timed root, including
    /// the resolved unit's target chain.
    pub nominal_nodes: Vec<TimeNominalNode>,
}

impl DebugWith for ModelTimeSource {
    fn fmt_with(&self, f: &mut std::fmt::Formatter<'_>, table: &DebugTable) -> std::fmt::Result {
        let Self {
            model,
            span,
            model_node,
            tick_operation_key,
            tick_operation_span,
            tick_operation_anchor,
            source,
            unit,
            unit_node,
            time_dimension,
            nominal_nodes,
        } = self;
        f.debug_struct("ModelTimeSource")
            .field("model", &model)
            .field("span", &span)
            .field("model_node", &NodeKeyDebug(model_node, table))
            .field("tick_operation_key", &tick_operation_key)
            .field("tick_operation_span", &tick_operation_span)
            .field(
                "tick_operation_anchor",
                &tick_operation_anchor
                    .as_ref()
                    .map(|key| NodeKeyDebug(key, table)),
            )
            .field("source", &source)
            .field("unit", &unit)
            .field("unit_node", &NodeKeyDebug(unit_node, table))
            .field("time_dimension", &NodeKeyDebug(time_dimension, table))
            .field("nominal_nodes", &With(nominal_nodes, table))
            .finish()
    }
}

impl ModelTimeSource {
    /// Whether this is the time source of `model`: the model of its alias
    /// whose state object is its model node. Two units may bind one alias to
    /// two packages (FR-480 "Selections and imports"), so the alias alone
    /// does not name the model.
    pub(crate) fn times(&self, model: &AdmittedModel) -> bool {
        (model.alias() == Some(self.model.as_str()) || model.alias() == Some(""))
            && model
                .object_type_model_node_keys()
                .contains(&self.model_node)
    }
}

impl std::fmt::Debug for ModelTimeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        standalone(f, self)
    }
}

/// The form of a nominal unit-graph node: a dimension or a unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TimeNominalForm {
    /// A dimension.
    Dimension,
    /// A unit.
    Unit,
}

impl TimeNominalForm {
    /// The checked wire's `semantic_form` word for the form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dimension => "dimension",
            Self::Unit => "unit",
        }
    }
}

/// A nominal unit-graph node that S4 must include when a timed model names it.
#[derive(Clone, Eq, PartialEq)]
pub struct TimeNominalNode {
    /// The exact nominal node identity.
    pub key: filament_exact::NodeKey,
    /// Whether the node is a dimension or a unit.
    pub form: TimeNominalForm,
    /// Who declares the node, and where.
    pub origin: TimeNominalOrigin,
    /// Nominal nodes this node names.
    pub dependencies: Vec<filament_exact::NodeKey>,
    /// The terms, target and exact conversion of a dimension or unit node,
    /// for source-owned and model-selected nodes alike.
    pub nominal: Option<crate::value::nominal_payload::NominalPayload>,
}

impl DebugWith for TimeNominalNode {
    fn fmt_with(&self, f: &mut std::fmt::Formatter<'_>, table: &DebugTable) -> std::fmt::Result {
        let Self {
            key,
            form,
            origin,
            dependencies,
            nominal,
        } = self;
        f.debug_struct("TimeNominalNode")
            .field("key", &NodeKeyDebug(key, table))
            .field("form", &form)
            .field("origin", &origin)
            .field("dependencies", &NodeList(dependencies.iter(), table))
            .field("nominal", &nominal)
            .finish()
    }
}

impl std::fmt::Debug for TimeNominalNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        standalone(f, self)
    }
}

impl std::hash::Hash for TimeNominalNode {
    /// Hashes the identity fields; the payload is a function of them, so
    /// equal nodes still hash equal.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.form.hash(state);
        self.origin.hash(state);
        self.dependencies.hash(state);
    }
}

/// The declaring side of a [`TimeNominalNode`]: what an owner, a
/// `declaration` and a source region require of each other (FR-322).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TimeNominalOrigin {
    /// A node of the checked unit's own unit graph: owned by the unit's
    /// source (whose `package_name` is the manifest package name), with a
    /// declaration, located at the declared name.
    Source {
        /// The unit's source owner.
        owner: crate::value::semantic_node::OwnerSubject,
        /// The declaration's qualified name segments.
        qualified_declaration: Vec<String>,
        /// The span of the declared name in the checked source, where the
        /// node's `declaration` occurrence is located (FR-322).
        name_span: fsl_foundation::Span,
    },
    /// A node a selected domain package declares (its `Time` dimension or a
    /// unit of the `Time` dimension): owned by that model (`identity` the
    /// package identity, `node` the declaration's IR node identity), with
    /// no declaration and no source form of its own (FR-322, AC-105).
    Model(crate::value::semantic_node::ModelSubject),
}

impl TimeNominalNode {
    /// The facts the node carries beyond its key: a dimension's terms, or a
    /// unit's dimension, target and exact `factor`/`offset`.
    pub fn nominal_payload(&self) -> Option<&crate::value::nominal_payload::NominalPayload> {
        self.nominal.as_ref()
    }

    /// The declaration's qualified name segments (FR-322): `Some` for a
    /// source-declared node, `None` for a model-owned node.
    pub fn declaration(&self) -> Option<&[String]> {
        match &self.origin {
            TimeNominalOrigin::Source {
                qualified_declaration,
                ..
            } => Some(qualified_declaration),
            TimeNominalOrigin::Model(_) => None,
        }
    }

    /// The span of the declared name, for a source-declared node.
    pub fn name_span(&self) -> Option<fsl_foundation::Span> {
        match &self.origin {
            TimeNominalOrigin::Source { name_span, .. } => Some(*name_span),
            TimeNominalOrigin::Model(_) => None,
        }
    }
}

/// A clause's `model-time` binding (FR-233's `ModelClockBinding::ModelTime`):
/// the model, its time source kind and its unit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ModelTimeBinding {
    /// The model the clause's subject belongs to.
    pub model: String,
    /// The model's time source kind.
    pub source: TimeSourceKind,
    /// The unit every interval bound converts to.
    pub unit: String,
}

/// A timed clause's property form (FR-234 "Forms"; ADR-026 DF-4, TT-1 to
/// TT-4).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TimedPropertyForm {
    /// TT-1: `always holds(I)`.
    TimedInvariant,
    /// TT-2: activation `on origin` with only interval operators; `horizon`
    /// is the sum of the formula's upper bounds, in the binding's unit.
    BoundedWindow {
        /// The sum of the formula's upper bounds.
        horizon: Rational,
    },
    /// TT-3: a formula in ADR-014 A-4's safety fragment.
    TimedSafety,
    /// TT-4: any other admitted formula.
    TimedLiveness,
}

/// What the timed check reads of the package beyond the clause.
pub(crate) struct TimedReader<'a> {
    pub(crate) declarations: Option<&'a TemporalDeclarations<'a>>,
    pub(crate) units: &'a crate::check::time_units::TimeUnits,
    pub(crate) rational_bits: u64,
    pub(crate) target: TimeScale,
    /// Whether a unit-free bound is a magnitude in the binding unit (a
    /// clock role over a record trace subject, FR-933 "Bounds"); under
    /// `model-time` such a bound refuses `ill_typed`/`type-mismatch`.
    pub(crate) unit_free: bool,
}

/// The model a clause over the object type `object` binds, and its time
/// source when the model is timed.
fn model_of<'a>(
    declarations: &'a TemporalDeclarations<'_>,
    object: filament_exact::EffectiveId,
) -> Option<(String, Option<&'a ModelTimeSource>)> {
    let model = declarations
        .models
        .iter()
        .find(|model| model.declaration_key(object.clone()).is_some())?;
    let name = model
        .alias()
        .map_or_else(|| model.selection().identity.clone(), str::to_owned);
    let source = declarations
        .timed
        .time_sources
        .iter()
        .find(|source| source.times(model));
    Some((name, source))
}

/// Whether a clause whose `over` parameter has type `over` is over a timed
/// model, one with a `time` member of either source, `dense` or `tick`
/// (FR-243): its item is EN-6's, under any profile.
pub(crate) fn over_timed(over: &ValueType, declarations: &TemporalDeclarations<'_>) -> bool {
    let ValueType::Reference(object) = over else {
        return false;
    };
    model_of(declarations, object.clone())
        .and_then(|(_, source)| source)
        .is_some()
}

/// How a clause under the timed profile binds its clock (FR-233, FR-642):
/// `model-time` over a timed model, or a clock role over a record trace
/// subject bound by the role's `timed` configuration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TimedBinding {
    /// `model-time` over a timed model.
    Model(ModelTimeBinding),
    /// A clock role over a record trace subject, read in `unit`, the
    /// declared time unit its `timed` configuration names (FR-933).
    Role {
        /// The configured unit's declared name.
        unit: String,
    },
}

impl TimedBinding {
    /// The binding unit every interval bound converts to.
    pub(crate) fn unit(&self) -> &str {
        match self {
            Self::Model(binding) => &binding.unit,
            Self::Role { unit } => unit,
        }
    }
}

/// The clause's timed clock binding under the timed profile (FR-233,
/// ADR-026 CB-1, FR-642, FR-933), or the refusal its clock role, subject and
/// clock configuration give. `name` is the clause's declared name.
pub(crate) fn binding(
    clock: &fsl_forms::ClockForm,
    over: &ValueType,
    declarations: &TemporalDeclarations<'_>,
    name: &str,
    clause: Span,
) -> Result<TimedBinding, TemporalCause> {
    let ValueType::Reference(object) = over else {
        return role_binding(clock, over, declarations, name, clause);
    };
    let Some((model, source)) = model_of(declarations, object.clone()) else {
        return Err(TemporalCause::TimedTraceSubject { span: clause });
    };
    match (clock.text.as_str(), source) {
        (MODEL_TIME, Some(source)) => Ok(TimedBinding::Model(ModelTimeBinding {
            model,
            source: source.source.clone(),
            unit: source.unit.clone(),
        })),
        (MODEL_TIME, None) => Err(TemporalCause::ModelTimeOnUntimedModel {
            model,
            span: clock.span,
        }),
        // The clause's clock refused every other name over a model subject
        // before the binding is read (FR-233 "Other names").
        (other, _) => Err(TemporalCause::ModelClockName {
            clock: other.to_owned(),
            span: clock.span,
        }),
    }
}

/// FR-642 "Checking", FR-933 "The binding unit": a clause over a record
/// trace subject binds its clock role through the role's `timed`
/// configuration, whose `unit` names a declared unit of the `Time`
/// dimension. No unit is taken from the bounds, the trace or a default.
fn role_binding(
    clock: &fsl_forms::ClockForm,
    over: &ValueType,
    declarations: &TemporalDeclarations<'_>,
    name: &str,
    clause: Span,
) -> Result<TimedBinding, TemporalCause> {
    if !matches!(over, ValueType::Composite(_)) {
        return Err(TemporalCause::TimedTraceSubject { span: clause });
    }
    let role = clock.text.as_str();
    match role {
        MODEL_TIME => {
            return Err(TemporalCause::ModelTimeOnTraceSubject { span: clock.span });
        }
        MODEL_STEPS => {
            // FR-610 "Model binding names": a trace subject has no model
            // steps to bind.
            return Err(TemporalCause::ClockRoleSpellsTick {
                role: role.to_owned(),
                profile: super::TemporalProfile::Timed,
                span: clock.span,
            });
        }
        _ => {}
    }
    let selection = declarations
        .timed
        .clock_units
        .iter()
        .find(|selected| selected.declaration_span == clause)
        .map(|selected| &selected.clock);
    let fault = |fault: ClockSelectionFault| TemporalCause::ClockSelection {
        fault,
        clock: role.to_owned(),
        declaration: name.to_owned(),
        profile: super::TemporalProfile::Timed,
        span: clock.span,
    };
    let unit = match selection {
        None => return Err(fault(ClockSelectionFault::MissingClockSelection)),
        Some(ClockConfiguration::Timed { unit }) => unit.to_string(),
        Some(other) => {
            return Err(fault(ClockSelectionFault::ClockKindMismatch {
                supplied: configuration_kind(other),
            }))
        }
    };
    match declarations.timed.units.target(&unit) {
        Ok(Some(_)) => Ok(TimedBinding::Role { unit }),
        Ok(None) => Err(fault(ClockSelectionFault::ClockUnitDimension { unit })),
        Err(TimeUnitCause::UnknownUnit { .. } | TimeUnitCause::NoTimeDimension { .. }) => {
            Err(fault(ClockSelectionFault::UnknownClockUnit { unit }))
        }
        Err(
            TimeUnitCause::NotTimeDimension { .. }
            | TimeUnitCause::AffineUnit { .. }
            | TimeUnitCause::OtherTimeDimension { .. },
        ) => Err(fault(ClockSelectionFault::ClockUnitDimension { unit })),
    }
}

/// The `kind` tag of `clock`'s wire form (FR-640).
fn configuration_kind(clock: &ClockConfiguration) -> &'static str {
    match clock {
        ClockConfiguration::EventPosition { .. } => "event_position",
        ClockConfiguration::FixedSample { .. } => "fixed_sample",
        ClockConfiguration::TimestampedEvent { .. } => "timestamped_event",
        ClockConfiguration::Timed { .. } => "timed",
    }
}

/// Why a timed interval did not check: a cause that joins the clause's
/// refusals, or a failure that stops the check (a limit or a malformed
/// exact value).
pub(crate) enum IntervalFailure {
    Cause(TemporalCause),
    Stop(StageFailure<CheckRefusal>),
}

impl TimedReader<'_> {
    /// One bound converted exactly to the binding's unit.
    fn bound(&self, quantity: &QuantityForm) -> Result<Rational, IntervalFailure> {
        let ceiling = self.rational_bits;
        let magnitude = exact_value(&quantity.magnitude, ceiling).map_err(|fault| match fault {
            ExactFault::ZeroDenominator => IntervalFailure::Cause(TemporalCause::ZeroDenominator {
                span: quantity.magnitude.span,
            }),
            ExactFault::AboveCeiling { bits } => {
                let limit = LimitExceeded::new(Setting::S3RationalBits, ceiling, u128::from(bits));
                IntervalFailure::Stop(match self.declarations {
                    Some(declarations) => located(limit, declarations, quantity.magnitude.span),
                    None => StageFailure::Limit(limit),
                })
            }
        })?;
        self.units
            .convert(&magnitude, &quantity.unit.name, &self.target)
            .map_err(|cause| {
                IntervalFailure::Cause(TemporalCause::TimeQuantity {
                    cause: Box::new(cause),
                    span: quantity.unit.span,
                })
            })
    }

    /// The timed interval the unit-free step interval `written` denotes in
    /// the binding unit: each bound the magnitude it writes (FR-933
    /// "Bounds"), read when [`Self::unit_free`] holds.
    pub(crate) fn unit_free_interval(
        written: &fsl_forms::IntervalForm,
    ) -> Result<TimedInterval, TemporalCause> {
        let magnitude = |value: u64| Rational::from_integer(Integer::from(value));
        let fsl_forms::IntervalUpper::Finite(upper) = written.upper else {
            return TimedInterval::unbounded(magnitude(written.lower), true).map_err(|reason| {
                TemporalCause::InvalidTimedInterval {
                    reason,
                    span: written.span,
                }
            });
        };
        TimedInterval::new(magnitude(written.lower), true, magnitude(upper), true).map_err(
            |reason| TemporalCause::InvalidTimedInterval {
                reason,
                span: written.span,
            },
        )
    }

    /// The validated interval `written` denotes in the binding's unit.
    pub(crate) fn interval(
        &self,
        written: &TimedIntervalForm,
    ) -> Result<TimedInterval, IntervalFailure> {
        let lower = self.bound(&written.lower)?;
        let TimedUpper::Finite(upper) = &written.upper else {
            // `[q, *]` (FR-943): a least distance and no upper bound.
            return TimedInterval::unbounded(lower, written.lower_closed).map_err(|reason| {
                IntervalFailure::Cause(TemporalCause::InvalidTimedInterval {
                    reason,
                    span: written.span,
                })
            });
        };
        let upper = self.bound(upper)?;
        TimedInterval::new(lower, written.lower_closed, upper, written.upper_closed).map_err(
            |reason| {
                IntervalFailure::Cause(TemporalCause::InvalidTimedInterval {
                    reason,
                    span: written.span,
                })
            },
        )
    }
}

/// The timed interval a node of `formula` carries, if any.
fn timed(node: &FormulaNode) -> Option<&TimedInterval> {
    match node {
        FormulaNode::Unary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        }
        | FormulaNode::Binary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        } => Some(interval),
        FormulaNode::Unary { .. }
        | FormulaNode::Binary { .. }
        | FormulaNode::Constant(_)
        | FormulaNode::Holds { .. }
        | FormulaNode::Aggregate { .. }
        | FormulaNode::Not(_)
        | FormulaNode::And(..)
        | FormulaNode::Or(..)
        | FormulaNode::Implies(..) => None,
    }
}

/// The clause's timed property form (FR-234 "Forms"), in the order FR-234
/// states.
pub(crate) fn property_form(formula: &CheckedFormula, activation: Activation) -> TimedPropertyForm {
    let nodes = formula.nodes();
    let invariant = match nodes.get(formula.root().index()) {
        Some(FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval: None,
            operand,
        }) => matches!(nodes.get(operand.index()), Some(FormulaNode::Holds { .. })),
        _ => false,
    };
    if invariant {
        return TimedPropertyForm::TimedInvariant;
    }
    let only_interval_operators = nodes
        .iter()
        .filter(|node| is_temporal_node(node))
        .all(|node| timed(node).is_some_and(TimedInterval::is_bounded));
    if activation == Activation::Origin && only_interval_operators {
        let horizon = nodes
            .iter()
            .filter_map(timed)
            .filter_map(TimedInterval::upper)
            .fold(Rational::from_integer(Integer::zero()), |sum, upper| {
                sum.add(upper)
            });
        return TimedPropertyForm::BoundedWindow { horizon };
    }
    if in_safety_fragment(formula) {
        TimedPropertyForm::TimedSafety
    } else {
        TimedPropertyForm::TimedLiveness
    }
}

/// The span of the first punctual interval, in arena order, of a clause
/// DF-5 does not admit over a timed subject: one whose form is not
/// `BoundedWindow` (FR-234 "Punctuality").
pub(crate) fn unsupported_punctual(form: &TimedPropertyForm, punctual: &[Span]) -> Option<Span> {
    match form {
        TimedPropertyForm::BoundedWindow { .. } => None,
        TimedPropertyForm::TimedInvariant
        | TimedPropertyForm::TimedSafety
        | TimedPropertyForm::TimedLiveness => punctual.first().copied(),
    }
}
