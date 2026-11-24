// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration coverage for the FSL refinement forms acceptance criterion.

use fsl_forms::{
    build_unit, AbstractSideForm, ArgumentForm, DeclarationForm, ExprNode, PopulationForm,
    RefinementRowForm, StepClassKeyword, StepSourceForm, StepTargetForm,
};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

fn build(text: &str) -> fsl_forms::ParsedUnit {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("qsl-2093", "fr-310-ac-2"),
        "refinement.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    build_unit(&parsed).expect("S2 builds the forms")
}

fn slice(text: &str, span: fsl_foundation::Span) -> &str {
    &text[span.start..span.end]
}

#[trace("TC-889", "FR-310-AC-2")]
#[test]
fn refinement_items_open_arguments_and_protocol_steps_build_as_written() {
    let ring_source = "language \"ix:native\" edition \"1-draft\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         refinement RingIsQueue using inf abstract Spec concrete Impl {\n\
           object Spec::Queue from Impl::Ring {\n\
             items = seq(i in 0 .. self.size - 1: only(s in Impl::slots where s.ring = self: s.value));\n\
           }\n\
           step Impl::Slot::put -> Spec::Queue::enq(self.ring, _);\n\
         }";
    let ring = build(ring_source);
    let [ring_form] = ring.forms() else {
        panic!("one RingIsQueue declaration: {:?}", ring.forms());
    };
    let DeclarationForm::Refinement(ring_form) = ring_form.form() else {
        panic!("a refinement form: {:?}", ring_form.form());
    };
    let RefinementRowForm::Object(object) = &ring_form.rows[0] else {
        panic!("the object row: {:?}", ring_form.rows[0]);
    };
    let items = &object.fields[0].value.expression;
    let ExprNode::Seq { binder, body, .. } = items.root_node() else {
        panic!("items is a sequence expression: {:?}", items.root_node());
    };
    assert_eq!(binder, "i");
    assert!(matches!(
        items.at(*body).node(),
        ExprNode::Population {
            form: PopulationForm::Only,
            binder,
            population,
            condition: Some(_),
            body: Some(_),
            ..
        } if binder == "s" && population == "Impl::slots"
    ));
    let RefinementRowForm::Step(step) = &ring_form.rows[1] else {
        panic!("the put step row: {:?}", ring_form.rows[1]);
    };
    let StepTargetForm::Apply { arguments, .. } = &step.target else {
        panic!("an Apply target: {:?}", step.target);
    };
    let [ArgumentForm::Expression(receiver), ArgumentForm::Open { .. }] = arguments.as_slice()
    else {
        panic!("[self.ring, Open]: {arguments:?}");
    };
    assert_eq!(
        receiver.spans.at(&[]).map(|span| slice(ring_source, span)),
        Some("self.ring")
    );

    let twice = build(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         refinement CasTwice using inf abstract Spec::Twice concrete Impl {\n\
           step fork Impl::P::par -> node Spec::Twice::a1;\n\
           step Impl::P::a::S -> stutter;\n\
         }",
    );
    let [twice] = twice.forms() else {
        panic!("one CasTwice declaration: {:?}", twice.forms());
    };
    let DeclarationForm::Refinement(twice) = twice.form() else {
        panic!("a refinement form: {:?}", twice.form());
    };
    let AbstractSideForm::Protocol { alias, clause } = &twice.abstract_side else {
        panic!("a protocol abstract side: {:?}", twice.abstract_side);
    };
    assert_eq!(
        (alias.name.as_str(), clause.name.as_str()),
        ("Spec", "Twice")
    );
    assert_eq!(twice.concrete.name, "Impl");
    let [RefinementRowForm::Step(fork), RefinementRowForm::Step(bare)] = twice.rows.as_slice()
    else {
        panic!("two step rows: {:?}", twice.rows);
    };
    assert!(matches!(
        &fork.source,
        StepSourceForm::Named {
            kind: Some(StepClassKeyword::Fork),
            name,
        } if name.name == "Impl::P::par"
    ));
    assert!(matches!(
        &fork.target,
        StepTargetForm::Node { node, .. } if node.name == "Spec::Twice::a1"
    ));
    assert!(matches!(
        &bare.source,
        StepSourceForm::Named { kind: None, name } if name.name == "Impl::P::a::S"
    ));
}
