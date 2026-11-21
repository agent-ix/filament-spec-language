// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-448 agreement over real corpus compilation and reference evaluation.

mod backend_agreement {
    use std::collections::BTreeSet;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use filament_exact::{Cancel, Integer, Value as RuntimeValue};
    use fsl_foundation::bound::DomainKind;
    use fsl_foundation::BackendId;
    use fsl_replay::value_check::{ValueTarget, Verdict};
    use fsl_route::{BackendDescriptor, Candidate, Mode, ProviderOrigin, Registry};
    use fsl_semantics::check::Capability;
    use ix_trace_rs::trace;
    use serde_json::{json, Value};
    use xtask::backend_agreement::{
        self as gate, CandidateRecord, DomainBound, Item, Negotiated, Provider, Recheck, Settlement,
    };
    use xtask::error::{Error, Result};

    const HOLDS: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction Claim using v(x: Int[0,10]): Boolean pure { x >= 0 }\n";
    const FAILS: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction Claim using v(x: Int[0,10]): Boolean pure { x < 0 }\n";

    struct Corpus {
        directory: tempfile::TempDir,
        document: Value,
    }

    impl Corpus {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let mut corpus = Self {
                directory,
                document: json!({
                    "contract_version":"filament.conformance-corpus/v1",
                    "vectors":[],"results":[],"coverage":{
                        "primary":[{"capability_id":"V1-TOOL-004","applicability":{
                            "consumer_types":["language"],"semantic_profiles":["filament.value.complete/v1"]}}],
                        "criteria_rollups":[],"consumer_rollups":[]
                    }
                }),
            };
            corpus.add("P", HOLDS);
            corpus.add("R", FAILS);
            corpus
        }

        fn add(&mut self, id: &str, source: &str) {
            let path = format!("{id}.native");
            fs::write(self.directory.path().join(&path), source).unwrap();
            self.document["vectors"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "id":id,"capability_id":"V1-TOOL-004","class":"positive",
                    "subject":{"consumer_type":"language","operation":"compile"},
                    "applicability":{"consumer_types":["language"]},
                    "input":{"files":[{"kind":"source","path":path}]},
                    "expected":{"disposition":"value","value":{}}
                }));
        }

        fn run(&self, provider: &FixtureProvider, options: &[&str]) -> Result<xtask::Outcome> {
            fs::write(
                self.directory.path().join("complete-v1.json"),
                serde_json_canonicalizer::to_vec(&self.document).unwrap(),
            )
            .unwrap();
            let mut args = vec![OsString::from("--corpus"), self.directory.path().into()];
            args.extend(options.iter().map(OsString::from));
            gate::run_with_provider(&args, provider)
        }
    }

    #[derive(Clone, Copy)]
    enum Plan {
        Reference,
        WrongProof,
        WrongRefutation,
        Inconclusive,
        RequiresBound,
    }

    struct FixtureProvider {
        registry: Registry,
        second: Plan,
        calls: AtomicUsize,
        missing_tool: bool,
        replay: Option<Recheck>,
    }

    impl FixtureProvider {
        fn new(second: Plan, count: usize) -> Self {
            let mut registry = Registry::new();
            for index in 0..count {
                let descriptor = BackendDescriptor::new(
                    Candidate::new(BackendId::new(format!("reference-{index}"))),
                    ProviderOrigin::Linked,
                    [
                        (Capability::ValueValidity, Mode::Bounded),
                        (Capability::ValueValidity, Mode::Unbounded),
                    ],
                    Some(BTreeSet::from([DomainKind::Integer])),
                    None,
                )
                .unwrap();
                registry.register(descriptor).unwrap();
            }
            Self {
                registry,
                second,
                calls: AtomicUsize::new(0),
                missing_tool: false,
                replay: None,
            }
        }
    }

    impl Provider for FixtureProvider {
        fn registry(&self) -> &Registry {
            &self.registry
        }
        fn preflight(&self, backends: &[BackendId]) -> Result<()> {
            if self.missing_tool {
                Err(Error::QualificationRefusal(format!(
                    "{}: missing test-tool",
                    backends[0].as_str()
                )))
            } else {
                Ok(())
            }
        }
        fn execute(&self, item: &Item<'_>, backend: &BackendId) -> Result<CandidateRecord> {
            let plan = if backend.as_str() == "reference-1" {
                self.second
            } else {
                Plan::Reference
            };
            if matches!(plan, Plan::RequiresBound) {
                return Ok(CandidateRecord {
                    backend: backend.as_str().to_owned(),
                    disposition: Negotiated::RequiresBound,
                    record: None,
                    declared_domain: None,
                    replay: None,
                    not_compared: None,
                });
            }
            self.calls.fetch_add(1, Ordering::SeqCst);
            let target = ValueTarget::select(
                item.checked.package(),
                &item.claim.claim_name,
                &item.claim.item.claim,
            )
            .unwrap();
            let assignment = vec![RuntimeValue::Integer(Integer::from(4))];
            let reference = target.evaluate(&assignment, &Cancel::new()).unwrap();
            let result = match plan {
                Plan::Reference => {
                    if matches!(reference, Verdict::Holds) {
                        "proved"
                    } else {
                        "refuted"
                    }
                }
                Plan::WrongProof => "proved",
                Plan::WrongRefutation => "refuted",
                Plan::Inconclusive => "inconclusive",
                Plan::RequiresBound => unreachable!(),
            };
            let parameter =
                fsl_foundation::node_wire::node_key_to_json(&target.parameters()[0].wire());
            let record = json!({
                "record":{"request_index":item.claim.item.request_index.get(), "backend":backend.as_str(),
                    "result":result,"category":if result == "proved" {"success"} else if result == "refuted" {"violation"} else {"inconclusive"},
                    "basis":{"type":"exhaustive"},"cause":if result == "inconclusive" {json!({"kind":"replay-parity"})} else {Value::Null}},
                "counterexample":{"type":"assignments","values":[{"parameter":parameter,"value":{"type":"integer","value":"4"}}]}
            });
            Ok(CandidateRecord {
                backend: backend.as_str().to_owned(),
                disposition: Negotiated::Supported,
                record: Some(record),
                declared_domain: None,
                replay: None,
                not_compared: None,
            })
        }
        fn replay(&self, item: &Item<'_>, _record: &CandidateRecord) -> Recheck {
            if let Some(replay) = &self.replay {
                return replay.clone();
            }
            let target = ValueTarget::select(
                item.checked.package(),
                &item.claim.claim_name,
                &item.claim.item.claim,
            )
            .unwrap()
            .with_limits(item.limits.accounting);
            match target.evaluate(&[RuntimeValue::Integer(Integer::from(4))], &Cancel::new()) {
                Ok(Verdict::Violated(expected)) => {
                    assert!(target
                        .replay(
                            &[RuntimeValue::Integer(Integer::from(4))],
                            &expected,
                            &Cancel::new()
                        )
                        .unwrap()
                        .is_some());
                    Recheck::ReproducedWithEvaluatedWitness
                }
                Ok(Verdict::Holds) => Recheck::Inconclusive,
                other => panic!("reference evaluation must decide the fixture: {other:?}"),
            }
        }
    }

    fn document(outcome: &xtask::Outcome) -> Value {
        serde_json::from_str(&outcome.stdout).unwrap()
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-1")]
    fn agreeing_backends_pass() {
        let outcome = Corpus::new()
            .run(&FixtureProvider::new(Plan::Reference, 2), &[])
            .unwrap();
        assert_eq!(outcome.code, 0, "{}", outcome.stdout);
        let report = document(&outcome);
        assert_eq!(report["counts"]["agree"], 2);
        assert_eq!(
            report["entries"][1]["candidates"][0]["replay"]["result"],
            "reproduced-with-evaluated-witness"
        );
        assert_eq!(
            report["entries"][0]["candidates"][1]["disposition"],
            "supported"
        );
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-2")]
    fn conflicting_verdicts_fail_the_gate() {
        let outcome = Corpus::new()
            .run(&FixtureProvider::new(Plan::WrongProof, 2), &[])
            .unwrap();
        assert_eq!(outcome.code, 1);
        let report = document(&outcome);
        assert_eq!(report["entries"][0]["settlement"], "agree");
        assert_eq!(report["entries"][1]["settlement"], "conflict");
        assert_eq!(
            report["entries"][1]["candidates"][0]["record"]["record"]["result"],
            "refuted"
        );
        assert_eq!(
            report["entries"][1]["candidates"][1]["record"]["record"]["result"],
            "proved"
        );
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-2")]
    fn a_wrong_proof_from_one_backend_fails() {
        let outcome = Corpus::new()
            .run(&FixtureProvider::new(Plan::WrongProof, 2), &[])
            .unwrap();
        assert_eq!(outcome.code, 1);
        assert_eq!(document(&outcome)["counts"]["conflict"], 1);
    }

    fn record(result: &str) -> CandidateRecord {
        CandidateRecord {
            backend: result.to_owned(),
            disposition: Negotiated::Supported,
            record: Some(
                json!({"record":{"result":result,"category":if result == "proved" {"success"} else {"violation"}},
                "counterexample":{"values":[{"parameter":"x","value":{"value":"12"}}]}}),
            ),
            declared_domain: None,
            replay: if result == "refuted" {
                Some(Recheck::ReproducedWithEvaluatedWitness)
            } else {
                None
            },
            not_compared: None,
        }
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-3")]
    fn bounded_proofs_compare_only_inside_the_declared_domain() {
        let mut proof = record("proved");
        proof.record.as_mut().unwrap()["record"]["basis"] =
            json!({"type":"bounded-complete","depth":3});
        let mut records = [proof.clone(), record("refuted")];
        assert_eq!(gate::compare(&mut records), Settlement::Conflict);
        proof.declared_domain = Some(vec![DomainBound {
            parameter: json!("x"),
            lower: "0".into(),
            upper: "10".into(),
        }]);
        records = [proof, record("refuted")];
        assert_eq!(gate::compare(&mut records), Settlement::NotCompared);
        assert!(records[0]
            .not_compared
            .as_ref()
            .unwrap()
            .contains("declared domain"));
        records[1].record.as_mut().unwrap()["counterexample"]["values"][0]["value"]["value"] =
            json!("4");
        assert_eq!(gate::compare(&mut records), Settlement::Conflict);
        for category in ["inconclusive", "incomplete", "unsupported", "tested"] {
            let mut other = record("inconclusive");
            other.record.as_mut().unwrap()["record"]["category"] = json!(category);
            let mut records = [record("proved"), other];
            assert_eq!(gate::compare(&mut records), Settlement::NotCompared);
            assert!(records[1].not_compared.as_ref().unwrap().contains(category));
        }
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-4")]
    fn a_wrong_refutation_from_one_backend_fails() {
        let outcome = Corpus::new()
            .run(&FixtureProvider::new(Plan::WrongRefutation, 2), &[])
            .unwrap();
        assert_eq!(outcome.code, 1);
        assert_eq!(document(&outcome)["entries"][0]["settlement"], "conflict");
        assert_eq!(
            document(&outcome)["entries"][0]["candidates"][1]["replay"]["result"],
            "inconclusive"
        );
        let outcome = Corpus::new()
            .run(&FixtureProvider::new(Plan::Inconclusive, 2), &[])
            .unwrap();
        assert_eq!(outcome.code, 0);
        assert_eq!(document(&outcome)["counts"]["not-compared"], 2);
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-5")]
    fn preflight_refusals_run_no_backend() {
        let corpus = Corpus::new();
        let mut provider = FixtureProvider::new(Plan::Reference, 2);
        provider.missing_tool = true;
        assert!(corpus
            .run(&provider, &[])
            .err()
            .expect("missing tool refuses")
            .to_string()
            .contains("reference-0: missing test-tool"));
        for options in [
            &["--limit", "s3.nodes=ten"][..],
            &["--capability", "CAP-Z"][..],
        ] {
            assert!(corpus.run(&provider, options).is_err());
        }
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        let provider = FixtureProvider::new(Plan::Reference, 1);
        assert!(corpus
            .run(&provider, &[])
            .err()
            .expect("no comparable item refuses")
            .to_string()
            .contains("no-comparable-item"));
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-5")]
    fn a_case_with_one_backend_only_is_reported_not_silently_counted_as_agreement() {
        let mut candidates = [record("proved")];
        assert_eq!(gate::compare(&mut candidates), Settlement::NotCompared);
        assert!(candidates[0].not_compared.is_some());
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-6")]
    fn repeated_reports_are_byte_equal_and_sorted_by_vector_then_index() {
        let corpus = Corpus::new();
        let provider = FixtureProvider::new(Plan::Reference, 2);
        let first = corpus.run(&provider, &[]).unwrap();
        let second = corpus.run(&provider, &[]).unwrap();
        assert_eq!(first.stdout, second.stdout);
        let report = document(&first);
        assert_eq!(report["entries"][0]["vector"], "P");
        assert_eq!(report["entries"][1]["vector"], "R");
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-7")]
    fn a_requires_bound_candidate_never_runs() {
        let provider = FixtureProvider::new(Plan::RequiresBound, 2);
        let outcome = Corpus::new().run(&provider, &[]).unwrap();
        assert_eq!(outcome.code, 0);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
        assert_eq!(document(&outcome)["counts"]["not-compared"], 2);
        assert_eq!(
            document(&outcome)["entries"][0]["candidates"][1]["disposition"],
            "requires-bound"
        );
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-8")]
    fn compile_refusals_do_not_fail_but_missing_source_does() {
        let provider = FixtureProvider::new(Plan::Reference, 2);
        let mut corpus = Corpus::new();
        corpus.add("T", "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function Wrong using v(): Boolean pure { 1 }");
        let outcome = corpus.run(&provider, &[]).unwrap();
        assert_eq!(outcome.code, 0, "{}", outcome.stdout);
        assert_eq!(document(&outcome)["counts"]["not-compiled"], 1);
        fs::remove_file(corpus.directory.path().join("T.native")).unwrap();
        let outcome = corpus.run(&provider, &[]).unwrap();
        assert_eq!(outcome.code, 1);
        assert_eq!(document(&outcome)["counts"]["tool-failure"], 1);
        assert_eq!(document(&outcome)["counts"]["agree"], 2);
    }

    #[test]
    #[trace("TC-1193", "FR-448-AC-9")]
    fn replay_refusals_are_not_definite_and_faults_fail() {
        let mut provider = FixtureProvider::new(Plan::WrongProof, 2);
        provider.replay = Some(Recheck::Refused {
            code: "resource_exhausted".into(),
            cause: "replay-limit".into(),
        });
        let outcome = Corpus::new().run(&provider, &[]).unwrap();
        assert_eq!(outcome.code, 0);
        assert_eq!(
            document(&outcome)["entries"][1]["settlement"],
            "not-compared"
        );
        assert_eq!(
            document(&outcome)["entries"][1]["candidates"][0]["replay"]["code"],
            "resource_exhausted"
        );
        provider.replay = Some(Recheck::Fault {
            detail: "injected-replay-invariant".into(),
        });
        let outcome = Corpus::new().run(&provider, &[]).unwrap();
        assert_eq!(outcome.code, 1);
        assert_eq!(
            document(&outcome)["entries"][1]["settlement"],
            "tool-failure"
        );
    }
}
