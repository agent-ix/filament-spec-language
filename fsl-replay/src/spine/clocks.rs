// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640, FSpec FR-540: a compile or run request's `clocks` member, keyed by
//! clock name, turned into one [`TemporalClockSelection`] per clock-bearing
//! declaration.
//!
//! A clock name is the role a declaration writes; every declaration that
//! writes the same name reads the same clock, so one configuration per name
//! binds them all. A declaration over a model subject takes its binding from
//! the model and no member: under event-position its `model-steps`
//! configuration names the subject type's declaration identity text as its
//! sequence authority.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use filament_contract_model::ClockConfiguration;
use filament_exact::ValueType;
use fsl_foundation::source::provenance::RawSourceRef;
use fsl_foundation::Span;
use fsl_package::{
    clock_kind, is_model_binding_name, profile_clock_kind, ClockSelectionCause,
    ClockSelectionDeclaration, ClockSelectionRefusal, EmitRefusal, TemporalClockSelection,
};
use fsl_semantics::check::TemporalProfile;

use super::{region, CheckedUnit, CompileRefusal, ParsedUnitSource};

/// FR-640: the clock configurations a request supplies, by clock name.
/// Member order is not kept: two selections with equal members are equal.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequestClocks {
    members: BTreeMap<String, ClockConfiguration>,
}

impl RequestClocks {
    /// The selection of `members`, each a clock name and its configuration,
    /// or the first member whose name is empty or whose configuration is
    /// not well formed (an empty, oversized or noncanonical member, a
    /// non-positive `period`, a non-reduced rational).
    pub fn new(members: BTreeMap<String, ClockConfiguration>) -> Result<Self, InvalidClock> {
        for (name, clock) in &members {
            let field = if name.is_empty() {
                Some("name")
            } else {
                fsl_package::clock_configuration_invalid_field(clock)
            };
            if let Some(field) = field {
                return Err(InvalidClock {
                    clock: name.clone(),
                    field,
                });
            }
        }
        Ok(Self { members })
    }

    /// Whether the request supplied no member.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The configuration supplied for the clock `name`.
    pub fn get(&self, name: &str) -> Option<&ClockConfiguration> {
        self.members.get(name)
    }

    /// The selections S3 reads before the checked graph exists: each
    /// `temporal` clause of `parsed` whose clock name has a timestamped-event
    /// member, so S3 converts its `duration` windows to that clock's unit
    /// (FR-337 "Aggregate formula emission"). Whether the selection fits is
    /// decided after the check, by [`Self::package_selections`].
    pub fn check_selections(&self, parsed: &ParsedUnitSource) -> Vec<TemporalClockSelection> {
        let source = parsed.source().reference();
        parsed
            .unit()
            .forms()
            .iter()
            .filter_map(|form| {
                let fsl_forms::DeclarationForm::Temporal(clause) = form.form() else {
                    return None;
                };
                let clock = self.get(&clause.clock.as_ref()?.text)?;
                matches!(clock, ClockConfiguration::TimestampedEvent { .. }).then(|| {
                    TemporalClockSelection::new(source.clone(), form.span(), clock.clone())
                })
            })
            .collect()
    }

    /// The selection of every clock-bearing temporal clause and `compensate`
    /// template of `checked`, in source order, or the FR-640 refusal at stage
    /// `emit`. `parsed` are the program's parsed units, which locate a
    /// refusal at the declaration's `clock` argument.
    pub fn package_selections(
        &self,
        checked: &CheckedUnit,
        parsed: &[&ParsedUnitSource],
    ) -> Result<Vec<TemporalClockSelection>, Box<CompileRefusal>> {
        let refuse = |refusal: ClockSelectionRefusal, at: Option<(&RawSourceRef, Span)>| {
            Box::new(CompileRefusal::Emit(
                EmitRefusal::ClockSelection(refusal),
                at.and_then(|(source, span)| region(source, clock_span(parsed, source, span))),
            ))
        };
        if let Some(name) = self.members.keys().find(|name| is_model_binding_name(name)) {
            return Err(refuse(
                ClockSelectionRefusal {
                    cause: ClockSelectionCause::ReservedClockName,
                    clock: name.clone(),
                    declaration: None,
                    supplied: None,
                },
                None,
            ));
        }

        let package = checked.package();
        let graph = package.graph();
        let mut declarations = Vec::new();
        for node in graph.temporal_clauses() {
            let clause = node.clause();
            let Some(location) = graph.occurrence(node.identity(), node.claim()) else {
                continue;
            };
            let Some(source) = graph.region(location).map(|region| region.source().clone()) else {
                continue;
            };
            declarations.push(Declaration {
                name: clause.name(),
                profile: clause.profile(),
                clock: clause.clock(),
                subject: match &clause.over().1 {
                    ValueType::Reference(object) => Some(object.original()),
                    _ => None,
                },
                source,
                span: clause.span(),
            });
        }
        for protocol in graph.protocols() {
            for template in &protocol.obligations.compensations {
                let Some(profile) = template.temporal_profile else {
                    continue;
                };
                declarations.push(Declaration {
                    name: &template.name,
                    profile,
                    clock: &template.clock,
                    subject: None,
                    source: graph.source().clone(),
                    span: template.span,
                });
            }
        }
        declarations.sort_by(|left, right| {
            (&left.source, left.span.start).cmp(&(&right.source, right.span.start))
        });

        let mut used = BTreeSet::new();
        let mut selections = Vec::new();
        for declaration in &declarations {
            let Some(expected) = profile_clock_kind(declaration.profile) else {
                continue;
            };
            let clock = if is_model_binding_name(declaration.clock) {
                // A model binding name: the model supplies the binding, and
                // only event-position's `model-steps` has a configuration.
                match (declaration.subject, declaration.profile, declaration.clock) {
                    (
                        Some(subject),
                        TemporalProfile::EventPositionFalseExtension,
                        "model-steps",
                    ) => ClockConfiguration::EventPosition {
                        sequence_authority: fsl_semantics::model::key::declaration_identity_text(
                            &DeclarationKey {
                                package: subject.package().to_owned(),
                                node: subject.node().to_owned(),
                            },
                        )
                        .into(),
                    },
                    _ => continue,
                }
            } else if declaration.subject.is_some() && declaration.profile == TemporalProfile::Timed
            {
                continue;
            } else {
                let at_fault = || ClockSelectionDeclaration {
                    name: declaration.name.to_owned(),
                    profile: declaration.profile,
                };
                let locus = Some((&declaration.source, declaration.span));
                let Some(clock) = self.get(declaration.clock) else {
                    return Err(refuse(
                        ClockSelectionRefusal {
                            cause: ClockSelectionCause::MissingClockSelection,
                            clock: declaration.clock.to_owned(),
                            declaration: Some(at_fault()),
                            supplied: None,
                        },
                        locus,
                    ));
                };
                if clock_kind(clock) != expected {
                    return Err(refuse(
                        ClockSelectionRefusal {
                            cause: ClockSelectionCause::ClockKindMismatch,
                            clock: declaration.clock.to_owned(),
                            declaration: Some(at_fault()),
                            supplied: Some(clock_kind(clock)),
                        },
                        locus,
                    ));
                }
                used.insert(declaration.clock);
                clock.clone()
            };
            selections.push(TemporalClockSelection::new(
                declaration.source.clone(),
                declaration.span,
                clock,
            ));
        }
        if let Some(name) = self
            .members
            .keys()
            .find(|name| !used.contains(name.as_str()))
        {
            return Err(refuse(
                ClockSelectionRefusal {
                    cause: ClockSelectionCause::UnusedClockSelection,
                    clock: name.clone(),
                    declaration: None,
                    supplied: None,
                },
                None,
            ));
        }
        Ok(selections)
    }
}

/// A `clocks` member that is not a well-formed clock configuration.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("clocks member {clock:?} has an invalid {field}")]
pub struct InvalidClock {
    /// The member's clock name.
    pub clock: String,
    /// The configuration member at fault, or `name` for an empty name.
    pub field: &'static str,
}

/// One clock-bearing declaration of the checked graph.
struct Declaration<'a> {
    name: &'a str,
    profile: TemporalProfile,
    clock: &'a str,
    /// The model object type the clause is over; `None` over a trace
    /// subject and for a `compensate` template.
    subject: Option<&'a filament_exact::OriginalDeclarationKey>,
    source: RawSourceRef,
    span: Span,
}

/// The span of the `clock` argument of the `temporal` clause declared at
/// `span` in `source`, or `span` itself when no parsed clause matches.
fn clock_span(parsed: &[&ParsedUnitSource], source: &RawSourceRef, span: Span) -> Span {
    parsed
        .iter()
        .filter(|unit| unit.source().reference() == source)
        .flat_map(|unit| unit.unit().forms())
        .find_map(|form| match form.form() {
            fsl_forms::DeclarationForm::Temporal(clause) if form.span() == span => {
                clause.clock.as_ref().map(|clock| clock.span)
            }
            _ => None,
        })
        .unwrap_or(span)
}
