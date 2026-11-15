// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 (TC-2100): `analyze` runs a hybrid item on the hybrid provider
//! negotiation routed it to, and settles a hybrid item with no provider
//! `unsupported`, `HybridProviderAbsent`, and one routed to a backend no
//! engine serves `unsupported`, `tool-unavailable`.

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRequest, ClaimItem, Engines, HybridItem, ItemForm, ItemOptions,
    NegotiatedItem, SourceProvision, SubjectBinding,
};
use fsl_analyze::provider;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex, SourceIdentity};
use fsl_package::CheckedPackage;
use fsl_replay::{TerminalRecord, TerminalValue, UnavailabilityCause};
use fsl_route::{request::ExtentClassification, routing::Disposition, Candidate};
use fsl_semantics::check::{Capability, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input, PackageInput};
use fsl_semantics::model::observation::DocumentProvision;
use ix_trace_rs::trace;

/// FR-447's `Braking` and `SlowEnough`.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always [0, 10] holds(h.v < 30) }\n";

/// The checked hybrid package, its empty model input and snapshots, and the
/// provision of its one source.
struct World {
    package: CheckedPackage,
    packages: PackageInput,
    snapshots: DocumentProvision,
    provision: SourceProvision,
}

fn world() -> World {
    let packages = package_input(std::iter::empty::<&[u8]>()).expect("no model documents");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:hybrid-provider"),
        "hybrid.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("no model selection");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckMode::Linked, CheckingLimits::default())
    .expect("the unit checks");
    World {
        provision: SourceProvision::single(graph.source(), UNIT.as_bytes()),
        package: CheckedPackage::link(graph),
        packages,
        snapshots: DocumentProvision::new(),
    }
}

impl World {
    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: Vec::new(),
            universes: Vec::new(),
        }
    }

    /// `SlowEnough`'s hybrid item with `candidates` and `disposition`.
    fn item(&self, candidates: Vec<Candidate>, disposition: Disposition) -> NegotiatedItem {
        let [site] = self.package.graph().hybrid_claim_sites() else {
            panic!("one hybrid claim, SlowEnough");
        };
        NegotiatedItem {
            item: ClaimItem {
                request_index: RequestIndex::new(0),
                claim: OccurrenceKey::new(site.identity.clone(), site.claim.clone()),
                kind: Capability::TemporalSatisfaction,
                extent: ExtentClassification::Bounded,
                backend: None,
                candidates,
                options: ItemOptions {
                    required: true,
                    ..ItemOptions::default()
                },
                form: Some(ItemForm::Hybrid(HybridItem::default())),
            },
            disposition,
            diagnostic: None,
        }
    }

    /// The item's record through `analyze` and through `provider::run`.
    fn settle(&self, item: NegotiatedItem) -> [TerminalRecord; 2] {
        let engines = Engines::fsl().expect("the layer-A manifests register");
        let through_analyze = analyze(
            &self.package,
            &AnalyzeRequest {
                binding: self.binding(),
                provision: self.provision.clone(),
                items: vec![item.clone()],
            },
            &engines,
            &AnalyzeLimits::unbounded(),
            &Cancel::new(),
        )
        .expect("the binding is admitted")
        .into_value()
        .records
        .remove(0);
        let through_run = provider::run(
            &self.package,
            &self.binding(),
            &self.provision,
            &[item],
            &engines,
            &AnalyzeLimits::unbounded(),
            &Cancel::new(),
        )
        .records
        .remove(0);
        [through_analyze, through_run]
    }
}

/// FR-750-AC-3: with no hybrid provider registered (an empty candidate set),
/// the hybrid item settles `unsupported`, `HybridProviderAbsent`,
/// `unsupported_projection`/`unsupported-requested-capability`, through
/// `analyze` and through `provider::run`.
#[trace("TC-2100", "FR-750-AC-3")]
#[trace("FSpec-TC-373", "FSpec-FR-193-AC-7")]
#[test]
#[ignore = "QSL-2234: analyze settles a provider-less hybrid item HybridProviderAbsent not implemented yet"]
fn a_hybrid_item_with_no_provider_settles_hybrid_provider_absent() {
    let world = world();
    for record in world.settle(world.item(Vec::new(), Disposition::Unsupported)) {
        assert_eq!(
            record.value(),
            &TerminalValue::Unsupported(UnavailabilityCause::HybridProviderAbsent),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unsupported_projection",
                "unsupported-requested-capability"
            ))
        );
    }
}

/// FR-750-AC-4: a hybrid item routed to `hybrid-fixture`, which no
/// registered engine or process provider serves, settles `unsupported`,
/// `unsupported_projection`/`tool-unavailable`, naming that backend.
#[trace("TC-2100", "FR-750-AC-4")]
#[test]
#[ignore = "QSL-2234: analyze dispatch of a hybrid item to its negotiated provider not implemented yet"]
fn a_hybrid_item_routed_to_an_unserved_backend_settles_tool_unavailable() {
    let world = world();
    let fixture = Candidate::new(BackendId::new("hybrid-fixture"));
    for record in world.settle(world.item(
        vec![fixture.clone()],
        Disposition::Supported(fixture.clone()),
    )) {
        assert!(
            matches!(record.value(), TerminalValue::Unsupported(_)),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unsupported_projection",
                "tool-unavailable"
            )),
            "{record:?}"
        );
        assert_eq!(
            record.backend().map(BackendId::as_str),
            Some("hybrid-fixture")
        );
    }
}
