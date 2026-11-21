---
id: SR-20261121003343004
title: "spec-review/ears-conformance — package main-red repairs"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2271; frozen PR3343; fsl-package/src/introduced_binders.rs; spec/functional/FR-531; tests/it/{bare_package,temporal_package_route,temporal_package_route_formula}.rs"
review_set: subset
---

## Summary

Ticket: QSL-2271; draft PR #3343. Frozen head `92ef76b5db56573f8319f7d8faa3727e9e7ec6ae`, base `660a550b40b068df5337155009e009299654f3b1`. Exactly five changed files were examined. No Cargo, build, runtime test, pin, checkout, source edit or branch mutation was performed. Delivery remains after the current QSL-2726 slice; this is review readiness, not test acceptance or ticket completion.

Tools: quoin 0.36.8; quire 0.52.0 (engine 0.62.2); spec-artifacts-process v0.36.0. The installed authoring contract was fetched through `quoin write . --types SpecReview`. No applicable frontmatter `type: AssuranceProfile` exists in the local specification tree. The assigned bounded review selects code/Rust, scoped gap analysis, specification base and EARS. Other specification lenses are not separately applicable: no artifact IDs, relationship edges, schemas, wire grammar, evidence method, dependency boundary or domain-object definition changes. Existing requested capability inventory repair is not new repository file-tracking ceremony.

## Verdict

PASS — no findings in the explicitly assigned source/specification review scope. Runtime verification and delivery hold remain in force.

## Requirement-grammar assessment

The changed capability table row describes existing method names, argument context and accounting. It adds no SHALL statement, event/state trigger or altered acceptance criterion. Its two-model-variant clause names a concrete additional input and FR-107, without introducing an ambiguous response. Explicit ISO validation exits 0; the existing unrelated `ac:vague-response` advisory at unchanged line 1049 is not a new finding in the assigned row-only scope. No new EARS defect is found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Synthetic capability-row part suffixes locate verbatim fragments; they are not new requirement IDs. Longer existing criteria are split into verbatim excerpts.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-390-AC-68",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "Over recipe **R**'s `Full` recovery predicate (`sum(receipt in recovered.receipts: receipt) = target and not activated`), the slot lists the sources `target`, `activated` and `recovered` and not `receipt`, on the bare route; the same holds with the `sum` replaced by a `let` binder bound to it, with two nested collection queries of distinct names, with each of `forall`, `exists`, `filter` and `map`, and with a binder the body never reads (alone and nested), none of those binders being a source. The genuine `case Shape::Circle(sum<M::Total>(receipt in recoveredFull.receipts: receipt)) "
    },
    {
      "id": "FR-390-AC-68",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "{ Empty: targetFull; Circle(r): r; }` form in `Full`'s recovery predicate is admitted by the strict reader, its arm binder `r` is not a source on either route, and source and bare routes return equal Boolean outcomes and ordered charge logs for matching, mismatching, empty and activated supplies. A supply that adds a key for the introduced binder refuses `invalid_runtime_input`/`unknown-member` before any charge, and the predicate evaluates to the value its text states over a supply of the three sources. A query binder named as a readable source is refused by the checker as `Ambiguou"
    },
    {
      "id": "FR-390-AC-68",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "sName`."
    },
    {
      "id": "FR-531-AC-9",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "For each public method of `EvidencePackage` and each method of the `CheckedPackageEvaluation` trait, enumerated by a test that reads those two items' declarations in the crate source with `syn` and compares the names to the first column of the inventory table, the table has exactly one row: available from the bare package, or listed as not carried with a reason; a method added to either type without a row fails the test."
    },
    {
      "id": "FR-531-AC-24",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "`V2ReadLimits` has a required `storage` member and no `Default`; a read's evaluation view over limits whose `storage` is exactly the measured join storage succeeds and over one unit less returns `ViewFailure::Exhausted` with `ExhaustedStorage`, whose refusal names `i2.native_join_storage`; `EvidencePackage::from_read` and `from_emission` over a budget one unit short of the source join return `PackageMismatch::Exhausted`; the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted` and writes no directory."
    },
    {
      "id": "FR-531-capability-row-part-1",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "| `evaluate_holds`, `evaluate_model_holds`, `evaluate_holds_traced`, `evaluate_guard`, `evaluate_model_guard`, `evaluate_hyper_holds`, `evaluate_step_predicate`, `evaluate_clause`, `evaluate_clause_set`, `evaluate_value_invariants`, `check_separation`, `evaluate_frame`, `evaluate_collection_write`, `evaluate_state_predicate`, `evaluate_state_predicate_at_absent_key` | required in a package form for analysis of a selected package (FR-940 \"Package forms of the source-checked evaluators\"): each source form takes a source-checked clause, node or state clause, which analysis from source k"
    },
    {
      "id": "FR-531-capability-row-part-2",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "eeps reading from its `CheckedGraph`; the two model variants additionally take the reference bindings for FR-107 model-read accounting; the evidence route takes the `holds` valuations as request facts (FR-516) |"
    },
    {
      "id": "FSpec-FR-322-AC-460",
      "path": "filament-specification:spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined",
      "excerpt": "The package of a unit declaring the temporal clause `Bounded` and the invariant state clause `NotAboveThree` carries, on the `Bounded` `temporal_clause_names` row and the `NotAboveThree` `claim_names` row, a `declaration_region` whose bytes are the whole declaration from its first keyword to its closing `}` and which contains the occurrence's name-token `source_map` region; the same unit, with `Bounded` declared last, with whitespace added right before `Bounded`'s closing `}` changes no `node_id` or node."
    },
    {
      "id": "FSpec-FR-322-AC-461",
      "path": "filament-specification:spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined",
      "excerpt": "AC-460's package with the `Bounded` row's `declaration_region` removed refuses `malformed_wire` at the row (`/temporal_clause_names/0`), as the closed schema refuses any missing member; with it shrunk to one byte that does not contain the name token, or naming another source, refuses `invalid_package`/`invalid-value` at `/temporal_clause_names/0/declaration_region`; the same edits on the `NotAboveThree` row refuse at `/claim_names/{i}/declaration_region`."
    },
    {
      "id": "Walk::term::Opening::Let-part-1",
      "path": "fsl-package/src/introduced_binders.rs",
      "role": "examined",
      "excerpt": "                    arguments,\n                    ..\n                }) => match opening(operation.identity(), arguments) {\n                    Opening::Let {\n                        name,\n                        binder,\n                        value,\n                        body,\n                    } => {\n                        if let Some(binder) = binder {\n                            // The binder link declares a parameter; it does\n                            // not read that parameter in the outer scope.\n                            covered.insert(binder.clone());\n             "
    },
    {
      "id": "Walk::term::Opening::Let-part-2",
      "path": "fsl-package/src/introduced_binders.rs",
      "role": "examined",
      "excerpt": "           }\n                        let inner = self.scoped(&shadow, name);\n                        reading.push((Reading::Term(body), inner));\n                        reading.push((Reading::Term(value), shadow));\n                    }\n                    Opening::Query { source, name, body } => {\n                        let inner = self.scoped(&shadow, name);\n                        reading.push((Reading::Term(body), inner));\n"
    },
    {
      "id": "Built::dependency_artifacts-part-1",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "    /// The retained recursive dependency artifacts in selected-identity order.\n    pub(super) fn dependency_artifacts(\n        &self,\n    ) -> Vec<(&str, &Emission, &filament_contract_model::CheckedPackageV2)> {\n        let mut held = BTreeMap::new();\n        let mut pending = vec![self.unit.package()];\n        while let Some(package) = pending.pop() {\n            for (identity, dependency) in package.dependencies() {\n                if held.insert(identity, dependency.as_ref()).is_none() {\n                    pending.push(dependency.as_ref());\n                }\n            }\n      "
    },
    {
      "id": "Built::dependency_artifacts-part-2",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "  }\n        held.into_iter()\n            .map(|(identity, dependency)| {\n                (\n                    identity.as_str(),\n                    dependency.remembered_emission().expect(\"retained emission\"),\n                    self.unit\n                        .admitted_libraries()\n                        .package(identity)\n                        .expect(\"retained library admission\"),\n                )\n            })\n            .collect()\n    }\n\n    /// This unit's in-process package with the emission of `other`.\n"
    },
    {
      "id": "writer-budget-denial-part-1",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "        let mut oracle = limits.budget();\n        let _ = materialize_dependencies(&mut oracle);\n        let IdentityFileRefusal::Rows(expected) = built\n            .emission\n            .identity_rows(read.admitted(), &mut oracle)\n            .expect_err(\"one-short root-identity allowance refuses its final copy\")\n        else {\n            panic!(\"the root-identity denial retains its compact row cause\");\n        };\n        assert_eq!(expected.denied(), oracle.first_unpaid());\n        assert_eq!(expected.locus(), oracle.first_unpaid_locus());\n        let mut asked = request(\"event_po"
    },
    {
      "id": "writer-budget-denial-part-2",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "sition\", Corpus::None);\n        asked.emit.package_name = package_name.clone();\n        asked.native_join = limits;\n        let target = scratch.path().join(format!(\"{code:?}\"));\n        match write(&asked, &target) {\n            Err(error @ Error::NativeJoinExhausted(_)) => {\n                assert_eq!(error.code(), ErrorCode::NativeJoinExhausted);\n                let Error::NativeJoinExhausted(denial) = error else {\n                    unreachable!()\n                };\n                let fsl_package::NativeJoinFailure::Rows(native) = &denial.native else {\n                    panic"
    },
    {
      "id": "writer-budget-denial-part-3",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "!(\n                        \"the final producer copy retains its compact cause: {:?}\",\n                        denial.native\n                    )\n                };\n                assert_eq!(*native, expected, \"resource, amount, locus and full bound\");\n                assert_eq!(\n                    native.code(),\n"
    },
    {
      "id": "declaration-region-refusal-part-1",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "            let row = &mut wire[\"temporal_clause_names\"][0];\n            row.as_object_mut().unwrap().remove(\"declaration_region\");\n        });\n        let Err(refused) = removed.try_read(&removed.emission) else {\n            panic!(\"a row without declaration_region is refused\");\n        };\n        let refusal = native_refusal(refused);\n        assert_eq!(\n            refusal.code,\n            filament_contract_model::CheckedPackageRefusalCode::MalformedWire\n        );\n        assert_eq!(\n            refusal\n                .loci\n                .first()\n                .and_then(|lo"
    },
    {
      "id": "declaration-region-refusal-part-2",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "cation| location.path.as_ref())\n                .map(|path| path.as_str()),\n            Some(\"/temporal_clause_names/0\")\n"
    },
    {
      "id": "record-aggregate-fixture-part-1",
      "path": "tests/it/temporal_package_route_formula.rs",
      "role": "examined",
      "excerpt": "fn a_clause_with_an_aggregate_atom_is_read_from_its_samples() {\n    let clauses = \"record Sample { level: Int[0, 9]; }\\n\\\n        temporal Fed using t over (e: Sample) clock \\\"samples\\\" \\\n        on origin { always[0,2] (count over past[0,1] min_count 1 >= 1) }\\n\\\n        temporal Starved using t over (e: Sample) clock \\\"samples\\\" \\\n        on origin { always[0,2] (count over past[0,1] min_count 5 >= 1) }\";\n    let built = event_clauses(clauses, &[(\"Fed\", \"fed\"), (\"Starved\", \"starved\")]);\n    let read = built.read(&built.emission);\n    let package = EvidencePackage::from_emission(\n"
    }
  ],
  "findings": [],
  "bindings": []
}
```


## Dispositions

### Scope amendment 1

**PASS / review clear** at `c4b68629b62303ed3aa805766ef6a1e14b88f813`. Original review and findings above are preserved verbatim. This pass adds the actual host-reported `the_evaluator_reads_the_clause_only_through_the_facts` failure to scope; it is not an assertion that the historical three unit-test passes exercised this new test. No original or new finding remains.

Only `tests/it/bare_package.rs` changes after custody `32205d`: the existing tagged FR-532-AC-9 test now recursively visits directories, reads every regular file regardless of extension, sorts the complete source-path list, and reports path-specific failures. Unexpected entry kinds fail rather than being followed or skipped. No host-specific failing directory basename was provided or inferred. The original forbidden names, line-comment filtering, nonempty real-source assertion and LedgerBinding check remain.

The temporary layout proves both forbidden types in a nested regular source and SourceIdentity in a non-.rs regular file are detected, with a literal expected violation list. Therefore skipping directories, filtering to .rs, or dropping either forbidden-name check makes this fixture fail. The actual production directory still has to return no violations; this repairs layout handling rather than exempting a source-policy violation.

No Cargo or runtime test was run by this reviewer. The parent reports the three existing library unit tests passed on Titan at earlier custody `32205d`; those are contextual remote evidence only, not current-head root-test acceptance. The six exact root checks remain unexecuted at review time. Test requests belong to Titan under the newest lane instruction, and delivery remains held until #3354 lands. No integration, merge or new native behavior is authorized by this review.

The specification and capability row are unchanged since the initial reviewed source. Their original method verdict stands; no new requirement wording, EARS trigger or spec artifact is introduced by this test-only scope addition.

### Scope added in amendment 1

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-532-AC-9",
      "path": "spec/functional/FR-532-evaluate-temporal-evidence-from-the-checked-package-s-own-nodes.md",
      "role": "examined",
      "excerpt": "A search of `fsl-eval/src/temporal/evidence` for the symbols `CheckedTemporalClause::activation`, `::profile`, `::clock` and `SourceIdentity` finds that the evaluator and the three constructors reach the activation, the profile and the clock only through `ClauseFacts`: an inspection finds no read of `CheckedTemporalClause::activation`, `profile` or `clock` outside the cross-check, and `fsl_foundation::SourceIdentity` is no member of `LedgerBinding`."
    },
    {
      "id": "recursive-regular-file-scan-part-1",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "    fn violations(directory: &std::path::Path) -> Vec<(std::path::PathBuf, &'static str)> {\n        let mut pending = vec![directory.to_owned()];\n        let mut sources = Vec::new();\n        while let Some(directory) = pending.pop() {\n            let mut entries: Vec<_> = std::fs::read_dir(&directory)\n                .unwrap_or_else(|error| panic!(\"{}: {error}\", directory.display()))\n                .map(|entry| {\n                    entry.unwrap_or_else(|error| panic!(\"{}: {error}\", directory.display()))\n                })\n                .collect();\n            entries.sort_by_key"
    },
    {
      "id": "recursive-regular-file-scan-part-2",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "(std::fs::DirEntry::file_name);\n            for entry in entries {\n                let path = entry.path();\n                let kind = entry\n                    .file_type()\n                    .unwrap_or_else(|error| panic!(\"{}: {error}\", path.display()));\n                if kind.is_dir() {\n                    pending.push(path);\n                } else if kind.is_file() {\n                    sources.push(path);\n                } else {\n                    panic!(\"unexpected source entry kind: {}\", path.display());\n                }\n            }\n        }\n        sources.sort();\n   "
    },
    {
      "id": "recursive-regular-file-scan-part-3",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "     assert!(!sources.is_empty(), \"{}: no sources\", directory.display());\n        let mut found = Vec::new();\n        for path in sources {\n            let text = std::fs::read_to_string(&path)\n                .unwrap_or_else(|error| panic!(\"{}: {error}\", path.display()));\n            let code = text\n                .lines()\n                .filter(|line| !line.trim_start().starts_with(\"//\"))\n                .collect::<Vec<_>>()\n                .join(\"\\n\");\n            for forbidden in [\"SourceIdentity\", \"CheckedTemporalClause\"] {\n                if code.contains(forbidden) {\n       "
    },
    {
      "id": "recursive-regular-file-scan-part-4",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "             found.push((path.clone(), forbidden));\n                }\n            }\n        }\n        found\n    }\n"
    },
    {
      "id": "adversarial-layout-fixture-part-1",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "\n    // Module subdirectories and every regular file retain the same forbidden\n    // type checks; neither directory nor extension filtering may hide a read.\n    let fixture = tempfile::tempdir().expect(\"source-layout fixture\");\n    let nested = fixture.path().join(\"nested\");\n    std::fs::create_dir(&nested).expect(\"nested module directory\");\n    let nested_source = nested.join(\"mod.rs\");\n    std::fs::write(\n        &nested_source,\n        \"pub struct SourceIdentity;\\npub struct CheckedTemporalClause;\\n\",\n    )\n    .expect(\"nested forbidden-type source\");\n    std::fs::write(fixture.p"
    },
    {
      "id": "adversarial-layout-fixture-part-2",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "ath().join(\"mod.rs\"), \"mod nested;\\n\").expect(\"root source\");\n    let other_source = fixture.path().join(\"other.inc\");\n    std::fs::write(&other_source, \"pub struct SourceIdentity;\\n\")\n        .expect(\"a source without an rs extension\");\n    assert_eq!(\n        violations(fixture.path()),\n        vec![\n            (nested_source.clone(), \"SourceIdentity\"),\n            (nested_source, \"CheckedTemporalClause\"),\n            (other_source, \"SourceIdentity\"),\n        ]\n    );\n\n    let directory =\n        std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"fsl-eval/src/temporal/evidence"
    },
    {
      "id": "adversarial-layout-fixture-part-3",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "\");\n    let forbidden = violations(&directory);\n    assert!(forbidden.is_empty(), \"{forbidden:?}\");\n"
    },
    {
      "id": "real-evaluator-and-ledger-policy-part-1",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "    let ledger = read_source(\"fsl-eval/src/temporal/ledger.rs\");\n    let binding = ledger\n        .split(\"pub struct LedgerBinding\")\n        .nth(1)\n        .and_then(|rest| rest.split('}').next())\n        .expect(\"LedgerBinding\");\n    assert!(!binding.contains(\"SourceIdentity\"));\n}\n\n/// FR-531-AC-11 (TC-1331): FR-515 no longer calls the bytes-only evaluator\n/// out of scope, and FR-515, FR-516 and FR-520 each carry a \"Consumer entry\"\n/// section naming FR-531.\n#[trace(\"FR-531-AC-11\", \"TC-1331\")]\n"
    }
  ],
  "findings": [],
  "dispositions": []
}
```
