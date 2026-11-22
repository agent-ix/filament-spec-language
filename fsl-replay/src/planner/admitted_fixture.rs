// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine source/emission/strict-reader composition inputs for planner tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_contract_model::NativeJoinBudget;
use filament_contract_model::{CheckedOccurrenceRole, CheckedPackageV2};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    admit_checked_package_with_dependencies, emit_checked, read_evaluation_emission,
    read_import_view, AdmittedPackages, BarePackage, BareSupply, CheckedPackage,
    DependencyArtifact, Emission, EmitLimits, Import, NativeJoinFailure, ReadEqualityRows,
    ReadIdentityRows, SourceWitnessSupplier, V2ReadLimits,
};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::{self, PackageInput};

#[path = "../../../fsl-semantics/tests/support/composition.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the actual composition model is shared with package tests"
)]
mod model_fixture;

const ROOT: &str = "test/composition";
const PRODUCER: &str = "test/producer";
const CONSUMER: &str = "test/consumer";
const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";
profile v = \"filament.model.complete/v1\";
profile t = \"filament.temporal.infinite-trace/v1\";
model M = \"test/orders\";
";
const PRODUCER_CLAUSES: &str = "
temporal Sends using t over (p: M::Producer) clock \"model-steps\" on origin {
 fair weak each M::Producer::send; always eventually holds(p.sent)
}
temporal Shared using t over (p: M::Producer) clock \"model-steps\" on origin {
 always holds(p.sent_count >= 0)
}
";
const CONSUMER_CLAUSES: &str = "
temporal Done using t over (c: M::Consumer) clock \"model-steps\" on origin {
 always (holds(c.received) implies eventually holds(c.done))
}
temporal Shared using t over (c: M::Consumer) clock \"model-steps\" on origin {
 always holds(c.received_count >= 0)
}
";

/// Which actual authored dependency graph to compile.
#[derive(Clone, Copy)]
pub(super) enum CompositionCase {
    /// A reliance and one forward liveness dependency.
    Acyclic,
    /// A mutual dependency between two distinct declared components.
    Cyclic,
    /// A genuine supplied operation with no clause fairness on that operation.
    UnusedSupply,
    /// A source-admitted step-bounded clause in the composition.
    StepBounded,
    /// An actual relied-on clause with no declared guarantee interface.
    MissingInterface,
    /// An actual fairness requirement with no composition supply.
    MissingSupply,
}

/// Original producer inputs beside an independently byte-admitted root.
pub(super) struct AdmittedCompositionFixture {
    /// The actual S3/S4 source oracle; never passed to byte admission or `bare`.
    pub source: CheckedPackage,
    /// Authored root text, only for independent source-region assertions.
    pub source_text: String,
    /// The original root emission, including its actual witness producers.
    pub emission: Emission,
    /// Strict read-back's actual root Arc, before independent byte admission.
    pub readback: Arc<CheckedPackageV2>,
    /// Strict admission of `bytes` with the actual direct dependency Arcs.
    pub admitted: Arc<CheckedPackageV2>,
    /// Unmodified root artifact bytes.
    pub bytes: Arc<[u8]>,
    /// Root identities produced from `emission` and decoded from their own bytes.
    pub identities: ReadIdentityRows,
    /// Root equality evidence produced from `emission` and decoded from its bytes.
    pub equality: ReadEqualityRows,
    /// Exact serialized root identity evidence.
    pub identity_bytes: Arc<[u8]>,
    /// Exact serialized root equality evidence, including genuinely empty evidence.
    pub equality_bytes: Arc<[u8]>,
    /// Flat authentic owner artifacts, each with its own bytes and witnesses.
    pub dependencies: Vec<DependencyArtifact>,
    /// Each dependency's original emission, in the same order as `dependencies`.
    pub owner_emissions: Vec<Emission>,
    /// The genuine selected domain document supplied to strict admission.
    pub documents: PackageInput,
}

struct Dependency {
    package: Arc<CheckedPackage>,
    emission: Emission,
    import: fsl_semantics::check::AdmittedImport,
    identity: LibraryName,
}

fn compile(
    identity: &str,
    source: &str,
    documents: &PackageInput,
    dependencies: &[&Dependency],
) -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", identity),
        "composition.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the actual unit parses");
    assert!(
        parsed.is_admissible(),
        "{identity}: {:?}",
        parsed.diagnostics()
    );
    let raw = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("the actual unit assembles");
    let models = intake::admit_unit(
        &built.selections().models,
        documents,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the actual selected model is admitted");
    let declarations = PackageDeclarations::assemble(
        raw,
        identity.to_owned(),
        built,
        models,
        dependencies
            .iter()
            .map(|dependency| dependency.import.clone())
            .collect(),
    )
    .expect("source declarations link against genuine import views");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .expect("the actual source clauses check");
    CheckedPackage::link_with(
        graph,
        dependencies
            .iter()
            .map(|dependency| Import {
                identity: dependency.identity.clone(),
                package: Arc::clone(&dependency.package),
            })
            .collect(),
        EmitLimits::fixture(),
    )
    .expect("the actual checked closure links")
}

fn dependency(identity: &str, clauses: &str, documents: &PackageInput) -> Dependency {
    let package = Arc::new(compile(
        identity,
        &format!("{MODEL_HEADER}{clauses}"),
        documents,
        &[],
    ));
    let emission = emit_checked(&package, EmitLimits::fixture()).expect("the actual owner emits");
    let identity = LibraryName::new(identity).unwrap();
    let view = read_import_view(
        &package,
        &emission,
        identity.clone(),
        documents,
        &mut AdmittedPackages::default(),
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("the actual owner import view strictly admits");
    Dependency {
        import: fsl_semantics::check::AdmittedImport {
            identity: identity.clone(),
            view,
            graph: package.shared_graph(),
        },
        package,
        emission,
        identity,
    }
}

fn witnesses(
    emission: &Emission,
    admitted: &CheckedPackageV2,
    budget: &mut NativeJoinBudget,
) -> (ReadIdentityRows, ReadEqualityRows, Arc<[u8]>, Arc<[u8]>) {
    let identities = emission
        .identity_rows(admitted, budget)
        .expect("actual emitted identity rows");
    let equality = emission
        .equality_witness(admitted, budget)
        .expect("actual emitted equality rows");
    assert_eq!(
        identities.declarations().is_empty(),
        emission.selected_exact_declaration_rows().is_empty()
    );
    assert_eq!(
        identities.fields().is_empty(),
        emission.selected_exact_field_rows().is_empty()
    );
    assert_eq!(
        equality.applications().is_empty(),
        emission.equality_producer_identity_rows().is_empty()
    );
    assert_eq!(
        equality.facts().is_empty(),
        emission.equality_fact_witness_rows().is_empty()
    );
    assert_eq!(
        equality.source_units().is_empty(),
        emission.selected_source_unit_rows().is_empty()
    );
    let identity_bytes: Arc<[u8]> = identities.to_bytes(admitted).unwrap().into();
    let equality_bytes: Arc<[u8]> = equality.to_bytes(admitted).unwrap().into();
    let identities = ReadIdentityRows::read(&identity_bytes, admitted, budget)
        .expect("the genuine serialized identity evidence reads against its owner");
    let equality = ReadEqualityRows::read(&equality_bytes, admitted)
        .expect("the genuine serialized equality evidence reads against its owner");
    (identities, equality, identity_bytes, equality_bytes)
}

impl AdmittedCompositionFixture {
    /// Compile genuine owners and a root, then independently admit the root bytes.
    pub fn new(case: CompositionCase) -> Self {
        let document = serde_json::to_vec(&model_fixture::document()).unwrap();
        let documents = intake::package_input([document.as_slice()]).unwrap();
        let producer_source = if matches!(case, CompositionCase::StepBounded) {
            PRODUCER_CLAUSES.replace(
                "always holds(p.sent_count >= 0)",
                "always[0,1] holds(p.sent_count >= 0)",
            )
        } else {
            PRODUCER_CLAUSES.to_owned()
        };
        let producer = dependency(PRODUCER, &producer_source, &documents);
        let consumer = dependency(CONSUMER, CONSUMER_CLAUSES, &documents);
        let reverse = match case {
            CompositionCase::Acyclic
            | CompositionCase::UnusedSupply
            | CompositionCase::StepBounded
            | CompositionCase::MissingInterface
            | CompositionCase::MissingSupply => "",
            CompositionCase::Cyclic => "depends Producer::Sends on Consumer::Done;",
        };
        let unused_supply = if matches!(case, CompositionCase::UnusedSupply) {
            "supplies fair strong whole M::Consumer::receive;"
        } else {
            ""
        };
        let sends_guarantee = if matches!(case, CompositionCase::MissingInterface) {
            ""
        } else {
            "guarantee Producer::Sends;"
        };
        let send_supply = if matches!(case, CompositionCase::MissingSupply) {
            ""
        } else {
            "supplies fair weak each M::Producer::send;"
        };
        let source_text = format!(
            "language \"ix:native\" edition \"1-draft\";
profile v = \"filament.model.complete/v1\";
profile q = \"filament.value.complete/v1\";
import \"test/producer\" as P;
import \"test/consumer\" as Q;
function marker using q(): Boolean pure {{ true }}
compose Sys rule assume-guarantee {{
 component Producer from P using M; component Consumer from Q using M;
 guarantee Consumer::Shared; guarantee Producer::Shared;
 guarantee Consumer::Done; {sends_guarantee}
 rely Consumer::Done on Producer::Shared;
 depends Consumer::Done on Producer::Sends;
 {send_supply}
 {unused_supply}
 {reverse}
}}
"
        );
        let source = compile(ROOT, &source_text, &documents, &[&producer, &consumer]);
        let emission = emit_checked(&source, EmitLimits::fixture()).expect("the actual root emits");
        let identity = LibraryName::new(ROOT).unwrap();
        let read = read_evaluation_emission(
            &source,
            &emission,
            identity.clone(),
            &documents,
            V2ReadLimits::fixture(),
            EmitLimits::fixture(),
            SourceWitnessSupplier::new(&emission),
        )
        .expect("strict source read-back admits the authentic complete closure");
        let readback = read
            .admitted_packages()
            .shared_package(&identity)
            .expect("the actual root Arc");
        let mut setup = NativeJoinBudget::new(10_000_000, 10_000_000);
        setup.resume();
        let mut dependencies = Vec::new();
        let mut owner_emissions = Vec::new();
        for owner in [&producer, &consumer] {
            let package = read
                .admitted_packages()
                .shared_package(&owner.identity)
                .expect("the actual admitted owner Arc from this root's closure");
            assert_eq!(
                package.direct_dependencies().len(),
                0,
                "these real owners import no libraries"
            );
            let bytes: Arc<[u8]> = owner.emission.package().bytes().into();
            let (identities, equality, identity_bytes, equality_bytes) =
                witnesses(&owner.emission, &package, &mut setup);
            dependencies.push(DependencyArtifact {
                identity: owner.identity.to_string(),
                package,
                bytes,
                identity_bytes,
                equality_bytes,
                identities,
                equality,
            });
            owner_emissions.push(owner.emission.clone());
        }
        drop(read);
        let bytes: Arc<[u8]> = emission.package().bytes().into();
        let direct = dependencies
            .iter()
            .map(|owner| (owner.identity.clone(), Arc::clone(&owner.package)))
            .collect::<BTreeMap<_, _>>();
        let admitted = Arc::new(
            admit_checked_package_with_dependencies(
                &bytes,
                &documents,
                &direct,
                V2ReadLimits::fixture(),
            )
            .expect("independent strict root admission uses only artifact inputs"),
        );
        let (identities, equality, identity_bytes, equality_bytes) =
            witnesses(&emission, &admitted, &mut setup);
        Self {
            source,
            source_text,
            emission,
            readback,
            admitted,
            bytes,
            identities,
            equality,
            identity_bytes,
            equality_bytes,
            dependencies,
            owner_emissions,
            documents,
        }
    }

    /// Borrow only admitted artifacts and each owner's own witnesses on one ledger.
    pub fn bare(
        &self,
        budget: &mut NativeJoinBudget,
    ) -> Result<BarePackage<'_>, NativeJoinFailure> {
        BarePackage::new(&self.admitted)
            .with_canonical_bytes(&self.bytes)
            .with_dependency_artifacts(&self.dependencies, budget)?
            .with_supply(
                BareSupply {
                    identities: &self.identities,
                    equality: Some(&self.equality),
                },
                budget,
            )
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::*;
    use fsl_package::{admit_checked_package, CheckedTerm};

    fn ledger() -> NativeJoinBudget {
        let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
        budget.resume();
        budget
    }

    fn assert_artifacts(fixture: &AdmittedCompositionFixture) {
        assert_eq!(fixture.admitted.name(), ROOT);
        assert_eq!(fixture.readback.name(), ROOT);
        assert_eq!(fixture.bytes.as_ref(), fixture.emission.package().bytes());
        assert_eq!(fixture.admitted.direct_dependencies().len(), 2);
        assert_eq!(
            fixture
                .dependencies
                .iter()
                .map(|owner| owner.identity.as_str())
                .collect::<Vec<_>>(),
            [PRODUCER, CONSUMER]
        );
        assert_eq!(fixture.identities.package_name(), ROOT);
        assert_eq!(fixture.equality.package_name(), ROOT);
        assert_eq!(
            fixture.identity_bytes.as_ref(),
            fixture.identities.to_bytes(&fixture.admitted).unwrap()
        );
        assert_eq!(
            fixture.equality_bytes.as_ref(),
            fixture.equality.to_bytes(&fixture.admitted).unwrap()
        );
        // The root has no selected model of its own. This is observed from
        // its actual producer, never replaced with a fabricated empty supply.
        assert!(fixture
            .emission
            .selected_exact_declaration_rows()
            .is_empty());
        assert!(fixture.identities.declarations().is_empty());
        assert!(fixture.emission.selected_exact_field_rows().is_empty());
        assert!(fixture.identities.fields().is_empty());
        let mut shared = Vec::new();
        for (owner, emission) in fixture.dependencies.iter().zip(&fixture.owner_emissions) {
            assert_eq!(owner.identity, owner.package.name());
            assert_eq!(owner.bytes.as_ref(), emission.package().bytes());
            assert_eq!(owner.identities.package_name(), owner.identity);
            assert_eq!(owner.equality.package_name(), owner.identity);
            assert!(!emission.selected_exact_declaration_rows().is_empty());
            assert!(!owner.identities.declarations().is_empty());
            // This producer channel is for record fields; the fixture's
            // selected declarations are model objects, not record values.
            assert!(emission.selected_exact_field_rows().is_empty());
            assert!(owner.identities.fields().is_empty());
            assert_eq!(
                owner.identity_bytes.as_ref(),
                owner.identities.to_bytes(&owner.package).unwrap()
            );
            assert_eq!(
                owner.equality_bytes.as_ref(),
                owner.equality.to_bytes(&owner.package).unwrap()
            );
            assert!(std::ptr::eq(
                fixture
                    .readback
                    .direct_dependencies()
                    .find(|package| package.name() == owner.identity)
                    .unwrap(),
                owner.package.as_ref()
            ));
            assert!(std::ptr::eq(
                fixture
                    .admitted
                    .direct_dependencies()
                    .find(|package| package.name() == owner.identity)
                    .unwrap(),
                owner.package.as_ref()
            ));
            let independent =
                admit_checked_package(&owner.bytes, &fixture.documents, V2ReadLimits::fixture())
                    .expect("each owner's own original bytes strictly admit without source");
            assert_eq!(independent.name(), owner.identity);
            assert_eq!(independent.direct_dependencies().len(), 0);
            assert_eq!(
                independent.temporal_clause_names().unwrap(),
                owner.package.temporal_clause_names().unwrap()
            );
            let named = owner
                .package
                .temporal_clause_names()
                .unwrap()
                .iter()
                .find(|named| named.name.as_ref() == "Shared")
                .unwrap();
            assert_eq!(named.declaration.package.as_ref(), owner.identity);
            assert!(owner.package.term(&named.declaration).is_some());
            shared.push(named.declaration.clone());
        }
        assert_ne!(
            shared[0], shared[1],
            "same display names retain different authenticated owners"
        );
        assert!(
            admit_checked_package(&fixture.bytes, &fixture.documents, V2ReadLimits::fixture())
                .is_err(),
            "root bytes need the actual selected libraries; source is not a fallback"
        );

        let mut baseline = ledger();
        let bare = fixture
            .bare(&mut baseline)
            .expect("all real witnesses join on one ledger");
        assert_eq!(bare.package().name(), ROOT);
        assert_eq!(
            bare.dependency_views()
                .iter()
                .map(|view| view.package().name())
                .collect::<Vec<_>>(),
            [PRODUCER, CONSUMER]
        );
        let paid = baseline.consumed();
        assert!(paid.work > 0 && paid.storage > 0);
        let mut prefixed = ledger();
        let locus = filament_contract_model::NativeJoinLocus {
            source: filament_contract_model::NativeJoinLocusSource::Package,
            form: "fixture_caller_prefix",
        };
        prefixed
            .accounted_charge(
                filament_contract_model::NativeJoinChargeKind::Work,
                7,
                locus,
            )
            .unwrap();
        prefixed
            .accounted_charge(
                filament_contract_model::NativeJoinChargeKind::Storage,
                11,
                locus,
            )
            .unwrap();
        fixture
            .bare(&mut prefixed)
            .expect("the caller's admitted prefix is retained across every owner");
        assert_eq!(prefixed.consumed().work, paid.work + 7);
        assert_eq!(prefixed.consumed().storage, paid.storage + 11);
    }

    /// Trace: FR-530-AC-1, FR-940-AC-1
    #[test]
    fn admitted_fixture_keeps_genuine_acyclic_owner_artifacts_and_witnesses() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Acyclic);
        assert_artifacts(&fixture);
        let [composition] = fixture.source.graph().compositions() else {
            panic!("one actual source composition");
        };
        assert!(composition.cycles().is_empty());
        assert_eq!(
            composition
                .guarantees()
                .iter()
                .map(|member| member.reference.to_string())
                .collect::<Vec<_>>(),
            [
                "Consumer::Shared",
                "Producer::Shared",
                "Consumer::Done",
                "Producer::Sends"
            ]
        );
        let named = fixture.admitted.claim_name_for_path(&["Sys"]).unwrap();
        let Some(CheckedTerm::ComponentComposition {
            components,
            guarantees,
            relies,
            liveness_dependencies,
            ..
        }) = fixture.admitted.term(&named.declaration)
        else {
            panic!("the independently admitted composition term");
        };
        assert_eq!(
            (
                components.len(),
                guarantees.len(),
                relies.len(),
                liveness_dependencies.len()
            ),
            (2, 4, 1, 1)
        );
    }

    /// Trace: FR-530-AC-1, FR-1044-AC-6
    #[test]
    fn admitted_fixture_keeps_a_real_two_component_cycle_and_its_first_authored_edge_region() {
        let fixture = AdmittedCompositionFixture::new(CompositionCase::Cyclic);
        assert_artifacts(&fixture);
        let [composition] = fixture.source.graph().compositions() else {
            panic!("one actual source composition");
        };
        let [cycle] = composition.cycles() else {
            panic!("one actual mutual component cycle");
        };
        assert_eq!(
            cycle
                .guarantees
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["Consumer::Done", "Producer::Sends"]
        );
        let authored = "depends Consumer::Done on Producer::Sends;";
        assert_eq!(
            &fixture.source_text[cycle.source.start..cycle.source.end],
            authored
        );
        assert_eq!(
            cycle.source.start,
            fixture.source_text.find(authored).unwrap()
        );
        let named = fixture.admitted.claim_name_for_path(&["Sys"]).unwrap();
        let Some(CheckedTerm::ComponentComposition {
            liveness_dependencies,
            ..
        }) = fixture.admitted.term(&named.declaration)
        else {
            panic!("the independently admitted cyclic composition");
        };
        assert_eq!(liveness_dependencies.len(), 2);
        let entries = fixture
            .admitted
            .source_map()
            .iter()
            .filter(|entry| {
                entry.node_id == named.declaration && entry.role == CheckedOccurrenceRole::Generated
            })
            .flat_map(|entry| &entry.regions)
            .filter(|region| {
                region.start == u64::try_from(cycle.source.start).unwrap()
                    && region.end == u64::try_from(cycle.source.end).unwrap()
            })
            .collect::<Vec<_>>();
        let [region] = entries.as_slice() else {
            panic!("one authentic generated member attribution");
        };
        assert_eq!(region.source, named.declaration_region.source);
    }
}
