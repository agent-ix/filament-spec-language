// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-740: a checked random parameter and workload are emitted as
//! `probabilistic` nodes; a package whose rewards or delays check is refused
//! at emission, with the declaration's locus, rather than written without
//! them. FR-498: a probabilistic claim is emitted as its own
//! `claim`/`probabilistic_claim` node and sidecar row.

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

/// FR-185 "Package emission", FR-740: the checked workload is emitted, and
/// the reward beside it is refused `unsupported_construct`/
/// `not-yet-implemented`, naming the reward `cost`, its feature, the unit's
/// profile and the reward's own locus.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_checked_reward_refuses_emission_beside_an_emitted_workload() {
    let refusal = emit_checked(&probabilistic_package()).expect_err("emission refuses");
    assert!(matches!(
        &refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::Reward,
            profile: Some("filament.model.complete/v1"),
            region,
        } if declaration == "cost" && region.start() < region.end()
    ));
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
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
        .check(CheckMode::Linked, CheckingLimits::default())
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
    assert!(matches!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::Reward,
            profile: Some("filament.model.complete/v1"),
            region,
        } if declaration == "cost" && region.start() < region.end()
    ));
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
#[ignore = "QSL-2440: emission of rewards as v2 probabilistic nodes (the workload half is workload_round_trips)"]
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
        .check(CheckMode::Linked, CheckingLimits::default())
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
        &fsl_semantics::model::key::object_type_node_key(checked.operation().declaring_key())
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
        &fsl_semantics::model::key::object_type_node_key(operation.declaring_key())
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
    let nodes = bare.probabilistic().expect("the nodes read");
    model
        .probabilistic_model(&operations, &nodes)
        .unwrap_or_else(|refusal| panic!("the records rebuild: {refusal}"))
}

/// The accessor-visible content of an operation, which is all of it but the
/// `model` alias a package does not carry.
fn operation_content(
    operation: &fsl_semantics::check::ProbabilisticOperation,
) -> (
    &filament_checked::key::DeclarationKey,
    filament_exact::EffectiveId,
    &filament_checked::key::DeclarationKey,
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

/// FSL FR-740: an enum-valued random parameter is refused at emission with
/// the declaration's locus, because IR's reader maps no model variant type to
/// a support literal kind and would refuse the package. The rebuild of an
/// enum value's rank from its type is covered at S3
/// (`an_enum_valued_package_rebuilds_the_s3_records`).
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn an_enum_valued_random_parameter_refuses_emission_with_its_locus() {
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
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let refusal = emit_checked(&CheckedPackage::link(graph)).expect_err("emission refuses");
    assert!(matches!(
        &refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature: fsl_semantics::check::ProbabilisticFeature::RandomParameter,
            region,
            ..
        } if declaration == "promote(t)" && region.start() < region.end()
    ));
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
        .check(CheckMode::Linked, CheckingLimits::default())
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
    use super::abstraction::fixture;
    let document = crate::emit::tests::wire_bytes(&fixture::lineage_document())
        .expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
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
        .check(CheckMode::Linked, CheckingLimits::default())
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
            .check(CheckMode::Linked, CheckingLimits::default())
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

/// TC-2062 (FR-740-AC-4): a claim `under Steady` is emitted beside the
/// `Steady` workload node it resolves to, and the strict reader admits the
/// package holding the claim, the reward and the workload.
#[trace("TC-2062", "FR-740-AC-4")]
#[test]
#[ignore = "QSL-2441: a claim's workload resolves to its emitted workload node; the claim also reads a reward"]
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
