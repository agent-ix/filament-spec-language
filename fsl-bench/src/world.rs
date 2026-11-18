// SPDX-License-Identifier: AGPL-3.0-or-later
//! A parametrised model world for the model-check, sampling and
//! key-encoding axes: one domain package (Semantic IR 2.0.0) with one object
//! type in one closed population, a native unit that selects it as `M` and
//! writes the axis's contracts and claims after the header, and one initial
//! snapshot.
//!
//! Everything is built through the layers' public API: S1 parse, S2 build,
//! model intake, S3 check, link. A refusal is a defect of the generator, not
//! a measured result, so every constructor returns it as an error for the
//! axis's smallest-size test to fail on.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::ScalarLimits;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, meaning, package_input, PackageInput};
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use serde_json::{json, Value as Json};

/// The native Boolean type reference.
pub const BOOLEAN: &str = "ix://filament/native/Boolean";

/// The evaluation budget every axis runs under: ample, so a bench measures
/// the engine and never a meter stop.
pub const UNLIMITED: ScalarLimits = ScalarLimits {
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
};

/// The candidate ceiling of one expansion every axis runs under.
const MAX_CANDIDATES: u64 = 100_000_000;

/// Why a world did not build: the generator and the layer disagree.
#[derive(Debug)]
pub struct WorldError(pub String);

impl std::fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WorldError {}

fn refused(stage: &str, detail: impl std::fmt::Debug) -> WorldError {
    WorldError(format!("{stage}: {detail:?}"))
}

/// One operation: its name, its parameters as `(name, type reference)` and
/// the fields of its receiver its frame lets it write.
pub struct Operation {
    /// The operation's name.
    pub name: String,
    /// The parameters, `(name, type reference)`.
    pub parameters: Vec<(String, String)>,
    /// The receiver's fields the frame writes.
    pub modifies: Vec<String>,
}

/// One world's description.
pub struct Model {
    /// The domain package's identity, `bench/<name>`.
    pub package: String,
    /// The object type's name.
    pub object: String,
    /// The population's name.
    pub population: String,
    /// Whether the population is declared symmetric.
    pub symmetric: bool,
    /// Bounded integer types: name, lower, upper.
    pub integers: Vec<(String, i64, i64)>,
    /// The object type's fields: name and type reference.
    pub fields: Vec<(String, String)>,
    /// The object type's operations.
    pub operations: Vec<Operation>,
    /// Header lines after the model profile, for example further `profile`
    /// declarations.
    pub profiles: String,
    /// The unit's declarations after the model selection.
    pub body: String,
    /// The live objects of the initial snapshot: key and field values.
    pub objects: Vec<(String, Vec<(String, i64)>)>,
    /// Unrelated record types: each is an object type with one field, in a
    /// closed population of its own with no object, which the snapshot
    /// lists complete.
    pub unrelated: usize,
}

impl Model {
    /// The identity of `suffix` in this package.
    pub fn ident(&self, suffix: &str) -> String {
        format!("ix://{}/{suffix}", self.package)
    }

    /// The type reference of the bounded integer `name`.
    pub fn int(&self, name: &str) -> String {
        self.ident(name)
    }

    fn origin(identity: &str) -> Json {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    }

    fn definition(&self, name: &str, kind: &str, members: Json) -> Json {
        let identity = self.ident(name);
        let mut node = json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": self.package, "name": kind}, "roles": [],
            "origin": Self::origin(&identity), "constraints": [], "extensions": [],
            "unknownPolicy": "reject",
        });
        if let (Some(target), Some(source)) = (node.as_object_mut(), members.as_object()) {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        }
        node
    }

    fn integer_type(&self, name: &str, lower: i64, upper: i64) -> Json {
        let identity = self.ident(name);
        let constraint = |keyword: &str, value: i64| {
            json!({
                "identity": format!("{identity}/constraints/{keyword}"), "keyword": keyword,
                "operands": {"value": value.to_string()},
                "appliesTo": "ix://filament/native/Integer",
                "diagnosticCode": format!("bound.{keyword}"), "origin": Self::origin(&identity),
            })
        };
        self.definition(
            name,
            "value_type",
            json!({"scalar": "integer",
                "constraints": [constraint("min", lower), constraint("max", upper)]}),
        )
    }

    fn typed(identity: String, name: &str, type_ref: &str) -> Json {
        json!({
            "identity": identity.clone(), "name": name, "typeRef": type_ref,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": Self::origin(&identity),
        })
    }

    fn operation(&self, operation: &Operation) -> Json {
        let identity = self.ident(&format!("{}/{}", self.object, operation.name));
        let parameters: Vec<Json> = operation
            .parameters
            .iter()
            .map(|(name, type_ref)| Self::typed(format!("{identity}/{name}"), name, type_ref))
            .collect();
        let modifies: Vec<String> = operation
            .modifies
            .iter()
            .map(|field| self.ident(&format!("{}/{field}", self.object)))
            .collect();
        json!({
            "identity": identity, "name": operation.name, "params": parameters,
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": self.ident("spec"), "path": "spec.fspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": [], "modifies_receiver": modifies, "creates": [],
                "deletes": []},
        })
    }

    fn unrelated_name(index: usize) -> String {
        format!("Unrelated{index}")
    }

    fn unrelated_population(index: usize) -> String {
        format!("unrelated{index}s")
    }

    /// The domain package document.
    fn document(&self) -> Vec<u8> {
        let mut kinds = vec![
            ("object_type", meaning::OBJECT_TYPE),
            ("population", meaning::POPULATION),
        ];
        if !self.integers.is_empty() || self.unrelated > 0 {
            kinds.push(("value_type", meaning::VALUE_TYPE));
        }
        let constructs: Vec<Json> = kinds
            .into_iter()
            .map(|(name, meaning)| {
                json!({"kind": {"module": self.package, "name": name}, "moduleVersion": "1.0.0",
                    "construct": {"identity": "none", "shape": "record", "members": {},
                        "meaning": meaning}})
            })
            .collect();
        let fields: Vec<Json> = self
            .fields
            .iter()
            .map(|(name, type_ref)| {
                Self::typed(
                    self.ident(&format!("{}/{name}", self.object)),
                    name,
                    type_ref,
                )
            })
            .collect();
        let mut types: Vec<Json> = self
            .integers
            .iter()
            .map(|(name, lower, upper)| self.integer_type(name, *lower, *upper))
            .collect();
        if self.unrelated > 0 {
            types.push(self.integer_type("Spare", 0, 1));
        }
        types.push(self.definition(
            &self.object,
            "object_type",
            json!({
                "supertypes": [], "fields": fields,
                "operations": self.operations.iter().map(|operation| self.operation(operation))
                    .collect::<Vec<_>>(),
                "relationships": [],
            }),
        ));
        let mut populations =
            vec![self.population_node(&self.population, &self.ident(&self.object), self.symmetric)];
        for index in 0..self.unrelated {
            let name = Self::unrelated_name(index);
            let field = Self::typed(self.ident(&format!("{name}/x")), "x", &self.ident("Spare"));
            types.push(self.definition(
                &name,
                "object_type",
                json!({"supertypes": [], "fields": [field], "operations": [],
                    "relationships": []}),
            ));
            populations.push(self.population_node(
                &Self::unrelated_population(index),
                &self.ident(&name),
                false,
            ));
        }
        json!({
            "contractVersion": "2.0.0",
            "canonicalization": {"algorithm": "rfc8785-v1"},
            "source": {"identity": self.ident("spec"), "version": "1.0.0",
                "dialect": "spec-bundle"},
            "package": {"identity": self.package, "version": "1.0.0",
                "mappingVersions": [], "profileVersions": []},
            "occurrences": [], "extensions": [],
            "constructs": constructs,
            "types": types,
            "populations": populations,
        })
        .to_string()
        .into_bytes()
    }

    fn population_node(&self, name: &str, member: &str, symmetric: bool) -> Json {
        let population = self.ident(name);
        json!({
            "identity": population, "displayName": population,
            "kind": {"module": self.package, "name": "population"},
            "members": [member], "extent": "closed", "symmetric": symmetric,
            "origin": Self::origin(&population),
        })
    }

    /// The unit's source text.
    pub fn unit(&self) -> String {
        format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n\
             {}model M = \"{}\";\n{}\n",
            self.profiles, self.package, self.body
        )
    }

    fn snapshot(&self) -> Vec<u8> {
        let objects: Vec<Json> = self
            .objects
            .iter()
            .map(|(key, fields)| {
                let fields: serde_json::Map<String, Json> = fields
                    .iter()
                    .map(|(field, value)| (field.clone(), json!({"integer": value.to_string()})))
                    .collect();
                json!({"key": key, "type": self.ident(&self.object), "fields": fields})
            })
            .collect();
        let mut populations = vec![json!({"population": self.ident(&self.population),
            "complete": true, "objects": objects})];
        for index in 0..self.unrelated {
            populations.push(
                json!({"population": self.ident(&Self::unrelated_population(index)),
                "complete": true, "objects": []}),
            );
        }
        json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "bench", "identity": "s0"},
            "observation": "current", "model": {"identity": self.package},
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": populations,
        })
        .to_string()
        .into_bytes()
    }
}

/// A checked world with its initial snapshot.
pub struct World {
    /// The linked checked package.
    pub package: CheckedPackage,
    /// The domain package input.
    pub packages: PackageInput,
    /// The provisioned snapshots.
    pub snapshots: DocumentProvision,
    /// The initial snapshots' references.
    pub initial: Vec<DocumentRef>,
    /// The universes: the object population over the live keys, and each
    /// unrelated population over none.
    pub universes: Vec<PopulationUniverse>,
    /// The population's key.
    pub population: DeclarationKey,
    package_name: String,
}

impl World {
    /// `model`, checked.
    ///
    /// # Errors
    ///
    /// [`WorldError`] naming the stage that refused the generated unit.
    pub fn build(model: &Model) -> Result<Self, WorldError> {
        let document = model.document();
        let packages = package_input([document.as_slice()]).map_err(|e| refused("intake", e))?;
        let unit = model.unit();
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("bench", "mc"),
            "mc.native",
            unit.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .map_err(|e| refused("S1", e))?;
        if !parsed.is_admissible() {
            return Err(refused("S1", parsed.diagnostics()));
        }
        let built = fsl_forms::build_unit(&parsed).map_err(|e| refused("S2", e))?;
        let models = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .map_err(|e| refused("model selection", e))?
        .with_parsed_operation_clauses(|text| {
            fsl_forms::parse_clause_text(text, fsl_cst::Limits::default())
        })
        .map_err(|e| refused("operation clauses", e))?;
        let graph = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "package".to_owned(),
            built,
            models,
            Vec::new(),
        )
        .map_err(|e| refused("assemble", e))?
        .check(CheckMode::Linked, CheckingLimits::default())
        .map_err(|e| refused("S3", e))?;

        let snapshot = model.snapshot();
        let reference = fsl_semantics::model::observation::document_reference(&snapshot)
            .ok_or_else(|| refused("snapshot reference", "none"))?;
        let mut snapshots = BTreeMap::new();
        snapshots.insert(reference, snapshot);
        let key = |population: &str| DeclarationKey {
            package: model.package.clone(),
            node: model.ident(population),
        };
        let mut universes = vec![PopulationUniverse {
            population: key(&model.population),
            keys: model.objects.iter().map(|(key, _)| key.clone()).collect(),
        }];
        universes.extend((0..model.unrelated).map(|index| PopulationUniverse {
            population: key(&Model::unrelated_population(index)),
            keys: Vec::new(),
        }));
        Ok(Self {
            package: CheckedPackage::link(graph),
            packages,
            snapshots,
            initial: vec![DocumentRef {
                authority: "bench".to_owned(),
                identity: "s0".to_owned(),
            }],
            universes,
            population: key(&model.population),
            package_name: model.package.clone(),
        })
    }

    /// The declaration key of `suffix` in this world's package.
    pub fn key(&self, suffix: &str) -> DeclarationKey {
        DeclarationKey {
            package: self.package_name.clone(),
            node: format!("ix://{}/{suffix}", self.package_name),
        }
    }

    /// The simulation request over the initial snapshot.
    pub fn simulation(&self) -> ModelSimulationRequest<'_> {
        ModelSimulationRequest {
            emit: fsl_package::EmitLimits::new(1 << 32),
            input: fsl_eval::simulation::model::ModelCheckInput::source(&self.package),
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes.clone(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: MAX_CANDIDATES,
            },
            meter: UNLIMITED,
        }
    }

    /// The model subject over the initial snapshot.
    pub fn subject(&self) -> ModelSubject<'_> {
        ModelSubject {
            simulation: self.simulation(),
            proof_bounds: Vec::new(),
        }
    }
}
