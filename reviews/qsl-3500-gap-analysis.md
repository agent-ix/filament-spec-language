---
id: SR-96751
title: "gap-analysis of QSL-3500 definitional reference"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-definitional-reference/, fsl-analyze/, tools/arch-lint/, xtask/src/string_edge.rs, Cargo.toml, Cargo.lock, Makefile; PR4054 96e6b140d89ccb5fa1ba1db542cb475387701c61"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3500. PR agent-ix/filament-spec-language#4054 at 96e6b140d89ccb5fa1ba1db542cb475387701c61. gap analysis scoped to FR-1051 AC-16 to AC-19. Plan completion: not assessed.

## Verdict

**FAIL**. The oracle is not built. AC-16 and AC-17 are untagged. AC-1 to AC-15 belong to other tickets and were not assessed. The semantic review was limited to the reference's tests. Method: quire matrix plus manual criterion-to-test reading of the changed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | There is no interpreter. Nothing reads operations, effects, supports, guards or claims from a CheckedGraph to compute a truth value; TinyMdp takes hand-written rows that no checked model is converted into. The crate can decide no claim of any corpus model, so QSL-3500's oracle is not delivered. The PR body says so (draft). The crate doc ('The interpreter admits only literal reward bodies') describes an interpreter that does not exist. | fsl-definitional-reference/src/lib.rs:23-141 |
| FND-002 | medium | The module named false_claim_corpus holds no false claim: none of the six seeds, no route entry, no checker or analysis assertion. AC-16 and AC-17 are untagged in quire matrix. The PR body lists this as outstanding. | fsl-analyze/tests/it/false_claim_corpus.rs:1-199 |
| FND-003 | medium | AC-19's 'non-literal probability weight' clause has no code and no test, and the timed disjunctive-guard branch has no test, while four functions are tagged implements FR-1051-AC-19 and the PR lists the tag without saying which clauses are missing. If the front end already refuses non-literal weights, say so and test that. | fsl-definitional-reference/src/lib.rs:157-263 |
| FND-004 | low | model_construction_refuses_more_than_two_to_the_sixteenth_states is traced to FR-1051-AC-19, which says nothing about the state bound; the bound is in FR-1051's fragment text. TinyMdp::new carries implements AC-19 for the same reason. | fsl-definitional-reference/src/lib.rs:309-318 |
| FND-005 | low | The code implementing AC-18 (reference_base::scan) has no implements("FR-1051-AC-18") marker, although the PR body says AC-18 tags were added (only tests carry it). | tools/arch-lint/reference_base.rs:128-155 |
