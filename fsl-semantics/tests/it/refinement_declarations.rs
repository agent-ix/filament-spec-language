// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-135, FR-136, FR-137 (TC-540, TC-541, TC-542): the S3 check of a
//! `refinement` declaration over ADR-020 §8's compare-and-set counter and
//! ring buffer, each model a domain package admitted through I1 and the
//! refinement assembled and checked with the rest of its unit.
//!
//! The counters use unbounded `Integer` fields, which the checks cannot
//! tell from `Int[0, 2]`: what they decide is which names resolve, which
//! types conform and which rows exist.

use filament_semantic_value::checking::CheckMode;
use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::ValueType;
use filament_semantic_value::checking::CheckingLimits;
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_foundation::Code;
use fsl_semantics::check::{
    AssemblyRefusal, BindingValue, BodyTerm, CheckCause, CheckRefusal, CheckedGraph,
    CheckedRefinement, LeafTerm, LiteralValue, MemberTerm, PackageDeclarations, RefinementCause,
    RowClass, StepClass, StepTarget,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, meaning, package_input};
use ix_trace_rs::trace;
use serde_json::{json, Value};

pub(crate) const INTEGER: &str = "ix://filament/native/Integer";
const BOOLEAN: &str = "ix://filament/native/Boolean";

pub(crate) const SPEC: &str = "example/counter-spec";
pub(crate) const CAS: &str = "example/counter-cas";
const QUEUE: &str = "example/queue-spec";
const RING: &str = "example/ring-buffer";

pub(crate) fn node(package: &str, path: &str) -> String {
    format!("ix://{package}/{path}")
}

fn origin(package: &str, path: &str) -> Value {
    let identity = node(package, path);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(package: &str, name: &str, semantic_meaning: &str) -> Value {
    json!({
        "kind": {"module": package, "name": name},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none", "shape": "record", "members": {},
            "meaning": semantic_meaning,
        },
    })
}

fn declaration(package: &str, path: &str, kind: &str, members: Value) -> Value {
    let mut value = json!({
        "identity": node(package, path), "displayName": node(package, path),
        "kind": {"module": package, "name": kind},
        "roles": [], "origin": origin(package, path),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    value
        .as_object_mut()
        .expect("an object")
        .extend(members.as_object().expect("an object").clone());
    value
}

fn field(package: &str, owner: &str, name: &str, type_ref: &str) -> Value {
    let path = format!("{owner}/{name}");
    json!({
        "identity": node(package, &path), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(package, &path),
    })
}

fn parameter(package: &str, owner: &str, operation: &str, name: &str, type_ref: &str) -> Value {
    let path = format!("{owner}/{operation}/{name}");
    json!({
        "identity": node(package, &path), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(package, &path),
    })
}

/// The operation `name` of `owner`, taking `parameters` (name, type) and
/// returning `returns`, whose frame modifies `modifies`.
pub(crate) fn operation(
    package: &str,
    owner: &str,
    name: &str,
    parameters: &[(&str, &str)],
    returns: Option<&str>,
    modifies: &[&str],
) -> Value {
    let path = format!("{owner}/{name}");
    let mut value = json!({
        "identity": node(package, &path), "name": name,
        "params": parameters
            .iter()
            .map(|(parameter_name, type_ref)| parameter(package, owner, name, parameter_name, type_ref))
            .collect::<Vec<_>>(),
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": node(package, "spec"), "path": "spec.fspec",
            "startLine": 1, "startColumn": 1,
        }},
        "frame": {
            "modifies": modifies
                .iter()
                .map(|field| node(package, &format!("{owner}/{field}")))
                .collect::<Vec<_>>(),
            "creates": [], "deletes": [],
        },
    });
    if let Some(type_ref) = returns {
        value["returns"] = json!({
            "typeRef": type_ref,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        });
    }
    value
}

pub(crate) fn object_type(
    package: &str,
    name: &str,
    fields: &[(&str, &str)],
    operations: Vec<Value>,
) -> Value {
    object_type_under(package, name, &[], fields, operations)
}

/// An object type declaring `supertypes` (names in the same package).
pub(crate) fn object_type_under(
    package: &str,
    name: &str,
    supertypes: &[&str],
    fields: &[(&str, &str)],
    operations: Vec<Value>,
) -> Value {
    declaration(
        package,
        name,
        "object_type",
        json!({
            "supertypes": supertypes.iter().map(|ty| node(package, ty)).collect::<Vec<_>>(),
            "fields": fields
                .iter()
                .map(|(field_name, type_ref)| field(package, name, field_name, type_ref))
                .collect::<Vec<_>>(),
            "operations": operations,
            "relationships": [],
        }),
    )
}

pub(crate) fn population(package: &str, name: &str, member: &str) -> Value {
    json!({
        "identity": node(package, name), "displayName": node(package, name),
        "kind": {"module": package, "name": "population"},
        "members": [node(package, member)], "extent": "closed",
        "origin": origin(package, name),
    })
}

pub(crate) fn document(package: &str, types: Vec<Value>, populations: Vec<Value>) -> Vec<u8> {
    document_with(package, Vec::new(), types, populations)
}

fn document_with(
    package: &str,
    extra_constructs: Vec<Value>,
    types: Vec<Value>,
    populations: Vec<Value>,
) -> Vec<u8> {
    let constructs: Vec<Value> = [
        construct(package, "object_type", meaning::OBJECT_TYPE),
        construct(package, "population", meaning::POPULATION),
    ]
    .into_iter()
    .chain(extra_constructs)
    .collect();
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {
            "identity": node(package, "spec"), "version": "1.0.0",
            "dialect": "spec-bundle",
        },
        "package": {
            "identity": package, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": [],
        },
        "occurrences": [], "extensions": [],
        "constructs": constructs,
        "types": types,
        "populations": populations,
    })
    .to_string()
    .into_bytes()
}

pub(crate) const SUB: &str = "example/sub-counter";
const NARROW: &str = "example/narrow-counter";
const WIDE: &str = "example/wide-counter";

/// `Counter { value }` with `peek()`, its subtype `SubCounter { extra }` with
/// `bump()`, and the population `counters` of counters.
pub(crate) fn sub_counter() -> Vec<u8> {
    document(
        SUB,
        vec![
            object_type(
                SUB,
                "Counter",
                &[("value", INTEGER)],
                vec![operation(SUB, "Counter", "peek", &[], None, &[])],
            ),
            object_type_under(
                SUB,
                "SubCounter",
                &["Counter"],
                &[("extra", INTEGER)],
                vec![operation(SUB, "SubCounter", "bump", &[], None, &["extra"])],
            ),
        ],
        vec![population(SUB, "counters", "Counter")],
    )
}

/// `Counter { value: Int[0, upper] }` and the population `counters`.
fn bounded_counter(package: &str, upper: u32) -> Vec<u8> {
    let constraints: Vec<Value> = [("min", "0".to_owned()), ("max", upper.to_string())]
        .iter()
        .map(|(keyword, value)| {
            let path = format!("Bound/constraints/{keyword}");
            json!({
                "identity": node(package, &path), "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": INTEGER,
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": origin(package, &path),
            })
        })
        .collect();
    let bound = declaration(
        package,
        "Bound",
        "value_type",
        json!({
            "scalar": "integer",
            "constraints": constraints,
        }),
    );
    document_with(
        package,
        vec![construct(package, "value_type", meaning::VALUE_TYPE)],
        vec![
            bound,
            object_type(
                package,
                "Counter",
                &[("value", &node(package, "Bound"))],
                Vec::new(),
            ),
        ],
        vec![population(package, "counters", "Counter")],
    )
}

/// `Counter { value }` with `inc()` and the population `counters`, and the
/// record value type `Try { done }`, the attempt type of a compensation
/// template (FR-381).
pub(crate) fn counter_spec() -> Vec<u8> {
    counter_spec_with(Vec::new(), Vec::new())
}

/// [`counter_spec`] with `types` and `populations` added beside `Counter`
/// and `counters`.
pub(crate) fn counter_spec_with(types: Vec<Value>, populations: Vec<Value>) -> Vec<u8> {
    let attempt = declaration(
        SPEC,
        "Try",
        "record_value_type",
        json!({"fields": [field(SPEC, "Try", "done", BOOLEAN)]}),
    );
    let event = declaration(
        SPEC,
        "Evt",
        "record_value_type",
        json!({"fields": [
            field(SPEC, "Evt", "p", BOOLEAN),
            field(SPEC, "Evt", "q", INTEGER),
        ]}),
    );
    document_with(
        SPEC,
        vec![construct(
            SPEC,
            "record_value_type",
            meaning::RECORD_VALUE_TYPE,
        )],
        vec![
            object_type(
                SPEC,
                "Counter",
                &[("value", INTEGER)],
                vec![
                    operation(SPEC, "Counter", "inc", &[], None, &["value"]),
                    operation(
                        SPEC,
                        "Counter",
                        "quad",
                        &[("x", INTEGER)],
                        Some(INTEGER),
                        &[],
                    ),
                ],
            ),
            attempt,
            event,
        ]
        .into_iter()
        .chain(types)
        .collect(),
        [population(SPEC, "counters", "Counter")]
            .into_iter()
            .chain(populations)
            .collect(),
    )
}

/// The compare-and-set counter: two workers that read `value` into a scratch
/// field and write it back, and `peek`.
pub(crate) fn counter_cas() -> Vec<u8> {
    counter_cas_with(
        operation(CAS, "Counter", "peek", &[], None, &[]),
        &[],
        Vec::new(),
        Vec::new(),
    )
}

/// [`counter_cas`] with `peek` as given, `fields` added to `Counter`, and
/// `types` and `populations` added beside `Counter` and `counters`.
pub(crate) fn counter_cas_with(
    peek: Value,
    fields: &[(&str, &str)],
    types: Vec<Value>,
    populations: Vec<Value>,
) -> Vec<u8> {
    let step = |name: &str, modifies: &[&str]| operation(CAS, "Counter", name, &[], None, modifies);
    let mut counter_fields = vec![
        ("value", INTEGER),
        ("tmpA", INTEGER),
        ("busyA", BOOLEAN),
        ("tmpB", INTEGER),
        ("busyB", BOOLEAN),
    ];
    counter_fields.extend_from_slice(fields);
    let mut all_types = vec![object_type(
        CAS,
        "Counter",
        &counter_fields,
        vec![
            step("beginA", &["tmpA", "busyA"]),
            step("commitA", &["value", "busyA"]),
            step("retryA", &["busyA"]),
            step("beginB", &["tmpB", "busyB"]),
            step("commitB", &["value", "busyB"]),
            step("retryB", &["busyB"]),
            peek,
        ],
    )];
    all_types.extend(types);
    let mut all_populations = vec![population(CAS, "counters", "Counter")];
    all_populations.extend(populations);
    document(CAS, all_types, all_populations)
}

/// `Queue { size }` with `enq(x)`, `deq()` and a Boolean-valued `deqFlag()`,
/// and `Handle { queue }`, a reference to a queue.
fn queue_spec() -> Vec<u8> {
    document(
        QUEUE,
        vec![
            object_type(
                QUEUE,
                "Queue",
                &[("size", INTEGER)],
                vec![
                    operation(QUEUE, "Queue", "enq", &[("x", INTEGER)], None, &["size"]),
                    operation(QUEUE, "Queue", "deq", &[], Some(INTEGER), &["size"]),
                    operation(QUEUE, "Queue", "deqFlag", &[], Some(BOOLEAN), &["size"]),
                ],
            ),
            object_type(
                QUEUE,
                "Handle",
                &[("queue", &node(QUEUE, "Queue"))],
                Vec::new(),
            ),
        ],
        vec![
            population(QUEUE, "queues", "Queue"),
            population(QUEUE, "handles", "Handle"),
        ],
    )
}

fn queue_spec_with_relationship() -> Vec<u8> {
    let mut package: Value = serde_json::from_slice(&queue_spec()).expect("queue model JSON");
    package["types"][0]["relationships"] = json!([{
        "identity": node(QUEUE, "relationship/Queue-peers-Queue"),
        "category": "structural",
        "composite": false,
        "direction": "undirected",
        "origin": {"source": {
            "sourceIdentity": node(QUEUE, "spec"),
            "path": "spec.fspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "sourceEnd": {
            "role": "peers",
            "type": node(QUEUE, "Queue"),
            "multiplicity": {"lower": 0, "ordered": false, "unique": false},
        },
        "targetEnd": {
            "type": node(QUEUE, "Queue"),
            "multiplicity": {"lower": 0, "ordered": false, "unique": false},
        },
    }]);
    serde_json::to_vec(&package).expect("queue model with relationship")
}

/// FR-135-AC-7 (TC-540): an abstract model relationship is rejected at its
/// alias regardless of refinement support, with its form and selected profile.
#[trace("TC-540", "FR-135-AC-7")]
#[test]
fn an_abstract_model_relationship_refuses_the_refinement() {
    let (ring, queue) = (ring_buffer(), queue_spec_with_relationship());
    let found = refused(
        &[("Q", QUEUE, &queue), ("R", RING, &ring)],
        &ring_refinement("enq(self.ring, x)", "deq(self.ring)"),
    );
    let refusal = only(&found);
    assert_eq!(
        refusal.code_and_cause(),
        (Code::UnsupportedConstruct, Some("declaration-form"))
    );
    assert_eq!(refusal.at, "Q");
    assert_eq!(
        refusal.refinement(),
        &RefinementCause::AbstractRelationships {
            form: "relationship",
            refinement: "RingIsQueue".to_owned(),
            profile: "filament.temporal.infinite-trace/v1".to_owned(),
        }
    );
}

/// `Ring { head, size }` and `Slot { ring, next, index, value }`, whose
/// `put(x)` and `take()` are the concrete operations.
fn ring_buffer() -> Vec<u8> {
    document(
        RING,
        vec![
            object_type(
                RING,
                "Ring",
                &[("head", INTEGER), ("size", INTEGER)],
                Vec::new(),
            ),
            object_type(
                RING,
                "Slot",
                &[
                    ("ring", &node(RING, "Ring")),
                    ("next", &node(RING, "Slot")),
                    ("index", INTEGER),
                    ("value", INTEGER),
                ],
                vec![
                    operation(RING, "Slot", "put", &[("x", INTEGER)], None, &["value"]),
                    operation(RING, "Slot", "take", &[], Some(INTEGER), &["value"]),
                ],
            ),
        ],
        vec![
            population(RING, "rings", "Ring"),
            population(RING, "slots", "Slot"),
        ],
    )
}

/// The unit selecting each of `models` (alias, package identity, document)
/// and holding `body`, with the header `profiles`.
fn unit_text(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> (String, fsl_semantics::model::intake::PackageInput) {
    let mut packages = BTreeMap::new();
    let mut selections = String::new();
    for (alias, identity, bytes) in models {
        let single = package_input([*bytes]).unwrap();
        selections.push_str(&format!("model {alias} = {identity:?};\n"));
        packages.extend(single);
    }
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         {profiles}{selections}{body}\n"
    );
    (text, packages)
}

pub(crate) fn assemble(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> (String, Result<PackageDeclarations, AssemblyRefusal>) {
    let (text, packages) = unit_text(profiles, models, body);
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "tc-540"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), [], "the unit is admissible: {text}");
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        selected,
        vec![],
    );
    (text, declarations)
}

/// A refusal with the text of the unit it is located at.
#[derive(Debug)]
pub(crate) struct Refused {
    pub(crate) refusal: CheckRefusal,
    pub(crate) at: String,
    unit: String,
}

impl Refused {
    pub(crate) fn refinement(&self) -> &RefinementCause {
        let CheckCause::Refinement(refusal) = self.refusal.cause.as_ref() else {
            panic!("a refinement refusal: {:?}", self.refusal);
        };
        &refusal.cause
    }

    pub(crate) fn code_and_cause(&self) -> (Code, Option<&'static str>) {
        (self.refusal.cause.code(), self.refusal.cause.cause())
    }
}

pub(crate) fn check_with(
    profiles: &str,
    models: &[(&str, &str, &[u8])],
    body: &str,
) -> Result<CheckedGraph, Vec<Refused>> {
    let (text, declarations) = assemble(profiles, models, body);
    let declarations =
        declarations.unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    check_declarations(&text, declarations)
}

fn check_declarations(
    text: &str,
    declarations: PackageDeclarations,
) -> Result<CheckedGraph, Vec<Refused>> {
    let declarations = declarations.with_unwritten_refinements();
    let regions = declarations.regions();
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| {
                    let region = regions
                        .refusal_region(&refusal)
                        .unwrap_or_else(|| panic!("{refusal:?} names a region"));
                    let (start, end) = (
                        usize::try_from(region.start()).expect("a start"),
                        usize::try_from(region.end()).expect("an end"),
                    );
                    Refused {
                        at: text[start..end].to_owned(),
                        unit: text.to_owned(),
                        refusal,
                    }
                })
                .collect()
        })
}

pub(crate) fn refused(models: &[(&str, &str, &[u8])], body: &str) -> Vec<Refused> {
    match check_with("", models, body) {
        Ok(_) => panic!("{body}: expected a refusal"),
        Err(refusals) => refusals,
    }
}

/// The one refusal of `found`.
fn only(found: &[Refused]) -> &Refused {
    let [only] = found else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    only
}

pub(crate) fn counters() -> (Vec<u8>, Vec<u8>) {
    (counter_spec(), counter_cas())
}

pub(crate) fn checked_counter(body: &str) -> CheckedGraph {
    let (spec, cas) = counters();
    check_with("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], body).unwrap_or_else(|refusals| {
        panic!(
            "{body}: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    })
}

pub(crate) fn refused_counter(body: &str) -> Vec<Refused> {
    let (spec, cas) = counters();
    refused(&[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], body)
}

/// ADR-020 §8's `CasRefinesCounter`.
pub(crate) fn cas_refines_counter() -> String {
    "refinement CasRefinesCounter using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
     step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
     step Impl::Counter::beginA -> stutter;\n\
     step Impl::Counter::beginB -> stutter;\n\
     step Impl::Counter::retryA -> stutter;\n\
     step Impl::Counter::retryB -> stutter;\n\
     step Impl::Counter::peek -> stutter;\n\
     assume fair weak each Impl::Counter::beginA;\n\
     assume fair weak each Impl::Counter::beginB;\n\
     assume fair weak each Impl::Counter::commitA;\n\
     assume fair weak each Impl::Counter::commitB;\n\
     assume fair weak each Impl::Counter::retryA;\n\
     assume fair weak each Impl::Counter::retryB;\n\
     ensure fair weak each Spec::Counter::inc;\n\
     }"
    .to_owned()
}

/// Add refinement rows before the declaration's closing brace.
fn with_refinement_rows(mut body: String, rows: &str) -> String {
    let closing = body
        .rfind('}')
        .expect("a refinement body ends with a brace");
    body.insert_str(closing, rows);
    body
}

/// `body` with the one occurrence of `from` replaced by `to`.
pub(crate) fn mutated(body: &str, from: &str, to: &str) -> String {
    assert_eq!(body.matches(from).count(), 1, "{from} occurs once");
    body.replace(from, to)
}

pub(crate) fn only_refinement(graph: &CheckedGraph) -> &CheckedRefinement {
    let [refinement] = graph.refinements() else {
        panic!("one refinement: {:?}", graph.refinements());
    };
    refinement
}

fn spec_key(node_name: &str) -> DeclarationKey {
    DeclarationKey {
        package: SPEC.to_owned(),
        node: node(SPEC, node_name),
    }
}

fn cas_key(node_name: &str) -> DeclarationKey {
    DeclarationKey {
        package: CAS.to_owned(),
        node: node(CAS, node_name),
    }
}

/// FR-135-AC-1 (TC-540 step 1): the local-model case of `CasRefinesCounter`
/// checks to one population row, one object row with `value = self.value`
/// and no hidden field. This covers only the local-model half of FR-135-AC-1;
/// the model-subject dependency half has no dependency variant in the current
/// checked model-binding representation and remains uncovered.
#[trace("TC-540")]
#[test]
fn the_counter_refinement_checks_to_one_population_and_one_object_row() {
    let graph = checked_counter(&cas_refines_counter());
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.name(), "CasRefinesCounter");
    assert_eq!(refinement.abstract_model(), "Spec");
    assert_eq!(refinement.concrete_model(), "Impl");
    assert_eq!(
        refinement.abstract_model_selection(),
        graph
            .model_selections()
            .iter()
            .find(|selection| selection.identity == SPEC)
            .expect("selected abstract package")
    );
    assert_eq!(
        refinement.concrete_model_selection(),
        graph
            .model_selections()
            .iter()
            .find(|selection| selection.identity == CAS)
            .expect("selected concrete package")
    );
    assert_eq!(refinement.abstract_model_selection().identity, SPEC);
    assert_eq!(refinement.concrete_model_selection().identity, CAS);
    assert_ne!(refinement.abstract_model_selection().identity, "Spec");
    assert_ne!(refinement.concrete_model_selection().identity, "Impl");

    let [population] = refinement.populations() else {
        panic!("one population row: {:?}", refinement.populations());
    };
    assert_eq!(population.abstract_population, spec_key("counters"));
    assert_eq!(population.concrete_population, cas_key("counters"));

    let [object] = refinement.objects() else {
        panic!("one object row: {:?}", refinement.objects());
    };
    assert_eq!(object.abstract_type, spec_key("Counter"));
    assert_eq!(object.concrete_type, cas_key("Counter"));
    assert!(object.hidden.is_empty(), "{:?}", object.hidden);
    let [field] = &object.fields[..] else {
        panic!("one field row: {:?}", object.fields);
    };
    assert_eq!(field.field, spec_key("Counter/value"));
    assert_eq!(
        field
            .expression
            .parameters()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["self"]
    );
}

/// FR-135-AC-12 (TC-540): the checked binding keeps the selected package's
/// canonical identity when the source aliases are unrelated to those
/// identities. `DomainPackageRef` exposes identity only; source package
/// version is not retained on the checked refinement binding. This remains a
/// partial case; the version and zero/one/many object-type and state-machine
/// cardinality clauses are not covered by this assertion.
#[trace("TC-540")]
#[test]
fn refinement_bindings_use_canonical_package_identity_not_source_aliases() {
    let (spec, cas) = counters();
    let spec = with_package_version(spec, "2.7.1");
    let cas = with_package_version(cas, "4.3.0");
    let body = cas_refines_counter()
        .replace("Spec", "AbstractModel")
        .replace("Impl", "ConcreteModel");
    let graph = check_with(
        "",
        &[("AbstractModel", SPEC, &spec), ("ConcreteModel", CAS, &cas)],
        &body,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "renamed refinement aliases check: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.abstract_model(), "AbstractModel");
    assert_eq!(refinement.concrete_model(), "ConcreteModel");
    assert_eq!(refinement.abstract_model_selection().identity, SPEC);
    assert_eq!(refinement.concrete_model_selection().identity, CAS);
    assert_eq!(refinement.abstract_model_version(), Some("2.7.1"));
    assert_eq!(refinement.concrete_model_version(), Some("4.3.0"));
}

fn with_package_version(document: Vec<u8>, version: &str) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(&document).expect("fixture document is JSON");
    value["package"]["version"] = json!(version);
    serde_json::to_vec(&value).expect("fixture document serializes")
}

/// FR-136 and FSpec FR-375-AC-22: source provenance covers every authored
/// step row, including `any`, before any proof eligibility filtering.
#[trace("FR-375-AC-22")]
#[test]
fn checked_step_sources_keep_source_order_and_exact_rows_including_any() {
    let base = cas_refines_counter();
    let body = mutated(
        &base,
        "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitB -> any;",
    );
    let (spec, cas) = counters();
    let (text, declarations) = assemble("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &body);
    let declarations = declarations.expect("the unit assembles");
    let graph = check_declarations(&text, declarations).unwrap_or_else(|refusals| {
        panic!(
            "{body}: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.steps().len(), 7);
    assert_eq!(refinement.step_sources().len(), 7);
    assert!(matches!(&refinement.steps()[1].target, StepTarget::Any));
    for (ordinal, source_row) in refinement.step_sources().iter().enumerate() {
        assert_eq!(source_row.ordinal(), ordinal);
        let span = source_row.span();
        assert_eq!(
            &text[span.start..span.end],
            body.lines()
                .map(str::trim)
                .filter(|line| line.starts_with("step "))
                .nth(ordinal)
                .expect("the source row exists")
        );
    }
    assert!(body.contains("step Impl::Counter::commitB -> any;"));
}

/// FR-135-AC-2 (TC-540 step 2): each malformed row refuses at its span, with
/// no `CheckedRefinement`.
#[trace("TC-540", "FR-135-AC-2")]
#[test]
fn each_malformed_header_and_row_refuses_at_its_span() {
    let base = cas_refines_counter();
    let population_row = "population Spec::counters from Impl::counters;";
    let object_row = "object Spec::Counter from Impl::Counter { value = self.value; }";
    let missing = |class, name: &str| RefinementCause::MissingRow {
        class,
        name: name.to_owned(),
    };

    // An abstract alias that names no model.
    let found = refused_counter(&mutated(&base, "abstract Spec", "abstract Nope"));
    let nope = only(&found);
    assert_eq!(
        nope.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(nope.at, "Nope");

    // The population row removed.
    let found = refused_counter(&mutated(&base, population_row, ""));
    let row = only(&found);
    assert_eq!(
        row.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        row.refinement(),
        &missing(RowClass::Population, "Spec::counters")
    );
    assert_eq!(row.at, "CasRefinesCounter");

    // The row written twice: the second row conflicts with the first.
    let found = refused_counter(&mutated(
        &base,
        population_row,
        &format!("{population_row}\n{population_row}"),
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, population_row);
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Spec::counters");
    assert_eq!(&twice.unit[first.start..first.end], population_row);

    // The row reversed.
    let found = refused_counter(&mutated(
        &base,
        population_row,
        "population Impl::counters from Spec::counters;",
    ));
    let reversed = only(&found);
    assert_eq!(
        reversed.code_and_cause(),
        (Code::InvalidModelBinding, Some("malformed-declaration"))
    );
    assert_eq!(
        reversed.at,
        "population Impl::counters from Spec::counters;"
    );

    // The object row removed.
    let found = refused_counter(&mutated(&base, object_row, ""));
    let row = only(&found);
    assert_eq!(
        row.refinement(),
        &missing(RowClass::Object, "Spec::Counter")
    );
    assert_eq!(row.at, "CasRefinesCounter");

    // `value` mapped twice.
    let found = refused_counter(&mutated(
        &base,
        "{ value = self.value; }",
        "{ value = self.value; value = self.tmpA; }",
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, "value = self.tmpA;");
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Spec::Counter.value");
    assert_eq!(&twice.unit[first.start..first.end], "value = self.value;");

    // A header that selects a bounded profile.
    let bounded = "profile finite = \"filament.temporal.event-position.false-extension/v1\";\n";
    let (spec, cas) = counters();
    let found = match check_with(
        bounded,
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &mutated(&base, "using inf", "using finite"),
    ) {
        Ok(_) => panic!("a bounded profile refuses"),
        Err(found) => found,
    };
    let profile = only(&found);
    assert_eq!(
        profile.code_and_cause(),
        (Code::UnknownProfile, Some("wrong-selection-role"))
    );
    assert_eq!(profile.at, "finite");
}

/// FR-135-AC-3 (TC-540 step 3): a field mapped by an expression of the wrong
/// type refuses `ill_typed`/`type-mismatch` at the expression; a reference
/// to a mapped abstract type is lifted, so a reference to its concrete
/// source conforms and a reference to another concrete type does not.
#[trace("TC-540", "FR-135-AC-3")]
#[test]
fn a_field_row_is_typed_against_the_abstract_field_after_reference_lifting() {
    let base = cas_refines_counter();
    let found = refused_counter(&mutated(
        &base,
        "value = self.value;",
        "value = self.busyA;",
    ));
    let mismatch = only(&found);
    assert_eq!(
        mismatch.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(mismatch.at, "self.busyA");

    let ring = |handle: &str| {
        format!(
            "refinement RingIsQueue using inf abstract Q concrete R {{\n\
             population Q::queues from R::rings;\n\
             population Q::handles from R::slots;\n\
             object Q::Queue from R::Ring {{ size = self.size; }}\n\
             object Q::Handle from R::Slot {{ {handle} }}\n\
             step R::Slot::put -> Q::Queue::enq(self.ring, x);\n\
             step R::Slot::take -> Q::Queue::deq(self.ring);\n\
             }}"
        )
    };
    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let models = [("Q", QUEUE, &queue[..]), ("R", RING, &ring_buffer[..])];
    let graph = check_with("", &models, &ring("queue = self.ring;")).unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    assert_eq!(only_refinement(&graph).objects().len(), 2);

    let found = refused(&models, &ring("queue = self.next;"));
    let slot = only(&found);
    assert_eq!(
        slot.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(slot.at, "self.next");
}

/// TC-1273's negative vector: `Some(x)` has `Option<Integer>` and does not
/// coerce to the refinement row's required `Integer` type.
#[trace("TC-1273")]
#[test]
fn a_present_option_with_the_wrong_row_type_refuses_at_its_expression() {
    let body = mutated(
        &cas_refines_counter(),
        "value = self.value;",
        "value = Some(self.value);",
    );
    let refusals = refused_counter(&body);
    let refusal = only(&refusals);
    assert_eq!(
        refusal.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(refusal.at, "Some(self.value)");
}

const NESTED_SPEC: &str = "example/nested-spec";
const NESTED_IMPL: &str = "example/nested-impl";

/// `field` with the multiplicity `lower..upper` and the presence `presence`.
fn shaped(mut value: Value, (lower, upper, ordered): (u32, u32, bool), presence: &str) -> Value {
    value["multiplicity"] = json!({
        "lower": lower, "upper": upper, "ordered": ordered, "unique": !ordered,
    });
    value["presence"] = json!(presence);
    value
}

/// An abstract `Queue { size, head, chain, link, cells }` over `Cell {
/// value }`, whose `head` is a `Reference<Queue>`, `chain` a
/// `Sequence<Reference<Queue>>[0, 2]`, `link` an `Option<Reference<Queue>>`
/// and `cells` a `Set<Reference<Cell>>[0, 2]`, and a concrete `Ring { size,
/// chain, link }` over `Slot { value }`, whose `chain` is a
/// `Sequence<Reference<Ring>>[0, 2]` and `link` an `Option<Reference<Ring>>`.
fn nested_models() -> (Vec<u8>, Vec<u8>) {
    fn tuple_construct(package: &str) -> Value {
        json!({
            "kind": {"module": package, "name": "tuple_type"},
            "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "tuple", "members": {"positions": "required"}, "meaning": meaning::TUPLE_VALUE_TYPE}
        })
    }

    fn variant_construct(package: &str) -> Value {
        json!({
            "kind": {"module": package, "name": "variant_type"},
            "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "enumeration", "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE}
        })
    }

    /// The record value type `Box { target }`, holding a `Reference<object>`.
    fn boxed(package: &str, object: &str) -> Value {
        declaration(
            package,
            "Box",
            "record_value_type",
            json!({"fields": [field(package, "Box", "target", &node(package, object))]}),
        )
    }

    /// The value type `Maybe`, an `Option<Reference<object>>`.
    fn maybe(package: &str, object: &str) -> Value {
        json!({
            "identity": node(package, "Maybe"), "displayName": node(package, "Maybe"),
            "kind": "option", "roles": [], "origin": origin(package, "Maybe"),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "inner": node(package, object),
        })
    }

    /// The value type `BoxOption`, an `Option<Box>`.
    fn box_option(package: &str) -> Value {
        json!({
            "identity": node(package, "BoxOption"), "displayName": node(package, "BoxOption"),
            "kind": "option", "roles": [], "origin": origin(package, "BoxOption"),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "inner": node(package, "Box"),
        })
    }

    /// The value type `BoxMap`, a `Map<Text, Option<Box>>`.
    fn box_map(package: &str) -> Value {
        json!({
            "identity": node(package, "BoxMap"), "displayName": node(package, "BoxMap"),
            "kind": "map", "roles": [], "origin": origin(package, "BoxMap"),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "keys": "ix://filament/native/Text", "values": node(package, "BoxOption"),
            "bound": {"lower": 0},
        })
    }

    fn pair(package: &str, object: &str) -> Value {
        declaration(
            package,
            "Pair",
            "tuple_type",
            json!({"positions": [
                {"identity": node(package, "Pair/p0"), "typeRef": node(package, object), "origin": origin(package, "Pair/p0")},
                {"identity": node(package, "Pair/p1"), "typeRef": BOOLEAN, "origin": origin(package, "Pair/p1")},
            ]}),
        )
    }

    fn holder(package: &str, object: &str) -> Value {
        let source = |line| {
            json!({"source": {
                "sourceIdentity": node(package, "spec"), "path": "spec.fspec",
                "startLine": line, "startColumn": 1,
            }})
        };
        declaration(
            package,
            "Holder",
            "variant_type",
            json!({"variants": [
                {"identity": node(package, "variant/Holder-Empty"), "name": "Empty", "origin": source(1)},
                {"identity": node(package, "variant/Holder-Has"), "name": "Has", "payloadType": node(package, object), "origin": source(2)},
            ]}),
        )
    }

    /// The record value type `Parcel { owners: Sequence<Option<Reference<object>>> }`.
    fn parcel(package: &str) -> Value {
        let owners = shaped(
            field(package, "Parcel", "owners", &node(package, "Maybe")),
            (0, 2, true),
            "required",
        );
        declaration(
            package,
            "Parcel",
            "record_value_type",
            json!({"fields": [owners]}),
        )
    }

    let (queue, cell) = (node(NESTED_SPEC, "Queue"), node(NESTED_SPEC, "Cell"));
    let single = (1, 1, false);
    let spec = document_with(
        NESTED_SPEC,
        vec![
            construct(NESTED_SPEC, "record_value_type", meaning::RECORD_VALUE_TYPE),
            tuple_construct(NESTED_SPEC),
            variant_construct(NESTED_SPEC),
        ],
        vec![
            boxed(NESTED_SPEC, "Queue"),
            maybe(NESTED_SPEC, "Queue"),
            pair(NESTED_SPEC, "Queue"),
            holder(NESTED_SPEC, "Queue"),
            parcel(NESTED_SPEC),
            box_option(NESTED_SPEC),
            box_map(NESTED_SPEC),
            object_type(
                NESTED_SPEC,
                "Queue",
                &[
                    ("size", INTEGER),
                    ("head", &queue),
                    ("chain", &queue),
                    ("link", &queue),
                    ("maybes", &node(NESTED_SPEC, "Maybe")),
                    ("cells", &cell),
                    ("box", &node(NESTED_SPEC, "Box")),
                    ("pair", &node(NESTED_SPEC, "Pair")),
                    ("holder", &node(NESTED_SPEC, "Holder")),
                    ("parcel", &node(NESTED_SPEC, "Parcel")),
                    ("box_list", &node(NESTED_SPEC, "Box")),
                    ("box_map", &node(NESTED_SPEC, "BoxMap")),
                ],
                Vec::new(),
            ),
            object_type(NESTED_SPEC, "Cell", &[("value", INTEGER)], Vec::new()),
        ],
        vec![
            population(NESTED_SPEC, "queues", "Queue"),
            population(NESTED_SPEC, "cells", "Cell"),
        ],
    );
    let mut spec: Value = serde_json::from_slice(&spec).expect("a document");
    for declared in spec["types"].as_array_mut().expect("types") {
        if let Some(fields) = declared.get_mut("fields").and_then(Value::as_array_mut) {
            for member in fields {
                let shape = match member["name"].as_str() {
                    Some("chain" | "maybes") => ((0, 2, true), "required"),
                    Some("link") => (single, "optional"),
                    Some("cells") => ((0, 2, false), "required"),
                    Some("box_list") => ((0, 2, true), "required"),
                    _ => continue,
                };
                *member = shaped(member.clone(), shape.0, shape.1);
            }
        }
    }
    let ring = node(NESTED_IMPL, "Ring");
    let concrete = document_with(
        NESTED_IMPL,
        vec![
            construct(NESTED_IMPL, "record_value_type", meaning::RECORD_VALUE_TYPE),
            tuple_construct(NESTED_IMPL),
            variant_construct(NESTED_IMPL),
        ],
        vec![
            boxed(NESTED_IMPL, "Ring"),
            maybe(NESTED_IMPL, "Ring"),
            pair(NESTED_IMPL, "Ring"),
            holder(NESTED_IMPL, "Ring"),
            parcel(NESTED_IMPL),
            box_option(NESTED_IMPL),
            box_map(NESTED_IMPL),
            object_type(
                NESTED_IMPL,
                "Ring",
                &[
                    ("size", INTEGER),
                    ("chain", &ring),
                    ("link", &ring),
                    ("maybes", &node(NESTED_IMPL, "Maybe")),
                    ("cargo", &node(NESTED_IMPL, "Slot")),
                    ("box", &node(NESTED_IMPL, "Box")),
                    ("pair", &node(NESTED_IMPL, "Pair")),
                    ("holder", &node(NESTED_IMPL, "Holder")),
                    ("parcel", &node(NESTED_IMPL, "Parcel")),
                    ("box_list", &node(NESTED_IMPL, "Box")),
                    ("box_map", &node(NESTED_IMPL, "BoxMap")),
                ],
                Vec::new(),
            ),
            object_type(NESTED_IMPL, "Slot", &[("value", INTEGER)], Vec::new()),
        ],
        vec![
            population(NESTED_IMPL, "rings", "Ring"),
            population(NESTED_IMPL, "slots", "Slot"),
        ],
    );
    let mut concrete: Value = serde_json::from_slice(&concrete).expect("a document");
    for declared in concrete["types"].as_array_mut().expect("types") {
        if let Some(fields) = declared.get_mut("fields").and_then(Value::as_array_mut) {
            for member in fields {
                match member["name"].as_str() {
                    Some("chain" | "cargo" | "maybes") => {
                        *member = shaped(member.clone(), (0, 2, true), "required");
                    }
                    Some("link") => *member = shaped(member.clone(), single, "optional"),
                    Some("box_list") => *member = shaped(member.clone(), (0, 2, true), "required"),
                    _ => {}
                }
            }
        }
    }
    (
        spec.to_string().into_bytes(),
        concrete.to_string().into_bytes(),
    )
}

/// A refinement of the nested models whose `Queue` object row holds `rows`.
fn nested_refinement(rows: &str) -> String {
    format!(
        "refinement NestedRows using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         population Q::cells from R::slots;\n\
         object Q::Queue from R::Ring {{ size = self.size; {rows} }}\n\
         object Q::Cell from R::Slot {{ value = self.value; }}\n\
         }}"
    )
}

/// FR-135-AC-6 (TC-540 step 6): a `Sequence<Reference<Queue>>[0, 2]` field
/// mapped by a row of type `Sequence<Reference<Ring>>[0, 2]` checks, as do an
/// `Option<Reference<Ring>>` row for an option field, a population-reference
/// row `cells = R::slots` and a plain reference mapped by `self`; a
/// `Sequence<Reference<Slot>>[0, 2]` row, whose element type no object row
/// maps to `Queue`, refuses `ill_typed`/`type-mismatch` at the row's
/// expression, as a top-level reference of that type does.
#[trace("TC-540", "FR-135-AC-6")]
#[test]
fn a_row_nesting_a_reference_is_lifted_element_by_element() {
    let (spec, concrete) = nested_models();
    let models = [
        ("Q", NESTED_SPEC, &spec[..]),
        ("R", NESTED_IMPL, &concrete[..]),
    ];

    for rows in [
        "head = self; cells = R::slots;",
        "chain = self.chain;",
        "link = self.link;",
        "chain = self.chain; link = self.link; head = self; cells = R::slots;",
        // Two carriers deep: a `Sequence<Option<Reference<Ring>>>` row.
        "maybes = self.maybes;",
    ] {
        let graph = check_with("", &models, &nested_refinement(rows)).unwrap_or_else(|refusals| {
            panic!(
                "{rows}: {:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        });
        assert_eq!(only_refinement(&graph).objects().len(), 2, "{rows}");
    }

    let found = refused(&models, &nested_refinement("chain = self.cargo;"));
    let wrong = only(&found);
    assert_eq!(
        wrong.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(wrong.at, "self.cargo");
}

/// FR-135-AC-6 and AC-13..16: record fields, tuple positions, active union
/// payloads, and a nested sequence of optional references lift recursively.
#[trace(
    "TC-540",
    "FR-135-AC-6",
    "FR-135-AC-13",
    "FR-135-AC-14",
    "FR-135-AC-15",
    "FR-135-AC-16"
)]
#[test]
fn a_row_nesting_a_reference_in_a_composite_is_lifted() {
    let (spec, concrete) = nested_models();
    let models = [
        ("Q", NESTED_SPEC, &spec[..]),
        ("R", NESTED_IMPL, &concrete[..]),
    ];
    for row in [
        "box = self.box;",
        "pair = self.pair;",
        "holder = self.holder;",
        "parcel = self.parcel;",
    ] {
        let graph = check_with("", &models, &nested_refinement(row)).unwrap_or_else(|refusals| {
            panic!(
                "{row}: {:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        });
        assert_eq!(only_refinement(&graph).objects().len(), 2, "{row}");
    }
}

/// FR-135-AC-6 (TC-540): composite pairing reaches a record through a
/// sequence and through a map value nested under an option.
#[trace("TC-540", "FR-135-AC-6")]
#[test]
fn composite_pairing_reaches_through_sequence_and_map_option() {
    let (spec, concrete) = nested_models();
    let models = [
        ("Q", NESTED_SPEC, &spec[..]),
        ("R", NESTED_IMPL, &concrete[..]),
    ];

    let sequence = check_with("", &models, &nested_refinement("box_list = self.box_list;"))
        .unwrap_or_else(|refusals| {
            panic!(
                "Sequence<Box> must pair its record element: {:?}",
                refusals
                    .iter()
                    .map(|refusal| &refusal.refusal)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(only_refinement(&sequence).objects().len(), 2);

    let map_option = check_with("", &models, &nested_refinement("box_map = self.box_map;"))
        .unwrap_or_else(|refusals| {
            panic!(
                "Map<Text, Option<Box>> must pair its record value: {:?}",
                refusals
                    .iter()
                    .map(|refusal| &refusal.refusal)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(only_refinement(&map_option).objects().len(), 2);
}

/// FR-135-AC-6 (TC-540): an unpaired reference inside a collection or a
/// map/option value refuses the row instead of producing a checked graph.
#[trace("TC-540", "FR-135-AC-6")]
#[test]
fn composite_pairing_refuses_unmapped_references_through_carriers() {
    for (row, field_name) in [
        ("box_list = self.box_list;", "box_list"),
        ("box_map = self.box_map;", "box_map"),
    ] {
        let (spec, concrete) = nested_models();
        let mut concrete: Value = serde_json::from_slice(&concrete).expect("a concrete model");
        let boxed = concrete["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|declaration| declaration["identity"] == node(NESTED_IMPL, "Box"))
            .expect("Box");
        boxed["fields"][0]["typeRef"] = json!(node(NESTED_IMPL, "Slot"));
        let concrete = concrete.to_string().into_bytes();
        let models = [
            ("Q", NESTED_SPEC, &spec[..]),
            ("R", NESTED_IMPL, &concrete[..]),
        ];

        let found = refused(&models, &nested_refinement(row));
        let refusal = only(&found);
        assert_eq!(
            refusal.code_and_cause(),
            (Code::IllTyped, Some("type-mismatch")),
            "{field_name}"
        );
        assert_eq!(refusal.at, format!("self.{field_name}"));
    }
}

/// FR-135-AC-6 (TC-540): both sides sharing a declaration does not map the
/// object type it references. At top level, in an option, collection or map,
/// and in a record, tuple or union, a shared `Reference<Queue>` that the
/// `Queue` row maps is admitted; a shared `Reference<Ghost>` that no row maps
/// refuses at the row's expression.
#[trace("TC-540", "FR-135-AC-6")]
#[test]
fn a_shared_reference_without_an_object_row_is_refused() {
    let (spec, _) = nested_models();
    // The shared model with `head`, `link`, `chain`, the `box_map` values and
    // the `Box`, `Pair` and `Holder` components referencing `object`.
    let referencing = |object: &str| {
        let mut model: Value = serde_json::from_slice(&spec).expect("a model");
        let declarations = model["types"].as_array_mut().expect("types");
        let target = node(NESTED_SPEC, object);
        for declared in declarations.iter_mut() {
            if declared["identity"] == node(NESTED_SPEC, "Queue") {
                for member in declared["fields"].as_array_mut().expect("fields") {
                    if matches!(member["name"].as_str(), Some("head" | "link" | "chain")) {
                        member["typeRef"] = json!(target);
                    }
                }
            } else if declared["identity"] == node(NESTED_SPEC, "BoxOption") {
                declared["inner"] = json!(target);
            } else if declared["identity"] == node(NESTED_SPEC, "Box") {
                declared["fields"][0]["typeRef"] = json!(target);
            } else if declared["identity"] == node(NESTED_SPEC, "Pair") {
                declared["positions"][0]["typeRef"] = json!(target);
            } else if declared["identity"] == node(NESTED_SPEC, "Holder") {
                declared["variants"][1]["payloadType"] = json!(target);
            }
        }
        declarations.push(object_type(
            NESTED_SPEC,
            "Ghost",
            &[("value", INTEGER)],
            Vec::new(),
        ));
        model.to_string().into_bytes()
    };
    let shared_refinement = |row: &str| {
        format!(
            "refinement SharedRows using inf abstract Q concrete Q {{\n\
             population Q::queues from Q::queues;\n\
             population Q::cells from Q::cells;\n\
             object Q::Queue from Q::Queue {{ size = self.size; {row} }}\n\
             object Q::Cell from Q::Cell {{ value = self.value; }}\n\
             }}"
        )
    };
    // Each row with its expression and, inside a composite, the component
    // path the refusal names.
    let rows = [
        ("head = self.head;", "self.head", None),
        ("link = self.link;", "self.link", None),
        ("chain = self.chain;", "self.chain", None),
        ("box_map = self.box_map;", "self.box_map", None),
        ("box = self.box;", "self.box", Some("box.target")),
        ("pair = self.pair;", "self.pair", Some("[0]")),
        (
            "holder = self.holder;",
            "self.holder",
            Some("holder.variant[0]"),
        ),
    ];

    let mapped = referencing("Queue");
    let unmapped = referencing("Ghost");
    for (row, expression, path) in rows {
        let models = [("Q", NESTED_SPEC, &mapped[..])];
        let graph = check_with("", &models, &shared_refinement(row)).unwrap_or_else(|refusals| {
            panic!(
                "{row}: {:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        });
        assert_eq!(only_refinement(&graph).objects().len(), 2, "{row}");

        let models = [("Q", NESTED_SPEC, &unmapped[..])];
        let found = refused(&models, &shared_refinement(row));
        let refusal = only(&found);
        assert_eq!(
            refusal.code_and_cause(),
            (Code::IllTyped, Some("type-mismatch")),
            "{row}"
        );
        assert_eq!(refusal.at, expression, "{row}");
        let Some(path) = path else {
            continue;
        };
        let CheckCause::Refinement(refinement) = refusal.refusal.cause.as_ref() else {
            panic!("the unmapped reference has a refinement refusal");
        };
        let RefinementCause::Check(CheckCause::Refinement(nested)) = &refinement.cause else {
            panic!("the nested refusal retains its typed mismatch path");
        };
        assert_eq!(
            nested.cause,
            RefinementCause::CompositeTypeMismatch {
                path: path.to_owned(),
            },
            "{row}"
        );
    }
}

/// FR-135-AC-17 (TC-540): a record with no same-name component refuses at
/// S3, so it cannot produce a checked refinement.
#[trace("TC-540", "FR-135-AC-17")]
#[test]
fn a_record_with_a_missing_paired_field_is_refused() {
    let (spec, concrete) = nested_models();
    let mut concrete: Value = serde_json::from_slice(&concrete).expect("a concrete model");
    let declarations = concrete["types"].as_array_mut().expect("types");
    let boxed = declarations
        .iter_mut()
        .find(|declaration| declaration["identity"] == node(NESTED_IMPL, "Box"))
        .expect("Box");
    boxed["fields"] = json!([field(
        NESTED_IMPL,
        "Box",
        "link",
        &node(NESTED_IMPL, "Ring")
    )]);
    let concrete = concrete.to_string().into_bytes();
    let models = [
        ("Q", NESTED_SPEC, &spec[..]),
        ("R", NESTED_IMPL, &concrete[..]),
    ];
    let found = refused(&models, &nested_refinement("box = self.box;"));
    let wrong = only(&found);
    assert_eq!(
        wrong.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(wrong.at, "self.box");
}

/// FR-135-AC-17..19 (TC-540): every structural mismatch refuses the S3
/// refinement, so none can be represented by a `CheckedRefinement`.
#[trace("TC-540", "FR-135-AC-17", "FR-135-AC-18", "FR-135-AC-19")]
#[test]
fn mismatched_composite_reference_shapes_are_refused() {
    for mismatch in ["record field", "tuple arity", "union tag"] {
        let (spec, concrete) = nested_models();
        let mut concrete: Value = serde_json::from_slice(&concrete).expect("a concrete model");
        let declarations = concrete["types"].as_array_mut().expect("types");
        match mismatch {
            "record field" => {
                let boxed = declarations
                    .iter_mut()
                    .find(|declaration| declaration["identity"] == node(NESTED_IMPL, "Box"))
                    .expect("Box");
                boxed["fields"] = json!([field(
                    NESTED_IMPL,
                    "Box",
                    "link",
                    &node(NESTED_IMPL, "Ring")
                )]);
            }
            "tuple arity" => {
                let pair = declarations
                    .iter_mut()
                    .find(|declaration| declaration["identity"] == node(NESTED_IMPL, "Pair"))
                    .expect("Pair");
                pair["positions"]
                    .as_array_mut()
                    .expect("positions")
                    .push(json!({
                        "identity": node(NESTED_IMPL, "Pair/p2"),
                        "typeRef": BOOLEAN,
                        "origin": origin(NESTED_IMPL, "Pair/p2"),
                    }));
            }
            "union tag" => {
                let holder = declarations
                    .iter_mut()
                    .find(|declaration| declaration["identity"] == node(NESTED_IMPL, "Holder"))
                    .expect("Holder");
                holder["variants"][1]["name"] = json!("Points");
                holder["variants"][1]["identity"] =
                    json!(node(NESTED_IMPL, "variant/Holder-Points"));
            }
            _ => unreachable!("the fixture lists only owned mismatches"),
        }
        let concrete = concrete.to_string().into_bytes();
        let models = [
            ("Q", NESTED_SPEC, &spec[..]),
            ("R", NESTED_IMPL, &concrete[..]),
        ];
        let row = match mismatch {
            "record field" => "box = self.box;",
            "tuple arity" => "pair = self.pair;",
            "union tag" => "holder = self.holder;",
            _ => unreachable!(),
        };
        let found = refused(&models, &nested_refinement(row));
        let refusal = only(&found);
        assert_eq!(
            refusal.code_and_cause(),
            (Code::IllTyped, Some("type-mismatch")),
            "{mismatch}"
        );
        assert_eq!(
            refusal.at,
            match mismatch {
                "record field" => "self.box",
                "tuple arity" => "self.pair",
                "union tag" => "self.holder",
                _ => unreachable!(),
            }
        );
        let CheckCause::Refinement(refinement) = refusal.refusal.cause.as_ref() else {
            panic!("the structural mismatch has a refinement refusal");
        };
        let RefinementCause::Check(CheckCause::Refinement(structural)) = &refinement.cause else {
            panic!("the nested structural refusal retains its typed mismatch path");
        };
        assert_eq!(
            structural.cause,
            RefinementCause::CompositeTypeMismatch {
                path: match mismatch {
                    "record field" => "box.target",
                    "tuple arity" => "[2]",
                    "union tag" => "holder.variant",
                    _ => unreachable!(),
                }
                .to_owned(),
            },
            "{mismatch}"
        );
    }
}

/// FR-135-AC-4 (TC-540 step 4): with the `value` row removed, the refinement
/// records that abstract field as hidden and keeps no visible field rows.
#[trace("TC-540")]
#[test]
fn an_abstract_field_with_no_row_is_hidden() {
    let base = cas_refines_counter();
    let graph = checked_counter(&mutated(&base, "value = self.value;", ""));
    let [object] = only_refinement(&graph).objects() else {
        panic!("one object row");
    };
    assert!(object.fields.is_empty());
    assert_eq!(object.hidden, [spec_key("Counter/value")]);
}

/// FR-135-AC-4 (TC-540 step 4): changing the checked field mapping changes
/// the refinement's V2 node identity, even though its source name is stable.
/// The package_id half is not observable through the current checked binding.
#[trace("TC-540")]
#[test]
fn changing_a_field_mapping_changes_the_refinement_node_identity() {
    let base = cas_refines_counter();
    let direct = checked_counter(&base);
    let changed = checked_counter(&mutated(&base, "value = self.value;", "value = self.tmpA;"));
    let direct_node = only_refinement(&direct)
        .requirement_occurrence()
        .expect("the checked refinement has a V2 occurrence")
        .node();
    let changed_node = only_refinement(&changed)
        .requirement_occurrence()
        .expect("the checked refinement has a V2 occurrence")
        .node();
    assert_ne!(direct_node, changed_node);
}

/// FR-136-AC-1 (TC-541 step 1): `CasRefinesCounter` checks seven step rows,
/// `commitA` and `commitB` to `inc(self)` and the others to `stutter`; the
/// ring's `put` row checks as `enq(self.ring, x)` and its `take` row as
/// `deq(self.ring)` binding the result.
#[trace("TC-541", "FR-136-AC-1")]
#[test]
fn every_concrete_operation_has_a_checked_step_row() {
    let graph = checked_counter(&cas_refines_counter());
    let steps = only_refinement(&graph).steps();
    assert_eq!(steps.len(), 7);
    let mut applied = Vec::new();
    let mut stuttered = 0;
    for step in steps {
        let StepClass::Operation(concrete) = &step.class else {
            panic!("an operation row: {:?}", step.class);
        };
        match &step.target {
            StepTarget::Abstract {
                operation,
                target_type: _,
                receiver,
                arguments,
                binds_result,
            } => {
                assert_eq!(operation, &spec_key("Counter/inc"));
                let [(name, receiver_type)] = receiver.parameters() else {
                    panic!("self alone: {:?}", receiver.parameters());
                };
                assert_eq!(name, "self");
                assert_eq!(receiver.body().value_type(), receiver_type);
                assert!(matches!(receiver_type, ValueType::Reference(_)));
                assert!(arguments.is_empty());
                assert!(!binds_result);
                applied.push(concrete.clone());
            }
            StepTarget::Stutter => stuttered += 1,
            StepTarget::Any => panic!("no row maps to any"),
            StepTarget::Node { .. } => panic!("no row names a protocol node"),
        }
    }
    applied.sort();
    assert_eq!(
        applied,
        [cas_key("Counter/commitA"), cas_key("Counter/commitB")]
    );
    assert_eq!(stuttered, 5);

    let ring = ring_refinement("enq(self.ring, x)", "deq(self.ring)");
    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let graph = check_with(
        "",
        &[("Q", QUEUE, &queue), ("R", RING, &ring_buffer)],
        &ring,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let [put, take] = only_refinement(&graph).steps() else {
        panic!("two step rows");
    };
    let StepTarget::Abstract {
        arguments: put_arguments,
        binds_result: put_binds,
        ..
    } = &put.target
    else {
        panic!("put applies enq: {:?}", put.target);
    };
    let [Some(put_argument)] = &put_arguments[..] else {
        panic!("one argument: {put_arguments:?}");
    };
    assert_eq!(put_argument.body().value_type(), &ValueType::Integer);
    assert_eq!(
        put_argument
            .parameters()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["self", "x"]
    );
    assert!(!put_binds);
    let StepTarget::Abstract {
        arguments: take_arguments,
        binds_result: take_binds,
        ..
    } = &take.target
    else {
        panic!("take applies deq: {:?}", take.target);
    };
    assert!(take_arguments.is_empty());
    assert!(take_binds);
}

/// FR-1020-AC-2 (TC-2870): only an operation-to-operation model step map is
/// step-for-step. A single stutter row, an `any` row, or dense concrete time
/// removes that property without exploring the selected model.
#[trace("TC-2870", "FR-1020-AC-2")]
#[trace("FSpec-TC-1042", "FSpec-FR-380-AC-9")]
#[test]
fn a_step_for_step_refinement_is_decided_from_its_step_rows_and_time_kind() {
    let step_for_step =
        cas_refines_counter().replace(" -> stutter;", " -> Spec::Counter::inc(self);");
    let graph = checked_counter(&step_for_step);
    assert!(only_refinement(&graph).is_step_for_step(false));

    let stuttering = mutated(
        &step_for_step,
        "step Impl::Counter::beginA -> Spec::Counter::inc(self);",
        "step Impl::Counter::beginA -> stutter;",
    );
    let graph = checked_counter(&stuttering);
    assert!(!only_refinement(&graph).is_step_for_step(false));

    let arbitrary = mutated(
        &step_for_step,
        "step Impl::Counter::beginA -> Spec::Counter::inc(self);",
        "step Impl::Counter::beginA -> any;",
    );
    let graph = checked_counter(&arbitrary);
    assert!(!only_refinement(&graph).is_step_for_step(false));

    let graph = checked_counter(&step_for_step);
    assert!(!only_refinement(&graph).is_step_for_step(true));
}

/// ADR-020 §8's `RingIsQueue` over the two populations and `put` and `take`
/// rows `put` and `take`.
fn ring_refinement(put: &str, take: &str) -> String {
    format!(
        "refinement RingIsQueue using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         population Q::handles from R::slots;\n\
         object Q::Queue from R::Ring {{ size = self.size; }}\n\
         object Q::Handle from R::Slot {{ queue = self.ring; }}\n\
         step R::Slot::put -> Q::Queue::{put};\n\
         step R::Slot::take -> Q::Queue::{take};\n\
         }}"
    )
}

/// FR-136-AC-2 (TC-541 step 2): a concrete operation with no row refuses
/// `missing-name` naming it, two rows refuse `conflicting-binding`, and a row
/// naming no concrete operation refuses `missing-name`.
#[trace("TC-541", "FR-136-AC-2")]
#[test]
fn step_rows_cover_each_concrete_operation_exactly_once() {
    let base = cas_refines_counter();
    let peek = "step Impl::Counter::peek -> stutter;";

    let found = refused_counter(&mutated(&base, peek, ""));
    let missing = only(&found);
    assert_eq!(
        missing.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        missing.refinement(),
        &RefinementCause::MissingRow {
            class: RowClass::Step,
            name: "Impl::Counter::peek".to_owned(),
        }
    );
    assert_eq!(missing.at, "CasRefinesCounter");

    let found = refused_counter(&mutated(
        &base,
        peek,
        &format!("{peek}\nstep Impl::Counter::peek -> any;"),
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, "step Impl::Counter::peek -> any;");
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Impl::Counter::peek");
    assert_eq!(&twice.unit[first.start..first.end], peek);

    let found = refused_counter(&mutated(
        &base,
        peek,
        &format!("{peek}\nstep Impl::Counter::nope -> stutter;"),
    ));
    let nope = only(&found);
    assert_eq!(
        nope.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(nope.at, "Impl::Counter::nope");
}

/// FR-136-AC-3 (TC-541 step 3): the arguments of an abstract application
/// are counted and typed against the abstract operation, `_` leaves one
/// open, and the results must conform.
#[trace("TC-541", "FR-136-AC-3")]
#[test]
fn an_abstract_application_checks_its_arguments_and_result() {
    let base = cas_refines_counter();
    let commit = "step Impl::Counter::commitA -> Spec::Counter::inc(self);";
    let found = refused_counter(&mutated(
        &base,
        commit,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self, 1);",
    ));
    let count = only(&found);
    assert_eq!(
        count.code_and_cause(),
        (Code::IllTyped, Some("operator-ineligible"))
    );
    assert_eq!(
        count.at,
        "step Impl::Counter::commitA -> Spec::Counter::inc(self, 1);"
    );

    let (queue, ring_buffer) = (queue_spec(), ring_buffer());
    let models = [("Q", QUEUE, &queue[..]), ("R", RING, &ring_buffer[..])];
    let found = refused(
        &models,
        &ring_refinement("enq(self.ring, self.index = 0)", "deq(self.ring)"),
    );
    let argument = only(&found);
    assert_eq!(
        argument.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(argument.at, "self.index = 0");

    let graph = check_with(
        "",
        &models,
        &ring_refinement("enq(self.ring, _)", "deq(self.ring)"),
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let StepTarget::Abstract { arguments, .. } = &only_refinement(&graph).steps()[0].target else {
        panic!("put applies enq");
    };
    assert!(matches!(&arguments[..], [None]));

    let found = refused(
        &models,
        &ring_refinement("enq(self.ring, x)", "deqFlag(self.ring)"),
    );
    let result = only(&found);
    assert_eq!(
        result.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(
        result.at,
        "step R::Slot::take -> Q::Queue::deqFlag(self.ring);"
    );
}

fn constraints(refinement: &CheckedRefinement, concrete: bool) -> Vec<String> {
    let set: Vec<_> = if concrete {
        refinement.concrete_operation_fairness().collect()
    } else {
        refinement.abstract_operation_fairness().collect()
    };
    set.iter()
        .map(|constraint| {
            format!(
                "{:?} {:?} {}",
                constraint.kind(),
                constraint.granularity(),
                constraint.operation().name()
            )
        })
        .collect()
}

/// FR-137-AC-1 (TC-542 step 1): `F_C` is the six `Weak` `Each` constraints in
/// source order and `F_A` is `[Weak Each inc]`; an `ensure` row with no
/// granularity reads `Whole`, and with no `ensure` row `F_A` is empty. The
/// node-identity half needs S4 emission and is not tested.
#[trace("TC-542")]
#[test]
fn the_fairness_rows_form_the_two_sets_in_source_order() {
    let base = cas_refines_counter();
    let graph = checked_counter(&base);
    let refinement = only_refinement(&graph);
    assert_eq!(
        constraints(refinement, true),
        [
            "Weak Each beginA",
            "Weak Each beginB",
            "Weak Each commitA",
            "Weak Each commitB",
            "Weak Each retryA",
            "Weak Each retryB",
        ]
    );
    assert_eq!(constraints(refinement, false), ["Weak Each inc"]);

    let ensure = "ensure fair weak each Spec::Counter::inc;";
    let graph = checked_counter(&mutated(
        &base,
        ensure,
        "ensure fair weak Spec::Counter::inc;",
    ));
    assert_eq!(
        constraints(only_refinement(&graph), false),
        ["Weak Whole inc"]
    );
    let graph = checked_counter(&mutated(&base, ensure, ""));
    assert!(only_refinement(&graph).abstract_fairness().is_empty());
}

/// FR-137-AC-2 (TC-542 step 2): a row naming an operation of the other
/// model, or of neither, refuses `missing-name`, naming the other model when
/// the operation is its; a `weak` and a `strong` row on one operation are
/// different constraints.
#[trace("TC-542", "FR-137-AC-2")]
#[test]
fn a_fairness_row_names_an_operation_of_its_own_model() {
    let base = cas_refines_counter();
    let ensure = "ensure fair weak each Spec::Counter::inc;";

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nassume fair weak Spec::Counter::inc;"),
    ));
    let assumed = only(&found);
    assert_eq!(
        assumed.code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        assumed.refinement(),
        &RefinementCause::Unresolved {
            name: "Spec::Counter::inc".to_owned(),
            held_by: Some("Spec".to_owned()),
        }
    );

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nensure fair weak Impl::Counter::peek;"),
    ));
    let ensured = only(&found);
    assert_eq!(
        ensured.refinement(),
        &RefinementCause::Unresolved {
            name: "Impl::Counter::peek".to_owned(),
            held_by: Some("Impl".to_owned()),
        }
    );

    let found = refused_counter(&mutated(
        &base,
        ensure,
        &format!("{ensure}\nassume fair weak Impl::Counter::nope;"),
    ));
    let nope = only(&found);
    assert_eq!(
        nope.refinement(),
        &RefinementCause::Unresolved {
            name: "Impl::Counter::nope".to_owned(),
            held_by: None,
        }
    );

    // A `weak` and a `strong` row on one operation are different constraints.
    let repeated = "assume fair weak each Impl::Counter::beginA;";
    let graph = checked_counter(&mutated(
        &base,
        repeated,
        &format!("{repeated}\nassume fair strong each Impl::Counter::beginA;"),
    ));
    assert_eq!(only_refinement(&graph).concrete_fairness().len(), 7);
}

/// FR-137-AC-2 (TC-542 step 2): two rows of one set with the same kind,
/// operation and granularity -- an unwritten granularity read as `whole` --
/// refuse `conflicting-binding`, naming the first row and refusing at the
/// second; neither row is dropped.
#[trace("TC-542", "FR-137-AC-2")]
#[test]
fn two_identical_fairness_rows_conflict() {
    let base = cas_refines_counter();
    let repeated = "assume fair weak each Impl::Counter::beginA;";
    let found = refused_counter(&mutated(
        &base,
        repeated,
        &format!("{repeated}\n{repeated}"),
    ));
    let twice = only(&found);
    assert_eq!(
        twice.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(twice.at, repeated);
    let RefinementCause::Conflicting { name, first } = twice.refinement() else {
        panic!("a conflict: {:?}", twice.refinement());
    };
    assert_eq!(name, "Impl::Counter::beginA");
    assert_eq!(&twice.unit[first.start..first.end], repeated);
    assert_eq!(Some(first.start), twice.unit.find(repeated));

    // An unwritten granularity reads `whole`, so it conflicts with `whole`.
    let found = refused_counter(&mutated(
        &base,
        repeated,
        "assume fair weak Impl::Counter::beginA;\n\
         assume fair weak whole Impl::Counter::beginA;",
    ));
    let whole = only(&found);
    assert_eq!(
        whole.code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(whole.at, "assume fair weak whole Impl::Counter::beginA;");

    // The same holds for the `ensure` set.
    let ensure = "ensure fair weak each Spec::Counter::inc;";
    let found = refused_counter(&mutated(&base, ensure, &format!("{ensure}\n{ensure}")));
    assert_eq!(
        only(&found).code_and_cause(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
}

/// Over a model subject a step row's left side names a concrete operation,
/// so a step-class keyword or a `memory` kind names none and refuses
/// `missing-name` (FR-136, "a row whose left side names no concrete
/// operation"); a `node` right side refuses `malformed-declaration` (the
/// model-subject case of FR-147-AC-2, whose other four refusals need the
/// protocol side). The row is refused, not ignored.
#[trace("TC-552")]
#[test]
fn protocol_step_rows_refuse_over_a_model_subject() {
    let base = cas_refines_counter();
    let begin = "step Impl::Counter::beginB -> stutter;";

    let found = refused_counter(&mutated(
        &base,
        begin,
        &format!("{begin}\nstep fork Impl::Counter::par -> stutter;"),
    ));
    assert_eq!(
        only(&found).code_and_cause(),
        (Code::MissingDeclaration, Some("missing-name"))
    );

    let found = refused_counter(&mutated(
        &base,
        begin,
        &format!("{begin}\nstep memory flush -> stutter;"),
    ));
    assert_eq!(
        only(&found).refinement(),
        &RefinementCause::Unresolved {
            name: "flush".to_owned(),
            held_by: None,
        }
    );

    let commit = "step Impl::Counter::commitA -> Spec::Counter::inc(self);";
    let found = refused_counter(&mutated(
        &base,
        commit,
        "step Impl::Counter::commitA -> node Spec::Twice::a1(self);",
    ));
    let node = only(&found);
    assert_eq!(
        node.code_and_cause(),
        (Code::InvalidModelBinding, Some("malformed-declaration"))
    );
    assert_eq!(
        node.refinement(),
        &RefinementCause::Malformed {
            name: "Spec::Twice::a1".to_owned()
        }
    );
}

/// FR-361-AC-4 (TC-1015), the refinement half: the same refinement unit that
/// checks under the value profile checks with the model profile selected, so
/// a refinement is not refused by the model-profile gate.
#[trace("TC-1015", "FR-361-AC-4")]
#[test]
fn a_refinement_is_not_refused_by_the_model_profile_gate() {
    let (spec, cas) = counters();
    let graph = check_with(
        "profile m = \"filament.model.complete/v1\";\n",
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &cas_refines_counter(),
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    assert_eq!(graph.refinements().len(), 1);
}

/// FR-135-AC-8 (TC-540): refinement rows use the governing profile's
/// required value and model layers regardless of the unit's own header
/// selections, and refuse a protocol reference at its first token.
#[trace("TC-540", "FR-135-AC-8")]
#[test]
fn refinement_rows_use_the_governing_layers_and_refuse_protocol_references() {
    let (spec, cas) = counters();
    let body = cas_refines_counter();
    // The unit selects only `value.complete`; the row's reference to a
    // protocol construct is outside the refinement's required layers.
    let source = format!(
        "{}{}",
        mutated(&body, "value = self.value;", "value = protocol Twice;"),
        super::refinement_protocol::twice("Spec")
    );
    let found = refused(&[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &source);
    let refusal = only(&found);
    assert_eq!(
        refusal.code_and_cause(),
        (Code::UnsupportedConstruct, Some("expression-form"))
    );
    assert_eq!(refusal.at, "protocol Twice");
    assert_eq!(
        refusal.refinement(),
        &RefinementCause::UnsupportedRowForm {
            layer: "filament.protocol.complete/v1"
        }
    );
}

/// FR-137-AC-4 (TC-542 step 4): a `strong` row checks as `Strong`, with its
/// granularity.
#[trace("TC-542", "FR-137-AC-4")]
#[test]
fn a_strong_row_checks_as_strong() {
    let base = cas_refines_counter();
    let ensure = "ensure fair weak each Spec::Counter::inc;";
    let graph = checked_counter(&mutated(
        &mutated(&base, ensure, "ensure fair strong each Spec::Counter::inc;"),
        "assume fair weak each Impl::Counter::retryA;",
        "assume fair strong Impl::Counter::retryA;",
    ));
    let refinement = only_refinement(&graph);
    assert_eq!(constraints(refinement, false), ["Strong Each inc"]);
    let retry = refinement
        .concrete_operation_fairness()
        .find(|constraint| constraint.operation().name() == "retryA")
        .expect("retryA is assumed");
    assert_eq!(retry.kind(), FairnessKind::Strong);
    assert_eq!(retry.granularity(), FairnessGranularity::Whole);
}

/// FR-138-AC-9 (TC-543): present optional enum, integer and decimal initial
/// values lower through the checked-package route.
#[trace("TC-543", "FR-138-AC-9")]
#[test]
fn present_option_history_initials_lower_through_the_package_route() {
    let (spec, cas) = counters();
    let models = [("Spec", SPEC, &spec[..]), ("Impl", CAS, &cas[..])];
    let body = format!(
        "enum Phase {{ idle, busy }}\n{}",
        with_refinement_rows(
            cas_refines_counter(),
            "history Impl::Counter.last_phase : Option<Phase> = Phase::idle { }\n\
             history Impl::Counter.last_count : Option<Int[0, 1]> = 1 { }\n\
             history Impl::Counter.last_amount : Option<Decimal[0, 100; 0, 2; nearest-even]> = decimal(150, 1) { }\n",
        )
    );
    let graph = check_with("", &models, &body).unwrap_or_else(|refusals| {
        panic!(
            "present Option history initials must lower through the package route: {:?}",
            refusals
                .iter()
                .map(|refusal| &refusal.refusal)
                .collect::<Vec<_>>()
        )
    });
    let histories = only_refinement(&graph).history();
    assert_eq!(
        histories.len(),
        3,
        "all present Option histories are emitted"
    );
    for history in histories {
        assert!(
            matches!(&history.initial, filament_exact::Value::Option(option) if option.payload().is_some()),
            "{} has a present option initial: {:?}",
            history.field,
            history.initial
        );
    }
    let option_payloads = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "option_value")
        .map(|node| {
            let BodyTerm::Aggregate(body) = node.body() else {
                panic!("option_value has an aggregate body: {node:?}");
            };
            let [MemberTerm::Binding(binding)] = body.members.as_slice() else {
                panic!("option_value has one payload binding: {node:?}");
            };
            assert_eq!(binding.name, "value");
            match &binding.value {
                BindingValue::Leaf(LeafTerm::Literal { value, .. }) => value.clone(),
                BindingValue::Leaf(LeafTerm::Reference { target }) => {
                    let literal = graph
                        .semantic_graph()
                        .node(target.admitted().expect("literal reference is admitted").clone())
                        .expect("literal node exists");
                    let BodyTerm::Leaf(LeafTerm::Literal { value, .. }) = literal.body() else {
                        panic!("history Option payload resolves to a literal: {literal:?}");
                    };
                    value.clone()
                }
                _ => panic!("history Option payload is a literal: {node:?}"),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        option_payloads.len(),
        3,
        "one option_value per history row"
    );
    assert!(option_payloads.iter().any(
        |value| matches!(value, LiteralValue::Enum(case) if case == "idle")
    ));
    assert!(option_payloads.iter().any(|value| matches!(
        value,
        LiteralValue::Integer(number) if number.to_string() == "1"
    )));
    assert!(option_payloads.iter().any(
        |value| matches!(value, LiteralValue::Decimal(number) if number == "15.0")
    ));
}

/// FR-138-AC-2 (TC-543): an Option-wrapped integer remains subject to its
/// declared bounded payload type.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn an_out_of_set_option_history_payload_is_refused_at_its_literal() {
    let refusals = refused_counter(&with_refinement_rows(
        cas_refines_counter(),
        "history Impl::Counter.last : Option<Int[0, 1]> = 2 { }\n",
    ));
    let refusal = only(&refusals);
    assert_eq!(
        refusal.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(refusal.at, "2");
}

fn sub_refinement(source: &str, peek: &str) -> String {
    format!(
        "refinement SubRefines using inf abstract Spec concrete Impl {{\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::{source} {{ value = self.value; }}\n\
         step Impl::Counter::peek -> {peek};\n\
         step Impl::SubCounter::bump -> Spec::Counter::inc(self);\n\
         }}"
    )
}

/// ADR-016 SC-2 over a subtype: a step on `SubCounter` supplies `self` where
/// the abstract receiver lifts to a reference to its supertype `Counter`.
#[trace("TC-541")]
#[test]
fn a_subtype_receiver_conforms_to_the_supertype_it_lifts_to() {
    let (spec, sub) = (counter_spec(), sub_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", SUB, &sub[..])];
    let graph =
        check_with("", &models, &sub_refinement("Counter", "stutter")).unwrap_or_else(|refusals| {
            panic!(
                "{:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        });
    assert_eq!(only_refinement(&graph).steps().len(), 2);
}

/// The other direction does not conform: with the object row mapped from the
/// subtype, a step on the supertype `Counter` supplies a reference the lifted
/// receiver type, a reference to `SubCounter`, does not accept.
#[trace("TC-541")]
#[test]
fn a_supertype_receiver_does_not_conform_to_the_subtype_it_lifts_to() {
    let (spec, sub) = (counter_spec(), sub_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", SUB, &sub[..])];
    let found = refused(
        &models,
        &sub_refinement("SubCounter", "Spec::Counter::inc(self)").replace(
            "step Impl::SubCounter::bump -> Spec::Counter::inc(self);",
            "step Impl::SubCounter::bump -> stutter;",
        ),
    );
    let receiver = only(&found);
    assert_eq!(
        receiver.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(receiver.at, "self");
}

fn range_refinement() -> String {
    "refinement Ranges using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     }"
    .to_owned()
}

/// ADR-016 SC-2 over integer ranges: a field of `Int[0, 3]` defines a field of
/// `Int[0, 10]`, and a field of `Int[0, 10]` is refused for one of
/// `Int[0, 3]` rather than range-checked into it.
#[trace("TC-540")]
#[test]
fn an_integer_range_must_lie_inside_the_range_it_defines() {
    let (narrow, wide) = (bounded_counter(NARROW, 3), bounded_counter(WIDE, 10));
    let inside = [("Spec", WIDE, &wide[..]), ("Impl", NARROW, &narrow[..])];
    check_with("", &inside, &range_refinement()).unwrap_or_else(|refusals| {
        panic!(
            "{:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    });
    let outside = [("Spec", NARROW, &narrow[..]), ("Impl", WIDE, &wide[..])];
    let found = refused(&outside, &range_refinement());
    let range = only(&found);
    assert_eq!(
        range.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(range.at, "self.value");
}

/// `package` document `bytes` with a one-state `state_machine` named `name`
/// added beside its types.
/// `package` document with a state machine declaring `fields`.
fn with_machine_fields(
    bytes: &[u8],
    package: &str,
    name: &str,
    fields: &[(&str, &str)],
) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(bytes).expect("a document");
    let idle = node(package, &format!("{name}/Idle"));
    let tick = node(package, &format!("{name}/tick"));
    let transition = node(package, &format!("{name}/tick_transition"));
    let machine = declaration(
        package,
        name,
        "state_machine",
        json!({
            "supertypes": [],
            "fields": fields
                .iter()
                .map(|(field_name, type_ref)| field(package, name, field_name, type_ref))
                .collect::<Vec<_>>(),
            "clauses": [],
            "operations": [{
                "identity": tick, "name": "tick", "params": [], "pre": [], "post": [],
                "frame": {"modifies": [], "creates": [], "deletes": []},
                "origin": origin(package, &format!("{name}/tick")),
            }],
            "states": [{"identity": idle, "name": "Idle", "origin": origin(package, &format!("{name}/Idle"))}],
            "transitions": [{
                "identity": transition, "from": idle, "to": idle, "trigger": tick,
                "emits": [], "origin": origin(package, &format!("{name}/tick_transition")),
            }],
        }),
    );
    let object = document.as_object_mut().expect("an object");
    let constructs = object["constructs"].as_array_mut().expect("constructs");
    if !constructs
        .iter()
        .any(|existing| existing["construct"]["meaning"] == meaning::STATE_MACHINE)
    {
        let mut construct = construct(package, "state_machine", meaning::STATE_MACHINE);
        construct["construct"]["members"] =
            json!({"states": "required", "transitions": "required"});
        constructs.push(construct);
    }
    object["types"].as_array_mut().expect("types").push(machine);
    document.to_string().into_bytes()
}

/// FR-136-AC-12: object-type-only model subjects are not rejected for lacking
/// a state-machine anchor: the mapped refinement checks, its rows keyed.
#[trace("TC-541", "FR-136-AC-12")]
#[test]
fn an_object_type_only_refinement_subject_checks_and_is_emittable() {
    let (spec, cas) = counters();
    let (_, declarations) = assemble(
        "",
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &cas_refines_counter(),
    );
    let graph = declarations
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the refinement checks: {refusals:?}"));
    assert!(only_refinement(&graph).v2_emittable());
}

/// FR-136-AC-12: several state machines are selected by explicit rows rather
/// than refused as an ambiguous subject.
#[trace("TC-541", "FR-136-AC-12")]
#[test]
fn several_state_machines_are_not_an_ambiguous_refinement_subject() {
    let (spec, cas) = counters();
    let (spec, cas) = (
        with_machine_fields(&spec, SPEC, "SpecMachine", &[("value", INTEGER)]),
        with_machine_fields(&cas, CAS, "ImplMachine", &[("value", INTEGER)]),
    );
    let one = with_refinement_rows(
        cas_refines_counter(),
        "object Spec::SpecMachine from Impl::ImplMachine { value = self.value; }\n",
    );
    let (_, declarations) = assemble("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &one);
    declarations
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the refinement checks: {refusals:?}"));
    let twice = with_machine_fields(&spec, SPEC, "SpecMachineToo", &[("value", INTEGER)]);
    let two = with_refinement_rows(
        cas_refines_counter(),
        "object Spec::SpecMachine from Impl::ImplMachine { value = self.value; }\n\
         object Spec::SpecMachineToo from Impl::ImplMachine { value = self.value; }\n",
    );
    let (_, declarations) = assemble("", &[("Spec", SPEC, &twice), ("Impl", CAS, &cas)], &two);
    // Machine multiplicity is not a subject refusal.
    declarations
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the refinement checks: {refusals:?}"));
}

/// FR-136-AC-12: a build that keeps unwritten refinements (analysis, replay)
/// raises no subject refusal for an object-type refinement.
#[trace("TC-541", "FR-136-AC-12")]
#[test]
fn a_build_keeping_unwritten_refinements_raises_no_subject_refusal() {
    let graph = checked_counter(&cas_refines_counter());
    assert_eq!(graph.refinements().len(), 1);
}

/// FR-135-AC-11 / FSpec FR-375-AC-24: state-machine fields map through the
/// existing object-row checker without a population binding.
#[trace("TC-540", "TC-541", "FR-135-AC-11", "FR-136-AC-12")]
#[test]
fn state_machine_fields_are_checked_by_population_independent_object_rows() {
    let (spec, cas) = counters();
    let spec = with_machine_fields(&spec, SPEC, "SpecMachine", &[("value", INTEGER)]);
    let cas = with_machine_fields(&cas, CAS, "ImplMachine", &[("value", INTEGER)]);
    let body = with_refinement_rows(
        cas_refines_counter(),
        "object Spec::SpecMachine from Impl::ImplMachine { value = self.value; }\n",
    );
    let graph = check_with("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &body)
        .expect("one explicit machine field-set row checks without a population row");
    let refinement = graph.refinements().first().expect("one checked refinement");
    assert_eq!(refinement.objects().len(), 2);
    let machine_row = refinement
        .objects()
        .iter()
        .find(|row| row.abstract_type.node.ends_with("SpecMachine"))
        .unwrap_or_else(|| {
            panic!(
                "the state-machine row is retained: {:?}",
                refinement.objects()
            )
        });
    assert!(machine_row.concrete_type.node.ends_with("ImplMachine"));
    assert_eq!(machine_row.fields.len(), 1);
    assert!(machine_row.fields[0]
        .field
        .node
        .ends_with("SpecMachine/value"));
}

/// FR-135-AC-11: missing, duplicate and mixed-kind machine rows retain their
/// named typed refusals.
#[trace("TC-540", "FR-135-AC-11")]
#[test]
fn state_machine_rows_report_missing_duplicate_and_mixed_kinds() {
    let (spec, cas) = counters();
    let spec = with_machine_fields(&spec, SPEC, "SpecMachine", &[("value", INTEGER)]);
    let cas = with_machine_fields(&cas, CAS, "ImplMachine", &[("value", INTEGER)]);
    let models = [("Spec", SPEC, &spec[..]), ("Impl", CAS, &cas[..])];

    let missing = refused(&models, &cas_refines_counter());
    assert!(
        missing.iter().any(|found| matches!(
            found.refinement(),
            RefinementCause::MissingRow { class: RowClass::Object, name }
                if name == "Spec::SpecMachine"
        )),
        "missing machine row is named: {missing:?}"
    );

    let row = "object Spec::SpecMachine from Impl::ImplMachine { value = self.value; }\n";
    let duplicate = with_refinement_rows(cas_refines_counter(), &format!("{row}{row}"));
    let duplicate = refused(&models, &duplicate);
    assert!(
        duplicate.iter().any(|found| matches!(
            found.refinement(),
            RefinementCause::Conflicting { name, .. } if name == "Spec::SpecMachine"
        )),
        "duplicate machine rows conflict: {duplicate:?}"
    );

    let mixed = with_refinement_rows(
        cas_refines_counter(),
        "object Spec::SpecMachine from Impl::Counter { value = self.value; }\n",
    );
    let mixed = refused(&models, &mixed);
    assert!(
        mixed.iter().any(|found| matches!(
            found.refinement(),
            RefinementCause::Malformed { name } if name == "Impl::Counter"
        )),
        "mixed machine/object types are malformed: {mixed:?}"
    );
}

/// FR-138-AC-9 (TC-543): a present Option history does not prevent a terminal
/// and refinement from sharing the emitted package.
#[trace("TC-543", "FR-138-AC-9")]
#[test]
fn a_present_option_history_emits_beside_a_terminal() {
    let (spec, cas) = counters();
    let models = [("Spec", SPEC, &spec[..]), ("Impl", CAS, &cas[..])];
    let body = format!(
        "terminal Halt using v on Spec any;\n{}",
        with_refinement_rows(
            cas_refines_counter(),
            "history Impl::Counter.last : Option<Int[0, 1]> = 0 { }\n",
        )
    );
    let graph = check_with("", &models, &body).unwrap_or_else(|refusals| {
        panic!(
            "terminal and present Option history should both emit: {:?}",
            refusals
                .iter()
                .map(|refusal| &refusal.refusal)
                .collect::<Vec<_>>()
        )
    });
    assert!(!graph.terminals().is_empty());
    assert_eq!(graph.refinements().len(), 1);
    assert_eq!(graph.refinements()[0].history().len(), 1);
}

pub(crate) const DIAMOND: &str = "example/diamond-counter";

/// `Base1 { value }` and `Base2 { other }`, each with `op()`, and `Child`
/// specializing both, so `Child::op` names two unrelated declarations.
pub(crate) fn diamond_counter() -> Vec<u8> {
    document(
        DIAMOND,
        vec![
            object_type(
                DIAMOND,
                "Base1",
                &[("value", INTEGER)],
                vec![operation(DIAMOND, "Base1", "op", &[], None, &[])],
            ),
            object_type(
                DIAMOND,
                "Base2",
                &[("other", INTEGER)],
                vec![operation(DIAMOND, "Base2", "op", &[], None, &[])],
            ),
            object_type_under(DIAMOND, "Child", &["Base1", "Base2"], &[], Vec::new()),
        ],
        vec![population(DIAMOND, "counters", "Child")],
    )
}

/// A step row naming an operation that two unrelated ancestors declare is
/// refused at the row's name, not dropped.
#[trace("TC-541")]
#[test]
fn a_step_row_naming_an_ambiguous_operation_refuses_at_its_name() {
    let (spec, diamond) = (counter_spec(), diamond_counter());
    let models = [("Spec", SPEC, &spec[..]), ("Impl", DIAMOND, &diamond[..])];
    let body = "refinement Diamond using inf abstract Spec concrete Impl {\n\
        population Spec::counters from Impl::counters;\n\
        object Spec::Counter from Impl::Child { value = self.value; }\n\
        step Impl::Child::op -> stutter;\n\
        step Impl::Base1::op -> stutter;\n\
        step Impl::Base2::op -> stutter;\n\
        }";
    let found = refused(&models, body);
    let ambiguous = only(&found);
    assert_eq!(
        ambiguous.code_and_cause(),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
    assert_eq!(ambiguous.at, "Impl::Child::op");
}

/// FR-480 "Selections and imports" (QSL-1638): a refinement's model aliases
/// name the packages its own unit binds them to. Unit `a`, admitted first,
/// binds `Impl` to the sub-counter package; unit `b` binds `Impl` to the
/// CAS package and declares `CasRefinesCounter`, whose concrete side is
/// `b`'s.
#[trace("FR-480")]
#[test]
fn a_refinements_models_are_its_own_units_packages() {
    let ((spec, cas), sub) = (counters(), sub_counter());
    let header = "profile v = \"filament.value.complete/v1\";\n\
                  profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let a = format!("{header}model Impl = {SUB:?};\ntype Pad = Integer;");
    let b = format!(
        "{header}model Spec = {SPEC:?};\nmodel Impl = {CAS:?};\n{}",
        cas_refines_counter()
    );
    let graph = crate::model_operations::assemble_units(
        &[("a", &a), ("b", &b)],
        &[spec.as_slice(), cas.as_slice(), sub.as_slice()],
    )
    .with_unwritten_refinements()
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
    .unwrap_or_else(|refusals| panic!("b's refinement checks: {refusals:?}"));
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.abstract_model_selection().identity, SPEC);
    assert_eq!(refinement.concrete_model_selection().identity, CAS);
    let [population] = refinement.populations() else {
        panic!("one population row: {:?}", refinement.populations());
    };
    assert_eq!(population.concrete_population, cas_key("counters"));
}
