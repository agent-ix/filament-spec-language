---
id: SR-20261121043349
title: "gap-analysis of PR 3257 pin-check full-sha rev pins"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-760-AC-3, FR-760 Behavior 'Pins on every Filament dependency', xtask/src/pin_check.rs, xtask/tests/consumer_facades.rs; PR 3257 head 704c4431b951df98a17a0e0160f90a99a8497574"
review_set: subset
---

## Summary

Planless, scoped to the PR diff. Plan completion: not assessed. FR-760-AC-3 is bound by two tests, both tagged `TC-2141`/`FR-760-AC-3`, and both bindings are correct. The changed production code (`is_full_sha` and the new rule) all traces to FR-760-AC-3 and the FR-760 Behavior text; there are no stubs and no code without an owning requirement. The one gap is the Behavior clause "fails ... on an agent-ix git dependency without a tag or full-sha rev": no test exercises a dependency that has none of tag, rev or branch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-760's "without a tag or full-sha rev" failure has no test. Every pin-check negative carries a branch or a rev, so the no-selector path is never run. This is the same gap as SR-20261121043348 FND-001. | xtask/tests/consumer_facades.rs:66-167 |

## Scope and evidence

Bindings examined: `pin_check_passes_the_workspace_and_refuses_a_branch_dependency` covers FR-760-AC-3 correctly (workspace pass, branch refusal naming the manifest and dependency). `pin_check_accepts_tag_and_full_sha_rev_and_refuses_short_rev` also covers FR-760-AC-3 correctly (tag and full-rev pass; short, non-hex and branch refuse). `quoin matrix` was not run: the PR changes no trace tags, and its single AC mapping was checked by hand.

## Verdict

Mergeable once FND-001 has a test.
