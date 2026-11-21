---
id: SR-21282001
title: "QSL-2128 universal package hyper reader code review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/filament-spec-language; PR3396; fsl-package/src/checked_views.rs; fsl-package/src/checked_views/hyper.rs; fsl-package/src/checked_views/hyper/normal.rs; fsl-package/src/checked_views/subject.rs; fsl-package/src/emit/tests/hyper_clause.rs; fsl-semantics/src/check/hyper_clause.rs; fsl-semantics/src/check/mod.rs; fsl-semantics/src/check/temporal_clause.rs"
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-940", type: references}
---

## Summary

Ticket: QSL-2128. Independent code-review including the Rust lane at frozen c08a0e682f1980f6ec17003422dd5dceba122107 against ba280846beb6e8bf79c89b179c8841839c203090. All eight changed files were examined. No edits to the candidate, build, runtime test, benchmark, aggregate gate, or dependency update were performed by this reviewer.

## Verdict

CONDITIONAL: three medium defects require resolution in the original PR. This review covers the infinite-trace universal reader/constructor slice; it does not require or claim complete AC-16, AC-19 or the later AC-18 certificate journey. PR3395 must merge first, followed by a fresh code rebase and the lead's final gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Hyper facts depend on forbidden expression source-map regions: changing only a holds node's expression region changes the reconstructed formula span although FR-940 permits only the claim row's declaration_region. | fsl-package/src/checked_views/hyper.rs:272 |
| FND-002 | medium | Normal-form formula traversal and repeated body encodings bypass the shared work counter, allowing accepted formula/quantifier combinations to exceed FORMULA_VISITS and incur quadratic byte-copy work without the intended Pending refusal. | fsl-package/src/checked_views/hyper/normal.rs:295 |
| FND-003 | medium | The checked constructor accepts copy classes containing undeclared variables or inconsistent with their generators, publishing a checked clause whose declared symmetry group disagrees with its executable swaps. | fsl-semantics/src/check/hyper_clause.rs:714 |

## Evidence

FND-001: FR-940, immediately after the hyper-facts table, says: "It reads no source text, no `source_map` region other than the `declaration_region` of its row, no occurrence site label and no `NodeRef` path." The same text remains in proposed PR3395 head 190a304cc2c8e86748ad96bdd83320c0bc28f143. Lines 272-293 instead select expression regions by node/source/containment and put the selected coordinates into PackageFormulaNode::Holds. A valid expression provenance region moved within the unchanged declaration changes form.formula, with no semantic graph, claim row or selected model change. If multiple occurrences share the holds node, the fallback changes its span to the entire declaration. This concerns package provenance; no physical source file is needed to trigger it. Resolve the forbidden dependency and specify/test the source-free span policy; preserve the existing temporal constructor's public behavior.

FND-002: only expression() decrements work_left (normal.rs:114-117). body() visits every formula entry and combines full child bytes (295-350), without decrementing it. A universal clause with q same-alias quantifiers and m nested temporal/not operators around true has no atoms, hence expression() is never called. The initial body and q-1 successful swap trials all leave the counter unchanged. formula_graph_of bounds one expansion to m <= FORMULA_VISITS, but does not bound these repeated encodings: q*m can exceed FORMULA_VISITS. For a unary chain, each combine() copies the growing suffix, so one trial copies Theta(m^2) bytes; these are graph references, not recursively nested CheckedTerms, so term-depth admission is not a bound on the chain. Charge formula visits, swap bookkeeping and encoding sizes against explicit bounded work/storage before allocation, with a typed refusal on exhaustion. This is a static failure scenario; no benchmark claim is made.

FND-003: start from the real emitted Equal form used by the added positive test. Set form.copy_group.classes[0][1] to "ghost" and leave the generator {first:0, second:1, atoms:[0]} unchanged. The constructor never reads classes in its validation and therefore succeeds; rebuilt.copy_group() exposes [[a,ghost]] although the prefix is [a,b]. fsl-eval/src/hyper/copy_swap.rs uses the classes for copy_symmetry while its CopySwap uses the generators for actual exchanges, so these are observably different groups. Overlapping classes, missing generators and cross-alias generators are likewise structurally unchecked. Validate that classes are disjoint ordered known universal variables of one alias and that generators exactly describe their consecutive pairs, or restrict construction so only validated reader facts can enter. This finding does not require reconstructing source expressions or proving arbitrary supplied normal bytes.

## Scope

Examined: FR-940 hyper-form inputs, package normal form, outputs and malformed-node boundary; FR-940-AC-16's Equal reader/constructor subset; FR-940-AC-19's unknown-trace and duplicate-level boundary subset. Context only: FR-940-AC-18's later source overwrite/delete and certificate journey; FSpec FR-402 seven bindings/model-step facts; pinned IR a03c86b3's duplicate-level admission rule. No applicable AssuranceProfile was found. The spec changes in PR3395 are owned by a separate reviewer.

All changed public types/methods, temporal helper extraction, exact occurrence lookup, seven-binding reads, operation constraint metadata, copy-slot decoding, checked numeric conversions, atom-node references, type-domain classification and refusal variants were examined. No vendored code, duplicate arena implementation, warning suppression, gate weakening, unsafe code, new dependency, source-expression reconstruction or test double of the reviewed production path was added.

## Validation

The positive test exercises real emission, strict admission, identity supply and reconstruction. Its assertions compare name/span/aliases/parameters/quantifiers/arena/operators/copy group to S3, require [a,b] and atoms [0], and require source conditions absent with package atoms present. Its holds-only loop is preceded by exact arena equality and an exact one-atom assertion, so it is not an unguarded vacuous success. Negative tests check strict unknown-trace admission followed by the typed form refusal, pinned-reader duplicate-level rejection, and three constructor Shape refusals. They do not test the new findings.

Inspected implementation logs record the three new focused runtime passes and the existing four-temporal-clause parity pass before rebase. Package/analyzer test compilation passed before rebase; package library clippy passed after rebase. The semantics library log has three unchanged-main errors (typing.rs:2193,2197 wildcard_enum_match_arm; synthesis.rs:148 too_many_arguments). These are producer logs, not fresh reviewer executions or an exact-final-head gate. The temporal arena helper is a direct extraction with unchanged traversal, bounds, operators and horizon application. Reviewer git diff --check passed. The lead retains the final exact-head aggregate check.
