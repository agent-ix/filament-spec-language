// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-339-AC-9: a diagnostic made with memory fairness ignored carries no
//! replayable certificate, while a certificate under the original full TSO
//! fairness set still resolves and checks both branch `flush` members.

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::{Cancel, Meter, ScalarLimits, Value};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use fsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use fsl_eval::model_subject::{
    AtomStop, ModelInstance, ModelLetters, ModelSubject, ProtocolSubject, RoleBinding,
};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelCheckInput, ModelSimState, ModelSimulationRequest, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{
    FairnessOrigin, MemorySelection, ProtocolFairness, ProtocolSystem, Tso,
};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::{MemoryModelName, SourceIdentity};
use fsl_package::{CheckedPackage, EmitLimits};
use fsl_replay::{
    check_protocol_components, settle_protocol_check, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ProofCertificate, ProtocolCertificateRequest, TemporalCertificateCheck,
};
use fsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use fsl_semantics::check::{PackageDeclarations, Reward, RewardEvaluationFailure, RewardEvaluator};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, meaning, package_input};
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

const PACKAGE: &str = "test/tso-certificate";
const UNIT: &str = r#"
language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
profile t0 = "filament.temporal.infinite-trace/v1";
profile pp = "filament.protocol.finite-global/v1";
model M = "test/tso-certificate";
pre IsX using v on M::Cell::set { self.id = 0 }
post StoreOne using v on M::Cell::set { self.v = 1 }
pre ReadsX1 using v on M::Core::read1 { c.id = 0 }
post Read1 using v on M::Core::read1 { self.r1 = pre(c.v) }
protocol Spin using pp over (k: M::Core) on origin {
  role w on M::Cell;
  role c on M::Core;
  run parallel Both {
    branch w sequence W {
      attempt Sx by w on M::Cell::set ordering relaxed contracts [IsX, StoreOne]
          as (written: Boolean) { true };
    }
    branch r repeat Spin by c visible () while { true } sequence R {
      attempt Lx by c on M::Core::read1 ordering relaxed contracts [ReadsX1, Read1]
          as (loaded: Boolean) { true };
    }
  } join all [w, r];
  finish Done as (done: Boolean) { done };
}
temporal Settles using t0 over (k: M::Core) clock "model-steps" on origin {
  eventually holds(k.r1 = 1)
}
"#;

struct ZeroReward;

impl RewardEvaluator for ZeroReward {
    fn evaluate(
        &self,
        _graph: &fsl_semantics::check::CheckedGraph,
        _reward: &Reward,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Value, RewardEvaluationFailure> {
        Ok(Value::Integer(filament_exact::Integer::from(0_i64)))
    }
}

fn ident(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn origin(identity: &str) -> Json {
    json!({"generated":{"generatorIdentity":identity,"generatorVersion":"1.0.0","inputIdentities":[identity]}})
}

fn multiplicity() -> Json {
    json!({"lower":1,"upper":1,"ordered":false,"unique":true})
}

fn field(owner: &str, name: &str) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    json!({"identity":identity,"name":name,"typeRef":ident("Level"),
        "presence":"required","nullable":false,"defaultKind":"none",
        "multiplicity":multiplicity(),"origin":origin(&identity)})
}

fn operation(owner: &str, name: &str, parameters: &[&str], modifies: &[&str]) -> Json {
    let identity = ident(&format!("{owner}/{name}"));
    let params = parameters
        .iter()
        .map(|parameter| {
            let parameter_identity = format!("{identity}/{parameter}");
            json!({"identity":parameter_identity,"name":parameter,"typeRef":ident("Cell"),
                "presence":"required","nullable":false,"defaultKind":"none",
                "multiplicity":multiplicity(),"origin":origin(&parameter_identity)})
        })
        .collect::<Vec<_>>();
    json!({"identity":identity,"name":name,"params":params,"pre":[],"post":[],
        "origin":{"source":{"sourceIdentity":ident("spec"),"path":"spec.fspec","startLine":1,"startColumn":1}},
        "frame":{"modifies":[],"modifies_receiver":modifies.iter()
            .map(|name| ident(&format!("{owner}/{name}"))).collect::<Vec<_>>(),
            "creates":[],"deletes":[]}})
}

fn model_type(identity: &str, kind: &str, members: Json) -> Json {
    let mut ty = json!({"identity":ident(identity),"displayName":identity,
        "kind":{"module":PACKAGE,"name":kind},"roles":[],"origin":origin(&ident(identity)),
        "constraints":[],"extensions":[],"unknownPolicy":"reject","supertypes":[],
        "relationships":[]});
    for (name, value) in members.as_object().expect("model type members").iter() {
        ty[name] = value.clone();
    }
    ty
}

fn document() -> Vec<u8> {
    let level_identity = ident("Level");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity":format!("{level_identity}/constraints/{keyword}"),"keyword":keyword,
            "operands":{"value":value},"appliesTo":"ix://filament/native/Integer",
            "diagnosticCode":format!("bound.{keyword}"),"origin":origin(&level_identity),
        })
    };
    let level = model_type(
        "Level",
        "value_type",
        json!({"scalar":"integer","constraints":[constraint("min","0"),constraint("max","3")]}),
    );
    let constructs = [
        ("object_type", meaning::OBJECT_TYPE),
        ("population", meaning::POPULATION),
        ("value_type", meaning::VALUE_TYPE),
    ]
    .into_iter()
    .map(|(name, meaning)| {
        json!({"kind":{"module":PACKAGE,"name":name},"moduleVersion":"1.0.0",
        "construct":{"identity":"none","shape":"record","members":{},"meaning":meaning}})
    })
    .collect::<Vec<_>>();
    let cell = model_type(
        "Cell",
        "object_type",
        json!({"fields":[field("Cell","id"),field("Cell","v")],
            "operations":[operation("Cell","set",&[],&["v"])],
            "relationships":[]}),
    );
    let core = model_type(
        "Core",
        "object_type",
        json!({"fields":[field("Core","r1")],
            "operations":[operation("Core","read1",&["c"],&["r1"])],
            "relationships":[]}),
    );
    let population = |name: &str, member: &str| {
        json!({"identity":ident(name),"displayName":ident(name),
        "kind":{"module":PACKAGE,"name":"population"},"members":[ident(member)],
        "extent":"closed","origin":origin(&ident(name))})
    };
    json!({"contractVersion":"2.0.0","canonicalization":{"algorithm":"rfc8785-v1"},
        "source":{"identity":ident("spec"),"version":"1.0.0","dialect":"spec-bundle"},
        "package":{"identity":PACKAGE,"version":"1.0.0","mappingVersions":[],"profileVersions":[]},
        "occurrences":[],"extensions":[],"constructs":constructs,
        "types":[level,cell,core],"populations":[population("cells","Cell"),population("cores","Core")]})
        .to_string().into_bytes()
}

struct Documents {
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: DocumentRef,
    universes: Vec<PopulationUniverse>,
}

impl Documents {
    fn new() -> Self {
        let int = |value: i64| json!({"integer":value.to_string()});
        let snapshot = json!({"format":"filament.state.snapshot/v1",
            "identity":{"authority":"test","identity":"s0"},"observation":"current",
            "model":{"identity":PACKAGE},"anchor":{"kind":"initialization","name":"start"},
            "populations":[
                {"population":ident("cells"),"complete":true,"objects":[{"key":"x","type":ident("Cell"),
                    "fields":{"id":int(0),"v":int(0)}}]},
                {"population":ident("cores"),"complete":true,"objects":[{"key":"core","type":ident("Core"),
                    "fields":{"r1":int(3)}}]}]});
        let bytes = snapshot.to_string().into_bytes();
        let initial = fsl_semantics::model::observation::document_reference(&bytes)
            .expect("snapshot names itself");
        let packages = package_input([document().as_slice()]).expect("the model admits");
        let snapshots = BTreeMap::from([(initial.clone(), bytes)]);
        let universes = [("cells", vec!["x"]), ("cores", vec!["core"])]
            .into_iter()
            .map(|(name, keys)| PopulationUniverse {
                population: filament_checked::key::DeclarationKey {
                    package: PACKAGE.to_owned(),
                    node: ident(name),
                },
                keys: keys.into_iter().map(str::to_owned).collect(),
            })
            .collect();
        Self {
            packages,
            snapshots,
            initial,
            universes,
        }
    }

    fn subject<'a>(
        &'a self,
        package: &'a CheckedPackage,
        selection: &'a [MemorySelection],
    ) -> ProtocolSubject<'a> {
        let checked = package.graph().protocol("Spin").expect("protocol Spin");
        ProtocolSubject {
            model: ModelSubject {
                simulation: ModelSimulationRequest {
                    emit: EmitLimits::new(1 << 32),
                    input: ModelCheckInput::source(package),
                    packages: &self.packages,
                    normalization: ModelNormalizationLimits::default(),
                    snapshots: &self.snapshots,
                    initial: vec![self.initial.clone()],
                    universes: self.universes.clone(),
                    observation: ObservationLimits::default(),
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                    meter: ScalarLimits {
                        integer_bits: 1 << 20,
                        decimal_digits: 1 << 20,
                        scale_expansion: 1 << 20,
                        text_input_bytes: 1 << 20,
                        text_scalars: 1 << 20,
                        normalized_scalars: 1 << 20,
                        unit_edges: 1 << 20,
                        value_occurrences: 1 << 20,
                        work_units: 1 << 30,
                        result_units: 1 << 30,
                    },
                },
                proof_bounds: Vec::new(),
            },
            protocol: "Spin".to_owned(),
            over: ModelInstance {
                population: filament_checked::key::DeclarationKey {
                    package: PACKAGE.to_owned(),
                    node: ident("cores"),
                },
                key: "core".to_owned(),
            },
            roles: ["w", "c"]
                .into_iter()
                .map(|role| RoleBinding {
                    role: role.to_owned(),
                    model: PACKAGE.to_owned(),
                })
                .collect(),
            capacities: Vec::new(),
            memory: fsl_eval::simulation::protocol::ResolvedMemory::resolve(checked, selection)
                .expect("TSO selection resolves"),
        }
    }
}

fn checked_package() -> CheckedPackage {
    let models = package_input([document().as_slice()]).expect("model package input");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:tso-certificate"),
        "unit.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 forms");
    let admitted = admit_unit(
        &unit.selections().models,
        &models,
        ModelNormalizationLimits::default(),
    )
    .expect("model selected");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/tso-certificate".to_owned(),
        unit,
        admitted,
        Vec::new(),
    )
    .expect("I1 assembles");
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default(), &ZeroReward)
        .unwrap_or_else(|refusal| panic!("S3 checks: {refusal:?}"));
    CheckedPackage::link(graph)
}

struct ReadOne;

impl ModelLetters for ReadOne {
    fn atom(
        &self,
        _atom: fsl_semantics::check::FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let field_key = filament_checked::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: ident("Core/r1"),
        };
        let value = state
            .state()
            .populations
            .get(&instance.population)
            .and_then(|population| population.get(&instance.key))
            .and_then(|object| object.fields.get(&field_key))
            .ok_or_else(|| {
                AtomStop::Fault(InternalFault::new("tso-certificate", "missing-core-r1"))
            })?;
        let Value::Integer(value) = value else {
            return Err(AtomStop::Fault(InternalFault::new(
                "tso-certificate",
                "core-r1-is-not-an-integer",
            )));
        };
        let value = value
            .to_string()
            .parse::<i64>()
            .expect("the finite model value fits i64");
        Ok(if value == 1 {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &self,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(AtomReading::True)
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }

    fn reads(
        &self,
        _atom: fsl_semantics::check::FormulaNodeId,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }
}

static READ_ONE: ReadOne = ReadOne;

fn selection() -> [MemorySelection; 1] {
    [MemorySelection {
        parallel: "Spin::Both".to_owned(),
        model: MemoryModelName::Tso,
    }]
}

fn clause(package: &CheckedPackage) -> &fsl_semantics::check::CheckedTemporalClause {
    package.graph().temporal_clauses()[0].clause()
}

fn item(package: &CheckedPackage) -> ModelCheckItem<'_> {
    ModelCheckItem::Clause {
        clause: clause(package),
        letters: &READ_ONE,
    }
}

fn protocol_request<'a>(
    documents: &'a Documents,
    package: &'a CheckedPackage,
    selection: &'a [MemorySelection],
    ignore_memory_constraints: bool,
) -> ProtocolCheckRequest<'a> {
    ProtocolCheckRequest {
        temporal_clock_selections: &[],
        subject: documents.subject(package, selection),
        item: item(package),
        max_depth: None,
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
        limits: ModelCheckLimits::default(),
        ignore_memory_constraints,
    }
}

fn certificate_request<'a>(
    documents: &'a Documents,
    package: &'a CheckedPackage,
    selection: &'a [MemorySelection],
) -> ProtocolCertificateRequest<'a> {
    ProtocolCertificateRequest {
        temporal_clock_selections: &[],
        subject: documents.subject(package, selection),
        item: item(package),
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    }
}

fn flush(branch: &str) -> ProtocolFairness {
    ProtocolFairness::memory_step(MemoryModelName::Tso, "flush", "Both", branch)
}

/// The diagnostic outcome has no component-certificate variant and cannot
/// enter replay. The normal request checks the original complete TSO set.
#[trace("TC-673", "FR-228-AC-10", "FR-339-AC-9")]
#[test]
fn ignored_memory_diagnostic_has_no_certificate_and_full_fairness_still_checks() {
    let package = checked_package();
    let documents = Documents::new();
    let selection = selection();
    let diagnostic = check_protocol(
        protocol_request(&documents, &package, &selection, true),
        &Cancel::new(),
    )
    .expect("diagnostic request checks");
    assert!(matches!(
        diagnostic,
        ModelCheckOutcome::RefutedIgnoringConstraints { .. }
    ));

    let request = certificate_request(&documents, &package, &selection);
    let record = settle_protocol_check(
        &diagnostic,
        &ModelCheckLimits::default(),
        &request,
        &Cancel::new(),
        |_| panic!("a diagnostic lasso must not enter standard replay"),
    );
    assert!(matches!(
        record.value,
        fsl_replay::TerminalValue::Inconclusive(
            fsl_replay::InconclusiveCause::RefutedIgnoringConstraints
        )
    ));
    assert!(record.counterexample.is_none());

    let full = check_protocol(
        protocol_request(&documents, &package, &selection, false),
        &Cancel::new(),
    )
    .expect("the original TSO request checks");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(certificate),
        ..
    } = full
    else {
        panic!("full fairness yields its component certificate: {full:?}");
    };
    let request = certificate_request(&documents, &package, &selection);
    assert_eq!(
        check_protocol_components(
            &request,
            &certificate,
            &ModelCheckLimits::default(),
            &Cancel::new()
        )
        .expect("all original fairness constraints are checked"),
        TemporalCertificateCheck::Accepted
    );

    let subject = documents.subject(&package, &selection);
    let model = fsl_eval::simulation::model::ModelSystem::new(subject.model.simulation.clone())
        .expect("the subject model admits");
    let system = ProtocolSystem::new(model, &subject, Tso::new(4)).expect("TSO builds");
    let memory: Vec<_> = system
        .fairness_set(clause(&package).fairness())
        .expect("the complete fairness set resolves")
        .into_iter()
        .filter(|member| member.origin() == FairnessOrigin::Memory)
        .collect();
    assert_eq!(memory.len(), 2, "{memory:?}");
    assert!(memory.contains(&flush("w")), "{memory:?}");
    assert!(memory.contains(&flush("r")), "{memory:?}");
    assert!(memory
        .iter()
        .all(|member| (member.kind(), member.granularity())
            == (FairnessKind::Weak, FairnessGranularity::Each)));
}
