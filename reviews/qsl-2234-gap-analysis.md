---
id: SR-2026111612172400000055
title: "Gap analysis of QSL-2234 slice 1, hybrid provider seam and reference crate (filament-spec-language PR 2994)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@6c48a3ee242eff4e77c3557728eae87b1fb51563; git diff origin/main...HEAD (PR 2994); FR-750 AC-1..AC-5 at the PR head and AC-6..AC-11 as merged at origin/main bdb9a2efa (W68, TC-3035); FR-1019 AC-1..AC-13 (TC-2869, TC-2997); FR-1021-AC-3"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2234, slice 1. This is a manual AC-to-test check over the diff's tagged tests and
code. The computed matrix was not run because the PR head predates the W68 spec merge, so a
matrix at the head does not see FR-750-AC-6..11. Plan completion was not assessed. Partial
delivery is intended: the reference provider answers `solver-inconclusive`, and only what
the slice claims is judged as delivered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation. The new unit test `a_counterexample_wire_reads_back_to_itself` is tagged `FR-750-AC-5` but only round-trips a formula counterexample through `to_wire` and the plugin reader. AC-5 is a process provider's `refuted` settled by exact replay, plus `replay-parity` on non-reproduction. That path is unimplemented and its real test stays ignored, so a matrix shows AC-5 covered by an active test. Retag it (W68 FR-750-AC-9 covers the writer and reader round trip) or drop the tag | fsl-replay/src/hybrid_replay/tests.rs:1030-1046 |
| FND-002 | medium | FR-1019-AC-4's first clause, "under `Engines::fsl()` alone the same item still settles `unsupported`, `HybridProviderAbsent`", is not asserted. The un-ignored test only checks that `Engines::fsl().registrations()` excludes the reference, then tests the fixture route | fsl-analyze/tests/it/hybrid_reference.rs:124-150 |
| FND-003 | low | FR-750-AC-3's warning "naming `temporal-satisfaction` and `hybrid-dynamics`" is set in `diagnostic_detail` but not asserted. FR-750-AC-1's "dispatched once" is not asserted (the fixture counts no calls) | fsl-analyze/tests/it/hybrid_provider.rs:154-175; fsl-analyze/tests/it/hybrid_requests.rs:209-227 |
| FND-004 | low | The FR-1019-AC-6 ignore reason says "descriptor and budgets are not implemented yet", but the descriptor's bounds and `fsl_with_reference_hybrid` now exist. What is missing is the `options` defaults and the solver step budget. The reason overstates the gap and should name what remains | fsl-analyze/tests/it/hybrid_reference.rs:207-210 |
| FND-005 | medium | W68 added FR-750-AC-6..AC-11 and FR-1021-AC-3 (TC-3035). No test in the PR carries those tags, active or ignored. The in-PR parts (AC-6 wire refusals and string options, AC-8 in-process `Refuted`, the AC-9 writer, AC-11 `read_entry` arms, AC-7 in-process `Unsupported`) need active tests. The follow-on parts (AC-7/AC-11 process halves, AC-10 FR-098 hybrid row, FR-1021-AC-3) need ignored stubs keyed QSL-2234 with accurate reasons | fsl-analyze/tests/it/hybrid_requests.rs; fsl-replay/src/hybrid_replay/tests.rs |

## Scope and evidence

Examined, clean unless listed above:
- FR-750-AC-1: the active test drives an `Engine::hybrid` fixture through
  `analyze_with_providers` and settles `proved`/`bounded-solver`/`uncertified` naming the
  fixture. It is not tautological: the answer passes through `read_entry`, FR-250 settle and
  the record writer.
- FR-750-AC-2: active, refusal names the backend and `invalid-dynamics`.
- FR-750-AC-3: active through both `analyze` and `provider::run` (FND-003 for the warning).
- FR-750-AC-4: active, `tool-unavailable` naming `hybrid-fixture`.
- FR-750-AC-5: not delivered; the real test stays ignored with an accurate reason (FND-001).
- FR-1019-AC-4: partial (FND-002). AC-5: active, with the one candidate, named routing and
  `ambiguous-backend` naming both. AC-12: active layering check.
- FR-1019-AC-1/2/3/6..11/13: not delivered, all `#[ignore]` with QSL-2234 reasons
  (FND-004 for one stale reason).
- FR-751-AC-1..3 and FSpec FR-652/FR-193 stubs: still ignored, correctly.
- `Engines::fsl()` excludes the reference provider, as required.
- Code with no owning requirement: none. Each new item cites FR-750/FR-1019/FR-250.
- Stubs in production code: the reference `solve` is an honest stub, documented as such
  (code-review SR-2026111612172400000054 FND-007 for its method label).

## Verdict

The slice's claimed ACs (FR-750-AC-1..4, FR-1019-AC-4/5/12) are covered by real,
non-tautological tests. The remaining ACs stay ignored with honest reasons. The defects are
one inflating tag (FND-001), one unasserted AC clause (FND-002), and the W68 ACs with no
tests or stubs (FND-005).

## New findings (disposition pass 1)

Pass 1 was checked against head 544461c15e18d11b7a724eca22e28eccf1989a8b.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `item_options_reach_the_provider_as_strings` is tagged `TC-2100` for `FR-750-AC-6`, whose verification is TC-3035. The sibling AC-6 test in fsl-replay uses TC-3035 | fsl-analyze/tests/it/hybrid_requests.rs:330 |
| FND-007 | low | The W72 FSpec criteria FSpec-FR-193-AC-25/26 (FSpec-TC-2051) carry no FSpec-qualified tag, though the tests that exercise them are active (`read_entry_resolves_an_unsupported_entrys_locus_through_the_graph`, `an_undefined_evaluation_claim_agrees_only_with_the_reproduced_one`). The repo already tags FSpec criteria without their text being local (`FSpec-TC-373`/`FSpec-FR-193-AC-7`, NFR-140), so the missing local text is no reason to leave them untagged | fsl-replay/src/hybrid_replay/tests.rs:1639; fsl-replay/src/hybrid_replay/tests.rs:1718 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Retagged `#[trace("TC-3035", "FR-750-AC-9")]` as "the formula kind of the writer". The real AC-5 test stays ignored |
| FND-002 | fixed | The FR-1019-AC-4 test asserts that under `Engines::fsl()` alone the item settles `unsupported` naming `temporal-satisfaction` and `hybrid-dynamics` |
| FND-003 | fixed | The AC-3 test asserts `diagnostic_detail` `kind`/`requirement`. The AC-1 test asserts `seen.calls() == 1` |
| FND-004 | fixed | The AC-6 ignore reason now names what remains (`options` defaults and the solver step budget) |
| FND-005 | fixed | Active TC-3035/TC-3037 tests now cover AC-6, AC-7 (in process), AC-8, the AC-9 writer, AC-11, the analyze half of AC-12, AC-13 and the AC-14 comparison. Ignored QSL-2234 stubs with accurate reasons cover the AC-7 process half, AC-9/AC-14 read-back, AC-10, the AC-11 process half, the AC-12 exact-replay and replay-request halves, and FR-1021-AC-3 |
