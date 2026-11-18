// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use filament_contract_model::{
    CheckedBindingNameKind, CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm,
};

fn protocol_emission(model: &[u8]) -> fsl_package::Emission {
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use fsl_semantics::check::PackageDeclarations;

    let source = fsl_foundation::SourceIdentity::new("consumer", "protocol-contract");
    let parsed = fsl_cst::parse(
        source,
        "protocol.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("protocol parses");
    let raw_source = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("protocol forms build");
    let models = fsl_semantics::model::intake::package_input([model]).expect("model intake");
    let selected = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &models,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("model selections admit");
    let graph = PackageDeclarations::assemble(
        raw_source,
        "protocol-consumer".into(),
        built,
        selected,
        Vec::new(),
    )
    .expect("protocol assembles")
    .check(CheckMode::Linked, CheckingLimits::default())
    .expect("protocol checks");
    let source = graph.source().clone();
    let clock = filament_contract_model::ClockConfiguration::Timed {
        unit: "ticks".into(),
    };
    let selections = graph
            .protocols()
            .iter()
            .flat_map(|protocol| {
                protocol
                    .obligations
                    .compensations
                    .iter()
                    .filter(|template| {
                        matches!(
                            template.temporal_profile,
                            Some(
                                fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
                                    | fsl_semantics::check::TemporalProfile::FixedSampleFalseExtension
                                    | fsl_semantics::check::TemporalProfile::TimestampedEventFiniteWindow
                            )
                        )
                    })
                    .map(|template| template.span)
                    .chain(
                        protocol
                            .awaits
                            .iter()
                            .filter(|awaited| {
                                matches!(
                                    awaited.profile,
                                    Some(
                                        fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
                                            | fsl_semantics::check::TemporalProfile::FixedSampleFalseExtension
                                            | fsl_semantics::check::TemporalProfile::TimestampedEventFiniteWindow
                                    )
                                )
                            })
                            .map(|awaited| awaited.span),
                    )
            })
            .map(|span| {
                fsl_package::TemporalClockSelection::new(source.clone(), span, clock.clone())
            })
            .collect::<Vec<_>>();
    let package = fsl_package::CheckedPackage::link(graph);
    fsl_package::emit_checked_with_temporal_selections(
        &package,
        &selections,
        fsl_package::EmitLimits::new(1 << 20),
    )
    .expect("protocol package emits")
}

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile p = \"filament.protocol.finite-global/v1\";\n\
profile v = \"filament.value.complete/v1\";\n\
profile t = \"filament.temporal.timed/v1\";\n\
profile u = \"filament.temporal.timed/v1\";\n\
model M = \"acme/orders\";\n\
protocol Flow using p over (input: M::Widget) on origin {\n\
  role Buyer on M::Widget;\n\
  role Seller on M::Widget;\n\
  channel Ch from Buyer to Seller carries Boolean ordering unordered delivery at-least-once capacity 4 overflow block;\n\
  compensate Full for Main::Applied as (fwd: M::Widget) by Buyer on M::Widget::touch using t clock \"ticks\" {\n\
    activate first (trig: M::Widget) when { true } {}\n\
    within [0, 10]; attempts 2 of M::Attempt;\n\
    retry (early: M::Attempt, late: M::Attempt) { false };\n\
    commit never; recover (rec: M::Widget) { true };\n\
  }\n\
  run sequence Main {\n\
    attempt Update by Buyer on M::Widget::touch contracts [] as (tried: M::Widget) { true };\n\
    effect Applied of Update as (applied: M::Widget) { true };\n\
    send Offer via Ch as (sent: Boolean) { true };\n\
    receive Seen via Ch of Offer as (received: Boolean) { true };\n\
    await Wait after Main::Applied using u clock \"ticks\" within [0, 1] match event Arrived by Seller as (arrived: Boolean) { true }; then sequence Then {} timeout sequence Late {}\n\
  choice Pick by Seller visible (received) { case yes when { true } sequence Yes {} case no when { false } sequence No {} }\n\
  }\n\
  finish End as (outcome: Boolean) { true };\n\
}\n";

fn checked_protocol() -> filament_contract_model::CheckedPackageV2 {
    let model = super::code_generation::model_with_touch();
    let emitted = protocol_emission(&model);
    let evidence = evidence_for(emitted.package().bytes(), Some(("acme/orders", &model)));
    match filament_contract_model::read_checked_package(
        emitted.package().bytes(),
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => *package,
        other => panic!("strict reader refused emitted package: {other:?}"),
    }
}

fn package() -> &'static filament_contract_model::CheckedPackageV2 {
    static PACKAGE: std::sync::OnceLock<filament_contract_model::CheckedPackageV2> =
        std::sync::OnceLock::new();
    PACKAGE.get_or_init(checked_protocol)
}

fn protocol_id() -> filament_contract_model::CheckedNodeId {
    package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            node.node_tag.as_ref() == "protocol" && node.semantic_form.as_ref() == "protocol_clause"
        })
        .expect("protocol clause")
        .node_id
        .clone()
}

fn local_name(node: &CheckedSemanticNodeV2) -> Option<&str> {
    node.declaration
        .as_ref()?
        .qualified_name
        .last()
        .map(AsRef::as_ref)
}

fn node_for_id<'a>(package: &'a CheckedPackageV2, id: &CheckedNodeId) -> &'a CheckedSemanticNodeV2 {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("checked graph contains the listed node")
}

fn id_named<'a>(
    package: &'a CheckedPackageV2,
    ids: &[&'a CheckedNodeId],
    name: &str,
) -> &'a CheckedNodeId {
    ids.iter()
        .copied()
        .find(|id| local_name(node_for_id(package, id)) == Some(name))
        .expect("protocol view contains the declared node")
}

fn control_named<'a>(
    package: &'a CheckedPackageV2,
    operation: fsl_package::protocol_view::ControlOperation,
    name: &str,
) -> &'a CheckedSemanticNodeV2 {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            local_name(node) == Some(name)
                && fsl_package::protocol_view::control_operation(node).ok() == Some(operation)
        })
        .expect("checked graph contains the declared control")
}

fn binding_value<'a>(term: &'a CheckedTerm, name: &str) -> Option<&'a CheckedTerm> {
    match term {
        CheckedTerm::Binding {
            name: binding,
            value,
        } => {
            if matches!(binding.kind(), CheckedBindingNameKind::Other(found) if found == name) {
                Some(value)
            } else {
                binding_value(value, name)
            }
        }
        CheckedTerm::Application { arguments, .. } => arguments
            .iter()
            .find_map(|argument| binding_value(argument, name)),
        CheckedTerm::Aggregate { members } => members
            .iter()
            .find_map(|member| binding_value(member, name)),
        _ => None,
    }
}

fn collect_references<'a>(term: &'a CheckedTerm, references: &mut Vec<&'a CheckedNodeId>) {
    match term {
        CheckedTerm::Reference { target } => references.push(target),
        CheckedTerm::Binding { value, .. } => collect_references(value, references),
        CheckedTerm::Application { arguments, .. } => {
            for argument in arguments {
                collect_references(argument, references);
            }
        }
        CheckedTerm::Aggregate { members } => {
            for member in members {
                collect_references(member, references);
            }
        }
        _ => {}
    }
}

fn binding_references<'a>(node: &'a CheckedSemanticNodeV2, name: &str) -> Vec<&'a CheckedNodeId> {
    let value = binding_value(&node.body, name).expect("checked node has the declared member");
    let mut references = Vec::new();
    collect_references(value, &mut references);
    references
}

fn binding_reference<'a>(node: &'a CheckedSemanticNodeV2, name: &str) -> &'a CheckedNodeId {
    let references = binding_references(node, name);
    assert_eq!(
        references.len(),
        1,
        "member {name} is one checked reference"
    );
    references[0]
}

fn verify_protocol_facts() {
    let package = package();
    let id = protocol_id();
    let view = fsl_package::protocol_view::protocol_view(package, &id).expect("protocol view");
    assert_eq!(view.clause(), &id);
    assert_eq!(view.roles().len(), 2);
    let buyer = id_named(package, view.roles(), "Buyer");
    let seller = id_named(package, view.roles(), "Seller");
    assert_eq!(view.channels().len(), 1);
    let channel_id = id_named(package, view.channels(), "Ch");
    assert_eq!(view.queues().len(), 1);
    assert_eq!(view.compensations().len(), 1);
    let compensation_id = id_named(package, view.compensations(), "Full");
    let run = control_named(
        package,
        fsl_package::protocol_view::ControlOperation::Sequence,
        "Main",
    );
    let finish = control_named(
        package,
        fsl_package::protocol_view::ControlOperation::Finish,
        "End",
    );
    assert_eq!(view.run(), &run.node_id);
    assert_eq!(view.finish(), &finish.node_id);

    let relation =
        fsl_package::protocol_view::causal_relation(package, &id).expect("causal relation");
    let applied = control_named(
        package,
        fsl_package::protocol_view::ControlOperation::Effect,
        "Applied",
    );
    let await_node = control_named(
        package,
        fsl_package::protocol_view::ControlOperation::Await,
        "Wait",
    );
    let anchor_edge = fsl_package::protocol_view::CausalEdge {
        from: fsl_package::protocol_view::PortRef {
            node: applied.node_id.clone(),
            port: fsl_package::protocol_view::Port::Exit,
        },
        to: fsl_package::protocol_view::PortRef {
            node: await_node.node_id.clone(),
            port: fsl_package::protocol_view::Port::Enter,
        },
        kind: fsl_package::protocol_view::EdgeKind::AwaitAnchor,
    };
    assert!(relation.edges().contains(&anchor_edge));

    let channel = node_for_id(package, channel_id);
    let endpoints =
        fsl_package::protocol_view::channel_endpoints(channel).expect("channel endpoints");
    assert_eq!(endpoints.sender, buyer);
    assert_eq!(endpoints.receiver, seller);
    assert_eq!(endpoints.sender, binding_reference(channel, "from"));
    assert_eq!(endpoints.receiver, binding_reference(channel, "to"));
    assert_eq!(
        endpoints.message_type,
        binding_reference(channel, "carries")
    );

    let anchor = fsl_package::protocol_view::await_anchor(await_node).expect("await anchor");
    assert_eq!(
        anchor.kind,
        fsl_package::protocol_view::AwaitAnchorKind::Event
    );
    assert_eq!(anchor.target, &applied.node_id);

    let choice_node = control_named(
        package,
        fsl_package::protocol_view::ControlOperation::Choice,
        "Pick",
    );
    assert!(view.controls().contains(&&applied.node_id));
    assert!(view.controls().contains(&&await_node.node_id));
    assert!(view.controls().contains(&&choice_node.node_id));
    let choice =
        fsl_package::protocol_view::choice_visibility(choice_node).expect("choice visibility");
    assert_eq!(choice.choice, &choice_node.node_id);
    assert_eq!(choice.owner, seller);
    let expected_visible = binding_references(choice_node, "visible");
    assert_eq!(expected_visible.len(), 1);
    assert_eq!(choice.visible, expected_visible);

    let compensation = compensation_id;
    assert_eq!(
        fsl_package::protocol_view::attempts_maximum(package, compensation).unwrap(),
        2
    );

    let buyer_node = node_for_id(package, buyer);
    assert!(matches!(
        fsl_package::protocol_view::protocol_view(package, buyer),
        Err(fsl_package::protocol_view::ViewRefusal::NotAProtocolClause { .. })
    ));
    assert!(matches!(
        fsl_package::protocol_view::causal_relation(package, buyer),
        Err(fsl_package::protocol_view::ViewRefusal::NotAProtocolClause { .. })
    ));
    assert!(matches!(
        fsl_package::protocol_view::channel_endpoints(buyer_node),
        Err(fsl_package::protocol_view::ViewRefusal::MissingMember { member: "from", .. })
    ));
    assert!(matches!(
        fsl_package::protocol_view::await_anchor(buyer_node),
        Err(fsl_package::protocol_view::ViewRefusal::MissingMember {
            member: "after",
            ..
        })
    ));
    assert!(matches!(
        fsl_package::protocol_view::choice_visibility(await_node),
        Err(fsl_package::protocol_view::ViewRefusal::MalformedMember {
            member: "operator",
            ..
        })
    ));
    assert!(matches!(
        fsl_package::protocol_view::attempts_maximum(package, buyer),
        Err(fsl_package::protocol_view::ViewRefusal::NotACompensation { .. })
    ));
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_01_01_protocol_view_exposes_clause_members_and_controls() {
    verify_protocol_facts();
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_02_01_causal_relation_exposes_derived_edges() {
    verify_protocol_facts();
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_03_01_channel_endpoints_expose_all_three_nodes() {
    verify_protocol_facts();
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_05_01_await_anchor_exposes_event_target() {
    verify_protocol_facts();
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_06_01_choice_visibility_exposes_owner_and_visible_set() {
    verify_protocol_facts();
}

#[test]
#[trace(
    "TC-2575",
    "FR-911-AC-2",
    "FSpec-TC-1646",
    "FSpec-FR-661-AC-3",
    "TC-2576",
    "FR-911-AC-3"
)]
fn cf_pr_07_01_attempts_maximum_exposes_authored_retry_bound() {
    verify_protocol_facts();
}
