---
id: SR-202611210022442313144108528
title: "QSL-2108 standalone backend agreement code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2108; Makefile, xtask/src/backend_agreement.rs, xtask/src/backend_agreement/driver.rs, xtask/src/main.rs, xtask/src/lib.rs, xtask/src/qualification.rs, xtask/src/qualification/adapters.rs, xtask/tests/backend_agreement.rs, xtask/tests/ci_targets.rs, FR-448, TC-1193"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-448", type: references}
---

## Summary

Reviewed PR3319 at 6b0e39f4663c1ccbdce6b692bfa2734a7e010e5c using the Rust review lane and the approved standalone value-replay scope. Three new defects affect real outcome decoding, item identity and corpus accounting.

## Verdict

**FAIL** pending FND-001 through FND-003. This is one review pass; no implementation changes were made by the reviewer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Production outcome categories are read from record instead of the item, suppressing definite comparisons. | xtask/src/backend_agreement.rs:159-165 |
| FND-002 | medium | A one-item driver response for another claim is accepted and reindexed without verifying its claim or capability. | xtask/src/backend_agreement/driver.rs:130-160 |
| FND-003 | medium | Compiled vectors with no resolved claim disappear from mixed-corpus reports. | xtask/src/backend_agreement/driver.rs:405-410;486-487 |

## Finding evidence

### FND-001

A real driver refutation is an item with category violation beside record.result refuted. The nested record has no category. Driver::execute retains this shape, while definite reads category from that nested record and returns None. A corresponding real proof is also discarded. Opposite backend verdicts therefore produce not-compared and exit zero rather than conflict, unless a separate replay failure happens first. The fixture instead inserts category inside record, so it cannot catch this production defect. Read the authoritative item category and cover real-shaped outcomes through the production adapter. Observed driver output is retained in native-refuted.json beside this review.

### FND-002

The process boundary checks document format, operation, one-item cardinality and terminal backend only. It then overwrites indices without checking claim_name, the echoed claim identity or capability. A schema-valid proof for OtherClaim from the requested backend is consequently attributed to Claim and can contribute agreement. Reject a response whose subject, capability or original single-item index differs from the requested item before remapping it. Add a production-boundary regression for that case.

### FND-003

The compiler collects only ResolvedItem::Claim, and execute emits entries only inside the resulting claim loop. Add a valid declaration-only vector (for example a unit declaring a record and no Boolean function) beside the existing comparable P/R vectors: it compiles but produces zero entries. Already-settled resolution outcomes are likewise filtered out. The report contract requires an entry for each selected vector yielding no item. Preserve a truthful not-compared reason for valid vectors without comparable claims and retain any already-settled unsupported/refusal information.

## Validation

No Cargo command was run by the reviewer: the parent reserved build admission and delegated runtime checks. The author reports focused cargo check of xtask lib, bin and two touched test targets passed at this head; that is compilation evidence only. Reviewer executed the existing fsl0.1.4 binary over an integer value claim: real refutation exit10, unavailable backend exit21, and unbounded value request exit21. These runs establish the wire shapes; they are not agreement-tool runtime pass evidence. git diff --check passed.

## Accepted boundaries

No automatic or aggregate gate wiring changed. Corpus/schema ownership is reused through qualification preflight. Native ValueTarget assignment evaluation and replay are real production calls. Temporal/protocol/hybrid replay remains explicitly evaluator_unsupported; the missing generic replay handoff and external-driver registry discovery remain named work on open QSL-2108, per the dispatching architecture ruling. No new AssuranceProfile applies.

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
- FND-002: **fixed** — verified against the changed production code and regression assertions.
- FND-003: **fixed** — verified against the changed production code and regression assertions.

```yaml
dispositions:
  - {"fnd":"FND-001","outcome":"fixed","after_excerpt":"let terminal = record.get(\"record\")?;\n    match (\n        terminal.get(\"result\")?.as_str()?,\n        record.get(\"category\")?.as_str()?,\n    ) {"}
  - {"fnd":"FND-002","outcome":"fixed","after_excerpt":"if record.get(\"claim_name\").and_then(Value::as_str) != Some(item.claim.claim_name.as_str())\n            || record.get(\"request_index\").and_then(Value::as_u64) != Some(0)\n            || echo.get(\"request_index\").and_then(Value::as_u64) != Some(0)\n            || echo.get(\"claim\") != expected.get(\"claim\")\n            || echo.get(\"kind\") != expected.get(\"kind\")\n            || echo.get(\"extent\") != expected.get(\"extent\")"}
  - {"fnd":"FND-003","outcome":"fixed","after_excerpt":"if claims.is_empty() && entries.is_empty() {\n        entries.push(failed_vector(\n            vector,\n            Settlement::NotCompared,\n            json!({\"reason\":\"no-resolved-verification-item\"}),\n        ));\n    }"}
```

### Round 2

Reviewed frozen source 940db8e4b877c070c60e5ba1010a8ffeead2d07e. All original findings remain fixed. The first Draco run returned 4 passed and 12 failed because pure Boolean helper functions produced no automatic verification items, so those fixtures never reached the comparisons. The approved correction selects P/R Claim items through the public FSL ItemSelection type, the actual resolver and FR-075; no claim, candidate, checked package or backend result is fabricated.

The new public run_with_selected_items API maps corpus vector IDs to explicit selections, rejects selections that name a backend and rejects unknown vector IDs before compilation/backend execution. Existing corpus/schema/settings preflight is retained. Vectors without entries in that map use the unchanged automatic discovery path. The CLI still calls that automatic path; it does not discover every Boolean helper. The mixed-corpus regression now distinguishes a real automatic x+1 obligation from an unselected Boolean helper and a declaration-only unit. The native decoder helper resolves the same explicit Claim selection its captured real request names. Core item/category decoding, subject/capability/index validation, raw-record retention, value replay and unresolved-vector accounting are unchanged by this round. No new bounded-scope defect was found.

```yaml
dispositions:
  - {"fnd":"FND-001","outcome":"fixed","after_excerpt":"let items = BTreeMap::from_iter([\"P\", \"R\"].map(|id| {\n                (\n                    id.to_owned(),\n                    vec![fsl_analyze::analyze::document::ItemSelection {\n                        claim: Some(\"Claim\".to_owned()),\n                        kind: Some(\"value-validity\".to_owned()),\n                        ..Default::default()\n                    }],\n                )\n            }));\n            gate::run_with_selected_items(&args, provider, &items)"}
  - {"fnd":"FND-002","outcome":"fixed","after_excerpt":"let items = BTreeMap::from_iter([\"P\", \"R\"].map(|id| {\n                (\n                    id.to_owned(),\n                    vec![fsl_analyze::analyze::document::ItemSelection {\n                        claim: Some(\"Claim\".to_owned()),\n                        kind: Some(\"value-validity\".to_owned()),\n                        ..Default::default()\n                    }],\n                )\n            }));\n            gate::run_with_selected_items(&args, provider, &items)"}
  - {"fnd":"FND-003","outcome":"fixed","after_excerpt":"let items = BTreeMap::from_iter([\"P\", \"R\"].map(|id| {\n                (\n                    id.to_owned(),\n                    vec![fsl_analyze::analyze::document::ItemSelection {\n                        claim: Some(\"Claim\".to_owned()),\n                        kind: Some(\"value-validity\".to_owned()),\n                        ..Default::default()\n                    }],\n                )\n            }));\n            gate::run_with_selected_items(&args, provider, &items)"}
```

## Disposition verdict

**CONDITIONAL** — all original findings are fixed by inspection; no new defect was found in the bounded fix diff. Parent-owned focused runtime checks remain required before merge. The author reports focused lib/bin/test compilation passed. Reviewer ran no Cargo. The production native refuted and unsupported fixtures are byte-equal to the real driver outputs retained in this scratchpad. The decoder regression invokes production read_response and replay, rejects eight changed identity fields, and covers real-shaped agreement/conflict/non-comparison. Mixed-corpus and already-settled resolution regressions cover retained entries.

## Final disposition verdict — round 2

**CONDITIONAL** — original code and gap findings remain fixed at 940db8e4b877c070c60e5ba1010a8ffeead2d07e. Focused lib/bin/test compilation is reported passed by the author; the same 16 runtime checks are queued on Draco and their final counts are pending. Reviewer ran no Cargo and makes no runtime-pass claim. There is no request for a broader audit, aggregate gate or additional fixture scope.
