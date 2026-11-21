// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Clock-reading declarations": a function or predicate of the
//! unit's own source whose parameter or result is clock-bearing.
//!
//! Such a declaration has no checked node. [`check_declaration`] checks its
//! body by "Clock constraints and guards": a clock parameter is read only as
//! the clock operand of an atomic clock constraint whose bound is a time
//! quantity literal or a quantity parameter, as an argument for a clock
//! parameter of another clock-reading declaration, or as a branch of a
//! place expression; and a declaration whose result is `Clock` has a place
//! expression as its body.
//!
//! [`Expansion`] expands a call where a precondition reads it. The
//! arguments are read at the call site: a clock argument as the places it
//! can denote, each with the data condition that selects it; `self` for an
//! object parameter as the receiver; and a data argument as the call-site
//! term it is. The body is then read with its parameters bound to those
//! arguments, into the disjunctive form the precondition's guard takes:
//! each data predicate of the body is read at the call, its parameters
//! replaced by their arguments, so the recorded guard holds atomic clock
//! constraints over the receiver's clocks only.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use filament_exact::{EffectiveId, Integer, NonZeroInteger, Rational};
use fsl_forms::{
    BinaryOperator, BinderQuery, ExprId, ExprNode, Expression, ExpressionForm, ExpressionSpans,
    NameForm, TypeFormHead,
};
use fsl_foundation::source::provenance::RawSourceRef;
use fsl_foundation::Span;

use super::{
    timed, ClockConstraint, ClockId, ClockOp, Clocks, DataLiteral, Dnf, Located, PartBudget,
};
use crate::check::assemble::{ClockReadingDeclaration, ClockSlot};
use crate::check::lowering::AdmittedModel;
use crate::check::lowering::PlaceStep;
use crate::check::model_spelling::ModelSpellings;
use crate::check::refusal::{CheckCause, TemporalCause, TimedConstruct};
use filament_checked::key::DeclarationKey;

/// The places a place expression can denote, each with the data literals
/// that select it.
pub(super) type Places = Vec<(Vec<(DataLiteral, bool)>, Place)>;

/// A clock place relative to the receiver.
#[derive(Clone, Debug)]
pub(super) struct Place {
    /// The reference fields walked from the receiver to the object holding
    /// the clock; none for the receiver's own clock.
    pub(super) via: Vec<PlaceStep>,
    /// The clock.
    pub(super) clock: ClockId,
    /// The operation's `Clock` parameter position, for a place an instance
    /// binds that parameter to (FR-230 "Operation parameters").
    pub(super) parameter: Option<usize>,
}

/// The unit's clock-reading declarations, by name.
pub(crate) struct Declarations<'r> {
    by_name: BTreeMap<&'r str, &'r ClockReadingDeclaration>,
    /// The admitted models, whose object types hold the clock fields a body
    /// reads as `self.x` or `o.x`.
    models: &'r [AdmittedModel],
    /// Each unit's model aliases, through which a declaration's parameter
    /// type `M::T` names a model (FR-480 "Selections and imports").
    spellings: &'r ModelSpellings,
    /// The source unit of each declaration, by declared name.
    source_owners: &'r BTreeMap<String, RawSourceRef>,
    /// The cycle of calls through each recursive declaration, from it back
    /// to itself.
    cycles: BTreeMap<&'r str, Vec<String>>,
}

impl<'r> Declarations<'r> {
    pub(crate) fn new(
        declarations: &'r [ClockReadingDeclaration],
        models: &'r [AdmittedModel],
        spellings: &'r ModelSpellings,
        source_owners: &'r BTreeMap<String, RawSourceRef>,
    ) -> Self {
        let calls = call_graph(declarations);
        let cycles = declarations
            .iter()
            .enumerate()
            .filter_map(|(start, declaration)| {
                let members = cycle_from(&calls, start)?;
                let names = members
                    .iter()
                    .filter_map(|member| declarations.get(*member))
                    .map(|member| member.function.name.clone())
                    .collect();
                Some((declaration.function.name.as_str(), names))
            })
            .collect();
        Self {
            by_name: declarations
                .iter()
                .map(|declaration| (declaration.function.name.as_str(), declaration))
                .collect(),
            models,
            spellings,
            source_owners,
            cycles,
        }
    }

    /// The declarations on the cycle of calls through `name`, in call
    /// order from `name` back to itself; `None` when `name` is not
    /// recursive.
    fn cycle(&self, name: &str) -> Option<&[String]> {
        self.cycles.get(name).map(Vec::as_slice)
    }

    /// The declaration a written call name names: an unqualified name of
    /// the unit's own source.
    pub(super) fn named(&self, name: &NameForm) -> Option<&'r ClockReadingDeclaration> {
        if name.segments.len() > 1 {
            return None;
        }
        self.by_name.get(name.name.as_str()).copied()
    }

    /// Whether node `id` of `expression` is a call of a clock-reading
    /// declaration.
    pub(super) fn is_call(&self, expression: &Expression, id: ExprId) -> bool {
        matches!(
            expression.get(id).map(|node| node.node()),
            Some(ExprNode::Call { name, arguments: _ }) if self.named(name).is_some()
        )
    }

    /// What the path `steps` below `root` reaches. From an object
    /// parameter, the leading reference fields select the object the rest
    /// of the path is read on (FR-230-AC-10's `o.peer.x`).
    fn path_kind(&self, root: &PathRoot, steps: &[PlaceStep]) -> PathKind {
        let reached: Vec<(Vec<PlaceStep>, &[PlaceStep])> = match root {
            PathRoot::Object(object) => self
                .models
                .iter()
                .flat_map(|model| {
                    let (rest, holder) = match object {
                        Some(object) => {
                            let (walked, holder) = model.through_references(object.clone(), steps);
                            (&steps[walked..], Some(holder))
                        }
                        None => (steps, None),
                    };
                    model
                        .clock_places()
                        .iter()
                        .filter(|place| {
                            holder
                                .clone()
                                .is_none_or(|holder| model.conforms(holder, place.owner.clone()))
                        })
                        .map(|place| (place.path.clone(), rest))
                        .collect::<Vec<_>>()
                })
                .collect(),
            PathRoot::Record(model, key) => self
                .models
                .get(*model)
                .map(|model| {
                    model
                        .record_clock_paths(key)
                        .into_iter()
                        .map(|path| (path, steps))
                        .collect()
                })
                .unwrap_or_default(),
        };
        if reached
            .iter()
            .any(|(place, rest)| place.as_slice() == *rest)
        {
            PathKind::Clock
        } else if reached
            .iter()
            .any(|(place, rest)| crate::check::lowering::strictly_below(place, rest))
        {
            PathKind::Bearing
        } else {
            PathKind::Data
        }
    }

    /// The models, with their indices, a type name `name` written in
    /// `declaration` can name: for `M::T`, the model of the package the
    /// declaration's unit binds `M` to; for a bare name, every model.
    fn written_models<'n>(
        &self,
        declaration: &ClockReadingDeclaration,
        name: &'n NameForm,
    ) -> impl Iterator<Item = (usize, &'r AdmittedModel)> + 'n
    where
        'r: 'n,
    {
        let package = match &name.segments[..] {
            [qualifier, _] => Some(
                self.spellings
                    .selection(
                        self.source_owners.get(&declaration.function.name),
                        qualifier,
                    )
                    .map(|selection| selection.identity.as_str()),
            ),
            _ => None,
        };
        self.models.iter().enumerate().filter(move |(_, model)| {
            package.is_none_or(|package| package == Some(model.selection().identity.as_str()))
        })
    }

    /// The package record type a parameter's written type of `declaration`
    /// names, with the index of its model, when it names one that holds a
    /// clock.
    fn record_type(
        &self,
        declaration: &ClockReadingDeclaration,
        written: &fsl_forms::TypeForm,
    ) -> Option<(usize, DeclarationKey)> {
        let TypeFormHead::Name(name) = &written.head else {
            return None;
        };
        self.written_models(declaration, name)
            .find_map(|(index, model)| {
                let key = model.record_named(name)?;
                (!model.record_clock_paths(key).is_empty()).then(|| (index, key.clone()))
            })
    }

    /// The object type a parameter's written type of `declaration` names,
    /// with its model, when it names one.
    fn object_type(
        &self,
        declaration: &ClockReadingDeclaration,
        written: &fsl_forms::TypeForm,
    ) -> Option<(&'r AdmittedModel, EffectiveId)> {
        let TypeFormHead::Name(name) = &written.head else {
            return None;
        };
        self.written_models(declaration, name)
            .find_map(|(_, model)| Some((model, model.object_type_named(name)?)))
    }
}

/// Where a place path of a body starts.
enum PathRoot {
    /// An object: `self` (`None`, any object type) or an object parameter.
    Object(Option<EffectiveId>),
    /// A record parameter, by its model's index and its record type.
    Record(usize, DeclarationKey),
}

/// What a place path reaches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PathKind {
    /// A clock place.
    Clock,
    /// A value holding clock places below it.
    Bearing,
    /// A data value.
    Data,
}

/// A declaration's parameter: its name, how it is read, and the object
/// type or clock-bearing package record type it is typed by, when it is
/// one.
struct Parameter<'d> {
    name: &'d str,
    slot: ClockSlot,
    object: Option<EffectiveId>,
    record: Option<(usize, DeclarationKey)>,
}

/// The parameter a body node names, when it is a bare name of one.
fn parameter<'d>(
    declarations: &Declarations<'_>,
    declaration: &'d ClockReadingDeclaration,
    node: &ExprNode,
) -> Option<Parameter<'d>> {
    let ExprNode::Name(name) = node else {
        return None;
    };
    if name.segments.len() > 1 {
        return None;
    }
    let index = declaration
        .function
        .parameters
        .iter()
        .position(|(parameter, _)| *parameter == name.name)?;
    let (written, type_form) = declaration.function.parameters.get(index)?;
    Some(Parameter {
        name: written.as_str(),
        slot: declaration.parameters.get(index).copied()?,
        object: declarations
            .object_type(declaration, type_form)
            .map(|(_, object)| object),
        record: declarations.record_type(declaration, type_form),
    })
}

/// One declaration's body and the span of each of its nodes.
struct Body<'d> {
    declaration: &'d ClockReadingDeclaration,
    spans: Vec<Span>,
}

impl<'d> Body<'d> {
    fn of(declaration: &'d ClockReadingDeclaration) -> Self {
        let function = &declaration.function;
        let spans = function
            .spans()
            .and_then(|spans| spans.body.of_nodes(&function.body))
            .unwrap_or_default();
        Self { declaration, spans }
    }

    fn expression(&self) -> &'d Expression {
        &self.declaration.function.body
    }

    fn node(&self, id: ExprId) -> Option<&'d ExprNode> {
        self.expression().get(id).map(|node| node.node())
    }

    fn span(&self, id: ExprId) -> Span {
        self.spans
            .get(id.index())
            .copied()
            .or_else(|| {
                self.declaration
                    .function
                    .spans()
                    .map(|spans| spans.declaration)
            })
            .unwrap_or(Span { start: 0, end: 0 })
    }

    /// The parameter body node `id` names, when it is a bare name of one.
    fn parameter(&self, declarations: &Declarations<'_>, id: ExprId) -> Option<Parameter<'d>> {
        parameter(declarations, self.declaration, self.node(id)?)
    }

    /// The place path at `id`: the field steps of `r.f.g…` and the node `r`
    /// they start from, `self` or a parameter. `None` when `id` is no field
    /// path.
    fn path(&self, id: ExprId) -> Option<(ExprId, Vec<PlaceStep>)> {
        let mut steps = Vec::new();
        let mut at = id;
        while let Some(ExprNode::Field { operand, field, .. }) = self.node(at) {
            steps.push(PlaceStep::Field(field.clone()));
            at = *operand;
        }
        if steps.is_empty() {
            return None;
        }
        steps.reverse();
        Some((at, steps))
    }

    /// What the place path at `id` reaches, when it is one from `self`, an
    /// object parameter or a clock-bearing record parameter.
    fn path_kind(&self, id: ExprId, declarations: &Declarations<'_>) -> Option<PathKind> {
        let (root, steps) = self.path(id)?;
        let root = match self.node(root)? {
            ExprNode::SelfRef => PathRoot::Object(None),
            ExprNode::Name(_) => {
                let parameter = self.parameter(declarations, root)?;
                match (parameter.object, parameter.record) {
                    (Some(object), _) => PathRoot::Object(Some(object)),
                    (None, Some((model, key))) => PathRoot::Record(model, key),
                    (None, None) => return None,
                }
            }
            _ => return None,
        };
        Some(declarations.path_kind(&root, &steps))
    }

    /// Whether node `id` reads a clock place directly: `self.x`, `o.x` of an
    /// object parameter or `r.x` of a record parameter, through record
    /// fields.
    fn is_clock_field(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        self.path_kind(id, declarations) == Some(PathKind::Clock)
    }

    /// Whether node `id` can be the argument for a clock-bearing parameter:
    /// a clock-bearing parameter, or a place path to a clock-bearing value.
    fn is_bearing_place(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Bearing),
            Some(ExprNode::Field { .. }) => {
                self.path_kind(id, declarations) == Some(PathKind::Bearing)
            }
            _ => false,
        }
    }

    /// Whether node `id` is a place expression of this body: a clock
    /// parameter, a clock field of `self` or of an object parameter, an `if`
    /// whose branches are place expressions, or a call of a declaration
    /// whose result is `Clock`.
    fn is_place(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Clock),
            Some(ExprNode::Field { .. }) => self.is_clock_field(id, declarations),
            Some(ExprNode::If {
                then, otherwise, ..
            }) => self.is_place(*then, declarations) && self.is_place(*otherwise, declarations),
            Some(ExprNode::Call { name, .. }) => declarations
                .named(name)
                .is_some_and(|called| called.result == ClockSlot::Clock),
            _ => false,
        }
    }

    /// Whether node `id` is shaped as a bound: a time quantity literal, a
    /// negated one, or a data parameter.
    fn is_bound(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Quantity(_)) => true,
            Some(ExprNode::Negate(operand)) => {
                matches!(self.node(*operand), Some(ExprNode::Quantity(_)))
            }
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Data),
            _ => false,
        }
    }

    /// The first node under `id` that reads a clock: a clock or
    /// clock-bearing parameter, a clock field, or a call of a clock-reading
    /// declaration. `None` when the subtree is data.
    fn first_clock_read(&self, id: ExprId, declarations: &Declarations<'_>) -> Option<ExprId> {
        let mut pending = vec![id];
        let mut found: Option<ExprId> = None;
        while let Some(id) = pending.pop() {
            let Some(node) = self.node(id) else {
                continue;
            };
            let reads = match node {
                ExprNode::Name(_) => self
                    .parameter(declarations, id)
                    .is_some_and(|parameter| parameter.slot != ClockSlot::Data),
                ExprNode::Field { .. } => match self.path_kind(id, declarations) {
                    Some(PathKind::Clock | PathKind::Bearing) => true,
                    // A data field of a clock-bearing value reads no clock.
                    Some(PathKind::Data) => continue,
                    None => false,
                },
                ExprNode::Call { name, .. } => declarations.named(name).is_some(),
                _ => false,
            };
            if reads {
                let earlier =
                    found.is_none_or(|first| self.span(id).start < self.span(first).start);
                if earlier {
                    found = Some(id);
                }
                continue;
            }
            pending.extend(node.children());
        }
        found
    }
}

/// The bare-name nodes under `root` that name one of `names` free of every
/// binder above them in the subtree: the occurrences of a parameter.
pub(super) fn free_occurrences<'n>(
    expression: &Expression,
    root: ExprId,
    names: &BTreeSet<&'n str>,
) -> Vec<(ExprId, &'n str)> {
    let mut found = Vec::new();
    let mut pending: Vec<(ExprId, BTreeSet<&str>)> = vec![(root, BTreeSet::new())];
    while let Some((id, shadowed)) = pending.pop() {
        let Some(node) = expression.get(id).map(|node| node.node()) else {
            continue;
        };
        if let ExprNode::Name(name) = node {
            if name.segments.len() == 1 {
                if let Some(parameter) = names.get(name.name.as_str()) {
                    if !shadowed.contains(*parameter) {
                        found.push((id, *parameter));
                    }
                }
            }
            continue;
        }
        // The children outside each binder's scope: a `let` value, a
        // query's or accumulation's source, an accumulation's identity, a
        // sequence's bounds and a case's scrutinee.
        let outside: Vec<ExprId> = match node {
            ExprNode::Let { value, .. } => vec![*value],
            ExprNode::Query { source, .. }
            | ExprNode::Count { source, .. }
            | ExprNode::Sum { source, .. } => vec![*source],
            ExprNode::Accumulate {
                source, identity, ..
            } => std::iter::once(*source).chain(*identity).collect(),
            ExprNode::Seq { low, high, .. } => vec![*low, *high],
            ExprNode::Case(case) => vec![case.scrutinee],
            _ => Vec::new(),
        };
        let mut inner = shadowed.clone();
        for binder in node.binders() {
            if let Some(parameter) = names.get(binder) {
                inner.insert(*parameter);
            }
        }
        for child in node.children() {
            let scope = if outside.contains(&child) {
                shadowed.clone()
            } else {
                inner.clone()
            };
            pending.push((child, scope));
        }
    }
    found
}

/// Where a body node is read.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Position {
    /// As a place: a clock operand, a clock argument or a place branch.
    Place,
    /// Anywhere else.
    Other,
}

/// FR-230's check of one clock-reading declaration's body.
///
/// # Errors
///
/// `ill_typed`/`type-mismatch` at a `Clock`-result body that is no place
/// expression; `clock-comparison` at a comparison of two clocks;
/// `negative-clock-constant` or `clock-bound-not-constant` at a bound that
/// is not a time quantity literal or a quantity parameter;
/// `clock-read-outside-constraint` at any other read of a clock parameter
/// or clock field, including an equation that sets a data value from one.
pub(crate) fn check_declaration(
    declaration: &ClockReadingDeclaration,
    declarations: &Declarations<'_>,
) -> Result<(), TemporalCause> {
    let body = Body::of(declaration);
    let root = body.expression().root_id();
    let start = if declaration.result == ClockSlot::Clock {
        if !body.is_place(root, declarations) {
            return Err(TemporalCause::NotAPlaceExpression {
                span: body.span(root),
            });
        }
        Position::Place
    } else {
        Position::Other
    };
    let mut pending = vec![(root, start)];
    while let Some((id, position)) = pending.pop() {
        let Some(node) = body.node(id) else {
            continue;
        };
        match node {
            ExprNode::Name(_) => {
                let clock = body
                    .parameter(declarations, id)
                    .is_some_and(|parameter| parameter.slot != ClockSlot::Data);
                if clock && position != Position::Place {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
            }
            ExprNode::Binary {
                operator,
                left,
                right,
            } if ClockOp::of(*operator).is_some()
                && (body.is_place(*left, declarations) || body.is_place(*right, declarations)) =>
            {
                let (place, bound) = if body.is_place(*left, declarations) {
                    (*left, *right)
                } else {
                    (*right, *left)
                };
                if body.is_place(bound, declarations) {
                    return Err(TemporalCause::ClockComparison {
                        span: body.span(id),
                    });
                }
                if !body.is_bound(bound, declarations) {
                    // An equation that sets a data value from a clock reads
                    // the clock outside a constraint; an order comparison
                    // with a non-constant bound has no constant.
                    return Err(if *operator == BinaryOperator::Equal {
                        TemporalCause::ClockReadOutsideConstraint {
                            span: body.span(place),
                        }
                    } else {
                        TemporalCause::ClockBoundNotConstant {
                            span: body.span(bound),
                        }
                    });
                }
                if let Some(ExprNode::Negate(_)) = body.node(bound) {
                    return Err(TemporalCause::NegativeClockConstant {
                        span: body.span(bound),
                    });
                }
                pending.push((place, Position::Place));
                pending.push((bound, Position::Other));
            }
            ExprNode::If {
                condition,
                then,
                otherwise,
            } if position == Position::Place => {
                pending.push((*condition, Position::Other));
                pending.push((*then, Position::Place));
                pending.push((*otherwise, Position::Place));
            }
            ExprNode::Field { .. } if body.path_kind(id, declarations).is_some() => {
                // A clock or clock-bearing value is read only as a place; a
                // data field of one is a data read.
                if body.path_kind(id, declarations) != Some(PathKind::Data)
                    && position != Position::Place
                {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
            }
            // A quantifier over a clock collection: its source is read as a
            // place, and its body is checked where a call expands it
            // (FR-230 "Quantified reads").
            ExprNode::Query {
                query: BinderQuery::Forall | BinderQuery::Exists,
                source,
                body: quantified,
                ..
            } if body.is_bearing_place(*source, declarations) => {
                pending.push((*source, Position::Place));
                pending.push((*quantified, Position::Other));
            }
            ExprNode::Call { name, arguments } => {
                let called = declarations.named(name);
                if position != Position::Place
                    && called.is_some_and(|called| called.result == ClockSlot::Clock)
                {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
                for (index, argument) in arguments.iter().enumerate() {
                    let slot = called
                        .and_then(|called| called.parameters.get(index).copied())
                        .unwrap_or(ClockSlot::Data);
                    let place = match slot {
                        ClockSlot::Data => None,
                        ClockSlot::Clock => Some(body.is_place(*argument, declarations)),
                        ClockSlot::Bearing => Some(body.is_bearing_place(*argument, declarations)),
                    };
                    let read = match place {
                        None => Position::Other,
                        Some(true) => Position::Place,
                        Some(false) => {
                            return Err(TemporalCause::NotAPlaceExpression {
                                span: body.span(*argument),
                            });
                        }
                    };
                    pending.push((*argument, read));
                }
            }
            _ => pending.extend(
                node.children()
                    .into_iter()
                    .map(|child| (child, Position::Other)),
            ),
        }
    }
    Ok(())
}

/// A clock-reading declaration's argument, read at the call site.
#[derive(Clone)]
enum Argument {
    /// The places a clock argument can denote.
    Places(Places),
    /// A place path below the receiver: `self` (no steps) for an object
    /// parameter, or a clock-bearing record value for a record parameter.
    Path(Vec<PlaceStep>),
    /// A data argument: the call-site term it is, and its value as a
    /// constant time quantity, or why it is none.
    Value {
        term: DataLiteral,
        constant: Result<Rational, TemporalCause>,
        /// Its value, when it is an integer or Boolean constant.
        known: Option<Known>,
    },
}

impl Argument {
    /// Whether the argument is a constant the checker evaluates, or no data
    /// argument at all.
    fn is_constant(&self) -> bool {
        match self {
            Self::Value {
                constant, known, ..
            } => known.is_some() || constant.is_ok(),
            Self::Places(_) | Self::Path(_) => true,
        }
    }
}

/// A constant a data argument or a condition evaluates to.
#[derive(Clone, Debug, PartialEq)]
enum Known {
    Integer(Integer),
    Boolean(bool),
    /// A union value written `U::member(payload…)` or `U::member`.
    Union {
        member: String,
        payload: Vec<Known>,
    },
}

/// The constant node `id` of `expression` evaluates to: a literal, a name
/// `resolve` gives a constant, a union value, or an operation over
/// constants. `Err` carries the bit length of an integer result past
/// `bits`; `None` is a value that is not a constant.
fn fold(
    expression: &Expression,
    resolve: &dyn Fn(ExprId) -> Option<Known>,
    id: ExprId,
    bits: u64,
) -> Result<Option<Known>, u64> {
    let Some(node) = expression.get(id).map(|node| node.node()) else {
        return Ok(None);
    };
    let again = |id: ExprId| fold(expression, resolve, id, bits);
    match node {
        ExprNode::Integer(value) => bounded(value.clone(), bits),
        ExprNode::Boolean(value) => Ok(Some(Known::Boolean(*value))),
        ExprNode::Name(name) if name.segments.len() == 1 => Ok(resolve(id)),
        ExprNode::Name(name) => Ok(Some(Known::Union {
            member: member_of(name),
            payload: Vec::new(),
        })),
        ExprNode::Call { name, arguments } if name.segments.len() > 1 => {
            let mut payload = Vec::with_capacity(arguments.len());
            for argument in arguments {
                match again(*argument)? {
                    Some(known) => payload.push(known),
                    None => return Ok(None),
                }
            }
            Ok(Some(Known::Union {
                member: member_of(name),
                payload,
            }))
        }
        ExprNode::Negate(operand) => match again(*operand)? {
            Some(Known::Integer(value)) => bounded(value.neg(), bits),
            _ => Ok(None),
        },
        ExprNode::Not(operand) => match again(*operand)? {
            Some(Known::Boolean(value)) => Ok(Some(Known::Boolean(!value))),
            _ => Ok(None),
        },
        ExprNode::Binary {
            operator,
            left,
            right,
        } => {
            let (Some(left), Some(right)) = (again(*left)?, again(*right)?) else {
                return Ok(None);
            };
            operate(*operator, left, right, bits)
        }
        _ => Ok(None),
    }
}

/// The last segment of a union member's written name.
fn member_of(name: &NameForm) -> String {
    name.segments.last().cloned().unwrap_or_default()
}

fn bounded(value: Integer, bits: u64) -> Result<Option<Known>, u64> {
    let wide = value.magnitude_bits();
    if wide > bits {
        Err(wide)
    } else {
        Ok(Some(Known::Integer(value)))
    }
}

/// `left operator right` over constants.
fn operate(
    operator: BinaryOperator,
    left: Known,
    right: Known,
    bits: u64,
) -> Result<Option<Known>, u64> {
    let truth = |value: bool| Ok(Some(Known::Boolean(value)));
    match (left, right) {
        (Known::Integer(left), Known::Integer(right)) => match operator {
            BinaryOperator::Add => bounded(left.add(&right), bits),
            BinaryOperator::Subtract => bounded(left.sub(&right), bits),
            BinaryOperator::Multiply => bounded(left.mul(&right), bits),
            BinaryOperator::IntegerDiv
            | BinaryOperator::IntegerRem
            | BinaryOperator::IntegerMod => Ok(divide(operator, &left, &right)),
            BinaryOperator::Equal => truth(left == right),
            BinaryOperator::NotEqual => truth(left != right),
            BinaryOperator::Less => truth(left < right),
            BinaryOperator::LessOrEqual => truth(left <= right),
            BinaryOperator::Greater => truth(left > right),
            BinaryOperator::GreaterOrEqual => truth(left >= right),
            _ => Ok(None),
        },
        (Known::Boolean(left), Known::Boolean(right)) => match operator {
            BinaryOperator::And => truth(left && right),
            BinaryOperator::Or => truth(left || right),
            BinaryOperator::Implies => truth(!left || right),
            BinaryOperator::Equal => truth(left == right),
            BinaryOperator::NotEqual => truth(left != right),
            _ => Ok(None),
        },
        (left @ Known::Union { .. }, right @ Known::Union { .. }) => match operator {
            BinaryOperator::Equal => truth(left == right),
            BinaryOperator::NotEqual => truth(left != right),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

/// `div`, `rem` or `mod` of a non-negative dividend by a positive divisor,
/// where every division law gives the same result. A negative operand or a
/// zero divisor is left to the law and the value checker, so it is no
/// constant.
fn divide(operator: BinaryOperator, dividend: &Integer, divisor: &Integer) -> Option<Known> {
    if dividend.is_negative() || divisor.is_negative() {
        return None;
    }
    let divisor = NonZeroInteger::try_from(divisor).ok()?;
    let (quotient, remainder) = dividend.div_rem_truncating(divisor);
    Some(Known::Integer(if operator == BinaryOperator::IntegerDiv {
        quotient
    } else {
        remainder
    }))
}

/// The value of the `let` that binds the name at `name`, when one above it
/// does and no binder in between rebinds it.
fn let_value(expression: &Expression, name: ExprId) -> Option<ExprId> {
    let ExprNode::Name(written) = expression.get(name)?.node() else {
        return None;
    };
    let mut parent: BTreeMap<ExprId, ExprId> = BTreeMap::new();
    for node in expression.iter() {
        for child in node.node().children() {
            parent.insert(child, node.id());
        }
    }
    let mut child = name;
    while let Some(&up) = parent.get(&child) {
        let node = expression.get(up)?.node();
        if let ExprNode::Let {
            name: bound,
            value,
            body,
            ..
        } = node
        {
            if *bound == written.name && *body == child {
                return Some(*value);
            }
        } else if node.binders().contains(&written.name.as_str()) {
            return None;
        }
        child = up;
    }
    None
}

/// The frame a reading is in: the one it started in, or one a tail call
/// entered.
#[derive(Clone)]
enum Current<'b, 'e> {
    Outer(Frame<'b, 'e>),
    Inner(Rc<Entered<'e>>),
}

impl<'b, 'e> Current<'b, 'e> {
    fn frame(&self) -> Frame<'_, 'e> {
        match self {
            Self::Outer(frame) => *frame,
            Self::Inner(entered) => Frame {
                body: &entered.body,
                environment: &entered.environment,
                site: entered.site,
            },
        }
    }
}

/// A body a tail call entered: the called body, its arguments, and the
/// outermost call site.
struct Entered<'d> {
    body: Body<'d>,
    environment: BTreeMap<String, Argument>,
    site: ExprId,
}

/// What reading one Boolean body node comes to.
enum Step<'e> {
    /// The node's disjunctive forms, read positively and negated.
    Done((Dnf, Dnf)),
    /// The node is the negation of body node `0`.
    Negate(ExprId),
    /// The node combines two operands the reading reads in turn.
    Split {
        operator: BinaryOperator,
        left: ExprId,
        right: ExprId,
    },
    /// The node's value is that of body node `id`, in the frame `into`
    /// enters (the same frame when `None`).
    Read {
        id: ExprId,
        into: Option<Entered<'e>>,
    },
}

/// What a Boolean reading does with the result of the node it is reading.
enum Pending<'b, 'e> {
    /// Swap the positive and negated forms.
    Swap,
    /// Read `right` next, in `current`, with `stack` cut back to `depth`.
    Right {
        operator: BinaryOperator,
        right: ExprId,
        current: Current<'b, 'e>,
        depth: usize,
    },
    /// Combine the left operand's forms with the result.
    Join {
        operator: BinaryOperator,
        left: (Dnf, Dnf),
    },
}

/// What reading one place expression node comes to.
enum PlacesStep<'e> {
    Done(Places),
    Read {
        id: ExprId,
        into: Option<Entered<'e>>,
    },
}

/// The expansion of the calls of one precondition, at one call site.
pub(super) struct Expansion<'e, 'f, 'c> {
    pub(super) located: &'e Located<'f>,
    pub(super) clocks: &'e Clocks<'c>,
    pub(super) declarations: &'e Declarations<'e>,
    /// The object type `self` ranges over at the call site.
    pub(super) context: EffectiveId,
}

/// The not-yet-checked refusal of `construct`, which this build does not
/// expand, at `span`.
fn not_checked(construct: TimedConstruct, span: Span) -> CheckCause {
    CheckCause::TimedNotChecked { construct, span }
}

/// The bodies being expanded, outermost first, to refuse recursion.
type Stack = Vec<String>;

/// A body reading in progress: the body, its parameters' arguments, and
/// the call-site node the outermost call is.
#[derive(Clone, Copy)]
struct Frame<'b, 'd> {
    body: &'b Body<'d>,
    environment: &'b BTreeMap<String, Argument>,
    site: ExprId,
}

impl<'e, 'f, 'c> Expansion<'e, 'f, 'c> {
    /// The places call-site node `id` can denote when it is a place
    /// expression: `self.x`, an `if` whose branches are place expressions,
    /// or a call of a declaration whose result is `Clock`. `None` when it
    /// is none.
    pub(super) fn site_places(
        &self,
        id: ExprId,
        budget: &mut PartBudget,
    ) -> Result<Option<Places>, CheckCause> {
        if let Some(clock) = self.located.clock(id) {
            return Ok(Some(vec![(
                Vec::new(),
                Place {
                    via: Vec::new(),
                    clock,
                    parameter: None,
                },
            )]));
        }
        // A `Clock` parameter: one place for each clock field of its range,
        // the one an instance binds it to.
        if let Some(parameter) = self.located.parameter(id) {
            budget.charge(parameter.range.len())?;
            return Ok(Some(
                parameter
                    .range
                    .iter()
                    .map(|clock| {
                        (
                            Vec::new(),
                            Place {
                                via: Vec::new(),
                                clock: *clock,
                                parameter: Some(parameter.position),
                            },
                        )
                    })
                    .collect(),
            ));
        }
        match self.located.node(id) {
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) => {
                if self.located.reads_clock(*condition) {
                    return Err(timed(self.located.outside(*condition)));
                }
                let (Some(then), Some(otherwise)) = (
                    self.site_places(*then, budget)?,
                    self.site_places(*otherwise, budget)?,
                ) else {
                    return Ok(None);
                };
                let mut places = guarded(then, (DataLiteral::Site(*condition), true));
                places.extend(guarded(otherwise, (DataLiteral::Site(*condition), false)));
                budget.charge(places.len())?;
                Ok(Some(places))
            }
            Some(ExprNode::Call { name, arguments }) => {
                let Some(declaration) = self.declarations.named(name) else {
                    return Ok(None);
                };
                if declaration.result != ClockSlot::Clock {
                    return Ok(None);
                }
                budget.unfold(self.located.span(id))?;
                let environment = self.site_arguments(declaration, arguments, budget)?;
                let body = Body::of(declaration);
                let frame = Frame {
                    body: &body,
                    environment: &environment,
                    site: id,
                };
                let mut stack = vec![declaration.function.name.clone()];
                self.places(&frame, body.expression().root_id(), &mut stack, budget)
                    .map(Some)
            }
            _ => Ok(None),
        }
    }

    /// The disjunctive forms of the call at call-site node `id` of a
    /// clock-reading predicate, read positively and negated.
    pub(super) fn predicate(
        &self,
        id: ExprId,
        budget: &mut PartBudget,
    ) -> Result<Option<(Dnf, Dnf)>, CheckCause> {
        let Some(ExprNode::Call { name, arguments }) = self.located.node(id) else {
            return Ok(None);
        };
        let Some(declaration) = self.declarations.named(name) else {
            return Ok(None);
        };
        if declaration.result == ClockSlot::Clock {
            // A place expression: the comparison that reads it expands it.
            return Ok(None);
        }
        budget.unfold(self.located.span(id))?;
        let environment = self.site_arguments(declaration, arguments, budget)?;
        let body = Body::of(declaration);
        let frame = Frame {
            body: &body,
            environment: &environment,
            site: id,
        };
        let mut stack = vec![declaration.function.name.clone()];
        self.boolean(&frame, body.expression().root_id(), &mut stack, budget)
            .map(Some)
    }

    /// The arguments of a call at the call site, by parameter name.
    fn site_arguments(
        &self,
        declaration: &ClockReadingDeclaration,
        arguments: &[ExprId],
        budget: &mut PartBudget,
    ) -> Result<BTreeMap<String, Argument>, CheckCause> {
        let mut bound = BTreeMap::new();
        for ((name, type_form), (slot, argument)) in declaration.function.parameters.iter().zip(
            declaration
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        ) {
            let span = self.located.span(argument);
            let read = match slot {
                ClockSlot::Clock => match self.site_places(argument, budget)? {
                    Some(places) => Argument::Places(places),
                    None => {
                        return Err(timed(TemporalCause::NotAPlaceExpression { span }));
                    }
                },
                ClockSlot::Bearing if matches!(type_form.head, TypeFormHead::Collection(_)) => {
                    Argument::Path(self.site_collection(argument)?)
                }
                ClockSlot::Bearing => {
                    Argument::Path(self.site_record(declaration, type_form, argument)?)
                }
                ClockSlot::Data => {
                    if self.located.reads_clock(argument) {
                        return Err(timed(self.located.outside(argument)));
                    }
                    if let Some(path) = self.site_object(declaration, type_form, argument) {
                        // `self`, or an object `self` refers to.
                        Argument::Path(path)
                    } else {
                        Argument::Value {
                            term: DataLiteral::Site(argument),
                            constant: self.located.constant(argument, self.clocks),
                            known: self.site_known(argument, budget)?,
                        }
                    }
                }
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// The reference fields a call-site argument for an object parameter of
    /// type `type_form` walks from `self`: none for `self`, `[peer]` for
    /// `self.peer`, each a required single reference to an object of the
    /// parameter's type. `None` for any other argument, which is data.
    fn site_object(
        &self,
        declaration: &ClockReadingDeclaration,
        type_form: &fsl_forms::TypeForm,
        argument: ExprId,
    ) -> Option<Vec<PlaceStep>> {
        let (model, expected) = self.declarations.object_type(declaration, type_form)?;
        let path = match self.located.node(argument)? {
            ExprNode::SelfRef => Vec::new(),
            _ => super::self_path(self.located.form, argument)?,
        };
        let (walked, holder) = model.through_references(self.context.clone(), &path);
        (walked == path.len() && model.conforms(holder, expected)).then_some(path)
    }

    /// The record path a call-site argument for a clock-bearing parameter of
    /// type `type_form` denotes: a place path `self.f…` to a value of that
    /// record type.
    fn site_record(
        &self,
        declaration: &ClockReadingDeclaration,
        type_form: &fsl_forms::TypeForm,
        argument: ExprId,
    ) -> Result<Vec<PlaceStep>, CheckCause> {
        let span = self.located.span(argument);
        let not_a_place = || timed(TemporalCause::NotAPlaceExpression { span });
        let Some((_, expected)) = self.declarations.record_type(declaration, type_form) else {
            // A clock-bearing type of the unit's own source: no place of the
            // model holds one.
            return Err(not_a_place());
        };
        let path = super::self_path(self.located.form, argument).ok_or_else(not_a_place)?;
        if self.clocks.model.record_at(self.context.clone(), &path) != Some(&expected) {
            return Err(not_a_place());
        }
        Ok(path)
    }

    /// The collection path a call-site argument for a clock collection
    /// parameter denotes: a place path `self.f…` to a collection of the
    /// receiver holding clock places below its slot.
    fn site_collection(&self, argument: ExprId) -> Result<Vec<PlaceStep>, CheckCause> {
        let span = self.located.span(argument);
        let path = super::self_path(self.located.form, argument)
            .ok_or_else(|| timed(TemporalCause::NotAPlaceExpression { span }))?;
        let collection = self.clocks.fields.iter().any(|clock| {
            self.clocks
                .model
                .conforms(self.context.clone(), clock.owner.clone())
                && clock.path.starts_with(&path)
                && matches!(clock.path.get(path.len()), Some(PlaceStep::Slot(_)))
        });
        if collection {
            Ok(path)
        } else {
            Err(timed(TemporalCause::NotAPlaceExpression { span }))
        }
    }

    /// The receiver path the place path at body node `root` starts from:
    /// none for `self`, an object or record parameter's argument path.
    fn prefix(&self, frame: &Frame<'_, 'e>, root: ExprId) -> Result<Vec<PlaceStep>, CheckCause> {
        let body = frame.body;
        match body.node(root) {
            Some(ExprNode::SelfRef) => Ok(Vec::new()),
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, root)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Path(prefix)) => Ok(prefix.clone()),
                // A clock of another object: a place through a reference
                // (FR-230-AC-10's `peer_clock`).
                Some(Argument::Value { .. }) => Err(not_checked(
                    TimedConstruct::ReferencedClock,
                    body.span(root),
                )),
                _ => Err(timed(TemporalCause::NotAPlaceExpression {
                    span: body.span(root),
                })),
            },
            _ => Err(timed(TemporalCause::NotAPlaceExpression {
                span: body.span(root),
            })),
        }
    }

    /// The arguments of a call written in a body, by parameter name of the
    /// called declaration.
    fn body_arguments(
        &self,
        frame: &Frame<'_, 'e>,
        called: &ClockReadingDeclaration,
        arguments: &[ExprId],
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<BTreeMap<String, Argument>, CheckCause> {
        let body = frame.body;
        let mut bound = BTreeMap::new();
        for ((name, type_form), (slot, argument)) in called.function.parameters.iter().zip(
            called
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        ) {
            let read = match slot {
                ClockSlot::Clock => Argument::Places(self.places(frame, argument, stack, budget)?),
                ClockSlot::Bearing => {
                    if let Some(passed) = body
                        .parameter(self.declarations, argument)
                        .and_then(|parameter| frame.environment.get(parameter.name))
                    {
                        passed.clone()
                    } else {
                        let (root, steps) = body.path(argument).ok_or_else(|| {
                            timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(argument),
                            })
                        })?;
                        let mut path = self.prefix(frame, root)?;
                        path.extend(steps);
                        let expected = self.declarations.record_type(called, type_form);
                        if expected.as_ref().map(|(_, key)| key)
                            != self.clocks.model.record_at(self.context.clone(), &path)
                        {
                            return Err(timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(argument),
                            }));
                        }
                        Argument::Path(path)
                    }
                }
                ClockSlot::Data => {
                    if let Some(passed) = body
                        .parameter(self.declarations, argument)
                        .and_then(|parameter| frame.environment.get(parameter.name))
                    {
                        passed.clone()
                    } else if matches!(body.node(argument), Some(ExprNode::SelfRef))
                        && self.declarations.object_type(called, type_form).is_some()
                    {
                        Argument::Path(Vec::new())
                    } else {
                        let constant = match body.node(argument) {
                            Some(ExprNode::Quantity(quantity)) => self.clocks.quantity(quantity),
                            Some(ExprNode::Negate(operand))
                                if matches!(body.node(*operand), Some(ExprNode::Quantity(_))) =>
                            {
                                Err(TemporalCause::NegativeClockConstant {
                                    span: body.span(argument),
                                })
                            }
                            _ => Err(TemporalCause::ClockBoundNotConstant {
                                span: body.span(argument),
                            }),
                        };
                        let known = self.known(frame, argument, budget)?;
                        let term = match &known {
                            Some(known @ (Known::Integer(_) | Known::Boolean(_))) => {
                                self.literal(frame, known)?
                            }
                            _ => self.formed(frame, argument)?,
                        };
                        Argument::Value {
                            term,
                            constant,
                            known,
                        }
                    }
                }
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// The constant body node `id` evaluates to, with its parameters
    /// replaced by their constant arguments.
    ///
    /// # Errors
    ///
    /// The rational-bit limit, at an integer result too wide for it.
    fn known(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        budget: &PartBudget,
    ) -> Result<Option<Known>, CheckCause> {
        let body = frame.body;
        let resolve = |name: ExprId| -> Option<Known> {
            let ExprNode::Name(written) = body.node(name)? else {
                return None;
            };
            match frame.environment.get(&written.name)? {
                Argument::Value { known, .. } => known.clone(),
                _ => None,
            }
        };
        fold(body.expression(), &resolve, id, self.clocks.rational_bits).map_err(|bits| {
            budget.too_wide(
                self.clocks.rational_bits,
                bits,
                self.located.span(frame.site),
            )
        })
    }

    /// The constant call-site node `id` evaluates to; a name is a constant
    /// when the `let` that binds it has a constant value.
    ///
    /// # Errors
    ///
    /// The rational-bit limit, at an integer result too wide for it.
    fn site_known(&self, id: ExprId, budget: &PartBudget) -> Result<Option<Known>, CheckCause> {
        fold(
            &self.located.form.expression,
            &|name| self.site_let(name),
            id,
            self.clocks.rational_bits,
        )
        .map_err(|bits| budget.too_wide(self.clocks.rational_bits, bits, self.located.span(id)))
    }

    /// The constant the `let` binding the call-site name `name` gives it.
    fn site_let(&self, name: ExprId) -> Option<Known> {
        let expression = &self.located.form.expression;
        let value = let_value(expression, name)?;
        fold(
            expression,
            &|inner| self.site_let(inner),
            value,
            self.clocks.rational_bits,
        )
        .ok()
        .flatten()
    }

    /// The data term of constant `known`, located at the outermost call.
    fn literal(&self, frame: &Frame<'_, 'e>, known: &Known) -> Result<DataLiteral, CheckCause> {
        let expression = match known {
            Known::Integer(value) => Expression::integer(value.clone()),
            Known::Boolean(value) => Expression::boolean(*value),
            Known::Union { .. } => return Err(timed(self.located.outside(frame.site))),
        };
        let spans = ExpressionSpans::at_point(&expression, self.located.span(frame.site).start)
            .map_err(|_| timed(self.located.outside(frame.site)))?;
        Ok(DataLiteral::Formed(Rc::new(ExpressionForm {
            expression,
            spans,
        })))
    }

    /// The refusal of a read whose unfolding no finite set of atomic
    /// constraints stands for, at the outermost call `site` of the
    /// declaration `name`: the cycle's declarations when it is recursive.
    fn unfolding_refusal(&self, name: &str, site: ExprId) -> CheckCause {
        timed(match self.declarations.cycle(name) {
            Some(cycle) => TemporalCause::ClockReadingCycle {
                cycle: cycle.to_vec(),
                span: self.located.span(site),
            },
            None => self.located.outside(site),
        })
    }

    /// Enter the called declaration `called` from `frame`: charge the
    /// unfolding, read the arguments, and refuse a recursive call whose
    /// data is not constant.
    fn enter(
        &self,
        frame: &Frame<'_, 'e>,
        called: &'e ClockReadingDeclaration,
        arguments: &[ExprId],
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Entered<'e>, CheckCause> {
        let name = &called.function.name;
        let recursive = stack.contains(name);
        budget.unfold(self.located.span(frame.site))?;
        let environment = self.body_arguments(frame, called, arguments, stack, budget)?;
        if recursive && !environment.values().all(Argument::is_constant) {
            return Err(self.unfolding_refusal(name, frame.site));
        }
        if !recursive {
            stack.push(name.clone());
        }
        Ok(Entered {
            body: Body::of(called),
            environment,
            site: frame.site,
        })
    }

    /// Body node `id`, a data term, read at the call site: its parameters
    /// replaced by their arguments, located at the outermost call.
    ///
    /// # Errors
    ///
    /// `clock-read-outside-constraint` at the first clock read under `id`.
    fn formed(&self, frame: &Frame<'_, 'e>, id: ExprId) -> Result<DataLiteral, CheckCause> {
        let body = frame.body;
        if let Some(read) = body.first_clock_read(id, self.declarations) {
            return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                span: body.span(read),
            }));
        }
        // A bare data parameter is its argument.
        if let Some(Argument::Value { term, .. }) = body
            .parameter(self.declarations, id)
            .and_then(|parameter| frame.environment.get(parameter.name))
        {
            return Ok(term.clone());
        }
        Ok(DataLiteral::Formed(Rc::new(self.grafted(frame, id)?)))
    }

    /// Body node `id` read at the call: each parameter replaced by its
    /// argument (a data argument's term, `self.f…` for a path argument),
    /// every node located at the call site.
    ///
    /// # Errors
    ///
    /// `clock-read-outside-constraint` at a clock parameter, which no data
    /// term stands for.
    fn grafted(&self, frame: &Frame<'_, 'e>, id: ExprId) -> Result<ExpressionForm, CheckCause> {
        let body = frame.body;
        let site_span = self.located.span(frame.site);
        let names: BTreeSet<&str> = frame.environment.keys().map(String::as_str).collect();
        let mut trees: Vec<(ExprId, Expression)> = Vec::new();
        for (occurrence, name) in free_occurrences(body.expression(), id, &names) {
            match frame.environment.get(name) {
                Some(Argument::Value { term, .. }) => trees.push((occurrence, self.term(term)?)),
                // `self.f…`: the receiver's value at the path.
                Some(Argument::Path(path)) => {
                    let mut tree = Expression::self_ref();
                    for step in path {
                        // An argument path names a single record or object,
                        // never a collection slot.
                        let PlaceStep::Field(field) = step else {
                            return Err(timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(occurrence),
                            }));
                        };
                        tree = Expression::field(tree, field.clone());
                    }
                    trees.push((occurrence, tree));
                }
                // A clock parameter read as data was refused above.
                Some(Argument::Places(_)) | None => {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(occurrence),
                    }));
                }
            }
        }
        let grafts: BTreeMap<ExprId, &Expression> = trees
            .iter()
            .map(|(occurrence, tree)| (*occurrence, tree))
            .collect();
        let expression = body
            .expression()
            .graft(id, &grafts)
            .ok_or_else(|| timed(self.located.outside(frame.site)))?;
        let spans = ExpressionSpans::at_point(&expression, site_span.start)
            .map_err(|_| timed(self.located.outside(frame.site)))?;
        Ok(ExpressionForm { expression, spans })
    }

    /// The disjunctive forms of the quantifier at body node `id` read at
    /// the call, when it ranges over a clock collection of the receiver.
    fn quantified(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        budget: &mut PartBudget,
    ) -> Result<Option<(Dnf, Dnf)>, CheckCause> {
        let form = self.grafted(frame, id)?;
        let Some(located) = Located::of(&form, self.clocks, &self.context) else {
            return Ok(None);
        };
        super::quantified::read(&located, located.root(), self.clocks, &self.context, budget)
    }

    /// The expression a data term denotes.
    fn term(&self, term: &DataLiteral) -> Result<Expression, CheckCause> {
        match term {
            DataLiteral::Site(id) => self
                .located
                .form
                .cut(*id, &BTreeMap::new())
                .map(|form| form.expression)
                .ok_or_else(|| timed(self.located.outside(*id))),
            DataLiteral::Formed(form) | DataLiteral::Element(form, ..) => {
                Ok(form.expression.clone())
            }
            DataLiteral::EmptyCollection(form) => Ok(form.expression.clone()),
            DataLiteral::Family(_) | DataLiteral::Presence(_) => Err(CheckCause::TimedNotChecked {
                construct: crate::check::refusal::TimedConstruct::Clock,
                span: self.located.span(self.located.root()),
            }),
        }
    }

    /// The places body node `id` denotes.
    fn places(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Places, CheckCause> {
        let depth = stack.len();
        let read = self.read_places(Current::Outer(*frame), id, stack, budget);
        stack.truncate(depth);
        read
    }

    /// Read place expression `id` in `start`. A selected `if` branch and a
    /// call are tail positions, read in a loop, so a recursive declaration
    /// unfolds to any depth the work budget admits without nesting the
    /// reading.
    fn read_places(
        &self,
        start: Current<'_, 'e>,
        mut id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Places, CheckCause> {
        let mut current = start;
        loop {
            match self.places_step(&current.frame(), id, stack, budget)? {
                PlacesStep::Done(places) => return Ok(places),
                PlacesStep::Read { id: next, into } => {
                    id = next;
                    if let Some(into) = into {
                        current = Current::Inner(Rc::new(into));
                    }
                }
            }
        }
    }

    /// One step of reading place expression `id`.
    fn places_step(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<PlacesStep<'e>, CheckCause> {
        let body = frame.body;
        let not_a_place = || {
            timed(TemporalCause::NotAPlaceExpression {
                span: body.span(id),
            })
        };
        match body.node(id) {
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, id)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Places(places)) => Ok(PlacesStep::Done(places.clone())),
                _ => Err(not_a_place()),
            },
            Some(ExprNode::Field { .. }) => {
                let (root, steps) = body.path(id).ok_or_else(not_a_place)?;
                let mut path = self.prefix(frame, root)?;
                path.extend(steps);
                // A clock of another object: the leading reference fields
                // select the object holding it (FR-230-AC-10's `peer`).
                let (walked, holder) = self
                    .clocks
                    .model
                    .through_references(self.context.clone(), &path);
                let (via, rest) = path.split_at(walked);
                let clock = self
                    .clocks
                    .fields
                    .iter()
                    .position(|clock| {
                        clock.path == rest
                            && self
                                .clocks
                                .model
                                .conforms(holder.clone(), clock.owner.clone())
                    })
                    .map(ClockId)
                    .ok_or_else(not_a_place)?;
                Ok(PlacesStep::Done(vec![(
                    Vec::new(),
                    Place {
                        parameter: None,
                        via: via.to_vec(),
                        clock,
                    },
                )]))
            }
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) => {
                // A condition over constants keeps the branch it selects.
                match self.known(frame, *condition, budget)? {
                    Some(Known::Boolean(true)) => {
                        return Ok(PlacesStep::Read {
                            id: *then,
                            into: None,
                        });
                    }
                    Some(Known::Boolean(false)) => {
                        return Ok(PlacesStep::Read {
                            id: *otherwise,
                            into: None,
                        });
                    }
                    _ => {}
                }
                // A recursive unfolding has no finite set of places for a
                // condition that is not a constant.
                let name = &body.declaration.function.name;
                if self.declarations.cycle(name).is_some() {
                    return Err(self.unfolding_refusal(name, frame.site));
                }
                // The condition is data: a clock read there selects no place.
                let (selected, rejected) = self.boolean(frame, *condition, stack, budget)?;
                if selected
                    .iter()
                    .chain(&rejected)
                    .any(|(_, clocks)| !clocks.is_empty())
                {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(*condition),
                    }));
                }
                let then = self.places(frame, *then, stack, budget)?;
                let otherwise = self.places(frame, *otherwise, stack, budget)?;
                let mut places = Vec::new();
                for (condition, branch) in [(selected, then), (rejected, otherwise)] {
                    for (literals, _) in condition {
                        for (selecting, place) in &branch {
                            places.push((
                                literals.iter().chain(selecting).cloned().collect(),
                                place.clone(),
                            ));
                        }
                    }
                }
                budget.charge(places.len())?;
                Ok(PlacesStep::Done(places))
            }
            Some(ExprNode::Call { name, arguments }) => {
                let called = self
                    .declarations
                    .named(name)
                    .filter(|called| called.result == ClockSlot::Clock)
                    .ok_or_else(not_a_place)?;
                let entered = self.enter(frame, called, arguments, stack, budget)?;
                Ok(PlacesStep::Read {
                    id: entered.body.expression().root_id(),
                    into: Some(entered),
                })
            }
            _ => Err(not_a_place()),
        }
    }

    /// The constant body node `id` denotes as a bound.
    fn bound(&self, frame: &Frame<'_, 'e>, id: ExprId) -> Result<Rational, CheckCause> {
        let body = frame.body;
        match body.node(id) {
            Some(ExprNode::Quantity(quantity)) => self.clocks.quantity(quantity).map_err(timed),
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, id)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Value { constant, .. }) => constant.clone().map_err(timed),
                _ => Err(timed(TemporalCause::ClockBoundNotConstant {
                    span: body.span(id),
                })),
            },
            Some(ExprNode::Negate(_)) => Err(timed(TemporalCause::NegativeClockConstant {
                span: body.span(id),
            })),
            _ => Err(timed(TemporalCause::ClockBoundNotConstant {
                span: body.span(id),
            })),
        }
    }

    /// The disjunctive forms of Boolean body node `id`, read positively and
    /// negated: a Boolean combination of atomic clock constraints, calls of
    /// clock-reading predicates and data predicates.
    fn boolean(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<(Dnf, Dnf), CheckCause> {
        let depth = stack.len();
        let read = self.read_boolean(Current::Outer(*frame), id, stack, budget);
        stack.truncate(depth);
        read
    }

    /// Read Boolean node `id` in `start`. The reading keeps its own stack of
    /// what is left to do with each result instead of nesting, so a
    /// recursive declaration unfolds to any depth the node limit and work
    /// budget admit without nesting the reading: a negation, a selected
    /// `if` branch, a call and each operand of `and`, `or` and `implies`
    /// are read in turn.
    fn read_boolean(
        &self,
        start: Current<'_, 'e>,
        mut id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<(Dnf, Dnf), CheckCause> {
        let mut current = start;
        let mut pending: Vec<Pending<'_, 'e>> = Vec::new();
        loop {
            let mut read = match self.boolean_step(&current.frame(), id, stack, budget)? {
                Step::Done(read) => read,
                Step::Negate(operand) => {
                    pending.push(Pending::Swap);
                    id = operand;
                    continue;
                }
                Step::Split {
                    operator,
                    left,
                    right,
                } => {
                    // Each operand left unread is held, so the node limit
                    // bounds how many.
                    budget.charge(1)?;
                    pending.push(Pending::Right {
                        operator,
                        right,
                        current: current.clone(),
                        depth: stack.len(),
                    });
                    id = left;
                    continue;
                }
                Step::Read { id: next, into } => {
                    id = next;
                    if let Some(into) = into {
                        current = Current::Inner(Rc::new(into));
                    }
                    continue;
                }
            };
            loop {
                match pending.pop() {
                    None => return Ok(read),
                    Some(Pending::Swap) => read = (read.1, read.0),
                    Some(Pending::Right {
                        operator,
                        right,
                        current: resumed,
                        depth,
                    }) => {
                        stack.truncate(depth);
                        pending.push(Pending::Join {
                            operator,
                            left: read,
                        });
                        current = resumed;
                        id = right;
                        break;
                    }
                    Some(Pending::Join {
                        operator,
                        left: (lp, ln),
                    }) => {
                        let (rp, rn) = read;
                        read = match operator {
                            BinaryOperator::And => (budget.product(&lp, &rp)?, [ln, rn].concat()),
                            BinaryOperator::Or => ([lp, rp].concat(), budget.product(&ln, &rn)?),
                            _ => ([ln, rp].concat(), budget.product(&lp, &rn)?),
                        };
                    }
                }
            }
        }
    }

    /// One step of reading Boolean body node `id`.
    fn boolean_step(
        &self,
        frame: &Frame<'_, 'e>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Step<'e>, CheckCause> {
        let body = frame.body;
        let name = &body.declaration.function.name;
        let truth = || vec![(Vec::new(), Vec::new())];
        let done = |read| Ok(Step::Done(read));
        match body.node(id) {
            Some(ExprNode::Boolean(true)) => done((truth(), Vec::new())),
            Some(ExprNode::Boolean(false)) => done((Vec::new(), truth())),
            Some(ExprNode::Not(operand)) => Ok(Step::Negate(*operand)),
            // An `if` that reads a clock keeps the branch its condition
            // selects; a condition that is not a constant has no finite set
            // of atomic constraints.
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) if body.first_clock_read(id, self.declarations).is_some() => {
                let selected = match self.known(frame, *condition, budget)? {
                    Some(Known::Boolean(true)) => *then,
                    Some(Known::Boolean(false)) => *otherwise,
                    _ => return Err(self.unfolding_refusal(name, frame.site)),
                };
                Ok(Step::Read {
                    id: selected,
                    into: None,
                })
            }
            // A `case` that reads a clock keeps the arm its scrutinee
            // selects, its binders bound to the payload.
            Some(ExprNode::Case(case))
                if body.first_clock_read(id, self.declarations).is_some() =>
            {
                let Some(Known::Union { member, payload }) =
                    self.known(frame, case.scrutinee, budget)?
                else {
                    return Err(self.unfolding_refusal(name, frame.site));
                };
                let Some(arm) = case.arms.iter().find(|arm| {
                    arm.member.segments.last() == Some(&member)
                        && arm.binders.len() == payload.len()
                }) else {
                    return Err(self.unfolding_refusal(name, frame.site));
                };
                let mut environment = frame.environment.clone();
                for (binder, value) in arm.binders.iter().zip(payload) {
                    environment.insert(
                        binder.name.clone(),
                        Argument::Value {
                            term: self.literal(frame, &value)?,
                            constant: Err(TemporalCause::ClockBoundNotConstant {
                                span: body.span(arm.body),
                            }),
                            known: Some(value),
                        },
                    );
                }
                Ok(Step::Read {
                    id: arm.body,
                    into: Some(Entered {
                        body: Body::of(body.declaration),
                        environment,
                        site: frame.site,
                    }),
                })
            }
            Some(ExprNode::Binary {
                operator:
                    operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
                left,
                right,
            }) if body.first_clock_read(id, self.declarations).is_some() => Ok(Step::Split {
                operator: *operator,
                left: *left,
                right: *right,
            }),
            Some(ExprNode::Binary {
                operator,
                left,
                right,
            }) if body.is_place(*left, self.declarations)
                || body.is_place(*right, self.declarations) =>
            {
                let Some(op) = ClockOp::of(*operator) else {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    }));
                };
                let (place, bound, op) = if body.is_place(*left, self.declarations) {
                    (*left, *right, op)
                } else {
                    (*right, *left, op.mirrored())
                };
                let places = self.places(frame, place, stack, budget)?;
                let bound = self.bound(frame, bound)?;
                done(constraints(
                    places,
                    op,
                    &bound,
                    self.located.span(frame.site),
                ))
            }
            Some(ExprNode::Call { name, arguments })
                if self
                    .declarations
                    .named(name)
                    .is_some_and(|called| called.result != ClockSlot::Clock) =>
            {
                let Some(called) = self.declarations.named(name) else {
                    return Err(timed(self.located.outside(frame.site)));
                };
                let entered = self.enter(frame, called, arguments, stack, budget)?;
                Ok(Step::Read {
                    id: entered.body.expression().root_id(),
                    into: Some(entered),
                })
            }
            // A quantifier over a clock collection (FR-230 "Quantified
            // reads"): read at the call site, its source the receiver's
            // collection its argument names.
            Some(ExprNode::Query {
                query: BinderQuery::Forall | BinderQuery::Exists,
                ..
            }) => match self.quantified(frame, id, budget)? {
                Some(read) => done(read),
                None => {
                    let literal = self.formed(frame, id)?;
                    done((
                        vec![(vec![(literal.clone(), true)], Vec::new())],
                        vec![(vec![(literal, false)], Vec::new())],
                    ))
                }
            },
            // A data predicate: read at the call site.
            _ => {
                let literal = self.formed(frame, id)?;
                done((
                    vec![(vec![(literal.clone(), true)], Vec::new())],
                    vec![(vec![(literal, false)], Vec::new())],
                ))
            }
        }
    }
}

/// `places`, each selected also by `literal`.
fn guarded(places: Places, literal: (DataLiteral, bool)) -> Places {
    places
        .into_iter()
        .map(|(mut literals, clock)| {
            literals.insert(0, literal.clone());
            (literals, clock)
        })
        .collect()
}

/// The disjunctive forms of `place ~ bound` over `places`, read positively
/// and negated. The places' selecting conditions are exclusive and
/// exhaustive (each place expression's branches partition its conditions),
/// so the negation is the same places with each constraint negated.
pub(super) fn constraints(places: Places, op: ClockOp, bound: &Rational, span: Span) -> (Dnf, Dnf) {
    let constraint = |place: &Place, op: ClockOp| ClockConstraint {
        clock: place.clock,
        via: place.via.clone(),
        slots: Vec::new(),
        binding: None,
        op,
        bound: bound.clone(),
        span,
        parameter: place.parameter,
    };
    let positive = places
        .iter()
        .map(|(literals, place)| (literals.clone(), vec![constraint(place, op)]))
        .collect();
    let negative = places
        .iter()
        .flat_map(|(literals, place)| {
            op.negated()
                .into_iter()
                .map(move |negated| (literals.clone(), vec![constraint(place, negated)]))
        })
        .collect();
    (positive, negative)
}

/// The span of the first call of a clock-reading declaration in
/// `expression`, which no clock rule admits there.
pub(super) fn first_call(
    declarations: &Declarations<'_>,
    expression: &Expression,
    spans: &ExpressionSpans,
) -> Option<Span> {
    let spans = spans.of_nodes(expression)?;
    expression
        .iter()
        .filter(|node| declarations.is_call(expression, node.id()))
        .filter_map(|node| spans.get(node.id().index()).copied())
        .min_by_key(|span| span.start)
}

/// The region site a clock-reading declaration's name is registered under,
/// which locates a refusal of the declaration as a whole.
pub(crate) fn site(name: &str) -> String {
    format!("clock-reading {name}")
}

/// The first cycle of calls among `declarations` (FR-146's call graph over
/// them) with a member written without `decreases`, as the member names
/// from the first member back to it, and the name of its first member: a
/// recursive declaration with no measure (FR-091-AC-30 `missing-measure`).
pub(crate) fn unmeasured_cycle(
    declarations: &[ClockReadingDeclaration],
) -> Option<(Vec<String>, &str)> {
    let calls = call_graph(declarations);
    for (start, declaration) in declarations.iter().enumerate() {
        let Some(members) = cycle_from(&calls, start) else {
            continue;
        };
        let unmeasured = members.iter().any(|member| {
            declarations
                .get(*member)
                .is_some_and(|member| member.function.measure.is_none())
        });
        if unmeasured {
            let names = members
                .iter()
                .filter_map(|member| declarations.get(*member))
                .map(|member| member.function.name.clone())
                .collect();
            return Some((names, declaration.function.name.as_str()));
        }
    }
    None
}

/// The declarations each of `declarations` calls, by position (FR-146's call
/// graph over them).
fn call_graph(declarations: &[ClockReadingDeclaration]) -> Vec<Vec<usize>> {
    let index: BTreeMap<&str, usize> = declarations
        .iter()
        .enumerate()
        .map(|(position, declaration)| (declaration.function.name.as_str(), position))
        .collect();
    declarations
        .iter()
        .map(|declaration| {
            let mut called: Vec<usize> = declaration
                .function
                .body
                .iter()
                .filter_map(|node| match node.node() {
                    ExprNode::Call { name, .. } if name.segments.len() == 1 => {
                        index.get(name.name.as_str()).copied()
                    }
                    _ => None,
                })
                .collect();
            called.sort_unstable();
            called.dedup();
            called
        })
        .collect()
}

/// The shortest call path from declaration `start` back to itself, as the
/// positions from `start` to `start`; `None` when `start` is not recursive.
fn cycle_from(calls: &[Vec<usize>], start: usize) -> Option<Vec<usize>> {
    let mut previous: BTreeMap<usize, usize> = BTreeMap::new();
    let mut pending = std::collections::VecDeque::from([start]);
    let mut closing = None;
    'walk: while let Some(at) = pending.pop_front() {
        for &next in calls.get(at).into_iter().flatten() {
            if next == start {
                closing = Some(at);
                break 'walk;
            }
            if previous.contains_key(&next) {
                continue;
            }
            previous.insert(next, at);
            pending.push_back(next);
        }
    }
    let mut at = closing?;
    let mut members = vec![start];
    let mut back = Vec::new();
    while at != start {
        back.push(at);
        at = previous.get(&at).copied().unwrap_or(start);
    }
    members.extend(back.into_iter().rev());
    members.push(start);
    Some(members)
}
