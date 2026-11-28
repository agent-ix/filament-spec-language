---
id: SR-20261125155847
title: "gap-analysis of PR 3945 recursion-group source-map terms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3945 head 464132bd7ae7cd710e050cd578d33147bdfc683d; git diff origin/main...HEAD"
review_set: subset
---

## Summary

Ticket: QSL-3217. Gap analysis scoped to the PR diff. The criteria are FSpec ones (filament-specification at 57c662b1), outside this repo's `spec/` root, so `quire matrix` over this repo does not compute them; a grep index of `#[trace(...)]` tags in the FSL source tree was used instead (declared fallback).

## Verdict

CONDITIONAL. Every criterion in scope is tagged by at least one test (FR-146-AC-1: 4 files, FR-146-AC-4: 3, FR-322-AC-274: 2, FR-322-AC-510: 5). The production change has an owning requirement (FR-146-AC-1 admission and evaluation of recursive functions; FR-322-AC-274 group-member keys). No stub or hollow test: each new test asserts a concrete row or run result, and the PR states each fails with the production change removed (two emitter tests 0/2, source_corpus 1 of 4 red on f6132087ad), consistent with the code: the old prefix test skips every group-member candidate, leaving no application for an always-application query or collection term and no deref row. The one low finding is the AC-510 tag breadth recorded in the code-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-146-AC-4's admission half ("admitted only when every substituted cycle edge decreases") is tagged by the new run test, which covers only admitted mutual components; the refusal half rests on existing tests outside this diff | tests/it/source_corpus.rs:1031 |

## Coverage

- Criteria in scope: FSpec-FR-146-AC-1, FSpec-FR-146-AC-4, FSpec-FR-322-AC-274, FSpec-FR-322-AC-510; all tagged.
- Reverse gap: none in the diff.
- Stubs: none.
- Semantic review: skipped (not requested).
- Plan completion: not assessed
