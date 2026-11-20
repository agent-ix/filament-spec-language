---
id: SR-1472
title: "Scoped gap analysis of claim grammar corpus additions"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2077; fsl-cst/tests/it/{claim_syntax,state_graph_claim,relation_clause,aggregate_atom,main}.rs"
review_set: subset
---

## Summary

Compared the frozen QSL-2077 additions at `f2b325878280771ec739aa057809f8cd8e27b4d5` with the current state-graph distinguishing table and relation/aggregate productions. The feature exercises real parser acceptance and nonlexical refusals; one explicit table example is absent.

## Verdict

**PASS** at `387405a2bcae3618602eaae662d3edd076b4053b` — the bodyless unique-path fairness refusal is covered. This is a scoped change review, not a repository-wide assurance verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The final state-graph table row has two syntax-error examples, but the new cases cover only the temporal `if` condition. Add the paired negative `unique path One using v from (c.a) to (c.d); fair weak M::Cfg::step`, checking its located syntax refusal; existing unique-path negatives only remove `path` or replace its terminator. This leaves the explicit no-body fairness boundary untested. | fsl-cst/tests/it/state_graph_claim.rs:149 |

## Examined scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FSpec FR-339-AC-1 | examined | Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors. |
| FSpec FR-339-AC-2 | examined | Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace. |
| FSL FR-002-AC-6 | examined | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. |
| FSL FR-165-AC-4, S1 clauses | examined | S1 parses `possible Anything using v { true }` (no `over`) to a claim with no `over` binding. |
| FSL FR-165-AC-6, unique-path example | examined | `unique path ... to (c.versionNumber = 2); fair weak Config::ConfigVersion::attemptUpdate` at `fair` |
| Pinned shared grammar, state-graph table final row | examined | Syntax errors: a temporal operator in an `if` condition, and a fairness constraint on a clause with no body. |
| Pinned relation and aggregate productions | context_only | `execution-binding = ident, ':', qualified-name ;` |

## Coverage

Plan completion: not assessed

The 13 accepted state-graph vectors and paired negative cases were inspected, together with six relation and four aggregate tests and their shared assertion helper. Tests are registered, execute the public parser, carry criterion trace tags and do not manufacture S1 errors from semantic uniqueness or bounds. Existing aggregate precedence cases remain unchanged. No stub or hollow test was found in this diff.

The current table includes hyper/temporal rows and both positive and negative state-graph examples; this review uses those meanings rather than the ticket's historical line numbers or count. The missing unique-path fairness vector is a specific table obligation, not a request to expand into an unrelated grammar family.

Matrix and repository-wide reverse-gap work were intentionally excluded by the user's scoped-review instruction. Author-focused test and Clippy evidence was accepted without remeasurement; no tests, builds, product probes, history audit or inventory were run. The final component gate remains the original coder's next stage.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At `387405a2bcae3618602eaae662d3edd076b4053b`, a new traced test parses the valid bodyless unique-path counterpart and then asserts the appended fairness constraint gives `InvalidSyntax`/`UnexpectedToken` at the exact `fair` byte span through the existing shared refusal helper. |

### Round 1 evidence

Only the confined new case and the existing shared helper usage were inspected, with the exact newly referenced FR-165-AC-6 example. No broader grammar or prior corpus review was repeated. Author-reported passing results for the modified/new cases and scoped Clippy were accepted; the feature now has 37 active cases and no pending cases. No tests, builds, product probes or known gate failures were rerun. Later PR opening at this unchanged reviewed code does not require another review.


