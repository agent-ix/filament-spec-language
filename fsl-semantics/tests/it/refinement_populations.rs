// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-139 (TC-544): the S3 check of the population-valued forms inside a
//! refinement row, and the subject-admission check of the bounds that depend
//! on a population's universe size, over ADR-020 §8's ring buffer refined to
//! a queue (`tests/support/ring_queue.rs`).

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_checked::node::NodeKind;
use filament_exact::{
    CardinalityBound, CollectionKind, CollectionType, IntegerInterval, ValueType,
};
use fsl_foundation::Code;
use fsl_semantics::check::{
    ApplicationTerm, Binding, BindingValue, BodyTerm, CheckedRefinement, CheckedRow, LeafTerm,
    MemberTerm, PopulationUse, PopulationValue, RefinementCause,
};
use ix_trace_rs::trace;

#[path = "../../../tests/support/ring_queue.rs"]
mod ring_queue;

use ring_queue::{all_rows, checked, key, refused, ring_is_queue, ITEMS, RING};

/// `Int[lower, upper]`.
fn interval(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("an interval"))
}

fn slots() -> DeclarationKey {
    key(RING, "slots")
}

/// The field rows of the `Queue` object row, in `all_rows` order: `items`,
/// `used`, `busy`, `cells`, `lone`, `ghost`, `solid`, `total`.
fn queue_row(refinement: &CheckedRefinement, field_index: usize) -> &CheckedRow {
    let [queue, _cell] = refinement.objects() else {
        panic!("the queue and the cell rows: {:?}", refinement.objects());
    };
    &queue.fields[field_index].expression
}

/// FR-139-AC-1 (TC-544 step 1): the `items` row checks at S3 with `n`
/// symbolic: it depends on no universe size, so it records no bound, and its
/// `seq` is the declared `Sequence<Int[0, 1]>[0, 2]` its maximum length 2
/// gives. `count` checks as an integer, `exists` as a Boolean and the
/// population as its objects: the rows whose value depends on `n` record the
/// bound the subject's admission checks.
#[trace(
    "TC-544",
    "FR-139-AC-1",
    "FSpec-TC-311",
    "FSpec-FR-376-AC-1",
    "FSpec-FR-376-AC-2"
)]
#[test]
fn the_ring_rows_check_at_s3_with_the_universe_size_symbolic() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let items = queue_row(refinement, 0);
    assert!(
        items.population_uses().is_empty(),
        "{:?}",
        items.population_uses()
    );
    assert_eq!(
        items.body().value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            interval(0, 1),
            Some(CardinalityBound::new(0, 2).expect("a bound")),
        ))
    );
    let busy = queue_row(refinement, 2);
    assert_eq!(busy.body().value_type(), &ValueType::Boolean);
    assert!(busy.population_uses().is_empty());
    let used = queue_row(refinement, 1);
    assert_eq!(used.body().value_type(), &ValueType::Integer);

    let [count] = used.population_uses() else {
        panic!("the count row records its bound");
    };
    assert_eq!(count.value(), &PopulationValue::Count);
    assert_eq!(count.population(), "R::slots");
    assert_eq!(count.population_key(), &slots());
    let [cells] = queue_row(refinement, 3).population_uses() else {
        panic!("the population row records its bound");
    };
    assert_eq!(cells.value(), &PopulationValue::Objects);
}

/// FR-139-AC-1 (TC-544 step 1), at subject admission: with `slots = {s0,
/// s1}`, `R::slots` is typed `Set<Reference<R::Slot>>[0, 2]` and `count` is
/// `Int[0, 2]`, which conform to the types they are used as.
#[trace("TC-544", "FR-139-AC-1")]
#[test]
fn admission_substitutes_the_universe_size_into_the_recorded_bounds() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let uses: Vec<&PopulationUse> = [1, 3]
        .into_iter()
        .flat_map(|index| queue_row(refinement, index).population_uses())
        .collect();
    let [count, objects] = uses[..] else {
        panic!("two recorded bounds");
    };
    assert_eq!(count.admitted_type(2), Some(interval(0, 2)));
    let admitted = objects.admitted_type(2);
    let Some(ValueType::Collection(set)) = &admitted else {
        panic!("a collection");
    };
    assert_eq!(set.kind(), CollectionKind::Set);
    assert_eq!(
        set.bound(),
        Some(CardinalityBound::new(0, 2).expect("a bound"))
    );
    assert_eq!(admitted.as_ref(), Some(objects.required()));
    let universes = BTreeMap::from([(slots(), 2)]);
    assert_eq!(refinement.admit_universes(&universes), Ok(()));
}

/// FR-139-AC-5 (TC-544 step 5): the `count` row mapped to an abstract field
/// of type `Int[0, 2]` checks at S3 with `n` symbolic and is admitted with
/// `slots = {s0, s1}`; with `{s0, s1, s2}` it is refused
/// `ill_typed`/`type-mismatch` before exploration, naming the row,
/// `R::slots` and the size 3. This is verified at the `admit_universes`
/// seam; a request that calls it is wired by QSL-544, which must add the
/// request-level assertion.
#[trace("TC-544")]
#[test]
fn a_universe_that_overflows_the_used_type_is_refused_at_admission() {
    let graph = checked(&ring_is_queue(&format!(
        "{ITEMS}\nused = count(s in R::slots where s.ring = self);"
    )))
    .graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    assert_eq!(
        refinement.admit_universes(&BTreeMap::from([(slots(), 2)])),
        Ok(())
    );
    let Err(found) = refinement.admit_universes(&BTreeMap::from([(slots(), 3)])) else {
        panic!("a universe of three slots overflows Int[0, 2]");
    };
    let [refusal] = &found[..] else {
        panic!("one refusal: {found:?}");
    };
    assert_eq!(
        refusal.cause,
        RefinementCause::PopulationBound {
            row: "Q::Queue.used".to_owned(),
            population: "R::slots".to_owned(),
            size: 3,
        }
    );
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::IllTyped, Some("type-mismatch"))
    );
    // A subject that supplies no universe for the population admits nothing
    // about it.
    let Err(missing) = refinement.admit_universes(&BTreeMap::new()) else {
        panic!("no universe for R::slots");
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].cause.code(), Code::MissingDeclaration);
}

/// The one refusal of `rows`, with its code, cause and the text it is at.
fn only_refusal(rows: &str) -> (Code, Option<&'static str>, String) {
    let found = refused(&ring_is_queue(rows));
    let [refusal] = &found[..] else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    (
        refusal.refusal.cause.code(),
        refusal.refusal.cause.cause(),
        refusal.at.clone(),
    )
}

/// FR-998-AC-1: a refinement `only` lowers to a catalogued collection
/// operation over the population's all-instances value.
#[trace("TC-2822", "FR-998-AC-1", "FSpec-FR-376-AC-3")]
#[test]
fn only_over_a_population_lowers() {
    let checked = checked(&ring_is_queue(
        "lone = only(s in R::slots where s.ring = self: s.value);",
    ));
    let graph = checked.graph.semantic_graph();
    let application_at = |reference: &fsl_semantics::check::NodeRef, identity: &str| {
        let key = reference.admitted().expect("a lowered node reference");
        let node = graph.node(key.clone()).expect("a node for the reference");
        let BodyTerm::Application(application) = node.body() else {
            panic!("{identity} names an application");
        };
        assert_eq!(application.operation.identity(), identity);
        application
    };
    let only_nodes: Vec<&ApplicationTerm> = graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.collection.only" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    let [only] = only_nodes.as_slice() else {
        panic!("one collection.only application: {only_nodes:?}");
    };
    let result_type = only.result_type.admitted().expect("the result type is resolved");
    assert_eq!(
        only.operation.member(),
        Some(&fsl_semantics::value::Member::TypeArgument {
            declaration: result_type.clone(),
                alias: None,
        })
    );
    let [MemberTerm::Leaf(LeafTerm::Reference { target: filtered }), MemberTerm::Binding(Binding {
        name: only_binder,
        value: BindingValue::Leaf(LeafTerm::Reference { target: only_body }),
    })] = only.arguments.as_slice()
    else {
        panic!("collection.only has a filtered collection and one value binder");
    };
    assert_eq!(only_binder, "s");
    let filter = application_at(filtered, "filament.op.collection.filter");
    let [MemberTerm::Leaf(LeafTerm::Reference { target: population }), MemberTerm::Binding(Binding {
        name: filter_binder,
        value: BindingValue::Leaf(LeafTerm::Reference { target: predicate }),
    })] = filter.arguments.as_slice()
    else {
        panic!("collection.filter has the population and its condition binder");
    };
    assert_eq!(filter_binder, "s");
    let all_instances = application_at(population, "filament.op.model.all_instances");
    let [MemberTerm::Leaf(LeafTerm::Reference { target: population_node })] =
        all_instances.arguments.as_slice()
    else {
        panic!("all_instances has one population reference");
    };
    let population_node = graph
        .node(population_node.admitted().expect("the population is resolved").clone())
        .expect("the population node exists");
    assert_eq!(population_node.semantic_form(), "population");
    application_at(predicate, "filament.op.reference.eq");
    let only_body = graph
        .node(only_body.admitted().expect("the only body is resolved").clone())
        .expect("the only body node exists");
    assert_eq!(only_body.semantic_type(), Some(result_type.clone()));
}

/// FR-998-AC-1: without `where`, `only` takes the population's
/// `all_instances` application directly as its source.
#[trace("TC-2822", "FR-998-AC-1", "FSpec-FR-376-AC-3")]
#[test]
fn only_without_where_uses_all_instances_as_its_source() {
    let checked = checked(&ring_is_queue("lone = only(s in R::slots: s.value);"));
    let graph = checked.graph.semantic_graph();
    let only_nodes: Vec<&ApplicationTerm> = graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.collection.only" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    let [only] = only_nodes.as_slice() else {
        panic!("one collection.only application: {only_nodes:?}");
    };
    let result_type = only.result_type.admitted().expect("the result type is resolved");
    assert_eq!(
        only.operation.member(),
        Some(&fsl_semantics::value::Member::TypeArgument {
            declaration: result_type.clone(),
                alias: None,
        })
    );
    let [MemberTerm::Leaf(LeafTerm::Reference { target: source }), MemberTerm::Binding(Binding {
        name,
        value: BindingValue::Leaf(LeafTerm::Reference { target: body }),
    })] = only.arguments.as_slice()
    else {
        panic!("collection.only has a source and one value binder");
    };
    assert_eq!(name, "s");
    let source = graph
        .node(source.admitted().expect("the source is resolved").clone())
        .expect("the source node exists");
    let BodyTerm::Application(source) = source.body() else {
        panic!("the source is the all_instances application");
    };
    assert_eq!(
        source.operation.identity(),
        "filament.op.model.all_instances"
    );
    let body = graph
        .node(body.admitted().expect("the body is resolved").clone())
        .expect("the body node exists");
    assert_eq!(body.semantic_type(), Some(result_type.clone()));
}

/// FR-139-AC-2 (TC-544 step 2): `only` over a condition that is not Boolean
/// refuses `ill_typed`/`type-mismatch` at the condition.
#[trace("TC-544", "FR-139-AC-2", "FSpec-TC-311", "FSpec-FR-376-AC-4")]
#[test]
fn a_non_boolean_condition_refuses_at_the_condition() {
    assert_eq!(
        only_refusal("lone = only(s in R::slots where s.index: s.value);"),
        (Code::IllTyped, Some("type-mismatch"), "s.index".to_owned())
    );
}

/// FR-391-AC-10: a state-clause sequence lowers over its integer range, and a
/// population count in the same position is admitted.
#[trace("TC-544", "FR-139-AC-2", "FSpec-TC-311", "FSpec-FR-376-AC-4")]
#[test]
fn population_calls_in_clause_bodies_follow_the_ruling() {
    let sequence = checked(
        "invariant Total using v on R::Ring at current { \
         size(seq(i in 0 .. 1: i)) = 2 }",
    );
    let body = sequence
        .graph
        .state_clause("Total")
        .expect("clause checks")
        .body();
    let sequences: Vec<_> = crate::relationship_navigation::descendants(body)
        .into_iter()
        .filter(|node| matches!(node.kind(), NodeKind::Seq { .. }))
        .collect();
    let [sequence] = sequences.as_slice() else {
        panic!("the clause body contains the sequence form");
    };
    assert_eq!(
        sequence.value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            interval(0, 1),
            Some(CardinalityBound::new(0, 2).expect("bound")),
        ))
    );
    // `s.ring = self` compares references, which needs the state-graph
    // layer that `filament.model.complete/v1` includes.
    checked(
        "invariant Total using m on R::Ring at current { \
         count(s in R::slots where s.ring = self) = 2 }\n\
         invariant Anything using v on R::Ring at current { true }",
    );
}

/// FR-139 behavior: a binder over a name that is not a concrete population
/// of the refinement's concrete model refuses `missing_declaration`/
/// `missing-name`.
#[trace("TC-544", "FSpec-TC-311", "FSpec-FR-376-AC-4")]
#[test]
fn a_binder_over_an_unknown_population_refuses_as_a_missing_name() {
    let (code, cause, _) = only_refusal("busy = exists(s in R::nowhere where s.value = 1);");
    assert_eq!(
        (code, cause),
        (Code::MissingDeclaration, Some("missing-name"))
    );
}

/// FR-139 behavior: a `seq` bound that is not an integer refuses
/// `ill_typed`/`type-mismatch` at the bound.
#[trace("TC-544", "FSpec-TC-311", "FSpec-FR-376-AC-4")]
#[test]
fn a_seq_bound_that_is_not_an_integer_refuses_at_the_bound() {
    assert_eq!(
        only_refusal("items = seq(i in 0 .. self: i);"),
        (Code::IllTyped, Some("type-mismatch"), "self".to_owned())
    );
}

/// FR-139-AC-1/AC-5 at S3: a row whose result shape cannot fit the type it
/// is used as for any universe size refuses at S3, not at admission: a
/// `count` into a Boolean field.
#[trace("TC-544", "FR-139-AC-5")]
#[test]
fn a_count_used_as_a_boolean_refuses_whatever_the_universe_size() {
    let (code, cause, at) = only_refusal("busy = count(s in R::slots where s.value = 1);");
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
    assert_eq!(at, "count(s in R::slots where s.value = 1)");
}

/// FR-139: arithmetic around a population count preserves the size-dependent
/// bound and checks at subject admission.
#[trace("TC-544")]
#[test]
fn a_row_bound_depending_on_population_size_checks() {
    let graph = checked(&ring_is_queue(
        "used = count(s in R::slots where s.value = 1) + 0;",
    ))
    .graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let [count] = queue_row(refinement, 0).population_uses() else {
        panic!("the row retains its population-dependent count bound");
    };
    assert_eq!(count.value(), &PopulationValue::Count);
    assert_eq!(count.admitted_type(2), Some(interval(0, 2)));
    assert_eq!(refinement.admit_universes(&BTreeMap::from([(slots(), 2)])), Ok(()));
}

/// FR-145: population sums admit decimal summands and lower to the decimal
/// collection sum operation.
#[trace("TC-544", "FSpec-FR-376-AC-1")]
#[test]
fn sum_over_a_population_with_a_decimal_body_checks() {
    let source = ring_is_queue(
        "busy = sum(s in R::slots: convert<D>(convert<N>(s.value))) = \
         sum(s in R::slots: convert<D>(convert<N>(s.value)));",
    )
    .replacen(
        "invariant Anything",
        "type N = Rational[0, 2; 1, 1];\ntype D = Decimal[0, 100; 1, 1; exact];\ninvariant Anything",
        1,
    );
    let graph = checked(&source).graph;
    assert!(graph.semantic_graph().nodes().any(|node| {
        matches!(node.body(), fsl_semantics::check::BodyTerm::Application(application)
            if application.operation.identity() == "filament.op.collection.sum.decimal")
    }));
}

/// FR-139: `sum` refuses a Boolean summand as ill-typed.
#[trace("TC-544")]
#[test]
fn sum_over_a_population_with_a_boolean_body_refuses_ill_typed() {
    let (code, cause, at) = only_refusal(
        "total = sum(s in R::slots where s.ring = self: s.value = 1);",
    );
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
    assert_eq!(at, "s.value = 1");
}

/// FR-139 (admission): a population whose universe is smaller than the
/// abstract field's maximum is admitted: `Set<Reference<T>>[0, n]` fits
/// `Set<Reference<T>>[0, 2]` for every `n` up to 2.
#[trace("TC-544", "FR-139-AC-1")]
#[test]
fn a_universe_below_the_used_maximum_is_admitted() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    for size in [0, 1, 2] {
        assert_eq!(
            refinement.admit_universes(&BTreeMap::from([(slots(), size)])),
            Ok(()),
            "{size} slots"
        );
    }
    assert!(refinement
        .admit_universes(&BTreeMap::from([(slots(), 3)]))
        .is_err());
}

/// FR-139 (`sum`): a `sum` types as an integer, records its size-dependent
/// bound and is checked at admission against the type of its use. The bound
/// is `[min(0, n*lo), max(0, n*hi)]` (an open spec change).
#[trace("TC-544", "FSpec-TC-311", "FSpec-FR-376-AC-1")]
#[test]
fn a_sum_records_its_bound_and_is_checked_at_admission() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let total = queue_row(refinement, 7);
    assert_eq!(total.body().value_type(), &ValueType::Integer);
    let [used] = total.population_uses() else {
        panic!("the sum row records its bound");
    };
    assert_eq!(used.population(), "R::slots");
    let PopulationValue::Sum { summand } = used.value() else {
        panic!("a sum: {:?}", used.value());
    };
    assert_eq!(
        summand,
        &IntegerInterval::new(0_i64.into(), 1_i64.into()).expect("an interval")
    );
    assert_eq!(used.admitted_type(2), Some(interval(0, 2)));
    let Err(found) = refinement.admit_universes(&BTreeMap::from([(slots(), 3)])) else {
        panic!("three slots overflow Int[0, 2]");
    };
    assert!(found.iter().any(|refusal| matches!(
        &refusal.cause,
        RefinementCause::PopulationBound { row, size: 3, .. } if row == "Q::Queue.total"
    )));
}

/// FR-139 with FR-391: a population is no value in a state clause or a
/// `holds` atom (`ill_typed`/`operator-ineligible`), and names nothing in a
/// function body (`missing_declaration`/`missing-name`).
#[trace("TC-544", "FR-139-AC-2")]
#[test]
fn a_population_name_outside_a_refinement_refuses() {
    for (declaration, at, code, cause) in [
        (
            "invariant Total using v on R::Ring at current { size(R::slots) = 2 }",
            "R::slots",
            Code::IllTyped,
            "operator-ineligible",
        ),
        (
            "temporal W using inf over (r: R::Ring) clock \"model-steps\" on origin \
             { always holds(size(R::slots) = 2) }",
            "R::slots",
            Code::IllTyped,
            "operator-ineligible",
        ),
        (
            "function f using v(): Boolean pure { size(R::slots) = 2 }",
            "slots",
            Code::MissingDeclaration,
            "missing-name",
        ),
    ] {
        let found = refused(declaration);
        let [refusal] = &found[..] else {
            panic!("one refusal for {declaration}");
        };
        assert_eq!(
            (refusal.refusal.cause.code(), refusal.refusal.cause.cause()),
            (code, Some(cause)),
            "{declaration}"
        );
        assert_eq!(refusal.at, at);
    }
}

/// FR-391-AC-10: `seq` outside a population position refuses the named
/// `unsupported_construct`/`expression-form` outcome.
#[trace("TC-544", "FR-391")]
#[test]
fn a_population_call_in_a_function_body_still_refuses_expression_form() {
    let found = refused("function f using v(): Sequence<Integer> pure { seq(i in 0 .. 1: i) }");
    let [refusal] = &found[..] else {
        panic!("one refusal for the function population call");
    };
    assert_eq!(
        (refusal.refusal.cause.code(), refusal.refusal.cause.cause()),
        (Code::UnsupportedConstruct, Some("expression-form"))
    );
    assert_eq!(refusal.at, "seq(i in 0 .. 1: i)");
}
