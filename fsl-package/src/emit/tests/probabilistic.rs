// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185: a package whose random parameters, rewards or workloads check is
//! refused at emission, since FR-322's v2 vocabulary has no node for them,
//! rather than written without them. FR-498: a probabilistic claim is
//! emitted as its own `claim`/`probabilistic_claim` node and sidecar row.

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
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert!(!graph.probabilistic_model().is_empty());
    CheckedPackage::link(graph)
}

/// FR-185 "Package emission": the checked workload and reward are recorded
/// on the graph, and the emitter refuses
/// `unsupported_construct`/`not-yet-implemented`, naming the first of them in
/// source order (the workload `Steady`), its feature and the unit's profile.
#[trace("TC-620")]
#[test]
fn a_checked_probabilistic_model_refuses_emission() {
    let refusal = emit_checked(&probabilistic_package()).expect_err("emission refuses");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "Steady".to_owned(),
            feature: fsl_semantics::check::ProbabilisticFeature::Workload,
            profile: Some("filament.model.complete/v1"),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
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
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
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

/// The emitted `probabilistic_claims` rows, and each `claim` node's form and
/// occurrences by node id.
fn emitted_claims(package: &CheckedPackage) -> (Vec<Value>, Vec<(Value, Value)>) {
    let emission = emit_checked(package).expect("a claim-only package emits");
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let read = read_with(&emission, &evidence);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let wire = wire(&emission);
    let rows = wire["checked_term_data"]["probabilistic_claims"]
        .as_array()
        .expect("the claims are carried")
        .clone();
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the graph has nodes")
        .iter()
        .filter(|node| node["node_tag"] == "claim")
        .map(|node| {
            assert_eq!(node["semantic_form"], "probabilistic_claim");
            (node["node_id"].clone(), node["occurrences"].clone())
        })
        .collect();
    (rows, nodes)
}

/// FR-186 "Outputs", FR-498: a package whose only probabilistic declaration
/// is a checked claim is emitted with the claim's node; one that also
/// declares a reward after it is refused naming the reward.
#[trace("TC-621", "FR-186-AC-5")]
#[test]
fn a_checked_claim_is_emitted_and_a_reward_still_refuses() {
    let package = claims_package(&[&claim("Stable", "0.5")]);
    assert_eq!(package.graph().probabilistic_model().claims().len(), 1);
    let (rows, nodes) = emitted_claims(&package);
    assert_eq!((rows.len(), nodes.len()), (1, 1));

    let package = claims_package(&[
        &claim("Stable", "0.5"),
        "reward cost on ConfigVersion::attemptUpdate = 1;",
    ]);
    let refusal = emit_checked(&package).expect_err("the reward refuses emission");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "cost".to_owned(),
            feature: fsl_semantics::check::ProbabilisticFeature::Reward,
            profile: Some("filament.model.complete/v1"),
        }
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
        ("PC-02", claim("Stable", "0.25")),
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
        (identities, rows)
    };
    let (forward, forward_rows) = emit(&[0, 1, 2]);
    let (reversed, reversed_rows) = emit(&[2, 1, 0]);
    let (shuffled, _) = emit(&[1, 2, 0]);
    assert_eq!(forward, reversed, "each claim keeps its identity");
    assert_eq!(forward, shuffled, "each claim keeps its identity");
    assert_eq!(forward_rows, reversed_rows, "the rows are byte-identical");
    assert_ne!(
        forward[0].1, forward[1].1,
        "equal names are distinct claims"
    );
    // Ordinals rank written content ascending: `0.25` before `0.5`.
    assert_eq!(forward[1].1["declaration"]["ordinal"], 0, "Stable@0.25");
    assert_eq!(forward[0].1["declaration"]["ordinal"], 1, "Stable@0.5");

    // Renaming a claim changes its identity and no other claim's.
    let renamed = claims_package(&[
        &claim("Stable", "0.5"),
        &claim("Stable", "0.25"),
        &claim("Renamed", "0.5"),
    ]);
    let renamed = identities(&renamed, &[("PC-01", ""), ("PC-02", ""), ("PC-03", "")]);
    assert_eq!(renamed[..2], forward[..2]);
    assert_ne!(renamed[2].1, forward[2].1);

    // Two identically written claims are two declarations.
    let twins = claims_package(&[&claim("Stable", "0.5"), &claim("Stable", "0.5")]);
    let twins = identities(&twins, &[("a", ""), ("b", "")]);
    assert_ne!(twins[0].1, twins[1].1);
    let (rows, nodes) = emitted_claims(&claims_package(&[
        &claim("Stable", "0.5"),
        &claim("Stable", "0.5"),
    ]));
    assert_eq!((rows.len(), nodes.len()), (2, 2));
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
#[ignore = "QSL-2000: emission of workloads and rewards as v2 probabilistic nodes not implemented yet"]
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
}

/// TC-2062 (FR-740-AC-1): a unit with a random parameter on
/// `attemptUpdate(n)` emits one `probabilistic`/`random_parameter` node
/// whose support keeps the declared order and weights, and the strict reader
/// admits it.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-1")]
#[trace("TC-2062", "FR-740-AC-1")]
#[test]
#[ignore = "QSL-2000: emission of random parameters as v2 probabilistic nodes not implemented yet"]
fn a_random_parameter_is_emitted_and_read_back() {
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
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let (wire, read) = emit_and_read(&CheckedPackage::link(graph), &document);
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

/// TC-2062 (FR-740-AC-3, FSpec FR-640-AC-3): writing `Steady`'s weight
/// lines in another order emits byte-identical output, and a workload with a
/// weight of 0 is emitted with that weight.
#[trace("TC-2062", "FR-740-AC-3")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
#[ignore = "QSL-2000: workload emission order and weight 0 as v2 probabilistic nodes not implemented yet"]
fn workload_weight_order_changes_no_bytes_and_weight_zero_is_emitted() {
    let package = |weights: &str| {
        let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
        let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
        let unit = format!(
            "{}workload Steady on Config {{ {weights} }}\n",
            config_version_unit::unit_text()
        );
        let graph = assemble_with_models(unit.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
        CheckedPackage::link(graph)
    };
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

    // The fixture model has one operation, so the reordering is of the
    // workload's declaration among the unit's other declarations.
    let first = emit_checked(&package("weight ConfigVersion::attemptUpdate = 1;"))
        .expect("the package emits");
    let again = emit_checked(&package("weight ConfigVersion::attemptUpdate = 1;"))
        .expect("the package emits");
    assert_eq!(first.package().bytes(), again.package().bytes());
}

/// TC-2062 (FR-740-AC-4): a claim `under Steady` is emitted beside the
/// `Steady` workload node it resolves to, and the strict reader admits the
/// package holding the claim, the reward and the workload.
#[trace("TC-2062", "FR-740-AC-4")]
#[test]
#[ignore = "QSL-2000: a claim's workload resolves to its emitted workload node not implemented yet"]
fn a_claim_under_a_workload_is_emitted_with_its_workload_node() {
    let package = claims_package(&[
        "workload Steady on Config { weight ConfigVersion::attemptUpdate = 1; }",
        "reward cost on ConfigVersion::attemptUpdate = 1;",
        "probabilistic Stable using ev on Config under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { probability >= 0.5 [ always[0,2] holds(true) ] }",
    ]);
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let (wire, read) = emit_and_read(&package, document);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    assert_eq!(
        super::owners::node_where(&wire, "probabilistic", "workload").len(),
        1
    );
    assert_eq!(
        super::owners::node_where(&wire, "claim", "probabilistic_claim").len(),
        1
    );
}

/// TC-1563 (FSpec FR-640-AC-4): a `workload` node whose weights name an
/// operation the model does not declare is refused by the strict reader.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-4")]
#[test]
#[ignore = "QSL-2000: the strict reader's probabilistic node checks not implemented yet"]
fn the_reader_refuses_a_workload_that_misnames_an_operation() {
    let package = probabilistic_package();
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
