// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's same-run plugin replay context (FR-404): run one call as
//! [`run_with_requirements`](crate::run_with_requirements) does and return,
//! from that same compile, a [`CallReplayContext`] bound to the run.
//!
//! The context wraps a [`PluginReplayContext`] built from the run's own
//! units and limits, and names an item by a member's request index. A consumer reaches the entry
//! and the context through the `fsl_replay` root and never names
//! [`crate::spine`] (FR-060 T12-A).

use filament_exact::Cancel;
use fsl_foundation::bound::ProofBound;
use fsl_foundation::{BackendId, RequestIndex, SourceIdentity};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::object_environment::ObjectEnvironment;

use crate::call_requirements::{associate, package_name_of, CallRequirement, CallRequirements};
use crate::limits::CallerLimits;
use crate::proof_result::{PluginReplayContext, PluginReplayRefusal};
use crate::request::ReplayRequestWire;
use crate::spine::{run_reading, Call, CallOutcome, DependencyInput, RunRefusal};
use crate::witness::CanonicalAssignment;

/// The plugin replay context of one run (FR-404): it builds a replay request
/// only for an item of its own run's call. It has no public constructor, no
/// public field and no way to reach the wrapped [`PluginReplayContext`], so a
/// consumer obtains one only from [`run_with_replay_context`] (FR-404-AC-9).
/// None of these compiles:
///
/// ```compile_fail
/// let _ = fsl_replay::CallReplayContext { context: todo!(), members: todo!() };
/// ```
///
/// ```compile_fail
/// let _ = fsl_replay::CallReplayContext::new();
/// ```
///
/// ```compile_fail
/// let _ = fsl_replay::CallReplayContext::default();
/// ```
///
/// ```compile_fail
/// let _ = serde_json::from_str::<fsl_replay::CallReplayContext>("{}");
/// ```
///
/// ```compile_fail
/// fn unbound(context: fsl_replay::CallReplayContext) {
///     let _: &fsl_replay::PluginReplayContext = &*context;
/// }
/// ```
///
/// ```compile_fail
/// fn unbound(context: fsl_replay::CallReplayContext) {
///     let _: &fsl_replay::PluginReplayContext = context.as_ref();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CallReplayContext {
    package_name: LibraryName,
    context: PluginReplayContext,
    members: Vec<CallRequirement>,
}

impl CallReplayContext {
    /// The library name of the package of the run that produced this context.
    pub fn package_name(&self) -> &LibraryName {
        &self.package_name
    }

    /// The `ReplayRequestWire` for the item at `request_index`: FR-290's
    /// [`PluginReplayContext::request`] at the member's occurrence key.
    ///
    /// # Errors
    ///
    /// The first [`PluginReplayRefusal`], in this order: `NotACallItem` when
    /// `request_index` is no member's; then FR-290's `NotReplayable`, `UnresolvedDependency`,
    /// `DuplicateBoundKey` and `Fault`.
    pub fn request(
        &self,
        request_index: RequestIndex,
        proof_bounds: &[ProofBound],
        plugin: &BackendId,
        assignments: Vec<CanonicalAssignment>,
    ) -> Result<ReplayRequestWire, PluginReplayRefusal> {
        let member = self
            .members
            .iter()
            .find(|member| member.request_index() == request_index)
            .ok_or(PluginReplayRefusal::NotACallItem { request_index })?;
        self.context
            .request(member.occurrence(), proof_bounds, plugin, assignments)
    }
}

/// What [`run_with_replay_context`] returns: `run`'s outcome, the selected call's [`CallRequirements`] and the run's
/// [`CallReplayContext`], all from one compile.
#[derive(Debug)]
pub struct RunWithReplayContext {
    package_name: LibraryName,
    outcome: CallOutcome,
    requirements: CallRequirements,
    context: CallReplayContext,
}

impl RunWithReplayContext {
    /// The compiled package's library name: the identity of the source unit
    /// the run compiled.
    pub fn package_name(&self) -> &LibraryName {
        &self.package_name
    }

    /// The call's outcome, as `run` returns it for the same inputs.
    pub fn outcome(&self) -> &CallOutcome {
        &self.outcome
    }

    /// The selected call's requirement items.
    pub fn requirements(&self) -> &CallRequirements {
        &self.requirements
    }

    /// The run's replay context.
    pub fn context(&self) -> &CallReplayContext {
        &self.context
    }

    /// The outcome, the requirement items and the context.
    pub fn into_parts(self) -> (CallOutcome, CallRequirements, CallReplayContext) {
        (self.outcome, self.requirements, self.context)
    }
}

/// [`run_with_requirements`](crate::run_with_requirements) that also returns
/// the run's [`CallReplayContext`] (FR-404). The source compiles once; the
/// outcome, members and context come from that compile. The
/// run's `SpineLimits` are `limits.spine`; the context copies the rest of
/// `limits` into each request it builds.
///
/// The refusals are `run_with_requirements`' for `limits.spine`, and a
/// refusal returns no context and no members.
#[allow(
    clippy::too_many_arguments,
    reason = "run's signature (FR-100): the request's fields, the object context and the cancel handle"
)]
pub fn run_with_replay_context(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: CallerLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<RunWithReplayContext, Box<RunRefusal>> {
    let library_name = package_name_of(&package_name)?;
    let (outcome, (members, context)) = run_reading(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits.spine,
        call,
        objects,
        &fsl_eval::execution::SuppliedBackends::default(),
        &fsl_eval::execution::PrepareLimits::default(),
        None,
        cancel,
        |checked, emitted, models| {
            let graph = checked.package().graph();
            let members = associate(graph.requirements(), graph, &call.function)
                .map_err(|fault| Box::new(RunRefusal::Fault(fault)))?;
            let context = PluginReplayContext::new(checked, emitted, models, limits);
            Ok((members, context))
        },
    )?;
    Ok(RunWithReplayContext {
        package_name: library_name.clone(),
        outcome,
        requirements: CallRequirements::new(call, members.clone()),
        context: CallReplayContext {
            package_name: library_name,
            context,
            members,
        },
    })
}

#[cfg(test)]
mod tests {
    use filament_exact::{Integer, NodeKey, Origin, Role, Value};
    use fsl_foundation::bound::{DomainKey, DomainKind, FiniteBound};
    use fsl_foundation::source::provenance::OccurrenceKey;
    use std::collections::BTreeMap;

    use super::*;
    use crate::execute::callable_parameter_keys;
    use crate::spine::{self, default_accounting, ParseRequest};
    use crate::witness::WitnessValue;
    use crate::ReplayLimits;
    use crate::{TerminalRecord, TerminalValue};
    use fsl_foundation::SettingLimits;
    use ix_trace_rs::trace;

    const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
        function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n";

    fn plugin() -> BackendId {
        BackendId::new("acme")
    }

    fn run(source: &str, limits: CallerLimits) -> RunWithReplayContext {
        let call = Call {
            clocks: Default::default(),
            wire_arguments: Vec::new(),
            canonical_values: false,
            snapshot: None,
            function: "q".to_owned(),
            arguments: vec![crate::CallArgument {
                parameter: "x".to_owned(),
                value: Value::Integer(Integer::from(3_i64)),
            }],
            accounting: default_accounting(1_000_000),
            supplied: filament_semantic_value::supplied_stop::SuppliedLimits::default(),
            backend: filament_eval::ExecutionBackendChoice::Interpreter,
            live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
        };
        run_with_replay_context(
            SourceIdentity::new("a", "u"),
            "fixture".to_owned(),
            "unit.native",
            source.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            limits,
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("the call runs")
    }

    fn run_with_replay_context_kernel(
        source: &str,
        limits: CallerLimits,
        call: &Call,
    ) -> RunWithReplayContext {
        use filament_semantic_value::checking::CheckMode;

        let identity = SourceIdentity::new("agent-ix", "test:lifecycle");
        let packages = fsl_semantics::model::intake::package_input([]).expect("no model packages");
        let dependencies = DependencyInput::default();
        let cancel = Cancel::new();
        let function_modes = BTreeMap::new();
        let modes = spine::ComposeModes {
            check_mode: Some(CheckMode::Kernel),
            function_modes: &function_modes,
        };
        let composed = spine::compose_in_mode(
            identity,
            "unit.native",
            source.as_bytes(),
            &packages,
            &dependencies,
            limits.spine,
            modes,
        )
        .expect("composes under Kernel");
        let checked = &composed.checked;
        let models = &composed.models;
        let emitted = &composed.emitted_unit;
        let outcome = spine::run_compiled(
            checked,
            models,
            spine::CallSources {
                package_name: "fixture",
                packages: &packages,
                model_limits: limits.spine.model,
            },
            call,
            emitted,
            &ObjectEnvironment::default(),
            &fsl_eval::execution::SuppliedBackends::default(),
            &fsl_eval::execution::PrepareLimits::default(),
            &cancel,
        )
        .expect("the call runs");
        let library_name = package_name_of("fixture").expect("library name");
        let members = crate::call_requirements::associate(
            checked.package().graph().requirements(),
            checked.package().graph(),
            &call.function,
        )
        .expect("requirements associate with the call");
        let context = PluginReplayContext::new(checked, emitted, models, limits);
        RunWithReplayContext {
            package_name: library_name.clone(),
            outcome,
            requirements: CallRequirements::new(call, members.clone()),
            context: CallReplayContext {
                package_name: library_name,
                context,
                members,
            },
        }
    }

    /// `p`'s declaration occurrence and its parameter's node, read through
    /// a separate compile of `source`.
    fn declaration(source: &str, function: &str) -> (OccurrenceKey, Vec<NodeKey>) {
        declaration_for(source, function, SourceIdentity::new("a", "u"))
    }

    fn declaration_for(
        source: &str,
        function: &str,
        identity: SourceIdentity,
    ) -> (OccurrenceKey, Vec<NodeKey>) {
        let limits = CallerLimits::default().spine;
        let cancel = Cancel::new();
        let parsed = spine::parse(
            &ParseRequest {
                source: &identity,
                path: "unit.native",
                bytes: source.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("parses")
        .into_value();
        let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
            .expect("selects")
            .into_value();
        let checked = spine::check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        )
        .expect("checks")
        .into_value();
        let callable = checked
            .package()
            .graph()
            .callable(function)
            .expect("declared");
        let parameters =
            callable_parameter_keys(checked.package(), &callable).expect("a function node");
        (
            OccurrenceKey::new(
                callable.identity.clone(),
                Origin::new(Role::new("declaration"), 0),
            ),
            parameters,
        )
    }

    fn kernel_declaration_for(
        source: &str,
        function: &str,
        identity: SourceIdentity,
    ) -> (OccurrenceKey, Vec<NodeKey>) {
        use filament_semantic_value::checking::CheckMode;

        let limits = CallerLimits::default().spine;
        let packages = fsl_semantics::model::intake::package_input([]).expect("no model packages");
        let dependencies = DependencyInput::default();
        let function_modes = BTreeMap::new();
        let composed = spine::compose_in_mode(
            identity,
            "unit.native",
            source.as_bytes(),
            &packages,
            &dependencies,
            limits,
            spine::ComposeModes {
                check_mode: Some(CheckMode::Kernel),
                function_modes: &function_modes,
            },
        )
        .expect("the unit composes under Kernel");
        let callable = composed
            .checked
            .package()
            .graph()
            .callable(function)
            .expect("declared");
        let parameters = callable_parameter_keys(composed.checked.package(), &callable)
            .expect("a function node");
        (
            OccurrenceKey::new(
                callable.identity.clone(),
                Origin::new(Role::new("declaration"), 0),
            ),
            parameters,
        )
    }

    fn bound_at(node: NodeKey) -> ProofBound {
        ProofBound::new(
            DomainKey::Node { node, path: vec![] },
            Some(DomainKind::Integer),
            FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
        )
        .unwrap()
    }

    fn input(parameter: NodeKey, x: i128) -> Vec<CanonicalAssignment> {
        vec![CanonicalAssignment {
            parameter,
            value: WitnessValue::Integer(x),
        }]
    }

    /// A member's request carries the run's source and limits (FR-404-AC-6), and replays and settles.
    #[trace("TC-1173", "FR-404-AC-6", "FR-255-AC-24")]
    #[test]
    fn a_member_request_carries_the_run_and_replays() {
        let limits = CallerLimits::from_operands([
            "s1.tokens=200000",
            "library.definitions=9000",
            "accounting.work_units=5000",
        ])
        .unwrap();
        let result = run(SOURCE, limits);
        let (_, parameters) = declaration(SOURCE, "q");
        let parameter = parameters[0].clone();
        let member = result.requirements().members()[0].clone();
        let key = member.occurrence().clone();
        let context = result.into_parts().2;
        let bounds = [bound_at(parameter.clone())];
        let wire = context
            .request(
                member.request_index(),
                &bounds,
                &plugin(),
                input(parameter, 9),
            )
            .expect("an application item");
        assert_eq!(wire.replay_target.occurrence_key.as_ref(), Some(&key));
        assert_eq!(wire.byte_provision[0].2, SOURCE.as_bytes());
        assert_eq!(wire.stage_limits["s1.tokens"], 200_000);
        assert_eq!(wire.stage_limits["library.definitions"], 9000);
        assert_eq!(wire.accounting_limits.work_units, 5000);
        assert!(!wire
            .stage_limits
            .contains_key("certificate_check.max_certificate_bytes"));
        for (setting, bound) in CallerLimits::default().bounds() {
            let name = setting.name();
            if !matches!(
                name,
                "s1.tokens" | "library.definitions" | "replay.input_bytes" | "format.output_bytes"
            ) {
                // Use the same literal oracle as the plugin request test,
                // independent of the replay setting predicate.
                let expected = (!crate::request::REPLAY_WIRE_OMITTED_STAGE_LIMIT_NAMES
                    .contains(&name))
                .then_some(&bound);
                assert_eq!(wire.stage_limits.get(name), expected, "{name}");
            }
        }
        let replayed = crate::replay(wire, ReplayLimits::default());
        let settled =
            TerminalRecord::from_plugin_counterexample(RequestIndex::new(0), plugin(), &replayed);
        assert!(matches!(
            settled.value(),
            TerminalValue::Inconclusive(crate::InconclusiveCause::ReplayParity(_))
        ));
        assert_eq!(settled.backend(), Some(&plugin()));
    }

    /// The member's `obligation_identity` is the `process-provider` identity
    /// of its node, occurrence and proof bounds (FR-404-AC-3), and its
    /// replay finds the application undefined.
    #[trace("TC-1173", "FR-404-AC-3")]
    #[test]
    fn a_member_request_carries_the_process_provider_identity() {
        let source = "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"filament.value.complete/v1\";\n\
            profile Complete = \"filament.protocol.finite-global/v1\";\n\
            function r using v(x: Int[0, 9]): Integer pure { 10 mod x }\n"
            .to_owned();
        let call = Call {
            clocks: Default::default(),
            wire_arguments: Vec::new(),
            canonical_values: false,
            snapshot: None,
            function: "r".to_owned(),
            arguments: vec![crate::CallArgument {
                parameter: "x".to_owned(),
                value: Value::Integer(Integer::from(0_i64)),
            }],
            accounting: default_accounting(1_000_000),
            supplied: filament_semantic_value::supplied_stop::SuppliedLimits::default(),
            backend: filament_eval::ExecutionBackendChoice::Interpreter,
            live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
        };
        let result = run_with_replay_context_kernel(&source, CallerLimits::default(), &call);
        assert!(matches!(result.outcome(), CallOutcome::Undefined { .. }));
        let (_, parameters) = kernel_declaration_for(
            &source,
            "r",
            SourceIdentity::new("agent-ix", "test:lifecycle"),
        );
        let member = result.requirements().members()[0].clone();
        let bounds = [bound_at(parameters[0].clone())];
        let wire = result
            .context()
            .request(
                member.request_index(),
                &bounds,
                &plugin(),
                input(parameters[0].clone(), 0),
            )
            .expect("an application item");
        let expected = crate::execute::process_provider_obligation(
            member.occurrence().node(),
            member.occurrence().origin(),
            &crate::BoundEntries::new(bounds.to_vec()).unwrap(),
        )
        .unwrap();
        assert_eq!(wire.obligation_identity, expected);
        let replayed = crate::replay(wire, ReplayLimits::default());
        let refuted =
            TerminalRecord::from_plugin_counterexample(RequestIndex::new(0), plugin(), &replayed);
        assert_eq!(refuted.value(), &TerminalValue::Refuted);
        assert_eq!(refuted.backend(), Some(&plugin()));
    }

    fn kernel_member_request(
        source: &str,
        result: &RunWithReplayContext,
        function: &str,
        x: i128,
    ) -> ReplayRequestWire {
        let identity = SourceIdentity::new("agent-ix", "test:lifecycle");
        let (_, parameters) = kernel_declaration_for(source, function, identity.clone());
        let parameter = parameters[0].clone();
        let bounds = [bound_at(parameter.clone())];
        let member = &result.requirements().members()[0];
        let ours = result
            .context()
            .request(
                member.request_index(),
                &bounds,
                &plugin(),
                input(parameter.clone(), x),
            )
            .expect("a member is an application item");
        let packages = fsl_semantics::model::intake::package_input([]).unwrap();
        let function_modes = BTreeMap::new();
        let composed = spine::compose_in_mode(
            identity,
            "unit.native",
            source.as_bytes(),
            &packages,
            &DependencyInput::default(),
            CallerLimits::default().spine,
            spine::ComposeModes {
                check_mode: Some(filament_semantic_value::checking::CheckMode::Kernel),
                function_modes: &function_modes,
            },
        )
        .expect("the oracle compiles independently under Kernel");
        let oracle = PluginReplayContext::new(
            &composed.checked,
            &composed.emitted_unit,
            &composed.models,
            CallerLimits::default(),
        );
        let theirs = oracle
            .request(member.occurrence(), &bounds, &plugin(), input(parameter, x))
            .expect("the oracle builds the member request");
        assert_eq!(ours.package, theirs.package);
        assert_eq!(ours.obligation_identity, theirs.obligation_identity);
        assert_eq!(ours.replay_target, theirs.replay_target);
        assert_eq!(ours.dependencies, theirs.dependencies);
        assert_eq!(ours.backend, theirs.backend);
        assert_eq!(ours.stage_limits, theirs.stage_limits);
        assert_eq!(ours.accounting_limits, theirs.accounting_limits);
        assert_eq!(ours.byte_provision, theirs.byte_provision);
        assert_eq!(ours.declared_domains.len(), theirs.declared_domains.len());
        for (a, b) in ours.declared_domains.iter().zip(&theirs.declared_domains) {
            assert_eq!(format!("{a:?}"), format!("{b:?}"));
        }
        assert_eq!(ours.source, theirs.source);
        assert_eq!(ours.profile_selections, theirs.profile_selections);
        assert_eq!(ours.state_environment, theirs.state_environment);
        assert_eq!(ours.supplied_limits, theirs.supplied_limits);
        assert_eq!(ours.object_environment, theirs.object_environment);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(ours.package.bytes()).unwrap()["name"],
            result.package_name().as_str()
        );
        assert_eq!(
            ours.replay_target
                .members
                .function
                .as_ref()
                .map(ToString::to_string),
            Some(function.to_owned())
        );
        assert_eq!(
            ours.replay_target.occurrence_key.as_ref(),
            Some(member.occurrence())
        );
        ours
    }

    /// Runtime coverage of RK uses genuine Kernel preparation and the shared coordinator.
    #[trace("TC-1173", "FR-404-AC-3")]
    #[test]
    fn an_undefined_calls_member_refutes_and_an_incomplete_calls_request_is_unchanged() {
        let source =
            format!("{SOURCE}function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n");
        let call = |function: &str, x: i64, work_units: u64| Call {
            clocks: Default::default(),
            wire_arguments: Vec::new(),
            canonical_values: false,
            snapshot: None,
            function: function.to_owned(),
            arguments: vec![crate::CallArgument {
                parameter: "x".to_owned(),
                value: Value::Integer(Integer::from(x)),
            }],
            accounting: default_accounting(work_units),
            supplied: filament_semantic_value::supplied_stop::SuppliedLimits::default(),
            backend: filament_eval::ExecutionBackendChoice::Interpreter,
            live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
        };
        let undefined = run_with_replay_context_kernel(
            &source,
            CallerLimits::default(),
            &call("r", 0, 1_000_000),
        );
        assert!(matches!(undefined.outcome(), CallOutcome::Undefined { .. }));
        let settle = |wire| {
            TerminalRecord::from_plugin_counterexample(
                RequestIndex::new(0),
                plugin(),
                &crate::replay(wire, ReplayLimits::default()),
            )
        };
        let record = settle(kernel_member_request(&source, &undefined, "r", 0));
        assert_eq!(record.value(), &TerminalValue::Refuted);
        assert_eq!(record.backend(), Some(&plugin()));
        let r_x = kernel_member_request(&source, &undefined, "r", 3);
        assert!(matches!(
            settle(r_x).value(),
            TerminalValue::Inconclusive(crate::InconclusiveCause::ReplayParity(_))
        ));
        let incomplete =
            run_with_replay_context_kernel(&source, CallerLimits::default(), &call("q", 3, 0));
        assert!(matches!(incomplete.outcome(), CallOutcome::Incomplete(_)));
        let completed = run_with_replay_context_kernel(
            &source,
            CallerLimits::default(),
            &call("q", 3, 1_000_000),
        );
        let expected = kernel_member_request(&source, &completed, "q", 9);
        let built = kernel_member_request(&source, &incomplete, "q", 9);
        assert_eq!(built.package, expected.package);
        assert_eq!(built.obligation_identity, expected.obligation_identity);
        assert_eq!(built.replay_target, expected.replay_target);
        assert_eq!(built.byte_provision, expected.byte_provision);
    }
}
