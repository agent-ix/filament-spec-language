---
id: SR-202611210022442313144108529
title: "QSL-2108 backend agreement trace and evidence gap review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2108; Makefile, xtask/src/backend_agreement.rs, xtask/src/backend_agreement/driver.rs, xtask/src/main.rs, xtask/src/lib.rs, xtask/src/qualification.rs, xtask/src/qualification/adapters.rs, xtask/tests/backend_agreement.rs, xtask/tests/ci_targets.rs, FR-448, TC-1193"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-448", type: references}
---

## Summary

Computed the repository matrix and coverage reports, then inspected the changed production surface and FR-448 tests. All nine FR-448 criteria are tagged, but the fixture payload shape masks the production outcome decoder defect. Baseline repository gaps are reported separately from this PR.

## Verdict

**FAIL** for the changed-scope evidence defect below. This is not a full-repository assurance PASS and does not require this PR to repair unrelated historical gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Tagged agreement fixtures invent nested record.category and never expose the production reader mismatch. | xtask/tests/backend_agreement.rs:193-200 |

## Evidence

The reference fixture calls real native evaluation but constructs record.category, a member absent from the real FR-286 terminal record. Every agreement and wrong-proof seam test therefore exercises the synthetic shape rather than the production item. The malformed fixture and absence of a production-driver-shaped regression let code-review FND-001 survive. Retain the real evaluator seam, correct the item structure, and add a production adapter outcome test.

## Coverage

- Reconciliation: quoin matrix0.36.8 (quire0.52.0, engine0.62.2), exit0; quire coverage exit0. Raw JSON and stderr are in this review scratchpad.
- Plan completion: not assessed
- Matrix:632 requirements,6136 criteria; tagged4825, untagged1111, tagged-by-ignored-test83, method-without-symbol104, retired13. These whole-repository counts are baseline context, not new PR findings.
- FR-448:9 tagged criteria,0 untagged or ignored-only criteria. No run evidence exists for any of the6136 matrix criteria; the evidence store was not treated as a runtime pass.
- Coverage reports TC-1193 test-case tags as unmatched while resolving all FR-448 criterion tags. TC-1193 exists in spec/test-cases; those marker observations are retained as extraction context and not invented missing-criterion findings.
- Reverse inspection covered the new command, corpus intake, candidate execution, comparison, value replay, report and Make boundary. All map to FR-448 and its dependency contracts. No vendored corpus/schema and no hollow source stub were found in the changed surface. Known unavailable replay families are explicit reported limits and named remaining work, not disguised implementation.
- Full repository reverse inventory was not represented as a new-PR audit. Optional semantic-review workflow skipped; ordinary code/test alignment was performed under code-review.
- Reviewer runtime Cargo checks were not admitted; focused author compilation is reported separately in the code review.

## Examined scope

```yaml
scope:
  - {"id":"FR-448-AC-1","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"Over a fixture corpus with one item that two registered backends both prove and one item that both refute with a counterexample that replays `ReproducedWithEvaluatedWitness`, both items are `agree`, the report names each backend's disposition and terminal record and the gate's replay result for each refutation, and the gate exits 0."}
  - {"id":"FR-448-AC-2","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"With a test backend registered that answers `proved` for the item the other backend refutes, that item is `conflict`, the report names the item, both `BackendId`s and both terminal records, and the gate exits 1; the agreeing item stays `agree`."}
  - {"id":"FR-448-AC-3","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A `BoundedComplete{depth: 3}` proof against a reproduced counterexample is `conflict` and exits 1. A `bounded`-mode proof over the declared domain `0..=10` against a reproduced counterexample with witness 12 is `not-compared`, naming the domain, and exits 0; with witness 4 it is `conflict` and exits 1. A `proved` record against an `inconclusive` (including `BoundReached`), `incomplete`, `unsupported` or `tested` record is `not-compared`, naming that record's category, and exits 0."}
  - {"id":"FR-448-AC-4","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A test backend that returns a `refuted` record whose counterexample the gate's replay does not reproduce makes the item `conflict`, naming that backend, its record and the replay result, and the gate exits 1. A test backend whose provider settled `inconclusive` with `replay_parity` leaves the item `not-compared` against another backend's `proved`, the entry holding that cause, and the gate exits 0."}
  - {"id":"FR-448-AC-5","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A run with a candidate whose tool is not installed refuses naming that `BackendId` and the tool; a corpus whose in-scope items each have one candidate refuses with `no-comparable-item`; a malformed `--limit` and an unknown `--capability` each refuse naming the operand. Each exits 2 and runs no backend."}
  - {"id":"FR-448-AC-6","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"Two runs over the AC-1 corpus give byte-equal JSON reports, with entries in ascending bytewise vector id and then request index order."}
  - {"id":"FR-448-AC-7","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A candidate whose negotiation settles `requires-bound` does not run and is reported with that disposition; with one other candidate settled `supported`, the item is `not-compared`."}
  - {"id":"FR-448-AC-8","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"Adding a vector whose `compile` refuses `ill_typed` yields a `not-compiled` entry holding the refusal and leaves the exit status at 0. A vector naming a source file absent from the corpus yields a `tool-failure` entry holding the I/O error, the other entries keep their settlements, and the gate exits 1."}
  - {"id":"FR-448-AC-9","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A `refuted` record whose re-check replay is run under a replay limit it reaches is reported not definite, holding the replay refusal and its catalog code; against another backend's `proved` the item is `not-compared` and the gate exits 0. A `refuted` record whose re-check replay faults makes the item `tool-failure`, holding the fault, and the gate exits 1."}
  - {"id":"FR-448","path":"spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md","role":"examined","excerpt":"A JSON report (text with `--text`) and the exit status. The report holds one entry per in-scope item and one per vector that yields no item, in ascending order of vector id (bytewise) and then request index."}
  - {"id":"production-comparison","path":"xtask/src/backend_agreement.rs","role":"examined","excerpt":"Apply ADR-013 O-16 without choosing a preferred backend or repairing a verdict."}
  - {"id":"production-driver","path":"xtask/src/backend_agreement/driver.rs","role":"examined","excerpt":"The explicit driver process and native corpus/compiler adapters."}
  - {"id":"agreement-tests","path":"xtask/tests/backend_agreement.rs","role":"examined","excerpt":"FR-448 agreement over real corpus compilation and reference evaluation."}
  - {"id":"standalone-target","path":"Makefile; xtask/src/main.rs; xtask/src/lib.rs; xtask/tests/ci_targets.rs","role":"examined","excerpt":"backend-agreement remains a standalone command and Make target outside ci."}
  - {"id":"shared-preflight","path":"xtask/src/qualification.rs; xtask/src/qualification/adapters.rs","role":"examined","excerpt":"Shared corpus, schema, scope and settings preflight. No vector executes here."}
```

## Dispositions

### Round 1

Reviewed fix head a08c8ccbf91316b0e2ce20ce07a7f4ccff20213c. Original findings above are unchanged.

- FND-001: **fixed** — verified against the changed production code and regression assertions.

```yaml
dispositions:
  - {"fnd":"FND-001","outcome":"fixed","after_excerpt":"let document: Value = serde_json::from_slice(REFUTED).unwrap();\n            let backend = BackendId::new(\"value-checker\");\n            let mut refutation = driver.read_response(item, &backend, REFUTED).unwrap();\n            assert_eq!(refutation.record.as_ref().unwrap(), &document[\"items\"][0]);"}
```

### Round 2

Reviewed frozen source 940db8e4b877c070c60e5ba1010a8ffeead2d07e. All original findings remain fixed. The first Draco run returned 4 passed and 12 failed because pure Boolean helper functions produced no automatic verification items, so those fixtures never reached the comparisons. The approved correction selects P/R Claim items through the public FSL ItemSelection type, the actual resolver and FR-075; no claim, candidate, checked package or backend result is fabricated.

The new public run_with_selected_items API maps corpus vector IDs to explicit selections, rejects selections that name a backend and rejects unknown vector IDs before compilation/backend execution. Existing corpus/schema/settings preflight is retained. Vectors without entries in that map use the unchanged automatic discovery path. The CLI still calls that automatic path; it does not discover every Boolean helper. The mixed-corpus regression now distinguishes a real automatic x+1 obligation from an unselected Boolean helper and a declaration-only unit. The native decoder helper resolves the same explicit Claim selection its captured real request names. Core item/category decoding, subject/capability/index validation, raw-record retention, value replay and unresolved-vector accounting are unchanged by this round. No new bounded-scope defect was found.

```yaml
dispositions:
  - {"fnd":"FND-001","outcome":"fixed","after_excerpt":"let items = BTreeMap::from_iter([\"P\", \"R\"].map(|id| {\n                (\n                    id.to_owned(),\n                    vec![fsl_analyze::analyze::document::ItemSelection {\n                        claim: Some(\"Claim\".to_owned()),\n                        kind: Some(\"value-validity\".to_owned()),\n                        ..Default::default()\n                    }],\n                )\n            }));\n            gate::run_with_selected_items(&args, provider, &items)"}
```

## Disposition verdict

**CONDITIONAL** — all original findings are fixed by inspection; no new defect was found in the bounded fix diff. Parent-owned focused runtime checks remain required before merge. The author reports focused lib/bin/test compilation passed. Reviewer ran no Cargo. The production native refuted and unsupported fixtures are byte-equal to the real driver outputs retained in this scratchpad. The decoder regression invokes production read_response and replay, rejects eight changed identity fields, and covers real-shaped agreement/conflict/non-comparison. Mixed-corpus and already-settled resolution regressions cover retained entries.

## Final disposition verdict — round 2

**CONDITIONAL** — original code and gap findings remain fixed at 940db8e4b877c070c60e5ba1010a8ffeead2d07e. Focused lib/bin/test compilation is reported passed by the author; the same 16 runtime checks are queued on Draco and their final counts are pending. Reviewer ran no Cargo and makes no runtime-pass claim. There is no request for a broader audit, aggregate gate or additional fixture scope.
