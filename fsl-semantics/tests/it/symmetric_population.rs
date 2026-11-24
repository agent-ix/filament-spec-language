// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 (TC-565, TC-566): a population declared `symmetric` is carried
//! into the checked package, and S3 refuses the forms that observe the
//! identity of its objects, over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).
//!
//! Not covered here, because S3 has no such form yet: a reference literal
//! naming a key (FR-150-AC-3), a state constraint (FR-158), and a temporal
//! atom's condition, which S3 does not type (`temporal_fairness`).

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::{IntakeLimits, Span};
use fsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, IdentityForm, IdentityNode, IdentityObserving,
};
use fsl_semantics::model::domain_package::DomainPackageRecord;
use fsl_semantics::model::intake::{read_records, PackageDocument};
use fsl_semantics::model::refusal::{MemberSegment, ModelRefusalCause};
use ix_trace_rs::trace;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, admit_and_assemble_with_profiles,
    attempt_update_modifies_version_number, config_unit_with_body, config_version_document,
    PACKAGE_IDENTITY,
};

const POPULATION: &str = "ix://example/config-version/config_history";
const CONFIG_VERSION: &str = "ix://example/config-version/ConfigVersion";

const PARENT_ORDER: &str = "invariant ParentOrder using v on Config::ConfigVersion at current { \
    present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }\n";
const NO_CYCLE: &str = "invariant NoCycle using v on Config::ConfigVersion at current { \
    not reaches(self, self, parent) }\n";
const VERSION_UNCHANGED: &str = "post VersionUnchanged using v on \
    Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }\n";

/// An order-dependent traversal of the unordered `peers`: the last element
/// visited decides the result.
const LAST_PEER: &str = "fold<Flag>(acc, p in self.peers: p = self, identity: false)";
const FLAG: &str = "type Flag = Boolean;\ntype Tally = Integer;\n";

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// `document` with `config_history` declared `symmetric`.
fn annotated(document: &[u8]) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    for population in envelope["populations"].as_array_mut().expect("populations") {
        if population["identity"] == POPULATION {
            population["symmetric"] = json!(true);
        }
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// `document` with `ConfigVersion.peers`, an unordered set of `ConfigVersion`.
fn with_peers(document: &[u8]) -> Vec<u8> {
    with_peer_collection(document, true)
}

/// `document` with `ConfigVersion.peers`, an unordered set (`unique`) or bag
/// of `ConfigVersion`.
fn with_peer_collection(document: &[u8], unique: bool) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    let peers = format!("{CONFIG_VERSION}/peers");
    envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|entry| entry["identity"] == CONFIG_VERSION)
        .expect("ConfigVersion is declared")["fields"]
        .as_array_mut()
        .expect("ConfigVersion declares fields")
        .push(json!({
            "identity": peers,
            "name": "peers",
            "typeRef": CONFIG_VERSION,
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 0, "upper": 5, "ordered": false, "unique": unique},
            "origin": {
                "generated": {
                    "generatorIdentity": peers,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [peers],
                }
            },
        }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// Add a second, unannotated population of `Draft` objects and a
/// `ConfigVersion.drafts` reference set.
fn with_drafts(document: &[u8]) -> Vec<u8> {
    const DRAFT: &str = "ix://example/config-version/Draft";
    const DRAFTS: &str = "ix://example/config-version/drafts";
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    let types = envelope["types"].as_array_mut().expect("types");
    let config_version = types
        .iter_mut()
        .find(|entry| entry["identity"] == CONFIG_VERSION)
        .expect("ConfigVersion is declared");
    let fields = config_version["fields"].as_array_mut().expect("fields");
    let mut drafts = fields
        .iter()
        .find(|field| field["name"] == "parent")
        .expect("the parent field")
        .clone();
    drafts["identity"] = format!("{CONFIG_VERSION}/drafts").into();
    drafts["name"] = "drafts".into();
    drafts["typeRef"] = DRAFT.into();
    drafts["multiplicity"] = json!({
        "lower": 0, "upper": 5, "ordered": false, "unique": true,
    });
    drafts["origin"]["generated"]["generatorIdentity"] =
        format!("{CONFIG_VERSION}/drafts").into();
    drafts["origin"]["generated"]["inputIdentities"] =
        json!([format!("{CONFIG_VERSION}/drafts")]);
    fields.push(drafts);

    let config_type = types
        .iter()
        .find(|entry| entry["identity"] == CONFIG_VERSION)
        .expect("ConfigVersion is declared")
        .clone();
    let mut draft_type = config_type;
    draft_type["identity"] = DRAFT.into();
    draft_type["displayName"] = DRAFT.into();
    draft_type["fields"] = json!([{
        "identity": format!("{DRAFT}/versionNumber"),
        "name": "versionNumber",
        "typeRef": "ix://filament/native/Integer",
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {
            "generated": {
                "generatorIdentity": format!("{DRAFT}/versionNumber"),
                "generatorVersion": "1.0.0",
                "inputIdentities": [format!("{DRAFT}/versionNumber")],
            }
        },
    }]);
    draft_type["operations"] = json!([]);
    draft_type["relationships"] = json!([]);
    types.push(draft_type);
    envelope["populations"].as_array_mut().expect("populations").push(json!({
        "identity": DRAFTS,
        "displayName": DRAFTS,
        "kind": {"module": "example/config-version", "name": "population"},
        "members": [DRAFT],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": DRAFTS,
                "generatorVersion": "1.0.0",
                "inputIdentities": [DRAFTS],
            }
        },
    }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

/// Add the primitive integer set used by the population-independent fold
/// case in FR-150-AC-4.
fn with_integer_set(document: &[u8]) -> Vec<u8> {
    let mut envelope: serde_json::Value = serde_json::from_slice(document).expect("valid JSON");
    let owner = envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|entry| entry["identity"] == CONFIG_VERSION)
        .expect("ConfigVersion is declared");
    owner["fields"].as_array_mut().expect("fields").push(json!({
        "identity": format!("{CONFIG_VERSION}/s"),
        "name": "s",
        "typeRef": "ix://filament/native/Integer",
        "presence": "optional", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 0, "upper": 3, "ordered": false, "unique": true},
        "origin": {
            "generated": {
                "generatorIdentity": format!("{CONFIG_VERSION}/s"),
                "generatorVersion": "1.0.0",
                "inputIdentities": [format!("{CONFIG_VERSION}/s")],
            }
        },
    }));
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn check(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    admit_and_assemble_with_body(document, body)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"))
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// The one refusal of `body` over `document`: its payload, with its region
/// asserted to be exactly the first occurrence of `needle` in the unit.
fn identity_refusal(document: &[u8], body: &str, needle: &str) -> IdentityObserving {
    let refusals = check(document, body).expect_err("the unit refuses");
    let [refusal] = &refusals[..] else {
        panic!("{body}: one refusal, not {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(
        refusal.cause.cause(),
        Some("identity-observing-form"),
        "{body}"
    );
    let regions = admit_and_assemble_with_body(document, body)
        .expect("the unit assembles")
        .regions();
    let (text, _) = config_unit_with_body(document, body);
    let region = regions.refusal_region(refusal).expect("a region");
    let node = span_of(&text, needle);
    assert_eq!(
        (region.start(), region.end()),
        (node.start as u64, node.end as u64),
        "the refusal is at the refused node"
    );
    match &*refusal.cause {
        CheckCause::IdentityObservingForm(payload) => payload.as_ref().clone(),
        other => panic!("{body}: an identity-observing-form cause, not {other:?}"),
    }
}

fn causes(refusals: &[CheckRefusal]) -> Vec<(Code, Option<&'static str>)> {
    refusals
        .iter()
        .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
        .collect()
}

/// An annotated population read only by dereference, equality and `reaches`
/// checks, carries `symmetric: true` into its record, and the annotation
/// changes the package selection the unit checks against.
#[trace("TC-565", "FR-150-AC-1")]
#[test]
fn an_annotated_population_checks_and_changes_the_package_selection() {
    let body = format!("{PARENT_ORDER}{NO_CYCLE}{VERSION_UNCHANGED}");
    let plain = check(&document(), &body).expect("the unannotated unit checks");
    let marked = check(&annotated(&document()), &body).expect("the annotated unit checks");
    assert_eq!(plain.model_selections(), marked.model_selections());

    for (bytes, symmetric) in [(document(), false), (annotated(&document()), true)] {
        let package = PackageDocument::parse(&bytes, IntakeLimits::default()).expect("parses");
        let records = read_records(PACKAGE_IDENTITY, &package).expect("reads");
        let flags: Vec<bool> = records
            .iter()
            .filter_map(|record| match record {
                DomainPackageRecord::Population(population)
                    if population.key.node == POPULATION =>
                {
                    Some(population.symmetric)
                }
                _ => None,
            })
            .collect();
        assert_eq!(flags, [symmetric]);
    }
}

/// An order-dependent `fold` over a set of references into an annotated
/// population refuses `identity-observing-form` at the `fold`, naming the
/// form and the population; unannotated it keeps today's FR-145 refusal
/// (`operator-ineligible`, FR-145 wins per STD-177).
#[trace("TC-565", "FR-150-AC-2")]
#[test]
fn a_fold_over_references_into_an_annotated_population_refuses() {
    let body = format!(
        "{FLAG}invariant Last using v on Config::ConfigVersion at current {{ not {LAST_PEER} }}\n"
    );
    let payload = identity_refusal(&annotated(&with_peers(&document())), &body, LAST_PEER);
    assert_eq!(payload.node, IdentityNode::Fold);
    assert_eq!(payload.form, IdentityForm::OrderSensitiveTraversal);
    assert_eq!(payload.population.node, POPULATION);

    let unannotated = check(&with_peers(&document()), &body).expect_err("refuses as today");
    assert_eq!(
        causes(&unannotated),
        [(Code::IllTyped, Some("operator-ineligible"))]
    );
}

/// A counting `fold` over the reference set uses a commutative and associative
/// step, so it checks in the annotated and the unannotated unit alike.
#[trace("TC-565", "FR-150-AC-2")]
#[test]
fn a_counting_fold_over_references_checks_in_both_units() {
    let body = format!(
        "{FLAG}invariant Count using v on Config::ConfigVersion at current {{ \
           fold<Tally>(acc, p in self.peers: acc + 1, identity: 0) <= 5 }}\n"
    );
    check(&with_peers(&document()), &body).expect("unannotated counting fold checks");
    check(&annotated(&with_peers(&document())), &body).expect("annotated counting fold checks");
}

/// The counting `fold` over a `Bag` of references checks in both units, and
/// the order-dependent `fold` over that `Bag` refuses as over a `Set`.
#[trace("TC-565", "FR-150-AC-2")]
#[test]
fn a_counting_fold_over_a_bag_of_references_checks_and_a_chooser_refuses() {
    let bag = with_peer_collection(&document(), false);
    let count = format!(
        "{FLAG}invariant Count using v on Config::ConfigVersion at current {{ \
           fold<Tally>(acc, p in self.peers: acc + 1, identity: 0) <= 5 }}\n"
    );
    check(&bag, &count).expect("unannotated counting fold over a Bag checks");
    check(&annotated(&bag), &count).expect("annotated counting fold over a Bag checks");

    let last = format!(
        "{FLAG}invariant Last using v on Config::ConfigVersion at current {{ not {LAST_PEER} }}\n"
    );
    let payload = identity_refusal(&annotated(&bag), &last, LAST_PEER);
    assert_eq!(payload.node, IdentityNode::Fold);
    let unannotated = check(&bag, &last).expect_err("refuses as today");
    assert_eq!(
        causes(&unannotated),
        [(Code::IllTyped, Some("operator-ineligible"))]
    );
}

/// FR-150-AC-6: refusal precedence is decided for each fold from its own
/// source type. The annotated `config_history` fold gets the identity cause;
/// the unannotated `drafts` fold keeps FR-145's operator cause, in source
/// order and at each offending fold.
#[trace("TC-566", "FR-150-AC-6")]
#[test]
fn each_fold_uses_the_annotation_of_its_own_source_population() {
    let document = annotated(&with_drafts(&with_peers(&document())));
    let peers_fold = "fold<Flag>(acc, p in self.peers: p = self, identity: false)";
    let drafts_fold =
        "fold<Flag>(acc, p in self.drafts: p.versionNumber = self.versionNumber, identity: false)";
    let body = format!(
        "{FLAG}post Peer using v on Config::ConfigVersion::attemptUpdate {{ not {peers_fold} }}\n\
         post Draft using v on Config::ConfigVersion::attemptUpdate {{ not {drafts_fold} }}\n"
    );
    let refusals = check(&document, &body).expect_err("both order-sensitive folds refuse");
    assert_eq!(
        causes(&refusals),
        [
            (Code::IllTyped, Some("identity-observing-form")),
            (Code::IllTyped, Some("operator-ineligible")),
        ]
    );
    let regions = admit_and_assemble_with_body(&document, &body)
        .expect("the unit assembles")
        .regions();
    let (text, _) = config_unit_with_body(&document, &body);
    let actual: Vec<(u64, u64)> = refusals
        .iter()
        .map(|refusal| {
            let region = regions.refusal_region(refusal).expect("a source region");
            (region.start(), region.end())
        })
        .collect();
    let expected = [peers_fold, drafts_fold].map(|fold| {
        let span = span_of(&text, fold);
        (span.start as u64, span.end as u64)
    });
    assert_eq!(actual, expected, "each refusal names its own fold node");
    let CheckCause::IdentityObservingForm(cause) = &refusals[0].cause else {
        panic!("the annotated fold reports identity observation");
    };
    assert_eq!(cause.population.node, POPULATION);
}

/// `reduce`, and a conversion of a reference, refuse the same way.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_reduce_and_a_reference_conversion_refuse() {
    let document = annotated(&with_peers(&document()));
    let reduce = "reduce<Peer>(acc, p in self.peers: acc)";
    let body = format!(
        "type Peer = Config::ConfigVersion;\ninvariant Last using v on Config::ConfigVersion at current {{ self = {reduce} }}\n"
    );
    let payload = identity_refusal(&document, &body, reduce);
    assert_eq!(payload.node, IdentityNode::Reduce);

    let convert = "convert<String>(self)";
    let body = format!(
        "invariant Rendered using v on Config::ConfigVersion at current {{ {convert} = \"a\" }}\n"
    );
    let payload = identity_refusal(&document, &body, convert);
    assert_eq!(payload.node, IdentityNode::Convert);
    assert_eq!(payload.form, IdentityForm::ReferenceConversion);

    let body = format!(
        "invariant Rendered using v on Config::ConfigVersion at current {{ \
         let x = {convert} in x = \"a\" }}\n"
    );
    let payload = identity_refusal(&document, &body, convert);
    assert_eq!(payload.node, IdentityNode::Convert);
}

/// The refusal reaches a `terminal when` predicate, and two refused forms of
/// one unit are both reported, in source order.
#[trace("TC-566", "FR-150-AC-3")]
#[test]
fn a_terminal_predicate_refuses_and_two_refusals_report_in_source_order() {
    let document = annotated(&with_peers(&document()));
    let terminal = format!("{FLAG}terminal Done using v on Config when {{ {LAST_PEER} }};\n");
    let refusals = check(&document, &terminal).expect_err("the terminal refuses");
    assert_eq!(
        causes(&refusals),
        [(Code::IllTyped, Some("identity-observing-form"))]
    );

    let both = format!(
        "{FLAG}invariant A using v on Config::ConfigVersion at current {{ not {LAST_PEER} }}\n\
         invariant B using v on Config::ConfigVersion at current {{ convert<String>(self) = \"a\" }}\n"
    );
    let refusals = check(&document, &both).expect_err("both refuse");
    assert_eq!(
        causes(&refusals),
        [
            (Code::IllTyped, Some("identity-observing-form")),
            (Code::IllTyped, Some("identity-observing-form")),
        ]
    );
    let regions = admit_and_assemble_with_body(&document, &both)
        .expect("assembles")
        .regions();
    let starts: Vec<u64> = refusals
        .iter()
        .map(|refusal| regions.refusal_region(refusal).expect("a region").start())
        .collect();
    assert!(starts[0] < starts[1], "source order: {starts:?}");
}

/// Population quantification, a primitive-set fold, and a temporal atom that
/// compares references through fields remain admissible under the annotation.
#[trace("TC-566", "FR-150-AC-4")]
#[test]
fn non_identity_observing_population_reads_and_primitive_set_fold_check() {
    let document = annotated(&with_integer_set(&with_peers(&document())));
    let body = format!(
        "{FLAG}{PARENT_ORDER}{NO_CYCLE}\
         invariant Peers using v on Config::ConfigVersion at current {{ \
           size(self.peers) <= 5 and contains(self.peers, self) }}\n\
         invariant Quantified using v on Config::ConfigVersion at current {{ \
           forall p in config_history: p.parent = self implies p != self }}\n\
         invariant PrimitiveFold using v on Config::ConfigVersion at current {{ \
           fold<Tally>(acc, x in self.s: acc + 1, identity: 0) <= 3 }}\n\
         invariant Count using v on Config::ConfigVersion at current {{ \
           count<Tally>(p in self.peers: p = self) = 0 }}\n"
    );
    check(&document, &body).expect("transparent forms check");

    let temporal = "temporal SameParent using t over (a: Config::ConfigVersion, c: Config::ConfigVersion) \
        clock \"model-steps\" on origin { always holds(c.parent = a.parent) }";
    let package = admit_and_assemble_with_profiles(
        &document,
        "profile t = \"filament.temporal.infinite-trace/v1\";\n",
        temporal,
    )
    .expect("the temporal unit assembles");
    package
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect("the temporal equality atom checks");
}

/// FR-150-AC-7: a chooser fold in a function body keeps FR-145's typed
/// operator refusal without an identity-observing cause, while ordinary
/// reference equality, conversion and sequence reads in function bodies and
/// the order-independent population operators in a postcondition check.
#[trace("TC-566", "FR-150-AC-7")]
#[test]
fn function_bodies_keep_their_rules_and_population_operators_check() {
    let document = annotated(&with_peers(&document()));
    let chooser = "fold<Flag>(acc, p in s: p = p, identity: false)";
    let body = format!(
        "{FLAG}function pick using v(s: Set<Reference<Config::ConfigVersion>>): Boolean pure {{ {chooser} }}\n"
    );
    let refusals = check(&document, &body).expect_err("the function fold is not catalogued");
    assert_eq!(
        causes(&refusals),
        [(Code::IllTyped, Some("operator-ineligible"))]
    );
    let regions = admit_and_assemble_with_body(&document, &body)
        .expect("the function unit assembles")
        .regions();
    let (text, _) = config_unit_with_body(&document, &body);
    let region = regions.refusal_region(&refusals[0]).expect("a refusal locus");
    let span = span_of(&text, chooser);
    assert_eq!(
        (region.start(), region.end()),
        (span.start as u64, span.end as u64)
    );

    let functions = format!(
        "{FLAG}function same using v(p: Reference<Config::ConfigVersion>): Boolean pure {{ p = p }}\n\
         function render using v(p: Reference<Config::ConfigVersion>): String pure {{ convert<String>(p) }}\n\
         function ordered using v(s: Set<Reference<Config::ConfigVersion>>): Sequence<Reference<Config::ConfigVersion>>[0, 8] pure {{ convert<Sequence<Reference<Config::ConfigVersion>>[0, 8]>(s) }}\n"
    );
    check(&document, &functions).expect("function bodies use the unannotated rules");

    let post = format!(
        "{FLAG}type Tally = Integer;\n\
         post PopulationReads using v on Config::ConfigVersion::attemptUpdate {{\
           count<Tally>(p in self.peers: p.versionNumber >= 0) <= 5 and\
           sum<Tally>(p in self.peers: p.versionNumber) >= 0 and\
           exists(p in self.peers where p.versionNumber >= 0) and\
           forall(p in self.peers where p.versionNumber >= 0) and\
           contains(self.peers, self) and size(self.peers) <= 5 and\
           size(filter(p in self.peers where p.versionNumber >= 0)) <= 5 and\
           size(map(p in self.peers: p)) <= 5 }}\n"
    );
    check(&document, &post).expect("order-independent population operators check");
}

/// FR-150-AC-7: reference equality with a typed reference literal is admitted
/// in a function body, even when the selected population is symmetric.
#[trace("TC-566", "FR-150-AC-7")]
#[test]
fn a_function_body_can_compare_a_reference_parameter_to_a_literal() {
    let document = annotated(&with_peers(&document()));
    let function = "function isA using v(p: Reference<Config::ConfigVersion>): Boolean pure { \
                    p = Config::ConfigVersion#\"a\" }\n";
    check(&document, function).expect("literal equality in a function checks");
}

/// FR-150-AC-8: the precondition form rejects the literal at its own span
/// and preserves the symmetric population as the typed cause's locus.
#[trace("TC-566", "FR-150-AC-8")]
#[test]
fn an_operation_precondition_cannot_compare_self_to_a_reference_literal() {
    let document = annotated(&document());
    let literal = "Config::ConfigVersion#\"a\"";
    let body = format!(
        "pre Literal using v on Config::ConfigVersion::attemptUpdate {{ self = {literal} }}\n"
    );
    let payload = identity_refusal(&document, &body, literal);
    assert_eq!(payload.node, IdentityNode::ReferenceLiteral);
    assert_eq!(payload.population.node, POPULATION);
}

/// Ordered conversions of symmetric references refuse at the conversion;
/// unordered bag conversion is accepted, while choosing an element after
/// that conversion refuses at the fold. The same sequence conversion remains
/// valid when the population is not annotated.
#[trace("TC-566", "FR-150-AC-5")]
#[test]
fn ordered_reference_conversions_refuse_and_bag_conversion_preserves_scope() {
    let plain_document = with_peers(&document());
    let document = annotated(&plain_document);
    for convert in [
        "convert<Sequence<Config::ConfigVersion>[0, 5]>(self.peers)",
        "convert<OrderedSet<Config::ConfigVersion>[0, 5]>(self.peers)",
    ] {
        let body = format!(
            "invariant Order using v on Config::ConfigVersion at current {{ size({convert}) = 0 }}\n"
        );
        let payload = identity_refusal(&document, &body, convert);
        assert_eq!(payload.node, IdentityNode::Convert, "{convert}");
        assert_eq!(
            payload.form,
            IdentityForm::OrderSensitiveTraversal,
            "{convert}"
        );
        assert_eq!(payload.population.node, POPULATION, "{convert}");
    }

    let bag = "convert<Bag<Config::ConfigVersion>[0, 5]>(self.peers)";
    let body = format!(
        "invariant BagSize using v on Config::ConfigVersion at current {{ size({bag}) <= 5 }}\n"
    );
    check(&document, &body).expect("an unordered bag conversion checks");

    let chooser = format!(
        "fold<Flag>(acc, p in {bag}: p = self, identity: false)"
    );
    let body = format!(
        "{FLAG}invariant Chooser using v on Config::ConfigVersion at current {{ not {chooser} }}\n"
    );
    let payload = identity_refusal(&document, &body, &chooser);
    assert_eq!(payload.node, IdentityNode::Fold);
    assert_eq!(payload.form, IdentityForm::OrderSensitiveTraversal);
    assert_eq!(payload.population.node, POPULATION);

    let sequence = "convert<Sequence<Config::ConfigVersion>[0, 5]>(self.peers)";
    let body = format!(
        "invariant Order using v on Config::ConfigVersion at current {{ size({sequence}) = 0 }}\n"
    );
    check(&plain_document, &body)
        .expect("the same ordered conversion checks without the annotation");
}

/// A reference typed as a supertype of a symmetric population's member type
/// is a reference into that population. Verifies the supertype lookup, which
/// no FR-150 AC lists.
#[trace("TC-566")]
#[test]
fn a_supertype_reference_into_a_symmetric_subtype_population_refuses() {
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&annotated(&with_peers(&document()))).expect("valid JSON");
    let sub = "ix://example/config-version/Sub";
    envelope["populations"][0]["members"] = json!([sub]);
    let document = serde_json::to_vec(&envelope).expect("valid JSON");
    let document = crate::model_operations::add_sub_type(document, &[]);
    let mut envelope: serde_json::Value = serde_json::from_slice(&document).expect("valid JSON");
    envelope["populations"][0]["members"] = json!([sub]);
    let document = serde_json::to_vec(&envelope).expect("valid JSON");
    let body =
        format!("{FLAG}invariant Last using v on Config::Sub at current {{ not {LAST_PEER} }}\n");
    let payload = identity_refusal(&document, &body, LAST_PEER);
    assert_eq!(payload.node, IdentityNode::Fold);
    assert_eq!(payload.population.node, POPULATION);
}

/// An identity conversion of a reference in an equality operand is not a
/// conversion to another type, so it checks. Verifies that exemption only;
/// FR-150-AC-4 lists no identity conversion.
#[trace("TC-566")]
#[test]
fn an_identity_conversion_in_an_equality_operand_checks() {
    let document = annotated(&with_peers(&document()));
    let body = "invariant Same using v on Config::ConfigVersion at current { \
                convert<Config::ConfigVersion>(self) = self }\n";
    check(&document, body).expect("an identity conversion checks");
}

/// A non-Boolean `symmetric` refuses as a malformed declaration, and another
/// extra population member is still refused.
#[trace("TC-565")]
#[test]
fn a_malformed_marker_and_another_extra_member_refuse() {
    for (member, value) in [("symmetric", json!("yes")), ("unknown", json!(true))] {
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&document()).expect("valid JSON");
        envelope["populations"][0][member] = value;
        let bytes = serde_json::to_vec(&envelope).expect("valid JSON");
        let package = PackageDocument::parse(&bytes, IntakeLimits::default()).expect("parses");
        let refusals =
            read_records(PACKAGE_IDENTITY, &package).expect_err(&format!("{member} refuses"));
        let [refusal] = &refusals[..] else {
            panic!("{member}: one refusal, not {refusals:?}");
        };
        assert_eq!(refusal.cause.as_str(), "malformed-declaration", "{member}");
        if member == "symmetric" {
            let ModelRefusalCause::IntakeMalformedDeclaration { member: at, .. } = &refusal.cause
            else {
                panic!("{member}: an intake malformed declaration, not {refusal:?}");
            };
            assert_eq!(
                at.segments().last(),
                Some(&MemberSegment::Name("symmetric".to_owned()))
            );
        }
    }
}
