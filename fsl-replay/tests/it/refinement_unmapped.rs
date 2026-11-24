// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (TC-1086): the rules of `map_state` that name an unmapped type
//! reached through a reference or an object, the first failure in reading
//! order, and a population reference mapped element by element.

use filament_exact::Value;
use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::catalog::UndefinedReason;
use fsl_eval::simulation::model::operation_identity_text;
use fsl_eval::temporal::UndefinedCause;
use fsl_replay::ObjectKey;
use fsl_replay::{
    check_step, map_state, ConcreteStep, EvalFailure, HistoryValues, MappingFailure, MappingRowRef,
    RefinementPosition, StepArgument, StepVerdict, Taken, Undetermined,
};
use fsl_semantics::model::state::ModelState;
use ix_trace_rs::trace;

use crate::ring_world::{
    custom_state, key, ring_is_queue, ring_state, twin_refinement, MapWorld, RingSpec, RingWorld,
    Variant, ITEMS, QUEUE, RING, UNLIMITED,
};

const ITEMS_ROW: &str = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
const ONLY_ROW: &str =
    "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self: s.value));";

fn ring<'a>(key: &'a str, type_name: &'a str, next: Option<(&'a str, &'a str)>) -> RingSpec<'a> {
    RingSpec {
        key,
        type_name,
        head: 0,
        size: 0,
        next,
        chain: &[],
    }
}

fn next_world() -> RingWorld {
    RingWorld::with(
        Variant {
            next: true,
            ..Variant::default()
        },
        &ring_is_queue(&format!("{ITEMS_ROW} next = self.next;")),
    )
}

/// FR-140-AC-6 (TC-1086 step 1): a reference to an object of a type no object
/// row names is `Undetermined` at the referencing row and object, with
/// `UnmappedType`; with `q` of type `Ring` it maps to the `Queue` reference
/// with key `q`.
#[trace("TC-1086", "FR-140-AC-6")]
#[test]
fn a_reference_to_an_unmapped_type_is_undetermined_at_the_referencing_row() {
    let world = next_world();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let sub = key(RING, "SubRing");

    let state = custom_state(
        &concrete,
        &[
            ring("a", "Ring", Some(("q", "SubRing"))),
            ring("q", "SubRing", Some(("q", "SubRing"))),
        ],
        &[],
    );
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    ) else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(undetermined.cause, EvalFailure::UnmappedType(sub));
    assert_eq!(
        undetermined.row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/next"),
        }
    );
    assert_eq!(undetermined.object.expect("an object").key, "a");

    let state = custom_state(
        &concrete,
        &[
            ring("a", "Ring", Some(("q", "Ring"))),
            ring("q", "Ring", Some(("q", "Ring"))),
        ],
        &[],
    );
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["a"];
    let Some(Value::Reference(next)) = queue.fields.get(&key(QUEUE, "Queue/next")) else {
        panic!("a.next is a reference");
    };
    assert_eq!(
        *next,
        abstracted
            .object_reference(&key(QUEUE, "Queue"), "q")
            .expect("a reference")
    );
}

/// `RingIsQueue` over the `chain` variant: the abstract `enq` also takes the
/// chain `rs`, which the `put` row supplies as `self.ring.chain`.
fn chain_world() -> RingWorld {
    let refinement = ring_is_queue(&format!("{ITEMS_ROW} chain = self.chain;"))
        .replace("enq(self.ring, x)", "enq(self.ring, x, self.ring.chain)");
    RingWorld::with(
        Variant {
            chain: true,
            ..Variant::default()
        },
        &refinement,
    )
}

/// FR-141-AC-13 (TC-546 step 8): `put` on ring `a`, whose `chain` is `[b,
/// a]`, passes as the one abstract `enq(a, 1, rs)` whose argument `rs` is
/// the `Queue` references `[b, a]` the post state's `chain` holds, so the
/// postcondition `size(rs) = size(self.chain)` holds with `rs` the lifted
/// references: the argument row's nested concrete references are lifted
/// before the abstract operation reads them.
#[trace("TC-546", "FR-141-AC-13")]
#[test]
fn a_step_argument_nesting_references_reaches_the_abstract_operation_lifted() {
    let world = chain_world();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let chain: &[(&str, &str)] = &[("b", "Ring"), ("a", "Ring")];
    let state = |size: i64, value: i64| {
        custom_state(
            &concrete,
            &[
                RingSpec {
                    size,
                    chain,
                    ..ring("a", "Ring", None)
                },
                ring("b", "Ring", None),
            ],
            &[("a0", "a", 0, value), ("a1", "a", 1, 0)],
        )
    };
    let (pre, post) = (state(0, 0), state(1, 1));
    let put = operation_identity_text(&key(RING, "Slot/put")).expect("an identity text");
    let wanted = concrete.state_key_of(&post).expect("a key").0;
    let (transition, reached) = concrete
        .expand_state(&pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == put
                && concrete.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .expect("put takes the ring from empty to one item");

    let mapped = |state| {
        map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            state,
            &HistoryValues::default(),
        )
        .expect("the state maps")
        .visible
    };
    let (abstract_pre, abstract_post) = (mapped(&pre), mapped(&post));
    let enq = operation_identity_text(&key(QUEUE, "Queue/enq")).expect("an identity text");
    let post_key = abstracted.state_key_of(&abstract_post).expect("a key").0;
    let expected: Vec<_> = abstracted
        .expand_state(&abstract_pre)
        .expect("the abstract state expands")
        .successors
        .into_iter()
        .filter(|(transition, reached)| {
            transition.operation() == enq
                && abstracted.state_key_of(reached.state()).expect("a key").0 == post_key
        })
        .map(|(transition, _)| transition)
        .collect();
    assert!(
        !expected.is_empty(),
        "the abstract enq reaches the mapped post state"
    );

    let verdict = check_step(
        world.refinement(),
        &concrete,
        &abstracted,
        &RefinementPosition {
            concrete: pre.clone(),
            history: HistoryValues::default(),
            abstract_states: vec![abstract_pre],
            abstract_protocol_states: Vec::new(),
        },
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict;
    let StepVerdict::Passes { taken, .. } = verdict else {
        panic!("the put passes: {verdict:?}");
    };
    let Taken::Abstract(taken) = *taken else {
        panic!("the put takes an abstract step");
    };
    assert_eq!(taken.len(), 1);
    assert!(expected.contains(&taken[0]));
    let [x, rs] = taken[0].arguments() else {
        panic!("enq takes x and rs: {:?}", taken[0].arguments());
    };
    assert_eq!(x, r#"{"type":"integer","value":"1"}"#);
    let rs: serde_json::Value = serde_json::from_str(rs).expect("rs is JSON");
    let elements = rs["elements"].as_array().expect("rs is a sequence");
    let found: Vec<(&str, &str)> = elements
        .iter()
        .map(|element| {
            (
                element["identity"].as_str().expect("an identity"),
                element["object_type"]["original"]["node"]
                    .as_str()
                    .expect("a type"),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("b", "ix://example/ring-queue-spec/Queue"),
            ("a", "ix://example/ring-queue-spec/Queue")
        ]
    );
}

/// The world over the `twin` variant, with the `Cell` object row before the
/// `Queue` one when `cell_first`.
fn twin_world(cell_first: bool) -> RingWorld {
    RingWorld::with(
        Variant {
            twin: true,
            ..Variant::default()
        },
        &twin_refinement(cell_first),
    )
}

/// FR-140-AC-17 (TC-1086 step 11): the `Ring` is the source of both `Queue`
/// and `Cell`, whichever row is written first; the `twin` field is declared
/// `Reference<Cell>`, so `twin = self` lifts to the `Cell` reference with the
/// ring's key, not to the `Queue` the first row names.
#[trace("TC-1086", "FR-140-AC-17")]
#[test]
fn a_reference_field_lifts_to_the_declared_type_among_two_rows() {
    for cell_first in [false, true] {
        let world = twin_world(cell_first);
        let concrete = world.concrete_system(UNLIMITED);
        let abstracted = world.abstract_system(UNLIMITED);
        let mapped = map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            &ring_state(&concrete, 0, 0, [0, 0]),
            &HistoryValues::default(),
        )
        .expect("the state maps");
        let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["r"];
        let Some(Value::Reference(twin)) = queue.fields.get(&key(QUEUE, "Queue/twin")) else {
            panic!("twin is a reference (cell_first {cell_first})");
        };
        assert_eq!(
            *twin,
            abstracted
                .object_reference(&key(QUEUE, "Cell"), "r")
                .expect("a reference"),
            "cell_first {cell_first}"
        );
        let cells = &mapped.visible.populations[&key(QUEUE, "cells")];
        assert_eq!(cells.keys().collect::<Vec<_>>(), ["r"]);
    }
}

/// FR-141-AC-14 (TC-546 step 9): with `object Q::Cell from R::Ring` written
/// before `object Q::Queue from R::Ring`, the row `put -> enq(self.ring, x)`
/// lifts its receiver against the type that declares `enq`, so the one
/// abstract application is `enq` on the `Queue` `r`, not on the `Cell`.
#[trace("TC-546", "FR-141-AC-14")]
#[test]
fn the_receiver_lifts_against_the_type_that_declares_the_operation() {
    let world = twin_world(true);
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let (pre, post) = (
        ring_state(&concrete, 0, 0, [0, 0]),
        ring_state(&concrete, 0, 1, [1, 0]),
    );
    let put = operation_identity_text(&key(RING, "Slot/put")).expect("an identity text");
    let wanted = concrete.state_key_of(&post).expect("a key").0;
    let (transition, reached) = concrete
        .expand_state(&pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == put
                && concrete.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .expect("put takes the ring from empty to one item");
    let mapped = |state| {
        map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            state,
            &HistoryValues::default(),
        )
        .expect("the state maps")
        .visible
    };
    let verdict = check_step(
        world.refinement(),
        &concrete,
        &abstracted,
        &RefinementPosition {
            concrete: pre.clone(),
            history: HistoryValues::default(),
            abstract_states: vec![mapped(&pre)],
            abstract_protocol_states: Vec::new(),
        },
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict;
    let StepVerdict::Passes { taken, .. } = verdict else {
        panic!("the put passes: {verdict:?}");
    };
    let Taken::Abstract(taken) = *taken else {
        panic!("the put takes an abstract step");
    };
    assert_eq!(taken.len(), 1);
    let enq = operation_identity_text(&key(QUEUE, "Queue/enq")).expect("an identity text");
    assert_eq!(taken[0].operation(), enq);
    let receiver: serde_json::Value =
        serde_json::from_str(taken[0].receiver().expect("a receiver")).expect("JSON");
    assert_eq!(receiver["identity"], "r");
    assert_eq!(
        receiver["object_type"]["original"]["node"],
        "ix://example/ring-queue-spec/Queue"
    );
}

/// FR-141-AC-13 (TC-546 step 8): an argument that nests a reference to a
/// `SubRing`, which no object row names, is `Undetermined(MappingUndetermined)`
/// at the argument `rs` with `UnmappedType(SubRing)`, before the abstract
/// operation is applied.
#[trace("TC-546", "FR-141-AC-13")]
#[test]
fn a_step_argument_naming_an_unmapped_type_is_undetermined_at_the_argument() {
    let world = chain_world();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let state = |size: i64, value: i64, with_sub: bool| {
        let chain: &[(&str, &str)] = if with_sub {
            &[("c", "SubRing")]
        } else {
            &[("b", "Ring")]
        };
        let mut rings = vec![
            RingSpec {
                size,
                chain,
                ..ring("a", "Ring", None)
            },
            ring("b", "Ring", None),
        ];
        if with_sub {
            rings.push(ring("c", "SubRing", None));
        }
        custom_state(
            &concrete,
            &rings,
            &[("a0", "a", 0, value), ("a1", "a", 1, 0)],
        )
    };
    let (pre, post) = (state(0, 0, true), state(1, 1, true));
    let put = operation_identity_text(&key(RING, "Slot/put")).expect("an identity text");
    let wanted = concrete.state_key_of(&post).expect("a key").0;
    let (transition, reached) = concrete
        .expand_state(&pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == put
                && concrete.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .expect("put takes the ring from empty to one item");
    // The position's abstract state is the image of the same ring without
    // the `SubRing`, which maps.
    let clean = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(0, 0, false),
        &HistoryValues::default(),
    )
    .expect("the clean state maps")
    .visible;
    let verdict = check_step(
        world.refinement(),
        &concrete,
        &abstracted,
        &RefinementPosition {
            concrete: pre.clone(),
            history: HistoryValues::default(),
            abstract_states: vec![clean],
            abstract_protocol_states: Vec::new(),
        },
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict;
    let StepVerdict::Undetermined(Undetermined::Mapping(undetermined)) = verdict else {
        panic!("the argument is undetermined: {verdict:?}");
    };
    assert_eq!(
        undetermined.cause,
        EvalFailure::UnmappedType(key(RING, "SubRing"))
    );
    assert_eq!(
        undetermined.row,
        MappingRowRef::Argument {
            operation: key(RING, "Slot/put"),
            argument: StepArgument::Parameter(1),
        }
    );
}

/// FR-140-AC-16 (TC-1086 step 10): a `Sequence<Reference<Ring>>[0, 2]` row
/// maps to the `Queue` references with the same keys in the same order; a
/// reference to a `SubRing`, which no object row names, is `Undetermined` at
/// the `chain` row and the holding ring with `UnmappedType(SubRing)`.
#[trace("TC-1086", "FR-140-AC-16")]
#[test]
fn a_sequence_of_references_maps_element_by_element_in_order() {
    let world = chain_world();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let state = custom_state(
        &concrete,
        &[
            RingSpec {
                chain: &[("b", "Ring"), ("a", "Ring")],
                ..ring("a", "Ring", None)
            },
            ring("b", "Ring", None),
        ],
        &[],
    );
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["a"];
    let Some(Value::Collection(chain)) = queue.fields.get(&key(QUEUE, "Queue/chain")) else {
        panic!("chain is a collection");
    };
    let expected: Vec<String> = ["b", "a"]
        .iter()
        .map(|ring| {
            format!(
                "{:?}",
                Value::Reference(
                    abstracted
                        .object_reference(&key(QUEUE, "Queue"), ring)
                        .expect("a reference")
                )
            )
        })
        .collect();
    let found: Vec<String> = chain.elements().iter().map(|e| format!("{e:?}")).collect();
    assert_eq!(found, expected);

    let state = custom_state(
        &concrete,
        &[
            RingSpec {
                chain: &[("c", "SubRing")],
                ..ring("a", "Ring", None)
            },
            ring("c", "SubRing", None),
        ],
        &[],
    );
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    ) else {
        panic!("the unmapped reference is undetermined");
    };
    assert_eq!(
        undetermined.cause,
        EvalFailure::UnmappedType(key(RING, "SubRing"))
    );
    assert_eq!(
        undetermined.row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/chain"),
        }
    );
    assert_eq!(undetermined.object.expect("an object").key, "a");
}

/// FR-140-AC-9 (TC-1086 step 4): an undefined row outranks an unmapped type
/// that sorts before it; with a defined `items` row the unmapped type is the
/// failure.
#[trace("TC-1086", "FR-140-AC-9")]
#[test]
fn an_undefined_row_outranks_an_unmapped_type_that_sorts_first() {
    let state = |concrete| {
        let mut ring_r = ring("r", "Ring", None);
        ring_r.head = 1;
        ring_r.size = 2;
        custom_state(
            concrete,
            &[ring("a", "SubRing", None), ring_r],
            &[("s0", "r", 0, 0), ("s1", "r", 1, 1)],
        )
    };
    let sub_ring = Variant {
        sub_ring: true,
        ..Variant::default()
    };

    let world = RingWorld::with(sub_ring, &ring_is_queue(ONLY_ROW));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let Err(MappingFailure::Undefined { row, object, cause }) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&concrete),
        &HistoryValues::default(),
    ) else {
        panic!("the undefined row outranks the unmapped type");
    };
    assert_eq!(object.expect("an object").key, "r");
    assert_eq!(
        row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/items"),
        }
    );
    let UndefinedCause::Family(record) = *cause else {
        panic!("the cause is a family record");
    };
    assert_eq!(record.reason, UndefinedReason::OnlyNotOne);

    let world = RingWorld::with(sub_ring, &ring_is_queue(ITEMS_ROW));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&concrete),
        &HistoryValues::default(),
    ) else {
        panic!("the unmapped type is the failure");
    };
    assert_eq!(
        undetermined.cause,
        EvalFailure::UnmappedType(key(RING, "SubRing"))
    );
    assert_eq!(undetermined.object.expect("an object").key, "a");
}

/// FR-140-AC-12 (TC-1086 step 7): a whole-population reference row maps to
/// the abstract references with the same keys, in key order, each of the
/// abstract type.
#[trace("TC-1086", "FR-140-AC-12")]
#[test]
fn a_population_reference_row_maps_element_by_element() {
    let world = MapWorld::new(&format!("{ITEMS_ROW}\ncells = R::slots;"));
    let concrete = world.concrete_system();
    let abstracted = world.abstract_system();
    let state = crate::ring_world::ring_state(&concrete, 1, 2, [0, 1]);
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Collection(cells)) = queue.fields.get(&key(QUEUE, "Queue/cells")) else {
        panic!("cells is a collection");
    };
    let expected: Vec<Value> = ["s0", "s1"]
        .iter()
        .map(|slot| {
            Value::Reference(
                abstracted
                    .object_reference(&key(QUEUE, "Cell"), slot)
                    .expect("a reference"),
            )
        })
        .collect();
    let found: Vec<String> = cells.elements().iter().map(|e| format!("{e:?}")).collect();
    let wanted: Vec<String> = expected.iter().map(|e| format!("{e:?}")).collect();
    assert_eq!(found, wanted);
}

/// FR-140-AC-11 (TC-1086 step 8): the row maps `Some(self.head)` to the
/// optional field's payload and `none` to an unset field whose key is the
/// same as the selected model's initial state.
#[trace("TC-545", "TC-1086", "FR-140-AC-11")]
#[test]
fn optional_abstract_field_rows_preserve_absence_and_unwrap_present_values() {
    let variant = Variant {
        optional_head: true,
        ..Variant::default()
    };
    let rows = format!("{ITEMS_ROW} head = if self.size >= 1 then Some(self.head) else none;");
    let world = RingWorld::with(variant, &ring_is_queue(&rows));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let expected_empty = abstracted.initial_states()[0].state();

    let empty = ring_state(&concrete, 1, 0, [0, 0]);
    let mapped_empty = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &empty,
        &HistoryValues::default(),
    )
    .expect("the empty ring maps");
    let queue = &mapped_empty.visible.populations[&key(QUEUE, "queues")]["r"];
    assert!(!queue.fields.contains_key(&key(QUEUE, "Queue/head")));
    assert!(mapped_empty.hidden.is_empty());
    assert_eq!(
        abstracted
            .state_key_of(&mapped_empty.visible)
            .expect("a key")
            .0,
        abstracted
            .state_key_of(expected_empty)
            .expect("the initial key")
            .0,
    );

    let full = ring_state(&concrete, 1, 2, [0, 1]);
    let mapped_full = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &full,
        &HistoryValues::default(),
    )
    .expect("the populated ring maps");
    let queue = &mapped_full.visible.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Integer(head)) = queue.fields.get(&key(QUEUE, "Queue/head")) else {
        panic!("a populated optional field takes its present payload");
    };
    assert_eq!(*head, filament_exact::Integer::from(1_i64));
}

/// FR-140-AC-11 (TC-1086 step 8): the same row checks for a required
/// `Option<Digit>` field, whose empty case is a set `none` value rather than
/// an absent field.
#[trace("TC-1086", "FR-140-AC-11")]
#[test]
fn explicit_option_abstract_field_rows_preserve_none_and_some_values() {
    let variant = Variant {
        explicit_option_head: true,
        ..Variant::default()
    };
    let rows = format!("{ITEMS_ROW} head = if self.size >= 1 then Some(self.head) else none;");
    let world = RingWorld::with(variant, &ring_is_queue(&rows));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let expected_empty = abstracted.initial_states()[0].state();

    let empty = ring_state(&concrete, 0, 0, [0, 0]);
    let mapped_empty = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &empty,
        &HistoryValues::default(),
    )
    .expect("the empty ring maps");
    let queue = &mapped_empty.visible.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Option(head)) = queue.fields.get(&key(QUEUE, "Queue/head")) else {
        panic!("the required option field contains an explicit value");
    };
    assert!(head.payload().is_none());
    assert!(mapped_empty.hidden.is_empty());
    assert_eq!(
        abstracted
            .state_key_of(&mapped_empty.visible)
            .expect("a key")
            .0,
        abstracted
            .state_key_of(expected_empty)
            .expect("the initial key")
            .0,
    );

    let full = ring_state(&concrete, 1, 2, [0, 1]);
    let mapped_full = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &full,
        &HistoryValues::default(),
    )
    .expect("the populated ring maps");
    let queue = &mapped_full.visible.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Option(head)) = queue.fields.get(&key(QUEUE, "Queue/head")) else {
        panic!("the populated option field contains a value");
    };
    let Some(Value::Integer(head)) = head.payload() else {
        panic!("the populated option contains its integer payload");
    };
    assert_eq!(*head, filament_exact::Integer::from(1_i64));
}

/// FR-140-AC-11 (TC-1086 step 8): package admission retains an explicit
/// `Some` value on a required option field under its declared field identity.
#[trace("TC-1086", "FR-140-AC-11")]
#[test]
fn required_option_fields_admit_present_snapshot_values() {
    let variant = Variant {
        explicit_option_head: true,
        explicit_option_head_initial: Some(1),
        ..Variant::default()
    };
    let world = RingWorld::with(variant, &ring_is_queue(ITEMS_ROW));
    let abstracted = world.abstract_system(UNLIMITED);
    let initial = abstracted.initial_states()[0].state();
    let queue = &initial.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Option(head)) = queue.fields.get(&key(QUEUE, "Queue/head")) else {
        panic!("the required option field is admitted under its field identity");
    };
    let Some(Value::Integer(payload)) = head.payload() else {
        panic!("the required option field retains its present payload");
    };
    assert_eq!(*payload, filament_exact::Integer::from(1_i64));
}

/// FR-140-AC-13 (TC-1086 step 4): the failure reported is the first in
/// reading order, which does not depend on the order the rows were written:
/// the undefined `items` row of object `a`, not `b`; and with `items`
/// defined, object `a`'s field `x` before `y`.
#[trace("TC-1086", "FR-140-AC-13")]
#[test]
fn the_reported_failure_does_not_depend_on_the_source_order_of_the_rows() {
    let g = "function g using v(i: Int[0, 1]): Int[0, 1] pure { if 1 mod i = 0 then 0 else 1 }";
    let xy = Variant {
        xy: true,
        ..Variant::default()
    };
    for (items, expected_field) in [(ONLY_ROW, "items"), (ITEMS_ROW, "x")] {
        for rows in [
            format!("{items} x = g(self.head); y = g(self.head);"),
            format!("y = g(self.head); x = g(self.head); {items}"),
        ] {
            let world = RingWorld::with_mode(
                filament_semantic_value::checking::CheckMode::Kernel,
                xy,
                &format!("{g}\n{}", ring_is_queue(&rows)),
            );
            let concrete = world.concrete_system(UNLIMITED);
            let abstracted = world.abstract_system(UNLIMITED);
            let mut ring_a = ring("a", "Ring", None);
            ring_a.size = 1;
            let mut ring_b = ring("b", "Ring", None);
            ring_b.size = 1;
            let state = custom_state(
                &concrete,
                &[ring_a, ring_b],
                &[
                    ("a0", "a", 0, 0),
                    ("a1", "a", 1, 1),
                    ("b0", "b", 0, 0),
                    ("b1", "b", 1, 1),
                ],
            );
            let Err(MappingFailure::Undefined { row, object, .. }) = map_state(
                world.refinement(),
                &concrete,
                &abstracted,
                &state,
                &HistoryValues::default(),
            ) else {
                panic!("the mapping is undefined");
            };
            assert_eq!(object.expect("an object").key, "a");
            assert_eq!(
                row,
                MappingRowRef::Field {
                    object_type: key(QUEUE, "Queue"),
                    field: key(QUEUE, &format!("Queue/{expected_field}")),
                },
                "{rows}"
            );
        }
    }
}

/// The failure of mapping `state` in `world`.
fn map_failure(world: &MapWorld, state: &ModelState) -> MappingFailure {
    map_state(
        world.refinement(),
        &world.concrete_system(),
        &world.abstract_system(),
        state,
        &HistoryValues::default(),
    )
    .expect_err("the state does not map")
}

/// The row and concrete object of an undefined row.
fn undefined_at(failure: MappingFailure) -> (MappingRowRef, ObjectKey) {
    let MappingFailure::Undefined { row, object, .. } = failure else {
        panic!("an undefined row: {failure:?}");
    };
    (row, object.expect("an object"))
}

/// FR-140-AC-15 (TC-545): over the two populations `queues` and `cells`,
/// each with a row that evaluates undefined (the `items` row where the ring
/// holds an item, the `Cell` `value` row where its slot holds 0), a state with
/// only one population undefined returns that population's undefined row and
/// object.
#[trace("TC-545", "FR-140-AC-15")]
#[test]
fn a_state_with_one_undefined_population_returns_that_populations_row() {
    let cell_value =
        "value = only(t in R::slots where t.index = self.index and t.value = 1: t.value);";
    let world = MapWorld::with_cell_rows(ONLY_ROW, cell_value);
    let concrete = world.concrete_system();
    let items = MappingRowRef::Field {
        object_type: key(QUEUE, "Queue"),
        field: key(QUEUE, "Queue/items"),
    };
    let ring = ObjectKey {
        population: key(RING, "rings"),
        key: "r".to_owned(),
    };
    let value = MappingRowRef::Field {
        object_type: key(QUEUE, "Cell"),
        field: key(QUEUE, "Cell/value"),
    };
    let slot = ObjectKey {
        population: key(RING, "slots"),
        key: "s0".to_owned(),
    };

    // Only `queues`: the slots hold 1.
    let only_queues = ring_state(&concrete, 1, 1, [1, 1]);
    assert_eq!(
        undefined_at(map_failure(&world, &only_queues)),
        (items, ring)
    );
    // Only `cells`: the ring holds no item.
    let only_cells = ring_state(&concrete, 1, 0, [0, 0]);
    assert_eq!(
        undefined_at(map_failure(&world, &only_cells)),
        (value, slot)
    );
}

/// FR-140-AC-14 (TC-1086 step 9): an internal fault returns
/// `MappingFailure::Fault`, and never the row outcome the state would
/// otherwise give. The source's `items` row would be undefined before the
/// malformed `s1.ring` reference is reached in object-reading order, but the
/// complete state observation faults before any row is evaluated.
#[trace("TC-1086", "FR-140-AC-14")]
#[test]
fn an_internal_fault_replaces_the_row_outcome_a_state_would_give() {
    let fault = CatalogCode::new("runtime_invariant", "established-invariant-broken");

    let world = MapWorld::new(ONLY_ROW);
    let concrete = world.concrete_system();
    let mut state = ring_state(&concrete, 1, 1, [0, 0]);
    let MappingFailure::Undefined { row, object, .. } = map_failure(&world, &state) else {
        panic!("the items row is undefined");
    };
    assert_eq!(
        row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/items"),
        }
    );
    assert_eq!(object.expect("an object").key, "r");
    let ghost = Value::Reference(
        concrete
            .object_reference(&key(RING, "Ring"), "ghost")
            .expect("a reference"),
    );
    state
        .populations
        .get_mut(&key(RING, "slots"))
        .and_then(|slots| slots.get_mut("s1"))
        .expect("the slot s1")
        .fields
        .insert(key(RING, "Slot/ring"), ghost);
    let MappingFailure::Fault(code) = map_failure(&world, &state) else {
        panic!("a dangling reference is a fault");
    };
    assert_eq!(code, fault);
}

/// The world over the `subtypes` variant with `refinement`.
fn subtype_world(refinement: &str) -> RingWorld {
    RingWorld::with(
        Variant {
            subtypes: true,
            ..Variant::default()
        },
        refinement,
    )
}

/// FR-140-AC-18 (TC-1086): with `queues` admitting `Queue` and its subtype
/// `FastQueue`, both rows naming `Ring`, the ring maps to the `FastQueue`
/// object whichever row is written first, and swapping the rows changes no
/// mapped state key.
#[trace("TC-1086", "FR-140-AC-18")]
#[test]
fn the_most_specific_admitted_row_maps_the_object_whatever_the_row_order() {
    let mut keys = Vec::new();
    for fast_first in [false, true] {
        let queue = format!("object Q::Queue from R::Ring {{ {ITEMS_ROW} }}");
        let fast = format!("object Q::FastQueue from R::Ring {{ {ITEMS_ROW} }}");
        let (first, second) = if fast_first {
            (fast, queue)
        } else {
            (queue, fast)
        };
        let world = subtype_world(&subtype_refinement(
            [&first, &second],
            ["Q::Queue", "Q::Queue"],
        ));
        let concrete = world.concrete_system(UNLIMITED);
        let abstracted = world.abstract_system(UNLIMITED);
        let state = ring_state(&concrete, 0, 0, [0, 0]);
        let mapped = map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            &state,
            &HistoryValues::default(),
        )
        .expect("the state maps")
        .visible;
        let queues = &mapped.populations[&key(QUEUE, "queues")];
        assert_eq!(queues["r"].type_identity, key(QUEUE, "FastQueue"));
        keys.push(abstracted.state_key_of(&mapped).expect("a key").0);
    }
    assert_eq!(keys[0], keys[1]);
}

/// FR-141-AC-16 (TC-546 step 11): `put -> FastQueue::enq(self.ring, x)` and
/// `put2 -> SlowQueue::enq(self.ring, x)`, both the one abstract `enq`
/// inherited from `Queue`, are each decided with the receiver lifted against
/// the row's own target type, so each applies `enq` to its subtype's object
/// and neither is a `Fault`.
#[trace("TC-546", "FR-141-AC-16")]
#[test]
fn an_inherited_operation_row_lifts_its_receiver_against_its_target_subtype() {
    let queue = format!("object Q::Queue from R::Ring {{ {ITEMS_ROW} }}");
    let fast = format!("object Q::FastQueue from R::Ring {{ {ITEMS_ROW} }}");
    let world = subtype_world(&subtype_refinement(
        [&queue, &fast],
        ["Q::FastQueue", "Q::SlowQueue"],
    ));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let (pre, post) = (
        ring_state(&concrete, 0, 0, [0, 0]),
        ring_state(&concrete, 0, 1, [1, 0]),
    );
    let wanted = concrete.state_key_of(&post).expect("a key").0;
    let mapped = |state: &ModelState| {
        map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            state,
            &HistoryValues::default(),
        )
        .expect("the state maps")
        .visible
    };
    for (operation, subtype) in [("Slot/put", "FastQueue"), ("Slot/put2", "SlowQueue")] {
        let identity = operation_identity_text(&key(RING, operation)).expect("an identity text");
        let (transition, reached) = concrete
            .expand_state(&pre)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(transition, reached)| {
                transition.operation() == identity
                    && concrete.state_key_of(reached.state()).expect("a key").0 == wanted
            })
            .unwrap_or_else(|| panic!("{operation} is enabled"));
        let verdict = check_step(
            world.refinement(),
            &concrete,
            &abstracted,
            &RefinementPosition {
                concrete: pre.clone(),
                history: HistoryValues::default(),
                abstract_states: vec![mapped(&pre)],
                abstract_protocol_states: Vec::new(),
            },
            &ConcreteStep {
                position: 1,
                transition: Some(&transition),
                post: &reached,
            },
        )
        .verdict;
        let StepVerdict::Passes { taken, .. } = verdict else {
            panic!("{operation} passes: {verdict:?}");
        };
        let Taken::Abstract(taken) = *taken else {
            panic!("{operation} takes an abstract step");
        };
        assert_eq!(taken.len(), 1);
        let receiver: serde_json::Value =
            serde_json::from_str(taken[0].receiver().expect("a receiver")).expect("JSON");
        assert_eq!(
            receiver["object_type"]["original"]["node"],
            format!("ix://example/ring-queue-spec/{subtype}")
        );
    }
}

/// The refinement of the `subtypes` variant: `queues` takes the two object
/// rows `objects` in order, `slows` the `SlowQueue` row; `put` and `put2` apply
/// the inherited `enq` on the types `targets` names.
fn subtype_refinement(objects: [&str; 2], targets: [&str; 2]) -> String {
    let [first, second] = objects;
    let [put, put2] = targets;
    format!(
        "invariant Anything using v on R::Ring at current {{ true }}\n\
         refinement RingIsQueue using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         population Q::slows from R::rings;\n\
         {first}\n{second}\n\
         object Q::SlowQueue from R::Ring {{ {ITEMS_ROW} }}\n\
         step R::Slot::put -> {put}::enq(self.ring, x);\n\
         step R::Slot::put2 -> {put2}::enq(self.ring, x);\n\
         step R::Slot::take -> Q::FastQueue::deq(self.ring);\n\
         }}"
    )
}
