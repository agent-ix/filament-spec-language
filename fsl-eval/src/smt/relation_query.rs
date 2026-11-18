// SPDX-License-Identifier: AGPL-3.0-or-later
//! The bounded FR-315 encoding of one concrete FR-179 relation tuple.

use filament_checked::node::NodeKind;
use filament_exact::Value;
use fsl_semantics::check::CheckedModelRelation;
use fsl_semantics::model::clause_observation::Observation;

use crate::model_subject::ModelSubject;
use crate::simulation::model::ModelSystem;
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout, StateRef};
use crate::smt::refusal::ExpressionLocus;
use crate::smt::script::{Script, Term};
use crate::smt::{Refusal, SmtEncodingRefusal};

/// Encode whether one concrete tuple of transition arguments refutes a
/// checked relation. This bounded encoder supports argument-only relation
/// bodies over Boolean, integer, enum, and receiver values; relation reads
/// from model state remain on the explicit evaluator path.
///
/// The emitted query is `sat` exactly when the body is undefined or false on
/// the supplied tuple. It reuses FR-315's checked-expression reader and
/// canonical SMT-LIB printer, so explicit-state relation tests can compare
/// each concrete tuple's verdict with the solver query form.
///
/// # Errors
///
/// [`SmtEncodingRefusal`] when the body has state reads, a value has no QF_LIA
/// sort, or the subject cannot be admitted to the symbolic model reader.
pub fn encode_relation_tuple(
    relation: &CheckedModelRelation,
    subject: &ModelSubject<'_>,
    arguments: &[Value],
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let mut pending = vec![relation.typed_body().root()];
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            NodeKind::Field { .. } | NodeKind::History { .. }
        ) {
            return Err(Refusal::OutOfFragment {
                form: "a model-state read in a concrete relation tuple",
                locus: ExpressionLocus {
                    clause: relation.name().to_owned(),
                    node: node.id().index(),
                    component: None,
                },
            }
            .into());
        }
        pending.extend(node.children());
    }
    if arguments.len() != relation.body_parameters().len() {
        return Err(SmtEncodingRefusal::fault(
            "one concrete value per relation binder",
        ));
    }
    for (argument, (_, value_type)) in arguments.iter().zip(relation.body_parameters()) {
        let admits = matches!(
            (argument, value_type),
            (Value::Reference(_), filament_exact::ValueType::Reference(_))
        ) || value_type.admits(argument);
        if !admits {
            return Err(SmtEncodingRefusal::fault(
                "a concrete relation value has its checked type",
            ));
        }
    }
    let system = ModelSystem::new(subject.simulation.clone())
        .map_err(|_| SmtEncodingRefusal::fault("the relation tuple subject is admitted"))?;
    let symbolic = system.symbolic()?;
    let layout = StateLayout::of(&symbolic)?;
    let slots = arguments
        .iter()
        .map(|value| match value {
            Value::Reference(_) => Some(Sym::Object(0)),
            Value::Boolean(_) | Value::Integer(_) | Value::Enum(_) => {
                crate::smt::layout::value_term(value).map(|term| match value {
                    Value::Boolean(_) => Sym::Bool(term),
                    _ => Sym::Int(term),
                })
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| SmtEncodingRefusal::fault("relation tuple values have QF_LIA terms"))?;
    let state = StateRef::Copy(StateCopy::at(0));
    let views = Views::of(state);
    let mut reader = Reader::new(
        &layout,
        &symbolic,
        relation.name(),
        Observation::Current,
        Vec::new(),
        &views,
        slots.into_iter().map(Some).collect(),
    );
    let (body, defined) = reader.boolean(relation.typed_body().root())?;
    let refuted = Term::any([!defined.clone(), Term::all([defined, !body])]);
    let mut script = Script::new();
    script.assert(&refuted);
    Ok(script.render())
}
