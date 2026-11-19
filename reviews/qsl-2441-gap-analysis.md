---
id: SR-2026111911313600000002
title: "Gap analysis of QSL-2441 (QSL-2000 slice D, partial) acceptance-criteria coverage (filament-spec-language PR 3015)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@d581681798b0839ff1689ff2fd212a0b0d686187; git diff origin/main...HEAD (PR 3015); FR-740-AC-4, FR-185-AC-12, FR-740 claim-resolution statement, FR-185-AC-5 (context), FR-498 (context), FSpec FR-640-AC-3 (context)"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2441. This is a manual AC-to-tests check over the tests the PR touches and the
ticket's scope. The computed `quoin matrix` was not run, because house rules forbid whole
test targets and unfiltered runs. Plan completion: not assessed. The PR says it is a
partial delivery. INDEX.md capability row: there is no INDEX.md in the repo (verified). The
`probabilistic_examples.rs` sources are not on this machine. The FR-640-AC-3 claim-identity
clause has a spec conflict open with arch. IR-1569 is still ignored. The reward clause kind
stays `precondition` TEMPORARY (QSL-2535).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185-AC-12 is covered only partly by the two tests tagged with it. The AC wants the locus to carry "that declaration's source reference and label span" and the message to "name the declaration in words". The package test checks only that the region is non-empty, and the command test only that `locus` is a JSON object. Neither checks that the region is the declaration's label span or that the source reference is the unit's. Neither checks that the message contains the declaration's name: `reward` comes from the feature display name. Random-parameter and workload refusals have no case at all. The tags are right but read as fuller coverage than they give | fsl-package/src/emit/tests/probabilistic.rs:1979-2024; fsl-command/tests/it/probabilistic_package_locus.rs:36-60 |
| FND-002 | medium | QSL-2441 is likely to close on merge (the branch `qsl-2441-...` is linked) while part of its scope is left: the probabilistic_examples.rs rover examples, the FR-640-AC-3 claim-identity clause (the ignore still cites QSL-2441), and the claim-workload resolution carried on the wire (see SR-2026111911313600000001 FND-002). These need follow-up tickets, or the ticket must be kept open when the PR merges, so the partial delivery does not read as done in Linear | fsl-package/src/emit/tests/probabilistic.rs:1900 |
| FND-003 | low | FR-740-AC-4 names "the package holding the claim, the random parameters, the reward and the workload". `a_claim_under_a_workload_is_emitted_with_its_workload_node` builds a package with no random parameter, so the strict-reader half of the AC has no random-parameter coverage in this test. The resolution half is checked against the in-memory `claim_workloads()` table, not anything the reader reads | fsl-package/src/emit/tests/probabilistic.rs:1009-1045 |

## Scope and evidence

Examined:
- FR-740-AC-4: "A claim `under Steady` emitted with `Service` resolves its scheduler to the
  `Steady` workload node, and the package holding the claim, the random parameters, the
  reward and the workload is admitted by the strict reader." Tested by
  `a_claim_under_a_workload_is_emitted_with_its_workload_node` and
  `each_claim_resolves_to_the_workload_it_is_stated_under`, both tagged `TC-2062`/`FR-740-AC-4`.
  The binding is correct. Coverage is partial (FND-003, and code-review FND-001).
- FR-185-AC-12: "An S4 refusal concerning a random parameter, a workload, a reward or a delay
  carries that declaration's source reference and label span in its locus member (never
  `null`), and its message names the declaration in words and holds no debug rendering of a
  span or source reference." Tested by `probabilistic_refusals_have_locus` and
  `a_package_stage_refusal_of_a_reward_carries_its_locus`, both tagged `TC-2062`/`FR-185-AC-12`.
  The binding is correct. Coverage is partial (FND-001).
- FR-740 statement: "A checked claim `under W` SHALL resolve, when S4 emits it (FR-498), to
  `W`'s workload node." The code meets it in memory only.
- FR-185-AC-5 (context): a second `workload Steady` anywhere in the package is refused
  `duplicate-declaration`. This makes name-based resolution unambiguous.
- `a_claim_that_reads_a_reward_changes_identity_with_the_reward_expression`
  (`FSpec-TC-1563`/`FSpec-FR-640-AC-3`) stays ignored. The new reason states the FR-498 vs
  FSpec FR-640-AC-3 conflict accurately.
- `the_reader_refuses_a_workload_that_misnames_an_operation` was already present. The PR
  body says so accurately.

## Verdict

The partial framing is honest. The PR body, test names, tags and ignore reasons match what
was delivered. No stubs, tautologies or `#[allow]`. The two tagged ACs are partly covered,
and the remaining scope needs tracking before the ticket closes (FND-002).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-185-AC-12's "source reference" half is still not asserted. The package test checks the label span by slicing the unit text, but not `region`'s source reference. The fsl-command twin still checks only that `locus` is an object and that the message contains `reward` (the feature word), not `cost` or the locus's source/span members | fsl-package/src/emit/tests/probabilistic.rs:2075-2087; fsl-command/tests/it/probabilistic_package_locus.rs:51-59 |

## Dispositions

Round 1, head 9588a1a78f391de7c609d8e5b0cca221398b30b9.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `probabilistic_refusals_have_locus` now covers all four features. It asserts that the declaration name is in the message and that the region is the label span (the unit-text slice contains the name). The remaining source-reference gap is FND-004 |
| FND-002 | fixed | The remaining scope is tracked: QSL-2546 (claim body, identity, wire resolution; the ignore cites it) and QSL-2547 (vendored examples and probabilistic_examples.rs) |
| FND-003 | deferred | QSL-2547. A random parameter in the AC-4 test needs a fixture that combines a parameterised operation, a workload and a claim. None exists, and the abstraction fixture hits QSL-2444. Random-parameter emission and reader admission are covered by other TC-2062 tests, and QSL-2547's staged-vote-probability examples combine all four, so deferring is acceptable. The coordinator should add the AC-4 random-parameter case to QSL-2547's scope |

Round 2, head 357352b6f08618d43f9cfdfdeccd25d057d90820.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | The package test asserts `region.source() == graph.source()` for all four cases. The fsl-command test asserts that the message contains `cost`, that `locus.source` is `{"authority": "offered issuer", "identity": "offered:unit"}`, and that `UNIT[start..end]` contains `cost`. Both pass |
