---
id: SR-1471
title: "Code and Rust review of claim grammar near-misses"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2077; fsl-cst/tests/it/{claim_syntax,state_graph_claim,relation_clause,aggregate_atom,main}.rs"
review_set: subset
---

## Summary

Reviewed frozen branch `codex/qsl-2077-claim-near-misses` at `f2b325878280771ec739aa057809f8cd8e27b4d5` before PR opening. The real parser tests check located refusals, source identity, lossless bytes and accepted productions; one binding assertion is missing.

## Verdict

**PASS** at `387405a2bcae3618602eaae662d3edd076b4053b` — the ordinary-expression binding assertion is present. No production implementation, public API, dependency or workflow is changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ordinary-not table row checks only admission, claim prefix, over presence and body count, so it still passes if `not c.n = 2` is incorrectly parsed as graph-unary `not (c.n = 2)` instead of ordinary expression `(not c.n) = 2`. Assert the relevant CST expression/unary grouping or production spans to distinguish those parses. | fsl-cst/tests/it/state_graph_claim.rs:151 |

## Examined scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FSpec FR-339-AC-1 | examined | Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors. |
| FSpec FR-339-AC-2 | examined | Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace. |
| FSL FR-002-AC-6 | examined | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. |
| FSL FR-165-AC-4, S1 clauses | examined | S1 parses `possible Anything using v { true }` (no `over`) to a claim with no `over` binding. |
| Pinned shared grammar, state-graph overlap rule | examined | a predicate or parenthesised group that derives as an `expr` is read as that `expr` |

## Rust and test evidence

All five changed files were inspected. Shared assertions use the public parser and CST, exact typed causes and spans, and deterministic authored inputs; no doubles, skips or new production paths are introduced. The source header and local integration registration conventions are followed. The ordinary-not gap concerns the normative binding, not a preference for a different representation.

The author's 36 focused passing cases and scoped CST integration Clippy result are accepted without rerunning them. The original coder runs one final applicable component gate after review. Known unrelated component failures belong to PR3146 and were not inspected or reproduced here.

## Coverage limits

Review scope is the frozen feature diff. No matrix, repository inventory, historical corpus audit, test, build or product probe was run. Current repository requirements and exact sections of pinned FSpec `f32bca6c7937e0652742b2bbe27a52419bf5a0ae` were used. One diagnostic lookup included unrelated recovery criteria; that read was stopped and reported, and none became a finding or was reverified.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At `387405a2bcae3618602eaae662d3edd076b4053b`, the same parsed ordinary-not vector asserts comparison text `not c.n = 2`, a unary node `not c.n`, and no `StateGraphUnary`. These assertions distinguish the required ordinary-expression binding from graph-unary negation. |

### Round 1 evidence

Inspected only the confined fix diff. The accepted helper now returns its already parsed source so the original test can inspect its structure; no second parse or replacement parser is introduced. The author reports the modified and new named cases and scoped Clippy passed; those receipts were accepted without rerunning them. The feature now has 37 active cases and no pending cases. No new finding, test, build, old precedence check or unrelated component failure check was added by this disposition.


