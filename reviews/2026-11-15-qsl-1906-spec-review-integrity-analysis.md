---
id: SR-599007
title: 'Spec review (integrity) of MeasurementPlans MP-007 to MP-011 in PR #2714'
type: SpecReview
analysis: integrity
scope: 'agent-ix/filament-spec-language@508b98459; spec/assurance/MP-007..MP-011 (new), spec/assurance/MP-006 (context); ticket QSL-1906'
review_set: subset
date: '2026-11-15'
---

## Summary

Ticket: QSL-1906. PR: filament-spec-language#2714, head 508b98459. This review checks the five new MeasurementPlans for structure, id uniqueness, relationship targets, stored thresholds and invented requirements, using `spec-integrity-analysis` and `spec-dependency-analysis` (new `relationships:` edges). `quire validate --scope . 'spec/assurance/MP-0*.md'` exits 0. MP-007 to MP-011 are not used on origin/main or in the filament-specification checkout (17d49a98a). No plan stores a threshold: every decision rule compares against the same-session current-main collection. No plan invents a requirement.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| MP-007 | spec/assurance/MP-007-model-check-wall-time.md | examined |
| MP-008 | spec/assurance/MP-008-reach-workers-wall-time.md | examined |
| MP-009 | spec/assurance/MP-009-key-encode-wall-time.md | examined |
| MP-010 | spec/assurance/MP-010-en4-sample-wall-time.md | examined |
| MP-011 | spec/assurance/MP-011-model-check-probe-rates-and-rss.md | examined |
| MP-006 | spec/assurance/MP-006-probe-check-wall-time-and-rss.md | examined |
| FR-101, FR-126, FR-167, FR-189, FR-205, FR-206 | spec/functional/ (this repo) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | MP-007's `measures` targets are `ix://agent-ix/filament-specification/FR-167`, `FR-126` and `FR-101`. These FRs live in this repo (`spec/functional/FR-167-explore-a-state-graph-subject...` and so on). None of them exists in filament-specification, so the trace dangles. The target repo should be `filament-spec-language`. | spec/assurance/MP-007-model-check-wall-time.md:63-69 |
| FND-002 | high | MP-008 targets `ix://agent-ix/filament-specification/FR-205` and `FR-206`. In filament-specification those ids are "Exact rational value" and "Numeric domain", so the plan traces protocol reach to the wrong requirements. The intended FRs are this repo's FR-205 (protocol subject and state key) and FR-206 (protocol step kinds). | spec/assurance/MP-008-reach-workers-wall-time.md:63-67 |
| FND-003 | high | MP-009 targets `ix://agent-ix/filament-specification/FR-101`, which does not exist there. FR-101 is this repo's. | spec/assurance/MP-009-key-encode-wall-time.md:63-65 |
| FND-004 | high | MP-010 targets `ix://agent-ix/filament-specification/FR-189`, which does not exist there. FR-189 is this repo's. | spec/assurance/MP-010-en4-sample-wall-time.md:63-65 |
| FND-005 | high | MP-011 targets filament-specification FR-167, FR-126, FR-189 (none exists there) and FR-205 ("Exact rational value" there). All four are this repo's FRs. | spec/assurance/MP-011-model-check-probe-rates-and-rss.md:63-71 |
| FND-006 | medium | MP-011 declares `objective.direction: lower`. Its body Population adds states, edges and samples per second as dimensions, where higher is better, and the frontmatter `statistical_design.population` lists only peak RSS and marginal bytes. The two lists do not agree, and the single direction is wrong for the rate dimensions. Drop the rates from the plan's dimensions (keep them as context), or split the plan. | spec/assurance/MP-011-model-check-probe-rates-and-rss.md:11-12, spec/assurance/MP-011-model-check-probe-rates-and-rss.md:82-84 |
| FND-007 | low | MP-011's apparatus-edit control names "the bench file, its input generator and the bench crate manifest". Its protected apparatus is the probe binary, mc.rs, world.rs and the RSS reader. The text was copied from MP-007 and does not describe this plan. | spec/assurance/MP-011-model-check-probe-rates-and-rss.md:50-54 |
| FND-008 | low | The PR edits fsl-bench/src/rss.rs, which MP-006 lists as protected apparatus, and does not acknowledge MP-006's apparatus-edit control. MP-006 still describes the reader as `VmHWM` only. The Linux path is unchanged, so no fresh Linux comparison is needed, but the PR or MP-006 should say so. MP-006 should also note that macOS now reports peak physical footprint. | spec/assurance/MP-006-probe-check-wall-time-and-rss.md:24, spec/assurance/MP-006-probe-check-wall-time-and-rss.md:41, spec/assurance/MP-006-probe-check-wall-time-and-rss.md:87, fsl-bench/src/rss.rs:794-831 |

## Verdict

Not mergeable as written: FND-001 to FND-005 put wrong or dangling traceability into the spec. Each fix is a one-word change of the target repo. FND-006 needs a decision about which dimensions MP-011 has. The plans are otherwise structurally valid, unique and threshold-free.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | MP-011's title ("Model-check axes rates, peak RSS and marginal bytes per state") and metric (`fsl.bench.probe.mc_rates_and_rss`) still name rates, which the plan now says are context and not dimensions. | spec/assurance/MP-011-model-check-probe-rates-and-rss.md:3, spec/assurance/MP-011-model-check-probe-rates-and-rss.md:7-8, spec/assurance/MP-011-model-check-probe-rates-and-rss.md:76 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | MP-007 targets are now ix://agent-ix/filament-spec-language/FR-167, FR-126 and FR-101, each resolving to spec/functional in this repo. |
| FND-002 | fixed | MP-008 targets are now this repo's FR-205 (protocol subject and state key) and FR-206 (protocol step kinds). |
| FND-003 | fixed | MP-009 targets this repo's FR-101. |
| FND-004 | fixed | MP-010 targets this repo's FR-189. |
| FND-005 | fixed | MP-011 targets this repo's FR-167, FR-126, FR-189 and FR-205. |
| FND-006 | fixed | The rates are removed from the body Population. Frontmatter and body populations now match (peak RSS and marginal bytes), and Decision Use marks the rates as context. |
| FND-007 | fixed | The apparatus-edit text now names the probe binary, mc.rs, world.rs and rss.rs. |
| FND-008 | fixed | MP-006 sampling and Population now describe both readers and state that the Linux path is unchanged, needs no fresh A/B, and that macOS starts a new series. |
