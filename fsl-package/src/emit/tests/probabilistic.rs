// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-740: a checked random parameter, workload, delay and reward are emitted
//! as `probabilistic` nodes; a declaration that cannot be written is refused
//! at emission, with its locus, rather than written without it. FR-498: a
//! probabilistic claim is emitted as its own `claim`/`probabilistic_claim`
//! node and sidecar row.

use super::admission_corpus::config_version_unit;
use super::*;
use filament_semantic_value::checking::CheckMode;

/// The ConfigVersion unit with a workload and a reward on `attemptUpdate`.
fn probabilistic_package() -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = format!(
        "{}workload Steady on Config {{ weight ConfigVersion::attemptUpdate = 1; }}\n\
         reward cost on ConfigVersion::attemptUpdate = 1;\n",
        config_version_unit::unit_text()
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert!(!graph.probabilistic_model().is_empty());
    CheckedPackage::link(graph)
}

/// The ConfigVersion unit with `declarations` appended.
pub(super) fn reward_package(declarations: &str) -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = format!("{}{declarations}\n", config_version_unit::unit_text());
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// The ConfigVersion unit with `weights` written in a workload `Steady`.
pub(super) fn workload_package(weights: &str) -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = format!(
        "{}workload Steady on Config {{ {weights} }}\n",
        config_version_unit::unit_text()
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// The ConfigVersion unit under the event-position profile `ev`, with
/// `declarations` appended in order.
fn claims_package(declarations: &[&str]) -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"filament.model.complete/v1\";\n",
        "profile v = \"filament.model.complete/v1\";\n\
         profile ev = \"filament.temporal.event-position.false-extension/v1\";\n",
    );
    let unit = format!("{unit}{}\n", declarations.join("\n"));
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// The ConfigVersion fixture with the Boolean fields used by QSL-2656's
/// two same-path conditional expressions.
fn repeated_operand_claims_package(declarations: &[&str]) -> (CheckedPackage, Vec<u8>) {
    let mut document: Value = serde_json::from_str(config_version_unit::DOMAIN_PACKAGE)
        .expect("the ConfigVersion fixture is JSON");
    let config = "ix://example/config-version/ConfigVersion";
    let config_type = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == config)
        .expect("ConfigVersion");
    for name in ["ok", "done"] {
        let identity = format!("{config}/{name}");
        config_type["fields"]
            .as_array_mut()
            .expect("ConfigVersion fields")
            .push(json!({
                "identity": identity,
                "name": name,
                "typeRef": "ix://filament/native/Boolean",
                "presence": "required",
                "nullable": false,
                "defaultKind": "none",
                "multiplicity": {
                    "lower": 1,
                    "upper": 1,
                    "ordered": false,
                    "unique": true,
                },
                "origin": {
                    "generated": {
                        "generatorIdentity": identity,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [identity],
                    }
                },
            }));
    }
    let document = serde_json::to_vec(&document).expect("the model stays JSON");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the extended model is admitted");
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"filament.model.complete/v1\";\n",
        "profile v = \"filament.model.complete/v1\";\n\
         profile ev = \"filament.temporal.event-position.false-extension/v1\";\n",
    );
    let unit = format!("{unit}{}\n", declarations.join("\n"));
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the repeated-operand claims check: {refusals:?}"));
    (CheckedPackage::link(graph), document)
}

/// `probabilistic name ... { probability >= threshold [ holds(true) ] }`.
fn claim(name: &str, threshold: &str) -> String {
    format!(
        "probabilistic {name} using ev on Config under every scheduler on origin \
         {{ probability >= {threshold} [ holds(true) ] }}"
    )
}

fn claim_each(name: &str, threshold: &str) -> String {
    format!(
        "probabilistic {name} using ev on Config under every scheduler \
         on each (m: Config::ConfigVersion) \
         {{ probability >= {threshold} [ holds(true) ] }}"
    )
}

/// Each checked claim's emitted `{declaration, occurrence}`, as wire JSON,
/// by the label of the claim content it was declared with.
fn identities(package: &CheckedPackage, labels: &[(&str, &str)]) -> Vec<(String, Value)> {
    package
        .graph()
        .probabilistic_model()
        .claims()
        .iter()
        .zip(labels)
        .map(|(claim, (label, _))| {
            let occurrence = claim
                .occurrence()
                .expect("an assembled claim has an identity");
            let row = filament_contract_model::CheckedProbabilisticClaim {
                declaration: checked_node_id_of(occurrence.node()),
                occurrence: checked_occurrence(occurrence.origin()).expect("a claim occurrence"),
            };
            ((*label).to_owned(), serde_json::to_value(row).unwrap())
        })
        .collect()
}

/// The `probabilistic_claims` rows of `package` as the wire carries them.
fn raw_claim_rows(package: &CheckedPackage) -> Vec<Value> {
    let emission = emit_checked(package).expect("a claim-only package emits");
    wire(&emission)["checked_term_data"]["probabilistic_claims"]
        .as_array()
        .expect("the claims are carried")
        .clone()
}

/// The emitted `probabilistic_claims` rows, and each `claim` node's form and
/// occurrences by node id.
fn emitted_claims(package: &CheckedPackage) -> (Vec<Value>, Vec<(Value, Value)>) {
    emitted_claims_with_document(package, config_version_unit::DOMAIN_PACKAGE.as_bytes())
}

fn emitted_claims_with_document(
    package: &CheckedPackage,
    document: &[u8],
) -> (Vec<Value>, Vec<(Value, Value)>) {
    let emission = emit_checked(package).expect("a claim-only package emits");
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let read = read_with(&emission, &evidence);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let wire = wire(&emission);
    // An operand index names a position, so ids compare through their
    // resolved form (FSpec FR-322 "Operand index segments").
    let rows: Vec<Value> = wire["checked_term_data"]["probabilistic_claims"]
        .as_array()
        .expect("the claims are carried")
        .iter()
        .map(|row| {
            let mut row = row.clone();
            row["declaration"] = resolved_id(&wire, &row["declaration"], 8);
            row
        })
        .collect();
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the graph has nodes")
        .iter()
        .filter(|node| node["node_tag"] == "claim")
        .map(|node| {
            assert_eq!(node["semantic_form"], "probabilistic_claim");
            (
                resolved_id(&wire, &node["node_id"], 8),
                node["occurrences"].clone(),
            )
        })
        .collect();
    (rows, nodes)
}

/// FR-186 "Outputs", FR-498: a package whose only probabilistic declaration
/// is a checked claim is emitted with the claim's node; one that also
/// declares a reward after it is emitted with the reward's node too.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn a_checked_claim_and_a_reward_are_both_emitted() {
    let package = claims_package(&[&claim("Stable", "0.5")]);
    assert_eq!(package.graph().probabilistic_model().claims().len(), 1);
    let (rows, nodes) = emitted_claims(&package);
    assert_eq!((rows.len(), nodes.len()), (1, 1));

    let package = claims_package(&[
        &claim("Stable", "0.5"),
        "reward cost on ConfigVersion::attemptUpdate = 1;",
    ]);
    let (rows, nodes) = emitted_claims(&package);
    assert_eq!((rows.len(), nodes.len()), (1, 1));
    let emission = emit_checked(&package).expect("the reward emits beside the claim");
    let wire = wire(&emission);
    assert_eq!(
        super::owners::node_where(&wire, "probabilistic", "reward").len(),
        1
    );
}

/// FR-186 Activation and FR-498: the checked package retains the `on each`
/// claim as one declaration whose activation ranges over its population.
#[trace("TC-2166", "FR-186-AC-8")]
#[test]
fn probabilistic_on_each_emits_per_member() {
    let package = claims_package(&[&claim_each("PerVersion", "0.5")]);
    let [checked] = package.graph().probabilistic_model().claims() else {
        panic!("one checked claim per source declaration");
    };
    assert!(
        matches!(
            &checked.activation,
            fsl_semantics::check::ClaimActivation::Each { .. }
        ),
        "activation is carried by the checked claim"
    );
    let (rows, nodes) = emitted_claims(&package);
    assert_eq!((rows.len(), nodes.len()), (1, 1));
}

/// FR-498 (TC-1288): two claims sharing a name, and a third, emit one
/// `claim`/`probabilistic_claim` node with one `claim` occurrence and one
/// matching row each, rows in structural order. Permuting the declarations
/// keeps each claim's `{declaration, occurrence}`; the checked claim
/// exposes the row's identity; renaming a claim changes its identity, and
/// two identically written claims are still two declarations.
#[trace("FR-498-AC-1", "FR-498-AC-2", "FR-498-AC-4", "TC-1288")]
#[test]
fn probabilistic_claim_identity_survives_reordering() {
    let declared = [
        ("PC-01", claim("Stable", "0.5")),
        ("PC-02", claim("Steady", "0.25")),
        ("PC-03", claim("Other", "0.5")),
    ];
    let emit = |order: &[usize]| {
        let labels: Vec<(&str, &str)> = order
            .iter()
            .map(|&at| (declared[at].0, declared[at].1.as_str()))
            .collect();
        let texts: Vec<&str> = labels.iter().map(|(_, text)| *text).collect();
        let package = claims_package(&texts);
        let mut identities = identities(&package, &labels);
        let (rows, nodes) = emitted_claims(&package);
        // One node, one `claim` occurrence and one row per claim, and the
        // checked claim exposes exactly its row.
        assert_eq!((rows.len(), nodes.len()), (3, 3));
        for (_, identity) in &identities {
            assert_eq!(rows.iter().filter(|row| *row == identity).count(), 1);
            let (_, occurrences) = nodes
                .iter()
                .find(|(node, _)| *node == identity["declaration"])
                .expect("the row names an emitted claim node");
            let occurrences = occurrences.as_array().unwrap();
            assert_eq!(occurrences.len(), 1);
            assert_eq!(occurrences[0], identity["occurrence"]);
        }
        let mut sorted = rows.clone();
        sorted.sort_by_key(|row| {
            let id = &row["declaration"];
            (
                id["package"].to_string(),
                id["path"].to_string(),
                id["ordinal"].as_u64(),
            )
        });
        assert_eq!(rows, sorted, "rows are in structural order");
        identities.sort_by(|left, right| left.0.cmp(&right.0));
        (identities, rows, raw_claim_rows(&package))
    };
    let (forward, forward_rows, forward_raw) = emit(&[0, 1, 2]);
    let (reversed, reversed_rows, reversed_raw) = emit(&[2, 1, 0]);
    let (shuffled, _, _) = emit(&[1, 2, 0]);
    assert_eq!(forward, reversed, "each claim keeps its identity");
    assert_eq!(forward, shuffled, "each claim keeps its identity");
    assert_eq!(forward_rows, reversed_rows, "the rows resolve identically");
    assert_eq!(forward_raw, reversed_raw, "the rows are byte-identical");
    assert_ne!(
        forward[0].1, forward[1].1,
        "distinct names are distinct claims"
    );
    // Each name is its own path, so each claim is the first of its name.
    assert_eq!(forward[1].1["declaration"]["ordinal"], 0, "Steady@0.25");
    assert_eq!(forward[0].1["declaration"]["ordinal"], 0, "Stable@0.5");

    // Renaming a claim changes its identity and no other claim's.
    let renamed = claims_package(&[
        &claim("Stable", "0.5"),
        &claim("Steady", "0.25"),
        &claim("Renamed", "0.5"),
    ]);
    let renamed = identities(&renamed, &[("PC-01", ""), ("PC-02", ""), ("PC-03", "")]);
    assert_eq!(renamed[..2], forward[..2]);
    assert_ne!(renamed[2].1, forward[2].1);

    // Two identically written claims with distinct names are two
    // declarations.
    let twins = claims_package(&[&claim("Stable", "0.5"), &claim("Steady", "0.5")]);
    let twins = identities(&twins, &[("a", ""), ("b", "")]);
    assert_ne!(twins[0].1, twins[1].1);
    let (rows, nodes) = emitted_claims(&claims_package(&[
        &claim("Stable", "0.5"),
        &claim("Steady", "0.5"),
    ]));
    assert_eq!((rows.len(), nodes.len()), (2, 2));
}

/// FR-498-AC-1: same-path expression nodes retain repeated operand
/// positions, so changing declaration order cannot swap their ordinals or
/// alter the claims' checked identities.
#[trace("TC-1288", "FR-498-AC-1")]
#[test]
fn probabilistic_claims_with_repeated_expression_operands_keep_identity_when_reordered() {
    let claims = [
        ("RepeatedTrue", "if s.ok then s.ok else s.done"),
        ("RepeatedFalse", "if s.ok then s.done else s.done"),
    ];
    let declaration = |(name, expression): (&str, &str)| {
        format!(
            "probabilistic {name} using ev on Config over (s: Config::ConfigVersion) \
             under every scheduler on origin \
             {{ probability >= 0.5 [ always[0,2] holds({expression}) ] }}"
        )
    };
    let emit = |order: &[usize]| {
        let sources: Vec<_> = order.iter().map(|&at| declaration(claims[at])).collect();
        let labels: Vec<_> = order
            .iter()
            .zip(&sources)
            .map(|(&at, source)| (claims[at].0, source.as_str()))
            .collect();
        let (package, document) = repeated_operand_claims_package(
            &sources.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let mut ids = identities(&package, &labels);
        let (rows, nodes) = emitted_claims_with_document(&package, &document);
        assert_eq!((rows.len(), nodes.len()), (2, 2));
        ids.sort_by(|left, right| left.0.cmp(&right.0));
        (ids, rows)
    };

    let (forward, forward_rows) = emit(&[0, 1]);
    let (reversed, reversed_rows) = emit(&[1, 0]);
    assert_ne!(
        forward[0].1, forward[1].1,
        "the two claims retain distinct identities"
    );
    assert_eq!(forward, reversed, "each claim keeps its checked identity");
    assert_eq!(
        forward_rows, reversed_rows,
        "the rows remain byte-identical"
    );
}

/// FR-940-AC-9 (TC-3015): a second probabilistic claim named as an earlier
/// one is refused `ambiguous_declaration`/`ambiguous-name` at the second,
/// naming both; with the second renamed the unit checks and its package
/// carries one `claim_names` row per claim.
#[trace("FR-940-AC-9", "TC-3015")]
#[test]
fn a_second_same_named_probabilistic_claim_is_ambiguous() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = config_version_unit::unit_text().replace(
        "profile v = \"filament.model.complete/v1\";\n",
        "profile v = \"filament.model.complete/v1\";\n\
         profile ev = \"filament.temporal.event-position.false-extension/v1\";\n",
    );
    let unit = format!(
        "{unit}{}\n{}\n",
        claim("Stable", "0.5"),
        claim("Stable", "0.25")
    );
    let refusals = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(|_| ())
        .expect_err("two claims named Stable are ambiguous");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    let fsl_semantics::check::CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("ambiguous-name: {refusal:?}");
    };
    assert_eq!(name, "Stable");
    assert_eq!(loci.len(), 2, "names both declarations");
    assert_eq!(loci[1], refusal.location, "located at the second");
    assert_ne!(loci[0], loci[1]);

    let package = claims_package(&[&claim("Stable", "0.5"), &claim("Other", "0.25")]);
    let wire = wire(&emit_checked(&package).expect("the package emits"));
    let names: Vec<&str> = wire["claim_names"]
        .as_array()
        .expect("claim_names rows")
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    for declared in ["Stable", "Other"] {
        let rows = names.iter().filter(|name| **name == declared).count();
        assert_eq!(rows, 1, "one claim_names row for {declared}: {names:?}");
    }
}

/// The emission of `package` read back by the strict reader with `document`
/// as its domain package evidence, and the emitted wire.
fn emit_and_read(package: &CheckedPackage, document: &[u8]) -> (Value, Read) {
    let emission = emit_checked(package)
        .unwrap_or_else(|refusal| panic!("a probabilistic package emits: {refusal:?}"));
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let read = read_with(&emission, &evidence);
    (wire(&emission), read)
}

/// TC-2062 (FR-740-AC-1, FR-185-AC-7): a checked workload and reward are
/// emitted as FSpec FR-640 `probabilistic`/`workload` and
/// `probabilistic`/`reward` nodes, each with one `declaration` occurrence,
/// and the strict reader admits the package.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("TC-2062", "FR-185-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn a_workload_and_a_reward_are_emitted_as_probabilistic_nodes() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (wire, read) = emit_and_read(&probabilistic_package(), document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [workload] = super::owners::node_where(&wire, "probabilistic", "workload")[..] else {
        panic!("one workload node");
    };
    let [reward] = super::owners::node_where(&wire, "probabilistic", "reward")[..] else {
        panic!("one reward node");
    };
    for node in [workload, reward] {
        assert_eq!(
            node["occurrences"],
            json!([{"role": "declaration", "ordinal": 0}])
        );
    }
    let body = workload["body"].to_string();
    assert!(body.contains("Steady"), "{body}");
    assert!(body.contains("attemptUpdate"), "{body}");
    let body = reward["body"].to_string();
    assert!(body.contains("cost"), "{body}");
    assert!(body.contains("attemptUpdate"), "{body}");
}

/// The fixture model's document, and a unit with the random parameter
/// `attemptUpdate(n)` over `{ 3: 1, 1: 3 }`
pub(super) fn random_parameter_package() -> (Vec<u8>, CheckedPackage) {
    use super::abstraction::fixture;
    let document =
        crate::emit::tests::wire_bytes(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         random ConfigVersion::attemptUpdate(n) ~ {{ 3: 1, 1: 3 }};\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (document, CheckedPackage::link(graph))
}

/// The admitted package and source map of a verified read.
fn admitted_of(read: Read) -> (Arc<CheckedPackageV2>, Vec<CheckedSourceMapEntry>) {
    let Read::Verified { admitted, .. } = read else {
        panic!("the strict reader admits the package: {read:?}");
    };
    let source_map = admitted.source_map().to_vec();
    (admitted, source_map)
}

/// The declaration region of `node` in `source_map`.
fn declaration_region(source_map: &[CheckedSourceMapEntry], node: &CheckedNodeId) -> (u64, u64) {
    let [entry] = source_map
        .iter()
        .filter(|entry| entry.node_id == *node && entry.role == CheckedOccurrenceRole::Declaration)
        .collect::<Vec<_>>()[..]
    else {
        panic!("one declaration entry for the node");
    };
    (entry.regions[0].start, entry.regions[0].end)
}

/// TC-2062 (FR-740-AC-1): a unit with a random parameter on
/// `attemptUpdate(n)` emits one `probabilistic`/`random_parameter` node
/// whose support keeps the declared order and weights, and the strict reader
/// admits it.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_random_parameter_is_emitted_and_read_back() {
    let (document, package) = random_parameter_package();
    let (wire, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [random] = super::owners::node_where(&wire, "probabilistic", "random_parameter")[..] else {
        panic!("one random_parameter node");
    };
    let body = random["body"].to_string();
    assert!(body.contains("attemptUpdate"), "{body}");
    let three = body.find("\"3\"").expect("the support value 3 is carried");
    let one = body.find("\"1\"").expect("the support value 1 is carried");
    assert!(three < one, "the support keeps its declared order: {body}");
}

/// TC-2062 (FR-740-AC-1): the typed read of the emitted package gives the
/// random parameter `n` of `attemptUpdate` with its support in declared
/// order, its exact weights and probabilities, and the declaration's region.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn random_parameter_round_trips() {
    let (document, package) = random_parameter_package();
    let [checked] = package.graph().probabilistic_model().random_parameters() else {
        panic!("one checked random parameter");
    };
    let (_, read) = emit_and_read(&package, &document);
    let (admitted, source_map) = admitted_of(read);
    let declaring = fsl_semantics::check::wire_id::structural_id(
        &fsl_semantics::model::key::object_type_node_key(
            checked.operation().declaring_key().unwrap(),
        )
        .expect("the declaring type has a node key"),
    );
    let reads = admitted.random_parameters(&declaring, checked.operation().name());
    let [read] = &reads[..] else {
        panic!("one random parameter read: {reads:?}");
    };
    assert_eq!(read.parameter, checked.parameter());
    let spelled = |rational: &filament_contract_model::CheckedRational| {
        format!("{}/{}", rational.numerator, rational.denominator)
    };
    let values: Vec<(Value, String, String)> = read
        .support
        .iter()
        .map(|row| {
            (
                serde_json::to_value(row.value).expect("a literal term")["value"].clone(),
                spelled(&row.weight),
                spelled(&row.probability),
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            (json!("3"), "1/1".to_owned(), "1/4".to_owned()),
            (json!("1"), "3/1".to_owned(), "3/4".to_owned()),
        ]
    );
    for (row, checked) in read.support.iter().zip(checked.support()) {
        assert_eq!(
            spelled(&row.weight),
            format!(
                "{}/{}",
                checked.weight.numerator(),
                checked.weight.denominator()
            )
        );
        assert_eq!(
            spelled(&row.probability),
            format!(
                "{}/{}",
                checked.probability.numerator(),
                checked.probability.denominator()
            )
        );
    }
    let region = package
        .graph()
        .region(&checked.declaration_location())
        .expect("the declaration has a region");
    assert_eq!(
        declaration_region(&source_map, read.node),
        (region.start(), region.end())
    );
}

/// TC-2062 (FR-740-AC-1): the typed read of the emitted package gives the
/// workload `Steady` with the weight of the model's operation and the
/// declaration's region.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn workload_round_trips() {
    let package = workload_package("weight ConfigVersion::attemptUpdate = 1.5;");
    let [checked] = package.graph().probabilistic_model().workloads() else {
        panic!("one checked workload");
    };
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (_, read) = emit_and_read(&package, document);
    let (admitted, source_map) = admitted_of(read);
    let [(operation, _)] = checked.weights() else {
        panic!("one weighted operation");
    };
    let model = fsl_semantics::check::wire_id::structural_id(
        &fsl_semantics::model::key::object_type_node_key(operation.declaring_key().unwrap())
            .expect("the model type has a node key"),
    );
    let reads = admitted.workloads(&model);
    let [read] = &reads[..] else {
        panic!("one workload read: {reads:?}");
    };
    assert_eq!(read.name, checked.name());
    let [weight] = &read.weights[..] else {
        panic!("one weight");
    };
    assert_eq!(
        (weight.declaring, weight.operation),
        (&model, operation.name())
    );
    assert_eq!(
        (&*weight.weight.numerator, &*weight.weight.denominator),
        ("3", "2")
    );
    let region = package
        .graph()
        .region(&checked.declaration_location())
        .expect("the declaration has a region");
    assert_eq!(
        declaration_region(&source_map, read.node),
        (region.start(), region.end())
    );
}

/// The records the package route reads back from the emission of `package`
/// over the domain package `document`: the emitted nodes, joined to the
/// selected domain package's view.
///
/// `units` names the units the domain package's operation types use: the
/// package route reads none from the bytes yet, so a package whose operations
/// type a unit supplies them (and is joined over S3's type environment).
fn read_back(
    package: &CheckedPackage,
    document: &[u8],
    units: &BTreeMap<String, filament_exact::UnitId>,
) -> fsl_semantics::check::CheckedProbabilisticModel {
    try_read_back(package, document, units, |_| {})
        .unwrap_or_else(|refusal| panic!("the records rebuild: {refusal}"))
}

/// [`read_back`] over the package nodes after `edit`, with the join's refusal
/// returned.
fn try_read_back(
    package: &CheckedPackage,
    document: &[u8],
    units: &BTreeMap<String, filament_exact::UnitId>,
    edit: impl FnOnce(&mut fsl_semantics::check::PackageProbabilistic),
) -> Result<
    fsl_semantics::check::CheckedProbabilisticModel,
    fsl_semantics::check::PackageProbabilisticRefusal,
> {
    use fsl_semantics::model::observation::ModelSelections;
    let emission = emit_checked(package)
        .unwrap_or_else(|refusal| panic!("a probabilistic package emits: {refusal:?}"));
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let (admitted, _) = admitted_of(read_with(&emission, &evidence));
    let mut rows_budget = V2ReadLimits::fixture().native_join_budget();
    let rows = emission
        .identity_rows(&admitted, &mut rows_budget)
        .expect("the identity rows join the package");
    let mut join_budget = V2ReadLimits::fixture().native_join_budget();
    let bare = crate::BarePackage::new(&admitted)
        .with_supply(
            crate::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut join_budget,
        )
        .expect("the bytes view admits its supply");
    let world = bare.world().expect("the declarations read");
    // The bare world cannot type an operation that names a unit (the package
    // route reads no unit names from the bytes yet), so such a package is
    // joined over S3's own type environment: the same declarations.
    let types = if units.is_empty() {
        world.types()
    } else {
        package.graph().scope().types()
    };
    let selections = crate::AnalysisView::model_selections(&bare);
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let model = fsl_semantics::model::state::StateModel::new(
        ModelSelections::new(&selections, &[]),
        types,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the selected packages re-admit");
    let operations = model.operation_table(units).expect("the operations type");
    let mut nodes = bare.probabilistic().expect("the nodes read");
    edit(&mut nodes);
    model.probabilistic_model(&operations, &nodes)
}

/// The accessor-visible content of an operation, which is all of it but the
/// `model` alias a package does not carry.
fn operation_content(
    operation: &fsl_semantics::check::ProbabilisticOperation,
) -> (
    &filament_checked::key::DeclarationKey,
    filament_exact::EffectiveId,
    Option<&filament_checked::key::DeclarationKey>,
    &str,
) {
    (
        operation.key(),
        operation.declaring(),
        operation.declaring_key(),
        operation.name(),
    )
}

/// FSL FR-740, FSpec FR-640-AC-1: the records read back from the emitted
/// package are the S3 records: the same operations, positions, parameter
/// types, support values with their weights and probabilities, and
/// workload weights.
fn assert_reads_back_equal(
    package: &CheckedPackage,
    document: &[u8],
    units: &BTreeMap<String, filament_exact::UnitId>,
) {
    let checked = package.graph().probabilistic_model();
    let read = read_back(package, document, units);
    assert_eq!(
        read.random_parameters().len(),
        checked.random_parameters().len()
    );
    assert_eq!(read.workloads().len(), checked.workloads().len());
    assert_eq!(read.delays().len(), checked.delays().len());
    for expected in checked.delays() {
        let found = read
            .delay(expected.operation().key())
            .unwrap_or_else(|| panic!("{} is read back", expected.operation().name()));
        assert_eq!(found, expected.distribution());
    }
    for expected in checked.random_parameters() {
        let found = read
            .random_parameter(expected.operation().key(), expected.parameter())
            .unwrap_or_else(|| panic!("{} is read back", expected.parameter()));
        assert_eq!(
            operation_content(found.operation()),
            operation_content(expected.operation())
        );
        assert_eq!(found.position(), expected.position());
        assert_eq!(found.value_type(), expected.value_type());
        assert_eq!(found.support(), expected.support());
    }
    for expected in checked.workloads() {
        let found = read
            .workload(expected.name())
            .unwrap_or_else(|| panic!("{} is read back", expected.name()));
        assert_eq!(found.model_key(), expected.model_key());
        assert_eq!(found.weights().len(), expected.weights().len());
        for (expected, weight) in expected.weights() {
            let (found, read_weight) = found
                .weights()
                .iter()
                .find(|(operation, _)| operation.key() == expected.key())
                .expect("the operation is weighted");
            assert_eq!(operation_content(found), operation_content(expected));
            assert_eq!(read_weight, weight);
        }
    }
}

/// FSL FR-740: an integer-valued random parameter reads back as S3's record.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_random_parameter_reads_back_as_the_s3_record() {
    let (document, package) = random_parameter_package();
    assert_reads_back_equal(&package, &document, &BTreeMap::new());
}

/// FSL FR-740: a workload reads back as S3's record, with a weight that is
/// not an integer.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_workload_reads_back_as_the_s3_record() {
    let package = workload_package("weight ConfigVersion::attemptUpdate = 1.5;");
    assert_reads_back_equal(
        &package,
        config_version_unit::DOMAIN_PACKAGE.as_bytes(),
        &BTreeMap::new(),
    );
}

/// FSL FR-740: an enum-valued random parameter is emitted with an `enum`
/// support literal typed at the enumeration and naming each case, and the
/// strict reader admits it.
#[trace("TC-2062")]
#[test]
fn an_enum_valued_random_parameter_is_emitted_and_read_back() {
    use super::variant_types::fixture;
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         random Account::promote(t) ~ {{ M::Tier::Standard: 3, M::Tier::Gold: 1 }};\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&CheckedPackage::link(graph), &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [random] = super::owners::node_where(&wire, "probabilistic", "random_parameter")[..] else {
        panic!("one random_parameter node");
    };
    let rows = random["body"]["members"][3]["value"]["members"]
        .as_array()
        .expect("the support rows");
    let cases: Vec<_> = rows
        .iter()
        .map(|row| {
            let value = &row["members"][0]["value"];
            assert_eq!(value["value_kind"], "enum", "{value}");
            value["value"].clone()
        })
        .collect();
    assert_eq!(cases, [json!("Standard"), json!("Gold")]);
}

/// FSL FR-740: a Boolean-valued random parameter is emitted with `boolean`
/// support literals `true` and `false` at their exact weights, and the strict
/// reader admits it.
#[trace("TC-2062")]
#[test]
fn a_boolean_valued_random_parameter_is_emitted_and_read_back() {
    use super::abstraction::fixture;
    let mut model = fixture::document();
    let parameter = model["types"]
        .as_array_mut()
        .expect("the types")
        .iter_mut()
        .find(|declaration| declaration["identity"] == fixture::id("ConfigVersion"))
        .map(|declaration| &mut declaration["operations"][0]["params"][0])
        .expect("the first parameter of attemptUpdate");
    parameter["typeRef"] = json!("ix://filament/native/Boolean");
    let document = crate::emit::tests::wire_bytes(&model).expect("the model serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         random ConfigVersion::attemptUpdate(n) ~ {{ true: 999, false: 1 }};\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&CheckedPackage::link(graph), &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [random] = super::owners::node_where(&wire, "probabilistic", "random_parameter")[..] else {
        panic!("one random_parameter node");
    };
    let rows = random["body"]["members"][3]["value"]["members"]
        .as_array()
        .expect("the support rows");
    let supports: Vec<_> = rows
        .iter()
        .map(|row| {
            let value = &row["members"][0]["value"];
            assert_eq!(value["value_kind"], "boolean", "{value}");
            (
                value["value"].clone(),
                row["members"][1]["value"]["value"].clone(),
            )
        })
        .collect();
    assert_eq!(
        supports,
        [(json!(true), json!("999")), (json!(false), json!("1"))]
    );
}

/// FSL FR-740: a quantity-valued random parameter reads back as S3's record,
/// each value in the unit of its parameter's type, and the strict reader
/// admits the unit node as the literal's type.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_quantity_valued_random_parameter_reads_back_as_the_s3_record() {
    use super::abstraction::fixture;
    let document = crate::emit::tests::wire_bytes(&fixture::quantity_document())
        .expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         random Timer::wait(d) ~ {{ 3 ms: 1, 1 ms: 3 }};\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let package = CheckedPackage::link(graph);
    assert!(matches!(
        package.graph().probabilistic_model().random_parameters()[0].support()[0]
            .value
            .0,
        filament_exact::Value::Quantity(_)
    ));
    let filament_exact::Value::Quantity(first) =
        &package.graph().probabilistic_model().random_parameters()[0].support()[0]
            .value
            .0
    else {
        unreachable!("a quantity support value");
    };
    let units = BTreeMap::from([("ms".to_owned(), first.unit().clone())]);
    assert_reads_back_equal(&package, &document, &units);
}

/// The unit over the `Base`/`Leaf` lineage domain package with `declarations`
/// appended, and the domain document.
fn lineage_package(declarations: &str) -> (Vec<u8>, CheckedPackage) {
    lineage_package_with_profiles("", declarations)
}

/// [`lineage_package`] with `profiles` declared before the model.
fn lineage_package_with_profiles(profiles: &str, declarations: &str) -> (Vec<u8>, CheckedPackage) {
    use super::abstraction::fixture;
    let document = crate::emit::tests::wire_bytes(&fixture::lineage_document())
        .expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         {profiles}\n\
         model M = {package:?};\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (document, CheckedPackage::link(graph))
}

/// FSpec FR-640 (FSL FR-740): a workload over two object types that share a
/// supertype is emitted on the one type whose supertype lineage declares
/// every weighted operation (`Leaf`, whose lineage holds `Base::op` and
/// `Leaf::step`), is admitted by the strict reader, and reads back as S3's
/// record.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn a_workload_over_two_types_with_a_shared_supertype_names_the_lineage_type() {
    let (document, package) =
        lineage_package("workload Mixed on M { weight Base::op = 1; weight Leaf::step = 3; }");
    let [checked] = package.graph().probabilistic_model().workloads() else {
        panic!("one checked workload");
    };
    let leaf = checked
        .model_key()
        .expect("one type covers the model")
        .clone();
    assert!(leaf.node.ends_with("/Leaf"), "{leaf:?}");
    let (_, read) = emit_and_read(&package, &document);
    let (admitted, _) = admitted_of(read);
    let model = fsl_semantics::check::wire_id::structural_id(
        &fsl_semantics::model::key::object_type_node_key(&leaf).expect("a node key"),
    );
    let reads = admitted.workloads(&model);
    let [workload] = &reads[..] else {
        panic!("one workload on Leaf: {reads:?}");
    };
    assert_eq!(
        workload
            .weights
            .iter()
            .map(|weight| (weight.operation, &*weight.weight.numerator))
            .collect::<Vec<_>>(),
        [("op", "1"), ("step", "3")]
    );
    assert_reads_back_equal(&package, &document, &BTreeMap::new());
}

/// FSpec FR-640 (FSL FR-740): a workload whose model has two unrelated types
/// (`Base` and `ConfigVersion`) has no model node whose lineage covers both,
/// so emission refuses it with the workload's locus rather than writing a
/// node the reader refuses (QSL-2444 tracks emitting such a workload).
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_workload_over_unrelated_types_refuses_with_its_locus_qsl_2444() {
    use super::abstraction::fixture;
    let document = crate::emit::tests::wire_bytes(&fixture::document()).expect("serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         workload Both on M {{ weight Base::op = 1; weight ConfigVersion::attemptUpdate = 1; }}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let refusal = emit_checked(&CheckedPackage::link(graph)).expect_err("emission refuses");
    assert!(matches!(
        &refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::Workload,
            region,
            ..
        } if declaration == "Both" && region.start() < region.end()
    ));
}

/// TC-2062 (FR-740-AC-3, FSpec FR-640-AC-3): writing `Steady`'s weight
/// lines in another order emits byte-identical output, and a workload with a
/// weight of 0 is emitted with that weight.
#[trace("TC-2062", "FR-740-AC-3")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn workload_weight_order_changes_no_bytes_and_weight_zero_is_emitted() {
    let package = workload_package;
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (zero, read) = emit_and_read(
        &package("weight ConfigVersion::attemptUpdate = 0;"),
        document,
    );
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [workload] = super::owners::node_where(&zero, "probabilistic", "workload")[..] else {
        panic!("one workload node");
    };
    assert!(workload["body"].to_string().contains("\"0\""), "{workload}");

    // Two operations (`Base::op`, `Leaf::step`) with the weight lines written
    // in both orders.
    let ordered = |weights: &str| {
        let (_, package) = lineage_package(&format!("workload Mixed on M {{ {weights} }}"));
        emit_checked(&package).expect("the package emits")
    };
    let forward = ordered("weight Base::op = 1; weight Leaf::step = 3;");
    let reverse = ordered("weight Leaf::step = 3; weight Base::op = 1;");
    assert_eq!(forward.package().bytes(), reverse.package().bytes());
}

/// FSpec FR-640-AC-3: changing one weight or one support value changes the
/// node's identity, while reordering weight lines (asserted separately in
/// `workload_weight_order_changes_no_bytes_and_weight_zero_is_emitted`)
/// does not.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn changing_one_weight_or_support_value_changes_the_node_identity() {
    let workload_id = |weights: &str| {
        let (document, package) = lineage_package(&format!("workload Mixed on M {{ {weights} }}"));
        let (wire, _) = emit_and_read(&package, &document);
        let [node] = super::owners::node_where(&wire, "probabilistic", "workload")[..] else {
            panic!("one workload node");
        };
        node["node_id"].clone()
    };
    let base = workload_id("weight Base::op = 1; weight Leaf::step = 3;");
    assert_ne!(
        base,
        workload_id("weight Base::op = 1; weight Leaf::step = 4;")
    );
    assert_eq!(
        base,
        workload_id("weight Leaf::step = 3; weight Base::op = 1;")
    );

    let random_id = |support: &str| {
        use super::abstraction::fixture;
        let document =
            crate::emit::tests::wire_bytes(&fixture::document()).expect("the fixture serializes");
        let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.model.complete/v1\";\n\
             model M = {package:?};\n\
             random ConfigVersion::attemptUpdate(n) ~ {{ {support} }};\n",
            package = fixture::PACKAGE,
        );
        let graph = assemble_with_models(unit.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
        let (wire, _) = emit_and_read(&CheckedPackage::link(graph), &document);
        let [node] = super::owners::node_where(&wire, "probabilistic", "random_parameter")[..]
        else {
            panic!("one random_parameter node");
        };
        node["node_id"].clone()
    };
    let base = random_id("3: 1, 1: 3");
    assert_ne!(base, random_id("3: 1, 1: 2"), "one weight differs");
    assert_ne!(base, random_id("3: 1, 2: 3"), "one support value differs");
}

const STEADY: &str = "workload Steady on Config { weight ConfigVersion::attemptUpdate = 1; }";
const BURSTY: &str = "workload Bursty on Config { weight ConfigVersion::attemptUpdate = 3; }";
const HOLDS: &str = "{ probability >= 0.5 [ always[0,2] holds(true) ] }";

/// `probabilistic name ... under scheduler` over `body` with confidence.
fn claim_under(name: &str, scheduler: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using ev on Config under {scheduler} on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 {body}"
    )
}

/// The claim nodes of `wire` by declared name.
fn claim_nodes(wire: &Value) -> Vec<(&str, &Value)> {
    super::owners::node_where(wire, "claim", "probabilistic_claim")
        .into_iter()
        .map(|node| (node["node_id"]["path"][2].as_str().expect("a name"), node))
        .collect()
}

fn claim_named<'w>(wire: &'w Value, name: &str) -> &'w Value {
    claim_nodes(wire)
        .into_iter()
        .find(|(declared, _)| *declared == name)
        .map(|(_, node)| node)
        .unwrap_or_else(|| panic!("claim {name}"))
}

/// The value of the binding `name` of the aggregate `term`, at any depth.
fn binding<'v>(term: &'v Value, name: &str) -> &'v Value {
    match term {
        Value::Object(object) => {
            if object.get("name").and_then(Value::as_str) == Some(name) {
                return &object["value"];
            }
            object
                .values()
                .map(|value| binding(value, name))
                .find(|found| !found.is_null())
                .unwrap_or(&Value::Null)
        }
        Value::Array(items) => items
            .iter()
            .map(|value| binding(value, name))
            .find(|found| !found.is_null())
            .unwrap_or(&Value::Null),
        _ => &Value::Null,
    }
}

/// Every reference target in `term`, in order.
fn targets(term: &Value) -> Vec<Value> {
    match term {
        Value::Object(object) => {
            let mut found: Vec<Value> = object.get("target").cloned().into_iter().collect();
            for (key, value) in object {
                if key != "target" {
                    found.extend(targets(value));
                }
            }
            found
        }
        Value::Array(items) => items.iter().flat_map(targets).collect(),
        _ => Vec::new(),
    }
}

fn node_ids(wire: &Value, tag: &str, form: &str) -> Vec<Value> {
    super::owners::node_where(wire, tag, form)
        .into_iter()
        .map(|node| node["node_id"].clone())
        .collect()
}

fn emit_wire(package: &CheckedPackage) -> Value {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (wire, read) = emit_and_read(package, document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    wire
}

/// TC-2062 (FR-740-AC-4), TC-1288 (FR-498-AC-5), FSpec FR-640-AC-3: a claim
/// `under W` binds `W`'s workload node in `scheduler` and holds it as a path
/// operand; a claim `under every scheduler` binds an empty fairness set; a
/// changed weight of `Steady` changes the claim under it and no other;
/// writing the claims in another order changes no claim's resolved identity (the
/// weight-line order is asserted by `a_claim_keeps_its_identity_whichever_order_its_workload_weights_are_written`).
#[trace("TC-2062", "FR-740-AC-4")]
#[trace("TC-1288", "FR-498-AC-5")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_binds_the_workload_it_is_stated_under() {
    let claims = |steady: &str, order: [usize; 3]| {
        let declared = [
            claim_under("A", "Steady", HOLDS),
            claim_under("B", "Bursty", HOLDS),
            "probabilistic C using ev on Config under every scheduler on origin \
             { probability >= 0.5 [ always[0,2] holds(true) ] }"
                .to_owned(),
        ];
        let mut texts = vec![steady.to_owned(), BURSTY.to_owned()];
        texts.extend(order.iter().map(|&at| declared[at].clone()));
        let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
        claims_package(&texts)
    };
    let package = claims(STEADY, [0, 1, 2]);
    let wire = emit_wire(&package);
    assert_eq!(claim_nodes(&wire).len(), 3);
    let workload_named = |name: &str| {
        super::owners::node_where(&wire, "probabilistic", "workload")
            .into_iter()
            .find(|node| binding(&node["body"], "name").to_string().contains(name))
            .map(|node| node["node_id"].clone())
            .expect("the workload node")
    };
    let root = node_ids(&wire, "model", "model_root");
    let [root] = root.as_slice() else {
        panic!("one model root");
    };
    for (name, workload) in [("A", Some("Steady")), ("B", Some("Bursty")), ("C", None)] {
        let claim = claim_named(&wire, name);
        let body = &claim["body"];
        assert_eq!(&targets(binding(body, "subject"))[..], std::slice::from_ref(root));
        let scheduler = targets(binding(body, "scheduler"));
        let path = claim["node_id"]["path"].to_string();
        match workload {
            Some(workload) => {
                assert_eq!(scheduler, [workload_named(workload)], "{name}");
            }
            None => {
                assert!(scheduler.is_empty(), "{name}: no workload");
                assert!(path.contains("\"every\""), "{name}: {path}");
                assert_eq!(
                    binding(binding(body, "scheduler"), "fairness")["members"],
                    json!([])
                );
            }
        }
        assert!(targets(binding(body, "reward")).is_empty(), "{name}");
        assert!(targets(binding(body, "delays")).is_empty(), "{name}");
        assert_eq!(claim["node_id"]["ordinal"], 0);
        assert_eq!(claim["dependencies"], json!(targets(body)));
    }

    // A changed weight of `Steady` changes A and no other claim.
    let heavier = emit_wire(&claims(
        "workload Steady on Config { weight ConfigVersion::attemptUpdate = 2; }",
        [0, 1, 2],
    ));
    let resolved =
        |wire: &Value, name: &str| resolved_id(wire, &claim_named(wire, name)["node_id"], 8);
    assert_ne!(resolved(&wire, "A"), resolved(&heavier, "A"));
    for name in ["B", "C"] {
        assert_eq!(resolved(&wire, name), resolved(&heavier, name), "{name}");
    }

    // The order the claims are written in changes no claim's identity.
    let reversed = emit_wire(&claims(STEADY, [2, 1, 0]));
    for name in ["A", "B", "C"] {
        assert_eq!(resolved(&wire, name), resolved(&reversed, name), "{name}");
    }
}

/// A unit over the abstraction fixture's model with `random` support
/// `support` and `claims`, as wire.
fn random_claims_wire(support: &str, claims: &[String]) -> Value {
    use super::abstraction::fixture;
    let document =
        crate::emit::tests::wire_bytes(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
         model M = {package:?};\n\
         random ConfigVersion::attemptUpdate(n) ~ {{ {support} }};\n{}\n",
        claims.join("\n"),
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    emit_and_read(&CheckedPackage::link(graph), &document).0
}

/// TC-1288 (FR-498-AC-5), FSpec FR-640-AC-3: a claim's `distributions`
/// holds every `random_parameter` node of its subject's model, whatever its
/// scheduler, and a changed support weight or value changes every claim on
/// the subject.
#[trace("TC-1288", "FR-498-AC-5")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_binds_every_random_parameter_of_its_subject() {
    let claims = [
        "probabilistic A using ev on M under every scheduler on origin \
         { probability >= 0.5 [ always[0,2] holds(true) ] }"
            .to_owned(),
        "probabilistic B using ev on M under every scheduler on origin \
         { probability >= 0.25 [ always[0,2] holds(true) ] }"
            .to_owned(),
    ];
    let base = random_claims_wire("3: 1, 1: 3", &claims);
    let randoms = node_ids(&base, "probabilistic", "random_parameter");
    assert_eq!(randoms.len(), 1);
    for (name, node) in claim_nodes(&base) {
        assert_eq!(
            targets(binding(&node["body"], "distributions")),
            randoms,
            "{name}"
        );
        assert!(
            node["node_id"]["path"]
                .to_string()
                .contains("\"distributions\"")
        );
    }
    for support in ["3: 1, 1: 2", "3: 1, 2: 3"] {
        let changed = random_claims_wire(support, &claims);
        for name in ["A", "B"] {
            assert_ne!(
                resolved_id(&base, &claim_named(&base, name)["node_id"], 8),
                resolved_id(&changed, &claim_named(&changed, name)["node_id"], 8),
                "{name} under {support}"
            );
        }
    }
}

const REWARD_DECLARATION: &str = "reward cost on ConfigVersion::attemptUpdate = 1;";

/// The claim of each reward-reading form and of each form that reads none.
fn reward_forms() -> [(&'static str, String, bool); 5] {
    let on = |name: &str, body: &str| {
        format!("probabilistic {name} using ev on Config under every scheduler on origin {body}")
    };
    [
        (
            "Weighted",
            on(
                "Weighted",
                "{ long_run_fraction holds(true) weighted by cost >= 0.5 }",
            ),
            true,
        ),
        (
            "Expected",
            on(
                "Expected",
                "{ expected accumulate cost until holds(true) <= 3 }",
            ),
            true,
        ),
        (
            "Plain",
            on("Plain", "{ long_run_fraction holds(true) >= 0.5 }"),
            false,
        ),
        (
            "Avail",
            on(
                "Avail",
                "{ probability >= 0.5 [ always[0,2] holds(true) ] }",
            ),
            false,
        ),
        (
            "Steps",
            on(
                "Steps",
                "{ probability >= 0.5 [ steps from holds(true) until holds(true) within 10 >= 3 ] }",
            ),
            false,
        ),
    ]
}

/// TC-1288 (a part of FR-498-AC-6), FSpec FR-640-AC-3 and AC-8: a claim whose form reads
/// reward `cost` binds that reward's node in `reward`, a claim that reads none
/// binds an empty `reward`, and changing the reward's expression changes
/// exactly the claims that read it.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-8")]
#[test]
fn a_claim_binds_the_reward_its_form_reads() {
    let forms = reward_forms();
    let wire_of = |reward: &str| {
        let mut texts = vec![reward.to_owned()];
        texts.extend(forms.iter().map(|(_, text, _)| text.clone()));
        let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
        emit_wire(&claims_package(&texts))
    };
    let base = wire_of(REWARD_DECLARATION);
    let rewards = node_ids(&base, "probabilistic", "reward");
    assert_eq!(rewards.len(), 1);
    let changed = wire_of("reward cost on ConfigVersion::attemptUpdate = 2;");
    let resolved_rewards = |wire: &Value| -> Vec<Value> {
        node_ids(wire, "probabilistic", "reward")
            .iter()
            .map(|id| resolved_id(wire, id, 8))
            .collect()
    };
    assert_ne!(resolved_rewards(&base), resolved_rewards(&changed));
    for (name, _, reads) in &forms {
        let claim = claim_named(&base, name);
        let bound = targets(binding(&claim["body"], "reward"));
        let moved = resolved_id(&base, &claim["node_id"], 8)
            != resolved_id(&changed, &claim_named(&changed, name)["node_id"], 8);
        if *reads {
            assert_eq!(bound, rewards, "{name}");
        } else {
            assert!(bound.is_empty(), "{name}");
        }
        assert_eq!(
            moved, *reads,
            "{name}: only a reading claim's identity moves"
        );
    }
}

/// FSpec FR-640-AC-3: changing a reward's expression changes the identity of
/// every claim that reads it.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_that_reads_a_reward_changes_identity_with_the_reward_expression() {
    let claim_id = |reward: &str| {
        probabilistic_nodes_of(&[reward, &reward_forms()[1].1])
            .into_iter()
            .find(|node| node["node_tag"] == "claim")
            .map(|node| node["resolved_id"].clone())
            .expect("the claim node")
    };
    assert_ne!(
        claim_id("reward cost on ConfigVersion::attemptUpdate = 1;"),
        claim_id("reward cost on ConfigVersion::attemptUpdate = 2;"),
    );
}

/// FSpec FR-640-AC-8: a claim `under every scheduler` binds its fairness set,
/// one member per constraint with its kind, granularity and operation, so
/// `fair weak X` and `fair weak each X` are two members and a changed
/// constraint changes the claim's path.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-8")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_binds_its_fairness_set() {
    let operation = "Config::ConfigVersion::attemptUpdate";
    let fair = |constraints: &str| {
        let text = format!(
            "probabilistic Fair using ev on Config under every scheduler {constraints} \
             on origin {HOLDS}"
        );
        let wire = emit_wire(&claims_package(&[&text]));
        claim_named(&wire, "Fair").clone()
    };
    let weak = fair(&format!("fair weak {operation}"));
    let members = binding(binding(&weak["body"], "scheduler"), "fairness");
    assert_eq!(members["members"].as_array().map(Vec::len), Some(1));
    let path = weak["node_id"]["path"].to_string();
    assert!(
        path.contains("\"every\",\"weak\",\"whole\""),
        "whole when none is written: {path}"
    );
    for other in [
        format!("fair strong {operation}"),
        format!("fair weak each {operation}"),
    ] {
        assert_ne!(weak["node_id"], fair(&other)["node_id"], "{other}");
    }
    let both = fair(&format!("fair weak {operation} fair weak each {operation}"));
    let both_members = binding(binding(&both["body"], "scheduler"), "fairness");
    assert_eq!(both_members["members"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        both["node_id"],
        fair(&format!("fair weak each {operation} fair weak {operation}"))["node_id"],
        "the order constraints are written in changes nothing"
    );
}

/// FSpec FR-640 Reader rule 6: a claim body that is not the nine bindings, or
/// whose `distributions` is not the subject's, is refused `invalid_package`.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-9")]
#[test]
fn the_reader_refuses_a_malformed_claim_body() {
    let package = claims_package(&[STEADY, &claim_under("A", "Steady", HOLDS)]);
    let dropped = refusal_summary(read_with_body_edit(&package, &|body| {
        body["members"].as_array_mut().unwrap().pop();
    }));
    assert_eq!(
        dropped,
        (
            "InvalidPackage".into(),
            "Some(InvalidValue)".into(),
            "/semantic_graph/nodes/1/body".into()
        )
    );
    let swapped = refusal_summary(read_with_body_edit(&package, &|body| {
        body["members"].as_array_mut().unwrap().swap(2, 3);
    }));
    assert_eq!(
        swapped,
        (
            "InvalidPackage".into(),
            "Some(InvalidValue)".into(),
            "/semantic_graph/nodes/1/body".into()
        )
    );
}

/// TC-1563 (FSpec FR-640-AC-4): a `workload` node whose weights name an
/// operation the model does not declare is refused by the strict reader.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-4")]
#[test]
fn the_reader_refuses_a_workload_that_misnames_an_operation() {
    let package = workload_package("weight ConfigVersion::attemptUpdate = 1;");
    let emission = emit_checked(&package).expect("a probabilistic package emits");
    let mut wire = wire(&emission);
    let workload = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["node_tag"] == "probabilistic" && node["semantic_form"] == "workload")
        .expect("a workload node");
    let body = workload["body"]
        .to_string()
        .replacen("attemptUpdate", "noSuchOperation", 1);
    workload["body"] = serde_json::from_str(&body).unwrap();
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let read = read_v2(&jcs(&wire), library(), V2ReadLimits::fixture(), &evidence);
    assert!(matches!(read, Read::Refused(_)), "{read:?}");
}

/// The ConfigVersion unit with `time Config dense unit ms` and the delay
/// declaration `delay`, over the domain package that declares `ms`; with the
/// document.
pub(super) fn delay_package(delay: &str) -> (Vec<u8>, CheckedPackage) {
    let document = super::timed_source::package_units_document();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         model Config = {:?};\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         time Config dense unit ms;\n\
         {delay}\n",
        config_version_unit::PACKAGE_IDENTITY,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the delay checks: {refusals:?}"));
    (document, CheckedPackage::link(graph))
}

const DELAYS: [(&str, &str); 4] = [
    ("uniform", "uniform"),
    ("uniform_interval", "uniform[1 ms, 4 ms]"),
    ("exponential", "exponential(rational(1, 2))"),
    ("discrete", "discrete { 1 ms: 90, 3 ms: 10 }"),
];

/// TC-2062 (FR-740-AC-3), FSpec FR-640-AC-2: each family of a checked delay
/// is emitted as one `probabilistic`/`delay_distribution` node with its
/// family, parameters and one `declaration` occurrence at the declaration's
/// label span; the strict reader admits the package and the typed read gives
/// the declaring type, the operation and the family.
#[trace("TC-2062", "FR-740-AC-3")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-2")]
#[test]
fn delay_round_trips() {
    for (family, written) in DELAYS {
        let (document, package) =
            delay_package(&format!("delay ConfigVersion::attemptUpdate ~ {written};"));
        let [checked] = package.graph().probabilistic_model().delays() else {
            panic!("one checked delay");
        };
        let (wire, read) = emit_and_read(&package, &document);
        let (admitted, source_map) = admitted_of(read);
        let [node] = super::owners::node_where(&wire, "probabilistic", "delay_distribution")[..]
        else {
            panic!("{family}: one delay_distribution node");
        };
        assert_eq!(
            node["occurrences"],
            json!([{"role": "declaration", "ordinal": 0}])
        );
        let body = node["body"].to_string();
        assert!(body.contains("attemptUpdate"), "{body}");
        assert!(body.contains(&format!("\"{family}\"")), "{body}");
        // FR-640-AC-2's values: each time is a quantity in the node of the unit
        // `ms`, each weight and the rate an exact rational.
        let expected: &[(&str, &str)] = match family {
            "uniform" => &[],
            "uniform_interval" => &[("quantity", "1"), ("quantity", "4")],
            "exponential" => &[("rational", "1/2")],
            _ => &[
                ("quantity", "1"),
                ("rational", "90"),
                ("quantity", "3"),
                ("rational", "10"),
            ],
        };
        let mut found = Vec::new();
        parameter_literals(&node["body"]["members"][3]["value"], &mut found);
        let kinds: Vec<(&str, &str)> = found
            .iter()
            .map(|(kind, value, _)| (kind.as_str(), value.as_str()))
            .collect();
        assert_eq!(kinds, expected, "{family}");
        let unit = package.graph().time_sources()[0].unit_node.clone();
        let unit = serde_json::to_value(fsl_semantics::check::wire_id::structural_id(&unit))
            .expect("the unit has a wire id");
        for (kind, _, type_id) in &found {
            if kind == "quantity" {
                assert_eq!(*type_id, unit, "{family}: a time is typed by the unit ms");
            }
        }
        let id: CheckedNodeId = serde_json::from_value(node["node_id"].clone()).expect("an id");
        let region = package
            .graph()
            .region(&checked.declaration_location())
            .expect("the declaration has a region");
        assert_eq!(
            declaration_region(&source_map, &id),
            (region.start(), region.end())
        );
        let bare = crate::BarePackage::new(&admitted);
        let read = bare.probabilistic().expect("the nodes read");
        let [delay] = &read.delays[..] else {
            panic!("{family}: one delay read");
        };
        assert_eq!(delay.operation, checked.operation().name());
    }
}

/// Each literal term under `term`, in order, as its kind, its value and its
/// type.
fn parameter_literals(term: &Value, found: &mut Vec<(String, String, Value)>) {
    if let Some(kind) = term.get("value_kind") {
        found.push((
            kind.as_str().expect("a kind").to_owned(),
            term["value"].as_str().expect("a spelled value").to_owned(),
            term["type"].clone(),
        ));
    }
    for member in term
        .get("members")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        parameter_literals(member, found);
    }
    if let Some(value) = term.get("value").filter(|value| value.is_object()) {
        parameter_literals(value, found);
    }
}

/// FSL FR-740: each delay family reads back as S3's record, with the times
/// and weights exact and the discrete delays in declared order.
#[trace("TC-2062", "FR-740-AC-3")]
#[test]
fn a_delay_reads_back_as_the_s3_record() {
    for (_, written) in DELAYS {
        let (document, package) =
            delay_package(&format!("delay ConfigVersion::attemptUpdate ~ {written};"));
        let unit = package.graph().time_sources()[0].unit_node.clone();
        let units = BTreeMap::from([("ms".to_owned(), filament_exact::UnitId::declared(unit))]);
        assert_reads_back_equal(&package, &document, &units);
    }
}

/// FSpec FR-640-AC-3: a changed delay parameter changes the node identity.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn changing_a_delay_parameter_changes_the_node_identity() {
    let delay_id = |written: &str| {
        let (document, package) =
            delay_package(&format!("delay ConfigVersion::attemptUpdate ~ {written};"));
        let (wire, _) = emit_and_read(&package, &document);
        let [node] = super::owners::node_where(&wire, "probabilistic", "delay_distribution")[..]
        else {
            panic!("one delay_distribution node");
        };
        node["node_id"].clone()
    };
    let base = delay_id("discrete { 3 ms: 1, 1 ms: 9 }");
    assert_ne!(base, delay_id("discrete { 3 ms: 1, 1 ms: 8 }"));
    assert_ne!(base, delay_id("discrete { 3 ms: 1, 2 ms: 9 }"));
    assert_ne!(
        delay_id("exponential(rational(1, 2))"),
        delay_id("exponential(2)")
    );
}

/// FSL FR-740, FR-253: a read-back delay that breaks FR-253 (`uniform_interval`
/// with the lower end above the upper, a negative or repeated `discrete` delay,
/// a rate or weight that is not positive) is refused by the join, as S3 would
/// have refused the declaration.
#[trace("TC-708", "FR-253-AC-3")]
#[test]
fn a_package_delay_that_breaks_fr_253_is_refused_on_read_back() {
    use fsl_semantics::check::{DelayFault, PackageDelayFamily, PackageProbabilisticRefusal};
    let rational = |numerator: i64, denominator: i64| {
        filament_exact::Rational::new(numerator.into(), denominator.into()).expect("a rational")
    };
    let (document, package) =
        delay_package("delay ConfigVersion::attemptUpdate ~ uniform[1 ms, 4 ms];");
    let unit = package.graph().time_sources()[0].unit_node.clone();
    let units = BTreeMap::from([("ms".to_owned(), filament_exact::UnitId::declared(unit))]);
    for (family, fault) in [
        (
            PackageDelayFamily::UniformInterval {
                lower: rational(4, 1),
                upper: rational(1, 1),
            },
            DelayFault::UniformRange,
        ),
        (
            PackageDelayFamily::UniformInterval {
                lower: rational(-1, 1),
                upper: rational(1, 1),
            },
            DelayFault::UniformRange,
        ),
        (
            PackageDelayFamily::Exponential {
                rate: rational(0, 1),
            },
            DelayFault::NonPositiveRate,
        ),
        (
            PackageDelayFamily::Discrete(vec![(rational(-1, 1), rational(1, 1))]),
            DelayFault::NegativeDelay,
        ),
        (
            PackageDelayFamily::Discrete(vec![
                (rational(1, 1), rational(1, 1)),
                (rational(1, 1), rational(2, 1)),
            ]),
            DelayFault::RepeatedDelay,
        ),
        (
            PackageDelayFamily::Discrete(vec![(rational(1, 1), rational(0, 1))]),
            DelayFault::NonPositiveWeight,
        ),
    ] {
        let refusal = try_read_back(&package, &document, &units, |nodes| {
            nodes.delays[0].family = family.clone();
        })
        .expect_err("the delay is refused");
        assert_eq!(
            refusal,
            PackageProbabilisticRefusal::InvalidDelay {
                operation: "attemptUpdate".to_owned(),
                fault,
            },
            "{family:?}"
        );
    }
}

/// FR-230: a timed model over a domain package that declares two object types
/// has no single timed root, so S3 refuses the time source at its span before
/// any delay is checked.
#[trace("TC-685", "FR-230-AC-38")]
#[test]
fn a_delay_beside_a_second_object_type_is_refused_at_the_time_source() {
    use super::abstraction::fixture;
    let document = crate::emit::tests::wire_bytes(&fixture::lineage_document())
        .expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         model M = {package:?};\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         delay Base::op ~ uniform;\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        );
    let Err(refusals) = graph else {
        panic!("a timed model over two object types checks");
    };
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let fsl_semantics::check::CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a temporal cause: {refusal:?}");
    };
    let fsl_semantics::check::TemporalCause::AmbiguousTimedModel { span, .. } = &**cause else {
        panic!("an ambiguous timed model: {cause:?}");
    };
    assert!(span.start < span.end, "the refusal carries its span");
}

/// FR-640 Reader rule 5 as `delay_node` joins it: the unit of a delay's
/// times is the dense source of the timed root whose model is the delay's
/// declaring node, and no other node has one.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-2")]
#[test]
fn the_delay_unit_is_that_of_the_dense_source_of_the_declaring_nodes_root() {
    let (_, package) = delay_package("delay ConfigVersion::attemptUpdate ~ uniform;");
    emit_checked(&package).expect("the package emits");
    let nodes = crate::emit::TIMED_GRAPH
        .with(|held| held.borrow().clone())
        .expect("the timed nodes were written")
        .0;
    let [delay] = package.graph().probabilistic_model().delays() else {
        panic!("one checked delay");
    };
    let declaring = fsl_semantics::check::wire_id::structural_id(
        &fsl_semantics::model::key::object_type_node_key(
            delay.operation().declaring_key().unwrap(),
        )
        .expect("the declaring type has a node key"),
    );
    let unit = super::super::probabilistic::dense_unit(&nodes, &declaring)
        .expect("the root of the declaring node has a dense source");
    let source = package.graph().time_sources()[0].unit_node.clone();
    assert_eq!(
        unit,
        fsl_semantics::check::wire_id::structural_id(&source),
        "the unit is the source's"
    );
    assert_eq!(
        super::super::probabilistic::dense_unit(&nodes, &unit),
        None,
        "a node that roots no timed model has no unit"
    );
}

/// The abstraction fixture's unit with the random parameter `attemptUpdate(n)`
/// over `{ 3: 1, 1: 3 }` and `declarations` after it.
fn reward_over_parameter(declarations: &str) -> (Vec<u8>, CheckedPackage) {
    use super::abstraction::fixture;
    let document =
        crate::emit::tests::wire_bytes(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         random ConfigVersion::attemptUpdate(n) ~ {{ 3: 1, 1: 3 }};\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (document, CheckedPackage::link(graph))
}

/// The value of the body binding `name` of the aggregate `term`.
fn bound<'t>(term: &'t Value, name: &str) -> &'t Value {
    term["members"]
        .as_array()
        .and_then(|members| members.iter().find(|member| member["name"] == name))
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("binding `{name}` in {term}"))
}

/// The node of `wire` with `id`.
fn node_by_id<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    super::nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("a node with id {id}"))
}

/// TC-2062 (FR-740-AC-1, FR-185-AC-7), FSpec FR-640-AC-1: `reward cost on
/// ConfigVersion::attemptUpdate = n + 1` is emitted as a `reward` node whose
/// bindings are `declaring`, `operation`, `name`, `value_type` and
/// `expression`, in that order, and whose `expression` is a clause function
/// owned by the operation member with the operation's parameter `n`, the
/// random one included, and the clause kind `reward` (FSpec FR-322).
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("TC-2062", "FR-185-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-322-AC-471")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn a_reward_is_emitted_with_its_clause_function() {
    let (document, package) =
        reward_over_parameter("reward cost on ConfigVersion::attemptUpdate = n + 1;");
    let (wire, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let [reward] = super::owners::node_where(&wire, "probabilistic", "reward")[..] else {
        panic!("one reward node");
    };
    let names: Vec<&str> = reward["body"]["members"]
        .as_array()
        .expect("the body is an aggregate")
        .iter()
        .map(|member| member["name"].as_str().expect("a binding"))
        .collect();
    assert_eq!(
        names,
        ["declaring", "operation", "name", "value_type", "expression"]
    );
    let body = &reward["body"];
    assert_eq!(bound(body, "operation")["value"], "attemptUpdate");
    assert_eq!(bound(body, "name")["value"], "cost");
    let declaring = &bound(body, "declaring")["target"];
    assert_eq!(node_by_id(&wire, declaring)["semantic_form"], "object_type");

    let function = node_by_id(&wire, &bound(body, "expression")["target"]);
    assert_eq!(function["node_tag"], "function");
    assert_eq!(function["owner"]["kind"], "model");
    assert_eq!(
        function["semantic_type"],
        bound(body, "value_type")["target"],
        "the function is typed by the reward's value type"
    );
    assert_eq!(bound(&function["body"], "clause")["value"], "reward");
    let parameters = bound(&function["body"], "parameters")["members"]
        .as_array()
        .expect("the parameters are an aggregate");
    let read: Vec<(&Value, &Value)> = parameters
        .iter()
        .map(|parameter| {
            let parameter = node_by_id(&wire, &parameter["target"]);
            (
                &bound(&parameter["body"], "name")["value"],
                &bound(&parameter["body"], "level")["value"],
            )
        })
        .collect();
    assert_eq!(
        read,
        [(&json!("n"), &json!("0")), (&json!("m"), &json!("1"))],
        "every parameter of the operation in declared order"
    );
    assert_eq!(
        reward["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
}

/// TC-2062 (FR-740-AC-1), FSpec FR-640-AC-1: the typed read of the emitted
/// package gives the reward `cost` of `attemptUpdate` with its value type, its
/// function, and the declaration's region.
#[trace("TC-2062", "FR-740-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[test]
fn reward_round_trips() {
    let (document, package) =
        reward_over_parameter("reward cost on ConfigVersion::attemptUpdate = n + 1;");
    let [checked] = package.graph().probabilistic_model().rewards() else {
        panic!("one checked reward");
    };
    let (wire, read) = emit_and_read(&package, &document);
    let (admitted, source_map) = admitted_of(read);
    let declaring = fsl_semantics::check::wire_id::structural_id(
        &fsl_semantics::model::key::object_type_node_key(
            checked.operation().declaring_key().unwrap(),
        )
        .expect("the declaring type has a node key"),
    );
    let reads = admitted.rewards(&declaring, checked.operation().name());
    let [read] = &reads[..] else {
        panic!("one reward read: {reads:?}");
    };
    assert_eq!(read.name, checked.name());
    let [reward] = super::owners::node_where(&wire, "probabilistic", "reward")[..] else {
        panic!("one reward node");
    };
    assert_eq!(
        serde_json::to_value(read.function).unwrap(),
        bound(&reward["body"], "expression")["target"]
    );
    let function = node_by_id(&wire, &serde_json::to_value(read.function).unwrap());
    assert_eq!(
        serde_json::to_value(read.value_type).unwrap(),
        function["semantic_type"]
    );
    let region = package
        .graph()
        .region(&checked.declaration_location())
        .expect("the declaration has a region");
    assert_eq!(
        declaration_region(&source_map, read.node),
        (region.start(), region.end())
    );
}

/// FSL FR-740: a reward reads back from the emitted package as S3's record:
/// the same operation, name and parameters, with the function node S3 lowered
/// the expression to and the reward's value type node.
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_reward_reads_back_as_the_s3_record() {
    let (document, package) =
        reward_over_parameter("reward cost on ConfigVersion::attemptUpdate = n + 1;");
    let checked = package.graph().probabilistic_model();
    let read = read_back(&package, &document, &BTreeMap::new());
    let [expected] = checked.rewards() else {
        panic!("one checked reward");
    };
    let [found] = read.read_rewards() else {
        panic!("one reward read back");
    };
    assert_eq!(found.name, expected.name());
    assert_eq!(
        operation_content(&found.operation),
        operation_content(expected.operation())
    );
    assert_eq!(found.parameters, expected.parameters());
    assert_eq!(Some(&found.function), expected.function());
    let function = package
        .graph()
        .semantic_graph()
        .node(found.function.clone())
        .expect("the function node is in the graph");
    assert_eq!(
        function.semantic_type().as_ref(),
        Some(&found.value_type),
        "the value type is the function's type node"
    );
}

/// FSpec FR-640 Reader rule 4, FSL FR-740: a package whose rewards repeat a
/// name on one operation, or name an operation the object type does not
/// declare, is refused on read-back.
#[test]
fn a_package_reward_that_breaks_the_reader_rules_is_refused_on_read_back() {
    use fsl_semantics::check::PackageProbabilisticRefusal as Refusal;
    let (document, package) =
        reward_over_parameter("reward cost on ConfigVersion::attemptUpdate = n + 1;");
    let refused = |edit: fn(&mut fsl_semantics::check::PackageProbabilistic)| {
        try_read_back(&package, &document, &BTreeMap::new(), edit)
            .expect_err("the edited rewards are refused")
    };
    assert_eq!(
        refused(|nodes| {
            let again = nodes.rewards[0].clone();
            nodes.rewards.push(again);
        }),
        Refusal::DuplicateReward {
            operation: "attemptUpdate".into(),
            reward: "cost".into(),
        }
    );
    assert_eq!(
        refused(|nodes| nodes.rewards[0].operation = "missing".into()),
        Refusal::UnknownOperation("missing".into())
    );
}

/// FSpec FR-640-AC-3: writing the rewards in another order emits the same
/// nodes in the same order, and changing the expression changes the reward's
/// identity. The function's own key holds no content (it ranks equal paths by
/// lowering order), so the function's id is not asserted to change.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn reward_order_changes_no_node_and_the_expression_changes_the_identity() {
    let nodes_of = |declarations: &str| {
        let (document, package) = reward_over_parameter(declarations);
        let (wire, read) = emit_and_read(&package, &document);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        super::nodes(&wire)
            .iter()
            .filter(|node| {
                node["node_tag"] == "probabilistic"
                    || (node["node_tag"] == "function" && node["owner"]["kind"] == "model")
            })
            .cloned()
            .map(|mut node| {
                node["resolved_id"] = resolved_id(&wire, &node["node_id"], 8);
                node
            })
            .collect::<Vec<Value>>()
    };
    let first = "reward cost on ConfigVersion::attemptUpdate = n + 1;";
    let second = "reward load on ConfigVersion::attemptUpdate = n;";
    let forward = nodes_of(&format!("{first}\n{second}"));
    let reverse = nodes_of(&format!("{second}\n{first}"));
    assert_eq!(
        forward.len(),
        5,
        "random parameter, two rewards, two functions"
    );
    let strip = |nodes: &[Value]| -> Vec<Value> {
        nodes
            .iter()
            .map(|node| {
                let mut node = node.clone();
                node["occurrences"] = Value::Null;
                node
            })
            .collect()
    };
    let ids = |nodes: &[Value]| -> Vec<Value> {
        nodes.iter().map(|node| node["node_id"].clone()).collect()
    };
    assert_eq!(ids(&forward), ids(&reverse), "same nodes, same order");
    for (forward, reverse) in strip(&forward).iter().zip(strip(&reverse)) {
        assert_eq!(forward["node_id"], reverse["node_id"]);
        assert_eq!(forward["body"], reverse["body"], "{}", forward["node_id"]);
        assert_eq!(forward["owner"], reverse["owner"]);
    }

    let identity = |declaration: &str| {
        let nodes = nodes_of(declaration);
        let [reward] = nodes
            .iter()
            .filter(|node| node["semantic_form"] == "reward")
            .collect::<Vec<_>>()[..]
        else {
            panic!("one reward");
        };
        reward["resolved_id"].clone()
    };
    let base = identity(first);
    assert_ne!(
        base,
        identity("reward cost on ConfigVersion::attemptUpdate = n + 2;"),
        "another expression is another reward"
    );
    assert_eq!(
        base,
        identity(first),
        "an unchanged declaration keeps its identity"
    );
}

/// `id` with each index segment `{"node": i}` of its path replaced by the
/// resolved id of the node at position `i` of `wire`'s graph, `depth` levels
/// deep: an index names a position, so two ids are the same identity only when
/// their resolved forms are (FSpec FR-322 "Operand index segments").
pub(super) fn resolved_id(wire: &Value, id: &Value, depth: usize) -> Value {
    let mut id = id.clone();
    if depth == 0 {
        return id;
    }
    let nodes = super::nodes(wire);
    if let Some(path) = id["path"].as_array_mut() {
        for segment in path {
            if let Some(at) = segment["node"].as_u64() {
                let target = &nodes[usize::try_from(at).expect("a small index")]["node_id"];
                *segment = resolved_id(wire, target, depth - 1);
            }
        }
    }
    id
}

/// The emitted `probabilistic` nodes and model-owned functions of the
/// ConfigVersion unit with `declarations` (claims use the profile `ev`), as
/// wire JSON with occurrences removed.
fn probabilistic_nodes_of(declarations: &[&str]) -> Vec<Value> {
    let package = claims_package(declarations);
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (wire, read) = emit_and_read(&package, document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    super::nodes(&wire)
        .iter()
        .filter(|node| {
            node["node_tag"] == "probabilistic"
                || node["node_tag"] == "claim"
                || (node["node_tag"] == "function" && node["owner"]["kind"] == "model")
        })
        .cloned()
        .map(|mut node| {
            node["occurrences"] = Value::Null;
            node["resolved_id"] = resolved_id(&wire, &node["node_id"], 8);
            node
        })
        .collect()
}

/// FSL FR-740 Behavior, FSpec FR-640-AC-3: writing the rewards in any order
/// relative to a workload and a claim, as well as to each other, emits the
/// same nodes, ids and bodies in the same order; changing one reward's
/// expression changes that reward's identity and no other node's.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn rewards_keep_their_nodes_whatever_unrelated_declarations_surround_them() {
    let cost = "reward cost on ConfigVersion::attemptUpdate = 1;";
    let load = "reward load on ConfigVersion::attemptUpdate = 7;";
    let workload = "workload Steady on Config { weight ConfigVersion::attemptUpdate = 1; }";
    let stable = claim("Stable", "0.5");
    let base = probabilistic_nodes_of(&[cost, load, workload, &stable]);
    assert_eq!(
        base.iter()
            .filter(|node| node["node_tag"] == "probabilistic")
            .count(),
        3,
        "two rewards and a workload"
    );
    for order in [
        [&stable[..], workload, load, cost],
        [load, &stable, cost, workload],
        [workload, load, &stable, cost],
    ] {
        assert_eq!(
            probabilistic_nodes_of(&order),
            base,
            "the declaration order {order:?} changes no node"
        );
    }
    let changed = probabilistic_nodes_of(&[
        "reward cost on ConfigVersion::attemptUpdate = 2;",
        load,
        workload,
        &stable,
    ]);
    let mut differing: Vec<&str> = base
        .iter()
        .filter(|node| !changed.contains(node))
        .map(|node| node["semantic_form"].as_str().unwrap_or_default())
        .collect();
    differing.sort_unstable();
    assert_eq!(base.len(), changed.len());
    assert_eq!(
        differing,
        ["pure_function", "reward"],
        "only the changed reward and the function that computes it differ"
    );
}

/// A unit with source-declared objects and `declarations`, no model package.
fn source_object_reward(declarations: &str) -> CheckedPackage {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         object O {{ n: Integer; operation step(x: Integer) modifies self.n; }}\n\
         {declarations}\n"
    );
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the package input is empty");
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("S3 accepts the reward: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// TC-2062 (FR-740-AC-1), FSpec FR-322: a reward on a source-declared object
/// is emitted as a `reward` node whose clause function is a `SourceOwner`
/// declaration `[O, step, reward, cost]` at ordinal 0. The test does not read the
/// package back: emission follows spec W76 and the strict reader (contract-ir
/// dd390f51) is behind it, refusing a source-declared `declaring` loudly
/// (QSL-2592).
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
fn a_reward_on_a_source_object_is_emitted() {
    let package = source_object_reward("reward cost on O::step = x;");
    let emission = emit_checked(&package).expect("the reward emits");
    let wire = wire(&emission);
    assert_eq!(
        super::owners::node_where(&wire, "probabilistic", "reward").len(),
        1
    );
}

/// The S4 refusal of the unit `source` over the domain `documents`.
fn refusal_of(
    source: &str,
    documents: &[&[u8]],
) -> (
    EmitRefusal,
    fsl_foundation::source::provenance::RawSourceRef,
) {
    let packages = fsl_semantics::model::intake::package_input(documents.iter().copied())
        .expect("the package input is valid");
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("S3 accepts the unit: {refusals:?}"));
    let unit = graph.source().clone();
    (
        emit_checked(&CheckedPackage::link(graph)).expect_err("emission refuses"),
        unit,
    )
}

/// TC-2062 (FR-185-AC-12): the S4 refusal of a random parameter and a
/// workload is located at the declaration (its label span in
/// the unit's text) and names the declaration in words, with no debug
/// rendering of a span or source reference.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn probabilistic_refusals_have_locus() {
    use super::abstraction::fixture as abstraction;
    use fsl_semantics::check::ProbabilisticFeature;
    let head = "language \"ix:native\" edition \"1-draft\";\n";
    let abstraction_document =
        crate::emit::tests::wire_bytes(&abstraction::document()).expect("serializes");
    // A random parameter over a `String`, which has no support literal kind,
    // is the refusal a random parameter still gets.
    let mut string_model = abstraction::document();
    string_model["types"]
        .as_array_mut()
        .expect("the types")
        .iter_mut()
        .find(|declaration| declaration["identity"] == abstraction::id("ConfigVersion"))
        .map(|declaration| {
            declaration["operations"][0]["params"][0]["typeRef"] =
                json!("ix://filament/native/String");
        })
        .expect("the first parameter of attemptUpdate");
    let string_document = crate::emit::tests::wire_bytes(&string_model).expect("serializes");
    let cases: [(String, Vec<u8>, ProbabilisticFeature, &str); 2] = [
        (
            format!(
                "{head}profile v = \"filament.model.complete/v1\";\nmodel M = {:?};\n\
                 random ConfigVersion::attemptUpdate(n) ~ {{ \"a\": 1, \"b\": 1 }};\n",
                abstraction::PACKAGE
            ),
            string_document,
            ProbabilisticFeature::RandomParameter,
            "attemptUpdate(n)",
        ),
        (
            format!(
                "{head}profile v = \"filament.model.complete/v1\";\nmodel M = {:?};\n\
                 workload Both on M {{ weight Base::op = 1; weight ConfigVersion::attemptUpdate = 1; }}\n",
                abstraction::PACKAGE
            ),
            abstraction_document,
            ProbabilisticFeature::Workload,
            "Both",
        ),
    ];
    for (source, document, expected, name) in cases {
        let documents: Vec<&[u8]> = if document.is_empty() {
            Vec::new()
        } else {
            vec![document.as_slice()]
        };
        let (refusal, unit) = refusal_of(&source, &documents);
        let EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature,
            region,
            ..
        } = &refusal
        else {
            panic!("{name}: {refusal:?}");
        };
        assert_eq!((*feature, declaration.as_str()), (expected, name));
        assert_eq!(
            region.source(),
            &unit,
            "{name}: the region is in the unit's source"
        );
        let label = source
            .get(region.start() as usize..region.end() as usize)
            .unwrap_or_else(|| panic!("{name}: the region lies in the unit: {refusal:?}"));
        let word = name.split('(').next().unwrap_or(name);
        assert!(label.contains(word), "{name}: the region is `{label}`");
        let message = refusal.to_string();
        assert!(message.contains(word), "{message}");
        assert!(!message.contains("Span {"), "{message}");
        assert!(!message.contains("RawSourceRef"), "{message}");
    }
}

/// FSpec FR-322 "Claim declaration names": the `claim_names` rows are in the
/// package's node order, which resolves operand IDs, so a package with
/// several state clauses and a claim carries its rows in the order the
/// strict reader checks them.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn claim_names_rows_follow_the_graphs_node_order() {
    let package = claims_package(&[&claim("Stable", "0.5")]);
    let wire = wire(&emit_checked(&package).expect("the package emits"));
    let nodes: Vec<&Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the graph has nodes")
        .iter()
        .map(|node| &node["node_id"])
        .collect();
    let at: Vec<usize> = wire["claim_names"]
        .as_array()
        .expect("claim_names rows")
        .iter()
        .map(|row| {
            nodes
                .iter()
                .position(|id| **id == row["declaration"])
                .expect("a row names a graph node")
        })
        .collect();
    assert!(at.len() > 1, "several claim-bearing nodes: {at:?}");
    assert!(at.windows(2).all(|pair| pair[0] <= pair[1]), "{at:?}");
}

/// FSpec FR-322, TC-1563: a reward on a source-declared object is a clause
/// function whose owner is the source object's declaration `[O, step, reward,
/// cost]` at ordinal 0, with the clause kind `reward`. The strict reader does not
/// yet admit a source-declared `declaring` for a reward (QSL-2592), so the
/// admission and the rule-4 rename refusal of FR-640-AC-7's source half are not
/// asserted here.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-7")]
#[test]
fn a_source_declared_reward_is_a_source_owned_declaration_at_ordinal_zero() {
    let package = source_object_reward("reward cost on O::step = x;");
    let emission = emit_checked(&package).expect("the reward emits");
    let wire = wire(&emission);
    let [reward] = super::owners::node_where(&wire, "probabilistic", "reward")[..] else {
        panic!("one reward node");
    };
    let function = node_by_id(&wire, &bound(&reward["body"], "expression")["target"]);
    assert_eq!(bound(&function["body"], "clause")["value"], "reward");
    assert_eq!(
        function["declaration"]["qualified_name"],
        json!(["O", "step", "reward", "cost"])
    );
    assert_eq!(function["node_id"]["ordinal"], 0);
    assert_eq!(
        node_by_id(&wire, &bound(&reward["body"], "declaring")["target"])["semantic_form"],
        "object_type"
    );
}

/// FSpec FR-640-AC-7, FR-322-AC-471: the strict reader admits the clause kind
/// `reward` for a reward's function. The refusals of any other kind or owner
/// are asserted, with code and locus, by
/// `operation_bodies::a_reward_function_with_another_clause_kind_or_owner_is_refused`.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-322-AC-471")]
#[test]
fn the_reader_admits_the_reward_clause_kind() {
    let (document, package) =
        reward_over_parameter("reward cost on ConfigVersion::attemptUpdate = n + 1;");
    let (_, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// `probabilistic name` over the workload `Steady` with `confidence` and
/// `body`.
fn claim_stated(name: &str, confidence: &str, body: &str) -> String {
    format!("probabilistic {name} using ev on Config under Steady on origin {confidence} {body}")
}

/// The path of the claim `name` after its `form` binding, each text segment
/// as written and each operand as `<node>`.
fn statement_spelling(wire: &Value, name: &str) -> Vec<String> {
    let path = claim_named(wire, name)["node_id"]["path"]
        .as_array()
        .expect("a path")
        .clone();
    let at = path
        .iter()
        .position(|segment| segment == "form")
        .expect("the path names the form");
    path[at..]
        .iter()
        .map(|segment| {
            segment
                .as_str()
                .map_or_else(|| "<node>".to_owned(), str::to_owned)
        })
        .collect()
}

/// `wire` of `package` with the claim body's binding `name` replaced by
/// `edit`, read by the strict reader.
fn read_with_body_edit(package: &CheckedPackage, edit: &dyn Fn(&mut Value)) -> Read {
    read_document_with_body_edit(
        package,
        config_version_unit::DOMAIN_PACKAGE.as_bytes(),
        edit,
    )
}

/// [`read_with_body_edit`] over the domain package `document`.
fn read_document_with_body_edit(
    package: &CheckedPackage,
    document: &[u8],
    edit: &dyn Fn(&mut Value),
) -> Read {
    let emission = emit_checked(package).expect("the package emits");
    let mut wire = wire(&emission);
    let claim = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["semantic_form"] == "probabilistic_claim")
        .expect("a claim node");
    edit(&mut claim["body"]);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    read_v2(&jcs(&wire), library(), V2ReadLimits::fixture(), &evidence)
}

/// TC-1288 (FR-498-AC-5, FR-498-AC-7), FSpec FR-640-AC-9: a claim's body
/// holds the nine bindings, and its path spells `form`, `over`, `activation`
/// and `confidence` after the nodes it reads, each exact value as its
/// `value_kind` and canonical text, the type of an integer or rational left
/// out.
#[trace("TC-1288", "FR-498-AC-5")]
#[trace("TC-1288", "FR-498-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-10")]
#[test]
fn a_claim_states_its_form_over_activation_and_confidence_in_body_and_path() {
    let wire = emit_wire(&claims_package(&[
        STEADY,
        &claim_stated(
            "S",
            "with alpha 0.01, beta 0.02, indifference 0.03",
            "{ probability >= 0.5 [ steps from holds(true) until holds(true) within 10 >= 3 ] }",
        ),
    ]));
    let claim = claim_named(&wire, "S");
    let names: Vec<&str> = claim["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "subject",
            "scheduler",
            "reward",
            "distributions",
            "delays",
            "form",
            "over",
            "activation",
            "confidence"
        ]
    );
    let spelled = statement_spelling(&wire, "S");
    let expected: Vec<&str> = "form kind probability event kind compare measure kind steps \
        from <node> until <node> within integer 10 bound direction at_least value integer 3 \
        bound direction at_least value rational 1/2 over activation kind origin confidence \
        alpha rational 1/100 beta rational 1/50 indifference rational 3/100"
        .split(' ')
        .collect();
    assert_eq!(spelled, expected);
}

/// TC-1288 (FR-498-AC-7), FSpec FR-640-AC-3: a claim's identity moves with
/// each thing it states and with nothing about how it is written: another
/// threshold, confidence, bound direction or activation is another claim,
/// while `0.5` and `0.50` and the order of the confidence items change no
/// segment of the path.
#[trace("TC-1288", "FR-498-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_path_moves_with_what_it_states_and_not_with_how_it_is_written() {
    let spelling_of = |confidence: &str, body: &str| {
        let wire = emit_wire(&claims_package(&[
            STEADY,
            &claim_stated("C", confidence, body),
        ]));
        resolved_id(&wire, &claim_named(&wire, "C")["node_id"], 8)
    };
    let confidence = "with alpha 0.01, beta 0.01, indifference 0.01";
    let base = spelling_of(
        confidence,
        "{ probability >= 0.5 [ always[0,2] holds(true) ] }",
    );
    assert_eq!(
        base,
        spelling_of(
            confidence,
            "{ probability >= 0.50 [ always[0,2] holds(true) ] }"
        )
    );
    assert_eq!(
        base,
        spelling_of(
            "with indifference 0.01, beta 0.01, alpha 0.01",
            "{ probability >= 0.5 [ always[0,2] holds(true) ] }"
        )
    );
    for (other_confidence, other) in [
        (
            confidence,
            "{ probability >= 0.25 [ always[0,2] holds(true) ] }",
        ),
        (
            confidence,
            "{ probability <= 0.5 [ always[0,2] holds(true) ] }",
        ),
        (
            confidence,
            "{ probability >= 0.5 [ always[0,3] holds(true) ] }",
        ),
        (
            confidence,
            "{ probability >= 0.5 [ always[1,2] holds(true) ] }",
        ),
        (
            confidence,
            "{ probability >= 0.5 [ eventually[0,2] holds(true) ] }",
        ),
        (
            "with alpha 0.02, beta 0.01, indifference 0.01",
            "{ probability >= 0.5 [ always[0,2] holds(true) ] }",
        ),
    ] {
        assert_ne!(base, spelling_of(other_confidence, other), "{other}");
    }
}

/// TC-1288 (FR-498-AC-7), FSpec FR-640-AC-9: an `on each` claim states
/// `activation` as `kind each`, its trigger parameter and its guard, a guard
/// left out the Boolean `true` node, and a claim over another activation is
/// another claim.
#[trace("TC-1288", "FR-498-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-10")]
#[test]
fn an_on_each_claim_states_its_trigger_and_guard() {
    let wire = emit_wire(&claims_package(&[
        &claim_each("E", "0.5"),
        &claim("O", "0.5"),
    ]));
    let each = statement_spelling(&wire, "E");
    let at = each
        .iter()
        .position(|segment| segment == "activation")
        .unwrap();
    assert_eq!(
        &each[at..at + 7],
        [
            "activation",
            "kind",
            "each",
            "trigger",
            "<node>",
            "guard",
            "<node>"
        ]
    );
    let activation = binding(&claim_named(&wire, "E")["body"], "activation");
    let nodes = super::nodes(&wire);
    let node_at = |term: &Value| {
        let id = &term["target"];
        nodes
            .iter()
            .find(|node| node["node_id"] == *id)
            .unwrap_or_else(|| panic!("a node with the id {id}"))
    };
    assert!(activation.is_object(), "{activation}");
    let trigger = node_at(binding(activation, "trigger"));
    assert_eq!(trigger["semantic_form"], "parameter");
    let guard = node_at(binding(activation, "guard"));
    assert_eq!(guard["node_tag"], "value");
    assert_eq!(guard["semantic_form"], "literal");
    assert_ne!(
        resolved_id(&wire, &claim_named(&wire, "E")["node_id"], 8),
        resolved_id(&wire, &claim_named(&wire, "O")["node_id"], 8)
    );
}

/// FSpec FR-640 Reader rule 7 over bindings 6 to 9: the strict reader refuses
/// a claim whose probability is out of range, whose exact value has another
/// type than its position names, whose `over` is no parameter and whose
/// confidence is incomplete, each at its exact code, cause and pointer.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-11")]
#[test]
fn the_reader_refuses_a_claim_statement_that_breaks_rule_7() {
    let package = claims_package(&[
        STEADY,
        &claim_stated(
            "S",
            "with alpha 0.01, beta 0.01, indifference 0.01",
            "{ probability >= 0.5 [ steps from holds(true) until holds(true) within 10 >= 3 ] }",
        ),
    ]);
    let verified = read_with_body_edit(&package, &|_| {});
    assert!(matches!(verified, Read::Verified { .. }), "{verified:?}");
    let text_edit = |from: &'static str, to: &'static str| {
        move |body: &mut Value| {
            let text = body.to_string().replacen(from, to, 1);
            *body = serde_json::from_str(&text).unwrap();
        }
    };
    let summary = |edit: &dyn Fn(&mut Value)| refusal_summary(read_with_body_edit(&package, edit));
    let expected =
        |code: &str, cause: &str, path: &str| (code.to_owned(), cause.to_owned(), path.to_owned());
    // 7.4: a probability outside (0, 1).
    assert_eq!(
        summary(&text_edit("\"1/2\"", "\"3/2\"")),
        expected(
            "InvalidPackage",
            "Some(InvalidValue)",
            "/semantic_graph/nodes/1/body/members/5/value/members/2/value/members/1/value"
        ),
        "a probability above one"
    );
    // 7.4: a confidence alpha outside (0, 1). A value of another kind or
    // type than its position is refused by FR-322's scalar-literal check
    // before rule 7, so rule 7.3 is reached only by a quantity (see the timed
    // statement test).
    assert_eq!(
        summary(&text_edit("\"1/100\"", "\"3/2\"")),
        expected(
            "InvalidPackage",
            "Some(InvalidValue)",
            "/semantic_graph/nodes/1/body/members/8/value/members/0/value"
        ),
        "an alpha above one"
    );
    // 7.5: an `over` that names no parameter.
    assert_eq!(
        summary(&|body| {
            let subject = body["members"][0]["value"].clone();
            body["members"][6]["value"] = subject;
        }),
        expected(
            "InvalidModelBinding",
            "Some(MalformedDeclaration)",
            "/semantic_graph/nodes/1/body/members/6/value/target"
        ),
        "an over that is not a parameter"
    );
    // 9: an unfinished confidence.
    assert_eq!(
        summary(&|body| {
            let members = body["members"].as_array_mut().unwrap();
            members.last_mut().unwrap()["value"]["members"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
        expected(
            "InvalidPackage",
            "Some(InvalidValue)",
            "/semantic_graph/nodes/1/body/members/8/value"
        ),
        "an unfinished confidence"
    );
    // An unknown form kind.
    assert_eq!(
        summary(&text_edit("\"probability\"", "\"certainty\"")),
        expected(
            "InvalidPackage",
            "Some(InvalidValue)",
            "/semantic_graph/nodes/1/body/members/5/value/members/0/value"
        ),
        "an unknown form"
    );
}

/// The code, cause and first locus pointer of a refused read.
fn refusal_summary(read: Read) -> (String, String, String) {
    let refusal = super::operation_bodies::refusal_of(read);
    (
        format!("{:?}", refusal.code),
        format!("{:?}", refusal.cause),
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|path| path.as_str().to_owned())
            .unwrap_or_default(),
    )
}

/// The ConfigVersion unit over a time source with `declarations` appended,
/// unchecked: the timed claims of FSpec FR-640 rule 7.3, under the model's
/// `time` declaration `time_clause`.
fn timed_claims_source_in(time_clause: &str, declarations: &[&str]) -> (Vec<u8>, Vec<u8>) {
    let document = super::timed_source::package_units_document();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         profile tm = \"filament.temporal.timed/v1\";\n\
         model Config = {:?};\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         {time_clause}\n\
         {STEADY}\n{}\n",
        config_version_unit::PACKAGE_IDENTITY,
        declarations.join("\n"),
    );
    (document, unit.into_bytes())
}

fn timed_claims_check(
    declarations: &[&str],
) -> (
    Vec<u8>,
    Result<CheckedPackage, Vec<fsl_semantics::check::CheckRefusal>>,
) {
    timed_claims_check_in("time Config dense unit ms;", declarations)
}

/// [`timed_claims_check`] over the model's `time` declaration `time_clause`.
fn timed_claims_check_in(
    time_clause: &str,
    declarations: &[&str],
) -> (
    Vec<u8>,
    Result<CheckedPackage, Vec<fsl_semantics::check::CheckRefusal>>,
) {
    let (document, unit) = timed_claims_source_in(time_clause, declarations);
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let checked = assemble_with_models(&unit, &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(CheckedPackage::link);
    (document, checked)
}

/// FSpec FR-640 rule 7.3 and FR-640-AC-10: over a timed subject the elapsed
/// and expected-elapsed forms and every window state a time quantity at the
/// model's unit node, and a quantity reward's bound is a quantity at the
/// reward's own unit node.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-10")]
#[test]
fn a_timed_claim_states_time_quantities_at_the_model_unit() {
    let claims = [
        "probabilistic E using tm on Config under Steady on origin \
         { expected elapsed until holds(true) <= 3 ms }",
        "probabilistic L using tm on Config under Steady on origin \
         { probability >= 0.5 [ elapsed from holds(true) until holds(true) within 5 ms >= 1 ms ] }",
        "probabilistic W using tm on Config under Steady on origin \
         { probability >= 0.5 [ steps from holds(true) until holds(true) within 5 ms >= 1 ] }",
    ];
    let (document, checked) = timed_claims_check(&claims);
    let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let spelled = |name: &str| statement_spelling(&wire, name).join(" ");
    assert_eq!(
        spelled("E"),
        "form kind expected_elapsed until <node> bound direction at_most value quantity <node> 3 over activation kind origin confidence"
    );
    assert_eq!(
        spelled("L"),
        "form kind probability event kind compare measure kind elapsed from <node> until <node> within quantity <node> 5 bound direction at_least value quantity <node> 1 bound direction at_least value rational 1/2 over activation kind origin confidence"
    );
    assert_eq!(
        spelled("W"),
        "form kind probability event kind compare measure kind steps from <node> until <node> within quantity <node> 5 bound direction at_least value integer 1 bound direction at_least value rational 1/2 over activation kind origin confidence"
    );
}

/// FSpec FR-640 rule 7.3: a step horizon over a timed subject has no
/// spelling, so lowering refuses it at the claim and emits no package the
/// strict reader would refuse.
#[test]
fn a_step_horizon_over_a_timed_subject_is_refused_at_the_claim() {
    let (_, checked) =
        timed_claims_check(
            &["probabilistic W using tm on Config under Steady on origin \
         { probability >= 0.5 [ steps from holds(true) until holds(true) within 5 >= 1 ] }"],
        );
    match checked {
        Ok(_) => panic!("a step horizon over a timed subject checks"),
        Err(refusals) => {
            let [refusal] = refusals.as_slice() else {
                panic!("one refusal, got {refusals:?}");
            };
            let text = format!("{refusal:?}");
            assert!(text.contains("TimeTypeMismatch"), "{text}");
            assert!(!text.contains("InternalFault"), "{text}");
        }
    }
}

/// FSpec FR-640-AC-10: a quantity reward's bound is a quantity at the
/// reward's unit node, and `P95`'s path ends as the spec spells it.
#[trace("TC-1288", "FR-498-AC-7")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-10")]
#[test]
fn a_quantity_reward_bound_is_spelled_at_the_reward_unit() {
    let (document, checked) = timed_claims_check(&[
        "reward duration on ConfigVersion::attemptUpdate = 2 ms;",
        "probabilistic P95 using tm on Config under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.95 of (accumulate duration from holds(true) until holds(true) within 10 ms) <= 5 ms }",
    ]);
    let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    assert_eq!(
        statement_spelling(&wire, "P95").join(" "),
        "form kind quantile level rational 19/20 measure kind accumulate from <node> until <node> within quantity <node> 10 bound direction at_most value quantity <node> 5 over activation kind origin confidence alpha rational 1/100 beta rational 1/100 indifference rational 1/100"
    );
    let id_for = |bound: &str| {
        let (document, checked) = timed_claims_check(&[
            "reward duration on ConfigVersion::attemptUpdate = 2 ms;",
            &format!(
                "probabilistic P95 using tm on Config under Steady on origin \
                 with alpha 0.01, beta 0.01, indifference 0.01 \
                 {{ quantile 0.95 of (accumulate duration from holds(true) until holds(true) within 10 ms) <= {bound} }}"
            ),
        ]);
        let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
        let (wire, _) = emit_and_read(&package, &document);
        resolved_id(&wire, &claim_named(&wire, "P95")["node_id"], 8)
    };
    assert_ne!(id_for("5 ms"), id_for("6 ms"));
}

/// TC-1288 (the delay part of FR-498-AC-6), FR-186: a long-run fraction
/// weighted by `delay` writes `weighting` kind `delay`, binds an empty
/// `reward` even when a reward is named `delay`, and holds the model's one
/// `delay_distribution` node in `delays`; changing that delay's weight
/// changes the claim's identity.
#[trace("TC-1288", "FR-498-AC-6")]
#[test]
fn weighting_by_delay_is_the_delay_weighting_even_when_a_reward_is_named_delay() {
    let emitted = |weight: u32| {
        let delay = format!(
            "delay ConfigVersion::attemptUpdate ~ discrete {{ 1 ms: 90, 3 ms: {weight} }};"
        );
        let (document, checked) = timed_claims_check(&[
            "reward delay on ConfigVersion::attemptUpdate = 1;",
            &delay,
            "probabilistic D using tm on Config under Steady on origin \
             { long_run_fraction holds(true) weighted by delay >= 0.5 }",
        ]);
        let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
        let (wire, read) = emit_and_read(&package, &document);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        wire
    };
    let wire = emitted(10);
    let claim = claim_named(&wire, "D");
    let spelled = statement_spelling(&wire, "D").join(" ");
    assert!(
        spelled.starts_with("form kind long_run_fraction predicate <node> weighting kind delay"),
        "{spelled}"
    );
    assert!(targets(binding(&claim["body"], "reward")).is_empty());
    assert_eq!(
        targets(binding(&claim["body"], "delays")),
        node_ids(&wire, "probabilistic", "delay_distribution")
    );
    let heavier = emitted(20);
    assert_ne!(
        resolved_id(&wire, &claim["node_id"], 8),
        resolved_id(&heavier, &claim_named(&heavier, "D")["node_id"], 8)
    );
}

/// FSpec FR-640-AC-3 and FSL FR-498 reordering: a claim under a workload of
/// two weight lines keeps its identity whichever order the lines are written
/// in.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_claim_keeps_its_identity_whichever_order_its_workload_weights_are_written() {
    let claim_id = |weights: &str| {
        let (document, package) = lineage_package_with_profiles(
            "profile ev = \"filament.temporal.event-position.false-extension/v1\";",
            &format!(
                "workload Mixed on M {{ {weights} }}\n\
             probabilistic C using ev on M under Mixed on origin \
             {{ probability >= 0.5 [ always[0,2] holds(true) ] }}"
            ),
        );
        let (wire, read) = emit_and_read(&package, &document);
        assert!(matches!(read, Read::Verified { .. }), "{read:?}");
        resolved_id(&wire, &claim_named(&wire, "C")["node_id"], 8)
    };
    assert_eq!(
        claim_id("weight Base::op = 1; weight Leaf::step = 3;"),
        claim_id("weight Leaf::step = 3; weight Base::op = 1;")
    );
}

/// TC-1288 (FR-498-AC-7), FSpec FR-322 path order: changing only the
/// interval of a lone claim's event moves its `node_id`, whether the interval
/// is another upper end, another lower end or an open end, and rewriting the
/// same interval with other spacing moves nothing.
#[trace("TC-1288", "FR-498-AC-7")]
#[test]
fn a_lone_claim_path_moves_with_the_interval_of_its_event() {
    let id_of = |event: &str| {
        let wire = emit_wire(&claims_package(&[
            STEADY,
            &claim_stated(
                "A",
                "with alpha 0.01, beta 0.01, indifference 0.01",
                &format!("{{ probability >= 0.5 [ {event} ] }}"),
            ),
        ]));
        resolved_id(&wire, &claim_named(&wire, "A")["node_id"], 8)
    };
    let base = id_of("always[0,2] holds(true)");
    assert_eq!(base, id_of("always[ 0 , 2 ] holds(true)"));
    for other in [
        "always[0,3] holds(true)",
        "always[1,2] holds(true)",
        "eventually[0,2] holds(true)",
    ] {
        assert_ne!(base, id_of(other), "{other}");
    }
}

/// FSpec FR-640-AC-3 and FSL FR-498 reordering: two claims whose events
/// differ only in an interval keep their identities whichever order they are
/// declared in.
#[trace("TC-1288", "FR-498-AC-1")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn claims_that_differ_only_in_an_interval_keep_their_identities_when_reordered() {
    let confidence = "with alpha 0.01, beta 0.01, indifference 0.01";
    let narrow = claim_stated(
        "A",
        confidence,
        "{ probability >= 0.5 [ always[0,2] holds(true) ] }",
    );
    let wide = claim_stated(
        "B",
        confidence,
        "{ probability >= 0.5 [ always[0,3] holds(true) ] }",
    );
    let ids_of = |declarations: &[&str]| {
        let wire = emit_wire(&claims_package(declarations));
        ["A", "B"].map(|name| resolved_id(&wire, &claim_named(&wire, name)["node_id"], 8))
    };
    let forward = ids_of(&[STEADY, &narrow, &wide]);
    let reversed = ids_of(&[STEADY, &wide, &narrow]);
    assert_eq!(forward, reversed);
    assert_ne!(forward[0], forward[1]);
}

/// FSpec FR-640-AC-11, rule 7.3: a quantity typed at a node that is not the
/// model's unit node is refused at the quantity's type, over a timed subject.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-11")]
#[test]
fn the_reader_refuses_a_quantity_typed_at_the_wrong_unit() {
    let (document, checked) =
        timed_claims_check(
            &["probabilistic E using tm on Config under Steady on origin \
         { expected elapsed until holds(true) <= 3 ms }"],
        );
    let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let verified = read_document_with_body_edit(&package, &document, &|_| {});
    assert!(matches!(verified, Read::Verified { .. }), "{verified:?}");
    fn retype(term: &mut Value, unit: &Value) {
        match term {
            Value::Object(object) => {
                if object
                    .get("value_kind")
                    .is_some_and(|kind| kind == "quantity")
                {
                    object.insert("type".to_owned(), unit.clone());
                }
                for value in object.values_mut() {
                    retype(value, unit);
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| retype(item, unit)),
            _ => {}
        }
    }
    let refused = refusal_summary(read_document_with_body_edit(&package, &document, &|body| {
        let subject = body["members"][0]["value"]["target"].clone();
        retype(body, &subject);
    }));
    assert_eq!(
        refused,
        (
            "IllTyped".to_owned(),
            "Some(OperatorIneligible)".to_owned(),
            "/semantic_graph/nodes/2/body/members/5/value/members/2/value/members/1/value/type"
                .to_owned()
        )
    );
}

/// FSpec FR-640-AC-11, rule 7.3, over FSL FR-233-AC-3's tick model: a window
/// of `5 ms` over `time ... tick ... period 1 ms unit ms` is the quantity 5 at
/// the unit node of `ms`, and the strict reader admits it.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-11")]
#[test]
fn a_time_window_over_a_tick_model_is_a_quantity_at_the_model_unit() {
    let (document, checked) = timed_claims_check_in(
        "time Config tick ConfigVersion::attemptUpdate period 1 ms unit ms;",
        &["probabilistic W using tm on Config under Steady on origin \
           { probability >= 0.5 [ steps from holds(true) until holds(true) within 5 ms >= 1 ] }"],
    );
    let package = checked.unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&package, &document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    assert_eq!(
        statement_spelling(&wire, "W").join(" "),
        "form kind probability event kind compare measure kind steps from <node> until <node> within quantity <node> 5 bound direction at_least value integer 1 bound direction at_least value rational 1/2 over activation kind origin confidence"
    );
}

/// TC-3012 (FR-740-AC-6): FSL's reader rebuilds the random parameter and the
/// workload declared on a source object from the emitted package: the
/// parameter with probabilities `1/4` and `3/4`, the workload equal to S3's.
/// The join runs over S3's own source packages; the production route, fsl-eval
/// `PackageModelInput::source_object_probabilistic_model`, rebuilds them from
/// the package's source-object rows (`source_object_probabilistic` in fsl-eval's
/// `it` target).
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_source_random_parameter_and_workload_are_read_back_equal() {
    use fsl_semantics::model::observation::ModelSelections;
    let package = source_object_reward(
        "random O::step(x) ~ { 1: 1, 3: 3 };\nworkload W on O { weight O::step = 1; }",
    );
    let emission = emit_checked(&package).expect("the package emits");
    let evidence = read_evidence(&emission);
    let (admitted, _) = admitted_of(read_with(&emission, &evidence));
    let mut rows_budget = V2ReadLimits::fixture().native_join_budget();
    let rows = emission
        .identity_rows(&admitted, &mut rows_budget)
        .expect("the identity rows join the package");
    let mut join_budget = V2ReadLimits::fixture().native_join_budget();
    let bare = crate::BarePackage::new(&admitted)
        .with_supply(
            crate::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut join_budget,
        )
        .expect("the bytes view admits its supply");
    let sources = package.graph().source_packages();
    let mut selections = crate::AnalysisView::model_selections(&bare);
    selections.extend(sources.iter().map(|source| source.model_selection.clone()));
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("the package input is empty");
    let model = fsl_semantics::model::state::StateModel::new(
        ModelSelections::new(&selections, sources),
        package.graph().scope().types(),
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the source package re-admits");
    let operations = model
        .operation_table(&BTreeMap::new())
        .expect("the operations type");
    let nodes = bare.probabilistic().expect("the nodes read");
    let read = model
        .probabilistic_model(&operations, &nodes)
        .unwrap_or_else(|refusal| panic!("the records rebuild: {refusal}"));
    let [random] = read.random_parameters() else {
        panic!("one random parameter read");
    };
    let probabilities: Vec<String> = random
        .support()
        .iter()
        .map(|support| spelling(&support.probability))
        .collect();
    assert_eq!(probabilities, ["1/4", "3/4"]);
    let (checked, read) = (package.graph().probabilistic_model(), &read);
    let [read_workload] = read.workloads() else {
        panic!("one workload read");
    };
    let [checked_workload] = checked.workloads() else {
        panic!("one checked workload");
    };
    assert_eq!(read_workload.name(), checked_workload.name());
    let content = |workload: &fsl_semantics::check::Workload| -> Vec<(String, String)> {
        workload
            .weights()
            .iter()
            .map(|(operation, weight)| (operation.name().to_owned(), spelling(weight)))
            .collect()
    };
    assert_eq!(content(read_workload), content(checked_workload));
    assert_eq!(read_workload.model_key(), checked_workload.model_key());
}

fn spelling(value: &filament_exact::Rational) -> String {
    format!("{}/{}", value.numerator(), value.denominator())
}
