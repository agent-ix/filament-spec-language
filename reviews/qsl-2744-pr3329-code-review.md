---
id: SR-20261121154325
title: "code review of PR 3329: analyze results carry document-held node identities"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3329 @ 2f028646b92f918fd0c160fea1819e9b4868278c; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/document/held_identity.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/tests/it/probability_from_source.rs, tools/fsl-probability-check/tests/it/roundtrip.rs"
review_set: subset
---

## Summary

Ticket: QSL-2744. Code review with the Rust lane (rust-review) folded in. The rewrite in `held_identity::hold` is a pure re-spelling: `WireGraph::wire_id` keeps `package`, every text segment and `ordinal`, and maps each operand to the graph index of the node with the same structural key, so `WireGraph::key` rebuilds the same key. The graph is built lazily, once per run. Four gaps remain. The independent checker cannot read the certificate document that analyze now writes. Counterexample, record and accounting members, and simulated items, keep structural ids. Dependency-operand ids are left nested without any marker, for every item and not only for composition premises. The walker does not skip foreign `{package, node}` containers, which `WireGraph::identity_value` does skip.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | fsl-probability-check cannot read the certificate document analyze now writes; TC-764 (FR-282-AC-1) only passes because the test converts the document back to structural form and sets model to null | tools/fsl-probability-check/tests/it/roundtrip.rs:571-592,629-632 |
| FND-002 | high | hold covers only certificate, counterpart, request and verdict. counterexample (TypedValue/NodeId, subsystem ProbabilityCertificate), record causes, accounting.constraint and simulated items keep structural ids, so the ticket's "every analyze result identity" is not met | fsl-analyze/src/analyze/document.rs:4385,4277 |
| FND-003 | medium | OperandNotInGraph is accepted silently for every item, including own-package items, which leaves a schema-invalid nested id with no marker | fsl-analyze/src/analyze/document/held_identity.rs:70-77 |
| FND-004 | medium | The walker does not skip foreign {package, node} and dependency_reference containers, so a foreign node whose operands also exist in the importer graph is written with importer indexes that FR-322 resolves against the dependency graph | fsl-analyze/src/analyze/document/held_identity.rs:60-95 |
| FND-005 | medium | A failed graph build (emit limit, strict-reader refusal) turns into AnalyzeDocumentError::Fault for the whole run, and its cause is lost; runs that wrote a result before now fault | fsl-analyze/src/analyze/document.rs:3806-3810,4388 |
| FND-006 | low | The settle_run closure re-emits and strictly re-reads the package even when the provider route already holds the shared admitted emission | fsl-analyze/src/analyze/document.rs:3806-3810 |
| FND-007 | low | The new refuted test is tagged FR-741-AC-3 but asserts none of AC-3 (counterexample, replay). No FR-621 AC states the document-held form or the non-null model | fsl-analyze/tests/it/probability_from_source.rs:1080-1106 |

## Scope and evidence

Diff-only focused review; pre-existing defects out of scope. No builds or test runs (dispatcher instruction). Quoin gap-analysis was not run as a separate method: the dispatcher scoped this to one diff pass. The manual AC-to-test check is recorded as FND-007. Governing text read: FSpec FR-322 "Operand index segments", "Scope" and "Dependency references"; backend-provider-v1 schema CertificateIdentity, ProbabilityCertificate, Locus, NodeId (checked-package-v2 PathSegment: string | NodeIndex | OperandSlot), ProbabilisticCounterexample, AccountingRecord; FSL FR-621 member table and FR-282-AC-1; FR-741-AC-2/3.

- FR-621 (spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md), examined: `identity` is FSpec FR-412's `CertificateIdentity` `{node_id, model, ordinal}` for every certificate kind.
- FR-282-AC-1 (spec/functional/FR-282-*.md), examined: The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and passed with the byte provision and the subject binding to the EN-5 checker in a fresh process that links only qualified-core crates, is accepted.
- FR-741-AC-3 (spec/functional/FR-741-request-and-report-probability-items-through-analyze.md), examined: An exact item for a claim whose exact value is below its threshold settles `refuted` with a `counterexample` and an item-level `replay` that is present and reproduced.
- FSpec FR-322 Operand index segments, context_only: A document-held id writes each operand as an index segment {"node": i} naming semantic_graph.nodes[i]; a node id that names a node of another package's graph is document-held in that graph; a path never holds a node_id object.

## Verdict

A fix round is needed (FND-001, FND-002 high). Verified clean: the re-spelling is identity-preserving; `is_structural_id` only matches 3-member id objects with a nested-id segment, so declared all-string ids and already-held ids are left alone; a user value could only collide through a TypedValue spelling that cannot produce that shape; the graph is built at most once per run and only when an item needs holding; the walk is linear in the member size; the diff contains no Python or logic-bearing shell. Leaving dependency-operand ids nested needs a spec change (FSL FR-621 plus one FSpec FR-322 Scope sentence) before the code can be completed; the proposal is in the ticket comment.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | Picks the graph by an id's own `package` (hold_each, map_ids for request.premises and provider premises), against FSpec FR-322 Scope ("a node_id's own package names the owner, not a graph"); changes the FR-331 provider payload with no FSpec change | fsl-analyze/src/analyze/document/held_identity.rs:106-175; fsl-analyze/src/provider.rs:371-381,545-548 |
| FND-009 | medium | Reading restated premises emits every direct dependency under AnalyzeLimits::unbounded().emit, ignoring caller limits, and silently drops any that do not emit | fsl-analyze/src/analyze/document/composition_item.rs:236-241,531-546 |
| FND-010 | medium | hold_item falls back to the own graph when the item's dependency has no graph (the package route passes none), yet still writes `package` naming the dependency | fsl-analyze/src/analyze/document.rs:4377; fsl-analyze/src/analyze/document/package_run.rs:538-541 |
| FND-011 | medium | FR-621 "Node ids in an item" has a broken sentence, claims simulation items are held (document.rs:4279 still skips them), reads restated premises "in either form", and puts dependency ids in the dependency graph by id package | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:135-154 |
| FND-012 | low | FR-621-AC-16 and TC-3154 claim the counterexample validates as CounterexampleEntry, which no test asserts; the AC-17 tag sits on a test that asserts none of AC-17; AC-17 omits the composition exception | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:310-311; fsl-analyze/tests/it/analyze_composition_cli.rs:397,790 |
| FND-013 | low | HeldCertificate::read spots the held form only from an index segment in identity.node_id.path | fsl-replay/src/probability_certificate/source.rs:406-413 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checker resolves the held document against request.package or the recompiled package, requires identity.model == request.item.model, and roundtrip passes item.certificate.document unchanged |
| FND-002 | still-open | Simulated items still skip hold (document.rs:4279). Counterexample schema validity deferred to QSL-2726 is acceptable, but AC-16/TC-3154 still claim it (FND-012) |
| FND-003 | still-open | hold_each never returns an error: every id that fails in the own graph and in the dependency graph picked by its package is silently left structural, for every item |
| FND-004 | still-open | The main path uses WireGraph::document_value (skips foreign containers); the fallback hold_each and map_ids walk into {package, node} containers and convert their node |
| FND-005 | still-open | NoGraph is typed in the library, but fsl-command maps AnalyzeDocumentError::Identity(_) to RunCause::AnalyzeFault (internal-failure), so the cause is lost at the CLI |
| FND-006 | fixed | analyze_bound reuses the run's strict read for the own graph; package_run uses package_bytes |
| FND-007 | fixed | Both schema tests are tagged TC-3154/FR-621-AC-16 and the FR-741-AC-3 tag is gone |
