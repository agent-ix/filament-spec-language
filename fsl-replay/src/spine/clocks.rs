// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640, FSpec FR-540: a compile or run request's `clocks` member, keyed by
//! clock name, turned into one [`TemporalClockSelection`] per clock-bearing
//! declaration.
//!
//! A clock name is the role a declaration writes; every declaration that
//! writes the same name reads the same clock, so one configuration per name
//! binds them all. A declaration over a model subject, and a hyper clause over
//! a model's behaviours, take their binding from the model and no member: the
//! configuration is derived from the binding and the declaration's profile.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use filament_contract_model::{ClockConfiguration, ClockExactNumber};
use filament_exact::ValueType;
use fsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use fsl_foundation::Span;
use fsl_package as pkg;
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
                // FR-610 "Unwritten clock": a clause with no `clock` reads the
                // role `default`.
                let role = clause
                    .clock
                    .as_ref()
                    .map_or(fsl_semantics::check::DEFAULT_CLOCK_ROLE, |clock| {
                        clock.text.as_str()
                    });
                let clock = self.get(role)?;
                matches!(clock, ClockConfiguration::TimestampedEvent { .. }).then(|| {
                    TemporalClockSelection::new(source.clone(), form.span(), clock.clone())
                })
            })
            .collect()
    }

    /// The selection of every clock-bearing temporal clause, hyper clause and
    /// `compensate` template of `checked`, in source order, or the FR-640
    /// refusal at stage `emit`. `parsed` are the program's parsed units,
    /// which locate a refusal at the declaration's `clock` argument. A member
    /// is unused when no declaration of the program or of any library unit
    /// it compiles with (FR-099) takes its configuration from `clocks`.
    pub fn package_selections(
        &self,
        checked: &CheckedUnit,
        parsed: &[&ParsedUnitSource],
    ) -> Result<Vec<TemporalClockSelection>, Box<CompileRefusal>> {
        let package = checked.package();
        let mut used = BTreeSet::new();
        let selections = self.selections(
            package,
            &ClockLocator::new(parsed.iter().copied()),
            &mut used,
        )?;
        let mut pending: Vec<&pkg::CheckedPackage> =
            package.dependencies().values().map(AsRef::as_ref).collect();
        while let Some(library) = pending.pop() {
            for declaration in declarations(library) {
                if declaration.model.is_none()
                    && configured_kind(declaration.profile).is_some()
                    && self.get(declaration.clock).is_some()
                {
                    used.insert(declaration.clock.to_owned());
                }
            }
            pending.extend(library.dependencies().values().map(AsRef::as_ref));
        }
        if let Some(name) = self.members.keys().find(|name| !used.contains(*name)) {
            return Err(emit_refusal(
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

    /// The selections of one library unit's checked `package` (FR-640, FR-099):
    /// its declarations read the same `clocks` members as the program's.
    /// Whether a member is unused is decided over the whole request, by
    /// [`Self::package_selections`].
    pub(crate) fn library_selections(
        &self,
        package: &pkg::CheckedPackage,
        locator: &ClockLocator,
    ) -> Result<Vec<TemporalClockSelection>, Box<CompileRefusal>> {
        self.selections(package, locator, &mut BTreeSet::new())
    }

    /// The selections of `package`'s own declarations, recording in `used`
    /// each clock name a member served.
    fn selections(
        &self,
        package: &pkg::CheckedPackage,
        locator: &ClockLocator,
        used: &mut BTreeSet<String>,
    ) -> Result<Vec<TemporalClockSelection>, Box<CompileRefusal>> {
        if let Some(name) = self.members.keys().find(|name| is_model_binding_name(name)) {
            return Err(emit_refusal(
                ClockSelectionRefusal {
                    cause: ClockSelectionCause::ReservedClockName,
                    clock: name.clone(),
                    declaration: None,
                    supplied: None,
                },
                None,
            ));
        }
        let mut selections = Vec::new();
        for declaration in declarations(package) {
            let Some(expected) = configured_kind(declaration.profile) else {
                continue;
            };
            let clock = if let Some(traces) = &declaration.model {
                // A model subject: the model supplies the binding, and the
                // configuration is derived from it and the profile.
                let Some(clock) = derived_configuration(&declaration, traces)? else {
                    continue;
                };
                clock
            } else if is_model_binding_name(declaration.clock) {
                continue;
            } else {
                let at_fault = || ClockSelectionDeclaration {
                    name: declaration.name.to_owned(),
                    profile: declaration.profile,
                };
                let locus = region(
                    &declaration.source,
                    locator.clock_span(&declaration.source, declaration.span),
                );
                let Some(clock) = self.get(declaration.clock) else {
                    return Err(emit_refusal(
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
                    return Err(emit_refusal(
                        ClockSelectionRefusal {
                            cause: ClockSelectionCause::ClockKindMismatch,
                            clock: declaration.clock.to_owned(),
                            declaration: Some(at_fault()),
                            supplied: Some(clock_kind(clock)),
                        },
                        locus,
                    ));
                }
                used.insert(declaration.clock.to_owned());
                clock.clone()
            };
            selections.push(TemporalClockSelection::new(
                declaration.source.clone(),
                declaration.span,
                clock,
            ));
        }
        Ok(selections)
    }
}

/// What the parsed units of one checked package tell a clock selection that
/// its checked graph does not: where each `temporal` clause's `clock`
/// argument is, which locates a refusal. Built before the check consumes the
/// parsed units.
#[derive(Clone, Debug, Default)]
pub(crate) struct ClockLocator {
    /// Each `temporal` clause's source, declaration span and `clock` span.
    clocks: Vec<(RawSourceRef, Span, Span)>,
}

impl ClockLocator {
    pub(crate) fn new<'a>(parsed: impl IntoIterator<Item = &'a ParsedUnitSource>) -> Self {
        let mut locator = Self::default();
        for unit in parsed {
            let source = unit.source().reference();
            for form in unit.unit().forms() {
                if let fsl_forms::DeclarationForm::Temporal(clause) = form.form() {
                    if let Some(clock) = &clause.clock {
                        locator
                            .clocks
                            .push((source.clone(), form.span(), clock.span));
                    }
                }
            }
        }
        locator
    }

    /// The span of the `clock` argument of the `temporal` clause declared at
    /// `span` in `source`, or `span` itself when no parsed clause matches.
    fn clock_span(&self, source: &RawSourceRef, span: Span) -> Span {
        self.clocks
            .iter()
            .find(|(at, declaration, _)| at == source && *declaration == span)
            .map_or(span, |(_, _, clock)| *clock)
    }
}

/// A clock-selection refusal at stage `emit`, located at `region`.
fn emit_refusal(
    refusal: ClockSelectionRefusal,
    region: Option<SourceRegion>,
) -> Box<CompileRefusal> {
    Box::new(CompileRefusal::Emit(
        EmitRefusal::ClockSelection(refusal),
        region,
    ))
}

/// Every clock-bearing declaration of `package`'s own graph, in source
/// order: its temporal clauses, its hyper clauses and its `compensate`
/// templates.
fn declarations(package: &pkg::CheckedPackage) -> Vec<Declaration<'_>> {
    let graph = package.graph();
    let mut declarations = Vec::new();
    for node in graph.temporal_clauses() {
        let clause = node.clause();
        let Some(source) = graph
            .occurrence(node.identity(), node.claim())
            .and_then(|location| graph.region(location))
            .map(|region| region.source().clone())
        else {
            continue;
        };
        declarations.push(Declaration {
            name: clause.name(),
            profile: clause.profile(),
            clock: clause.clock(),
            model: match &clause.over().1 {
                ValueType::Reference(object) => {
                    Some(vec![Some(subject_authority(object.original()))])
                }
                _ => None,
            },
            source,
            span: clause.span(),
        });
    }
    // FR-640: a hyper clause ranges over its models' behaviours, so each
    // quantified trace takes its binding from its own quantifier's model.
    // That trace's sequence authority is the declaration identity text of
    // the object type the clause's parameters bind for that model, the same
    // text a single-trace clause over that type uses; `None` when the
    // parameters bind no one type of it.
    for hyper in graph.hyper_clauses() {
        let Some(source) = hyper
            .assembled_occurrence()
            .and_then(|occurrence| graph.occurrence(occurrence.node().clone(), occurrence.origin()))
            .and_then(|location| graph.region(location))
            .map(|region| region.source().clone())
        else {
            continue;
        };
        let traces = hyper
            .quantifiers()
            .iter()
            .map(|quantifier| {
                let mut types = hyper
                    .parameters()
                    .iter()
                    .filter(|parameter| parameter.alias == quantifier.alias)
                    .map(|parameter| subject_authority(parameter.population.object().original()))
                    .collect::<Vec<_>>();
                types.sort();
                types.dedup();
                match types.as_slice() {
                    [only] => Some(only.clone()),
                    _ => None,
                }
            })
            .collect();
        declarations.push(Declaration {
            name: hyper.name(),
            profile: hyper.profile(),
            clock: "model-steps",
            model: Some(traces),
            source,
            span: hyper.span(),
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
                model: None,
                source: graph.source().clone(),
                span: template.span,
            });
        }
    }
    declarations.sort_by(|left, right| {
        (&left.source, left.span.start).cmp(&(&right.source, right.span.start))
    });
    declarations
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
    /// For a declaration that reads its binding from a model, the sequence
    /// authority of each trace it quantifies: one for a single-trace clause,
    /// one per quantifier for a hyper clause, `None` where it is not
    /// determined. `None` over a trace subject and for a `compensate`
    /// template.
    model: Option<Vec<Option<String>>>,
    source: RawSourceRef,
    span: Span,
}

/// FR-640: the configuration a declaration over model traces derives, the
/// one every trace derives (FR-640 "Behavior"); `None` when the profile and
/// binding configure none. A clause whose traces derive different
/// configurations, or an event-position trace with no determined sequence
/// authority, needs one configuration per trace, which a binding row cannot
/// carry yet: it refuses `unsupported_construct`/`not-yet-implemented` naming
/// the clause (FR-763's form).
fn derived_configuration(
    declaration: &Declaration<'_>,
    traces: &[Option<String>],
) -> Result<Option<ClockConfiguration>, Box<CompileRefusal>> {
    let mut derived: Option<Option<ClockConfiguration>> = None;
    for authority in traces {
        let clock = match authority {
            Some(authority) => {
                model_configuration(declaration.profile, declaration.clock, authority)
            }
            None if declaration.profile == TemporalProfile::EventPositionFalseExtension => {
                return Err(per_trace_clocks(declaration))
            }
            None => model_configuration(declaration.profile, declaration.clock, ""),
        };
        match &derived {
            Some(first) if first != &clock => return Err(per_trace_clocks(declaration)),
            Some(_) => {}
            None => derived = Some(clock),
        }
    }
    Ok(derived.flatten())
}

/// The implementation-gap refusal of a clause whose traces need different
/// clock configurations.
fn per_trace_clocks(declaration: &Declaration<'_>) -> Box<CompileRefusal> {
    Box::new(CompileRefusal::Emit(
        EmitRefusal::PerTraceClockConfigurations {
            declaration: declaration.name.to_owned(),
        },
        region(&declaration.source, declaration.span),
    ))
}

/// The `kind` of the configuration a declaration under `profile` takes a
/// selection for. A timed declaration takes none yet: the binding rows carry
/// no `timed` configuration until the contract's `ClockConfiguration` has the
/// variant (QSL-2254), so the emitter writes no timed row and a timed clause
/// or `compensate` template compiles, and replays, as it did before FR-640.
fn configured_kind(profile: TemporalProfile) -> Option<&'static str> {
    match profile {
        TemporalProfile::Timed => None,
        other => profile_clock_kind(other),
    }
}

/// The sequence authority of a temporal clause over the model object type
/// `subject`: the type's declaration identity text.
fn subject_authority(subject: &filament_exact::OriginalDeclarationKey) -> String {
    fsl_semantics::model::key::declaration_identity_text(&DeclarationKey {
        package: subject.package().to_owned(),
        node: subject.node().to_owned(),
    })
}

/// FR-640 "Behavior": the configuration a declaration over a model subject
/// derives from the model's binding `clock` under `profile`. The model's step
/// sequence is the authoritative sequence, so each step is one event, one
/// sample of period one step, or one time stamp one step after the last.
/// `None` for a binding the table does not configure: infinite-trace reads
/// the trace's own positions, a `model-time` binding outside the timed
/// profile is FR-233's refusal, and the timed configuration waits on the
/// contract's `timed` clock kind.
fn model_configuration(
    profile: TemporalProfile,
    clock: &str,
    authority: &str,
) -> Option<ClockConfiguration> {
    let step = || Box::<str>::from("step");
    match (profile, clock) {
        (TemporalProfile::EventPositionFalseExtension, "model-steps") => {
            Some(ClockConfiguration::EventPosition {
                sequence_authority: authority.into(),
            })
        }
        (TemporalProfile::FixedSampleFalseExtension, "model-steps") => {
            Some(ClockConfiguration::FixedSample {
                epoch: ClockExactNumber::Integer {
                    decimal: "0".into(),
                },
                period: ClockExactNumber::Integer {
                    decimal: "1".into(),
                },
                unit: step(),
            })
        }
        (TemporalProfile::TimestampedEventFiniteWindow, "model-steps") => {
            Some(ClockConfiguration::TimestampedEvent {
                timestamp_unit: step(),
            })
        }
        _ => None,
    }
}
