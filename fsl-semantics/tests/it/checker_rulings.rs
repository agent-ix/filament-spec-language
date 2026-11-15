// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checker rulings on recursion-group keying (FR-092-AC-31), navigation and
//! reachability over every relationship-end multiplicity (FR-363-AC-4,
//! FR-363-AC-7, FR-365-AC-3), population forms in a state clause
//! (FR-391-AC-10), a `Clock` function parameter (FR-230-AC-64), rational
//! range proofs (FR-700) and set folds (FR-701).

use std::collections::BTreeSet;

use filament_checked::key::DeclarationKey;
use filament_exact::{CardinalityBound, CollectionKind, CollectionType, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{CheckRefusal, CheckedGraph};
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, RelationshipDirection,
    RelationshipEnd, RelationshipRecord,
};
use ix_trace_rs::trace;

use crate::relationship_navigation::{
    check_invariant, key, mult, navigations, object, object_type,
};

use crate::named_population::fixture as population_fixture;

/// Check `body` after a header selecting the value profile as `v` and, when
/// `temporal`, the temporal layer as `t`; no model is selected.
fn check_source(temporal: bool, body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let temporal = if temporal {
        "profile t = \"filament.temporal.complete/v1\";\n"
    } else {
        ""
    };
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{temporal}{body}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "checker-rulings"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        fsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles");
    declarations.check(CheckMode::Linked, CheckingLimits::default())
}

/// `result` is exactly one refusal with `code` and `cause`.
fn assert_one_refusal(
    result: Result<CheckedGraph, Vec<CheckRefusal>>,
    what: &str,
    code: Code,
    cause: &str,
) {
    let Err(refusals) = result else {
        panic!("{what} is refused");
    };
    assert_eq!(refusals.len(), 1, "{what}: {refusals:?}");
    assert_eq!(refusals[0].cause.code(), code, "{what}: {refusals:?}");
    assert_eq!(
        refusals[0].cause.cause(),
        Some(cause),
        "{what}: {refusals:?}"
    );
}

fn checks(result: Result<CheckedGraph, Vec<CheckRefusal>>, what: &str) -> CheckedGraph {
    result.unwrap_or_else(|refusals| panic!("{what} checks: {refusals:?}"))
}

// ----- FR-092-AC-31: recursion groups are always keyed -----

const TWO_GROUPS: &str = "\
    function a using v(n: Int[0, 100]): Integer pure decreases(n) { if n > 0 then b(n - 1) else 0 }\n\
    function b using v(n: Int[0, 100]): Integer pure decreases(n) { if n > 0 then a(n - 1) else 0 }\n\
    function c using v(n: Int[0, 100]): Integer pure decreases(n) { if n > 0 then d(n - 1) else 0 }\n\
    function d using v(n: Int[0, 100]): Integer pure decreases(n) { if n > 0 then c(n - 1) else 0 }";

/// FR-092-AC-31: two recursion groups with equal bodies check, each keyed
/// apart by declaration identity, and nothing is refused
/// `unknown_required_feature`.
#[trace("TC-1900", "FR-092-AC-31")]
#[test]
fn two_recursion_groups_with_equal_bodies_key_apart() {
    let graph = checks(check_source(false, TWO_GROUPS), "two recursion groups");
    let grouped: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| {
            node.recursion()
                .map(|group| (node.key(), group.anchor().clone()))
        })
        .collect();
    let groups: BTreeSet<_> = grouped.iter().map(|(_, anchor)| anchor.clone()).collect();
    assert_eq!(groups.len(), 2, "two recursion groups: {grouped:?}");
    let keys: BTreeSet<_> = grouped.iter().map(|(key, _)| key.clone()).collect();
    assert_eq!(
        keys.len(),
        grouped.len(),
        "every grouped node has its own id"
    );
    let functions: BTreeSet<_> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "recursive_function")
        .filter_map(|node| node.declaration().map(|path| (path.to_vec(), node.key())))
        .collect();
    assert_eq!(
        functions.len(),
        4,
        "four recursive function nodes: {functions:?}"
    );
}

// ----- FR-363-AC-4, FR-363-AC-7, FR-365-AC-3: every end multiplicity -----

fn rel(
    id: &str,
    source: (&str, Option<&str>, Option<Multiplicity>),
    target: (&str, Option<&str>, Option<Multiplicity>),
) -> DomainPackageRecord {
    let end = |(object, role, multiplicity): (&str, Option<&str>, Option<Multiplicity>)| {
        RelationshipEnd {
            type_identity: key(object),
            role: role.map(str::to_owned),
            multiplicity,
        }
    };
    DomainPackageRecord::Relationship(RelationshipRecord {
        key: key(&format!("relationship/{}", id.replace('/', "-"))),
        declared_at: None,
        source: end(source),
        target: end(target),
        direction: RelationshipDirection::SourceToTarget,
    })
}

fn ordered(lower: u64, upper: Option<u64>, unique: bool) -> Option<Multiplicity> {
    Some(Multiplicity {
        ordered: true,
        unique,
        ..mult(lower, upper)
    })
}

fn non_unique(lower: u64, upper: Option<u64>) -> Option<Multiplicity> {
    Some(Multiplicity {
        unique: false,
        ..mult(lower, upper)
    })
}

/// `Order` with one relationship per destination multiplicity of
/// FR-363-AC-7, `notes` with none, and `Order`-to-`Order` ends for
/// `reaches`.
fn multiplicity_package() -> DomainPackage {
    let one = Some(mult(0, Some(1)));
    let records = vec![
        object_type("Order", &[]),
        object_type("Person", &[]),
        object_type("Step", &[]),
        object_type("Note", &[]),
        rel(
            "Order/watchers",
            ("Order", Some("watchers"), one),
            ("Person", None, Some(mult(0, None))),
        ),
        rel(
            "Order/crowd",
            ("Order", Some("crowd"), one),
            ("Person", None, non_unique(0, None)),
        ),
        rel(
            "Order/steps",
            ("Order", Some("steps"), one),
            ("Step", None, ordered(0, Some(3), true)),
        ),
        rel(
            "Order/queue",
            ("Order", Some("queue"), one),
            ("Step", None, ordered(0, Some(3), false)),
        ),
        rel(
            "Order/ranks",
            ("Order", Some("ranks"), one),
            ("Step", None, ordered(0, None, true)),
        ),
        rel(
            "Order/first",
            ("Order", Some("first"), one),
            ("Step", None, ordered(0, Some(1), true)),
        ),
        rel(
            "Order/notes",
            ("Order", Some("notes"), one),
            ("Note", None, None),
        ),
        rel(
            "Order/later",
            ("Order", Some("later"), one),
            ("Order", None, Some(mult(0, None))),
        ),
        rel(
            "Order/ranked",
            ("Order", Some("ranked"), one),
            ("Order", None, ordered(0, Some(3), true)),
        ),
        rel(
            "Order/bare",
            ("Order", Some("bare"), one),
            ("Order", None, None),
        ),
    ];
    DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records)
}

fn relationship_key(id: &str) -> DeclarationKey {
    key(&format!("relationship/{}", id.replace('/', "-")))
}

fn collection(kind: CollectionKind, element: ValueType, bound: Option<(u64, u64)>) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        element,
        bound.map(|(lower, upper)| CardinalityBound::new(lower, upper).expect("a bound")),
    ))
}

/// The collection type a navigation case expects, built from the checked graph.
type ExpectedType = Box<dyn Fn(&CheckedGraph) -> ValueType>;

/// FR-363-AC-7: unbounded and ordered destination ends navigate to the
/// collection types the destination multiplicity gives; none refuses.
#[trace("TC-1901", "FR-363-AC-7")]
#[test]
#[ignore = "QSL-2179: navigation over unbounded and ordered relationship ends not implemented yet"]
fn unbounded_and_ordered_ends_navigate_to_their_collection_types() {
    let cases: [(&str, &str, ExpectedType); 6] = [
        (
            "watchers",
            "size(self.watchers) >= 0",
            Box::new(|g| {
                collection(
                    CollectionKind::Set,
                    ValueType::Reference(object(g, "Person")),
                    None,
                )
            }),
        ),
        (
            "crowd",
            "size(self.crowd) >= 0",
            Box::new(|g| {
                collection(
                    CollectionKind::Bag,
                    ValueType::Reference(object(g, "Person")),
                    None,
                )
            }),
        ),
        (
            "steps",
            "size(self.steps) >= 0",
            Box::new(|g| {
                collection(
                    CollectionKind::OrderedSet,
                    ValueType::Reference(object(g, "Step")),
                    Some((0, 3)),
                )
            }),
        ),
        (
            "queue",
            "size(self.queue) >= 0",
            Box::new(|g| {
                collection(
                    CollectionKind::Sequence,
                    ValueType::Reference(object(g, "Step")),
                    Some((0, 3)),
                )
            }),
        ),
        (
            "ranks",
            "size(self.ranks) >= 0",
            Box::new(|g| {
                collection(
                    CollectionKind::OrderedSet,
                    ValueType::Reference(object(g, "Step")),
                    None,
                )
            }),
        ),
        (
            "first",
            "present(self.first)",
            Box::new(|g| ValueType::option(ValueType::Reference(object(g, "Step")))),
        ),
    ];
    for (role, body, expected) in cases {
        let graph = checks(check_invariant(multiplicity_package(), "Order", body), body);
        let id = format!("Order/{role}");
        assert_eq!(
            navigations(&graph),
            vec![(expected(&graph), relationship_key(&id), role.to_owned())],
            "{body}"
        );
    }
}

/// FR-363-AC-4: an end that declares no multiplicity refuses navigation as a
/// typed semantic refusal, `ill_typed`/`operator-ineligible`, never
/// `unsupported_construct`.
#[trace("TC-1901", "FR-363-AC-4")]
#[test]
#[ignore = "QSL-2179: navigation over a multiplicity-less end refuses ill_typed/operator-ineligible not implemented yet"]
fn a_multiplicity_less_end_refuses_as_operator_ineligible() {
    assert_one_refusal(
        check_invariant(multiplicity_package(), "Order", "size(self.notes) >= 0"),
        "size(self.notes) >= 0",
        Code::IllTyped,
        "operator-ineligible",
    );
}

/// FR-365-AC-3: `reaches` over an unbounded, an ordered and a
/// multiplicity-less end of the receiver's own type checks as `Boolean`.
#[trace("TC-1903", "FR-365-AC-3")]
#[test]
#[ignore = "QSL-2179: reaches over unbounded, ordered and multiplicity-less ends not implemented yet"]
fn reaches_admits_every_destination_multiplicity() {
    for end in ["later", "ranked", "bare"] {
        let body = format!("not reaches(self, self, {end})");
        checks(
            check_invariant(multiplicity_package(), "Order", &body),
            &body,
        );
    }
}

// ----- FR-391-AC-10: population forms and seq in a state clause -----

/// FR-391-AC-10: a state clause body may use `seq` and the population forms,
/// and each lowers: checking (which lowers) refuses none of them.
#[trace("TC-1904", "FR-391-AC-10", "FR-139-AC-2")]
#[trace("FSpec-TC-1401", "FSpec-FR-376-AC-8", "FSpec-FR-376-AC-9")]
#[test]
#[ignore = "QSL-2179: seq and population-form lowering in a state clause not implemented yet"]
fn population_forms_and_seq_in_a_state_clause_check_and_lower() {
    for clause in [
        "invariant S using v on M::Counter at current { size(seq(i in 0 .. 2: i)) = 3 }",
        "invariant C using v on M::Counter at current { count(o in M::counters where o.v = 2) <= 3 }",
        "invariant E using v on M::Counter at current { exists(o in M::counters where o.v = 2) or true }",
        "invariant O using v on M::Counter at current { only(o in M::counters where o.v = 2: o.v) = 2 }",
    ] {
        let declarations = format!("bound B on M::counters [3]\n{clause}");
        if let Err(refusals) = population_fixture::check(false, &declarations) {
            panic!("{clause} checks and lowers: {refusals:?}");
        }
    }
}

// ----- FR-230-AC-64: a Clock parameter of a function -----

const CLOCK_FUNCTIONS: &str = "dimension Time;\nunit ms : Time = rational(1, 1);\n\
    function f using v(c: Clock): Boolean pure { true }\n\
    predicate near using v(c: Clock, b: Quantity<ms>): Boolean { c <= b }";

/// FR-230-AC-64: under the temporal layer a `Clock` parameter makes a
/// clock-reading declaration and nothing is refused `not-yet-implemented`.
#[trace("TC-1905", "FR-230-AC-64")]
#[test]
fn a_clock_parameter_checks_under_the_temporal_layer() {
    checks(
        check_source(true, CLOCK_FUNCTIONS),
        "Clock parameters under the temporal layer",
    );
}

/// FR-230-AC-64: without the temporal layer each `Clock` parameter refuses
/// from the facet gate, `unsupported_construct`/`declaration-form`.
#[trace("TC-1905", "FR-230-AC-64")]
#[test]
fn a_clock_parameter_without_the_temporal_layer_is_gated() {
    let Err(refusals) = check_source(false, CLOCK_FUNCTIONS) else {
        panic!("the facet gate refuses each Clock parameter");
    };
    assert!(!refusals.is_empty());
    for refusal in &refusals {
        assert_eq!(
            refusal.cause.code(),
            Code::UnsupportedConstruct,
            "{refusal:?}"
        );
        assert_eq!(
            refusal.cause.cause(),
            Some("declaration-form"),
            "{refusal:?}"
        );
    }
}

// ----- FR-700: rational ranges by the FR-146 interval rule -----

const R9: &str = "Rational[1, 9; 1, 9]";

/// FR-700-AC-1 and AC-3: positive products and sums are admitted into the
/// exact normalized intervals.
#[trace("TC-1906", "FR-700-AC-1", "FR-700-AC-3")]
#[trace("FSpec-TC-1403", "FSpec-FR-146-AC-19")]
#[test]
#[ignore = "QSL-2168: rational product range by the FR-146 interval rule not implemented yet"]
fn rational_products_and_sums_meet_their_normalized_intervals() {
    for body in [
        format!("function f using v(a: {R9}, b: {R9}): Rational[1, 81; 1, 81] pure {{ a * b }}"),
        format!("function f using v(a: {R9}): Rational[1, 81; 1, 81] pure {{ a * a }}"),
        format!("function g using v(a: {R9}, b: {R9}): Rational[1, 162; 1, 81] pure {{ a + b }}"),
    ] {
        checks(check_source(false, &body), &body);
    }
}

/// FR-700-AC-2 and AC-3: a result type narrower than the normalized
/// intervals refuses `undefined_expression`/`unproved-range`.
#[trace("TC-1906", "FR-700-AC-2", "FR-700-AC-3")]
#[trace("FSpec-TC-1403", "FSpec-FR-146-AC-19")]
#[test]
fn narrower_rational_results_refuse_unproved_range() {
    for body in [
        format!("function f using v(a: {R9}, b: {R9}): Rational[1, 80; 1, 81] pure {{ a * b }}"),
        format!("function f using v(a: {R9}, b: {R9}): Rational[1, 81; 1, 80] pure {{ a * b }}"),
        format!("function g using v(a: {R9}, b: {R9}): Rational[1, 161; 1, 81] pure {{ a + b }}"),
    ] {
        assert_one_refusal(
            check_source(false, &body),
            &body,
            Code::UndefinedExpression,
            "unproved-range",
        );
    }
}

// ----- FR-701: set and bag fold step catalog -----

const FOLD_TYPES: &str = "type I = Integer;\ntype B = Boolean;\ntype R = Rational[0, 4; 1, 2];\n\
    function add using v(a: I, b: I): I pure { a + b }";

/// FR-701-AC-1 and AC-2: catalog steps over a set check, whatever the
/// element type; subtraction over a sequence checks.
#[trace("TC-1907", "FR-701-AC-1", "FR-701-AC-2")]
#[test]
fn catalog_steps_over_a_set_check() {
    for function in [
        "function p using v(s: Set<Int[0, 9]>[0, 5]): I pure { fold<I>(acc, x in s: acc + convert<I>(x), identity: 0) }",
        "function q using v(s: Set<Int[0, 9]>[0, 5]): I pure { fold<I>(acc, x in s: convert<I>(x) * acc, identity: 1) }",
        "function r using v(t: Set<Boolean>[0, 2]): B pure { fold<B>(acc, x in t: acc and x, identity: true) }",
        "function w using v(q: Sequence<Int[0, 9]>[0, 5]): I pure { fold<I>(acc, x in q: acc - convert<I>(x), identity: 0) }",
    ] {
        let body = format!("{FOLD_TYPES}\n{function}");
        checks(check_source(false, &body), function);
    }
}

/// FR-701-AC-2: a set step outside the catalog refuses
/// `ill_typed`/`operator-ineligible`.
#[trace("TC-1907", "FR-701-AC-2")]
#[test]
fn set_steps_outside_the_catalog_refuse() {
    for function in [
        "function p using v(s: Set<Int[0, 9]>[0, 5]): I pure { fold<I>(acc, x in s: acc - convert<I>(x), identity: 0) }",
        "function q using v(s: Set<Int[0, 9]>[0, 5]): I pure { fold<I>(acc, x in s: add(acc, convert<I>(x)), identity: 0) }",
        "function r using v(r: Set<Rational[0, 1; 1, 2]>[0, 2]): R pure { fold<R>(acc, x in r: acc + x, identity: rational(0, 1)) }",
    ] {
        let body = format!("{FOLD_TYPES}\n{function}");
        assert_one_refusal(
            check_source(false, &body),
            function,
            Code::IllTyped,
            "operator-ineligible",
        );
    }
}
