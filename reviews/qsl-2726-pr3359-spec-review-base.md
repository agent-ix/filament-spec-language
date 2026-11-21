---
id: SR-20261121003359002
title: "spec-review/base of existing temporal AC table layout"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-283-monitor-a-supplied-trace-offline.md; spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md; QSL-2726"
review_set: subset
---

## Summary

Reviewed the two empty-line deletions for content preservation and restored criterion extraction. No introduced defect was found.

## Verdict

**PASS** for the bounded layout change. No new normative behavior or runtime coverage is claimed.

## Evidence

Frozen head: `d17d31a8dffef6daaaaaa173d10dc75247616247`; base: `158cc4328568a0c864b90b07105a4276f19d6f0e`. Ticket: QSL-2726. PR: filament-spec-language#3359.

The raw diff deletes one empty line after FR-283-AC-17 and one after FR-501-AC-30. Both complete files compare byte-for-byte after empty-line removal; all normative text, criterion IDs, verification cells, and behavior are preserved. No source, tests, dependencies or gates change.

Current Quire 0.52.0 (engine 0.62.2), resolved at /home/peter/.npm-global/bin/quire, independently computes 19 FR-283 criteria and 32 FR-501 criteria, including AC18/19 and AC31/32. A scratch baseline built directly from the base git blobs computes 17 and 30. Both matrix commands exit 0; pre-existing criteria statements remain equal. Full current matrix stderr retains existing source-symbol collisions and trace grammar diagnostics.

Explicit installed spec-artifacts-iso validation of both changed FRs exits 0. Its newly visible existing FR-501-AC-32 emits advisory ac:vague-response for `support`; no assertion is made that baseline validation checked that previously hidden row. The advisory is disclosed in the PR body. Git diff --check passes. Git 2.34.1 --ignore-blank-lines --exit-code emits nothing but exits 1; empty emitted diff and per-file byte comparison, rather than that exit code, establish preservation.

The four restored criteria are untagged in this frozen specification-only head. This is extraction evidence, not proof of runtime coverage or qualification. Runtime PR #3354 is outside this review. The known Quoin CORE_REFUSED corpus-size limit uses the architecture-accepted current Quire static fallback; no old binary fallback or new tracker issue is introduced.

## Assurance Context

No active AssuranceProfile is declared in the local spec tree. No required enforced review run or selected assurance context applies; unavailable context: none needed for this bounded layout check; active exceptions: architecture-accepted current Quire matrix fallback for known Quoin CORE_REFUSED size limit.

Repository-wide gap-analysis is not applicable to the changed surface: no production code changes. The bounded matrix extraction check above is not a full repository gap audit. Reverse-gap/stub/runtime semantic audits and Cargo/Rust gates are not run. Other optional spec analyses are skipped because there are no edited requirement statements, domain objects, relationships, verification methods, risks, or behavior boundaries. Plan completion: not assessed.

## Scope

The six boundary/restored criteria below were examined for layout and extraction, including exact preservation of their text; remaining complete-file bytes were compared for preservation without a fresh semantic critique.

```yaml
{
  "scope": [
    {
      "id": "FR-283-AC-17",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "Over the `Finite` trace `c.value` 0, 1, 2, FR-330-AC-1's `Bounded` (event-position) returns `violation` at position 0 with `truth` `false`, and FR-330-AC-2's `Reaches` (infinite-trace) returns `success` with `truth` `true`, its prefix being a good prefix (FR-328). The same records come back when the request also sets `limits.max_window_values` to 1 and when it passes an empty `temporal_selections`. A `Document` trace of the same snapshots carrying `\"closed\": true`, and one carrying `\"open\": true`, are each refused `invalid_runtime_input`/`unknown-member` naming that member, before any position"
    },
    {
      "id": "FR-283-AC-18",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "On both source and checked-package routes, TC-1295 rows 45–47 give exact whole caller text and exact expression locus for a unique candidate, identical caller text and the actual coarser claim declaration locus for shared byte-identical candidates, and FR-931's literal canonical name `holds(p.ok)` and that declaration locus for shared differing spellings. The original verdict or refusal, category and support positions remain unchanged, with no attribution refusal for valid differing texts. Missing caller sources and invalid selected spans retain `SourceNotSupplied`; no handoff fallback supplie"
    },
    {
      "id": "FR-283-AC-19",
      "path": "spec/functional/FR-283-monitor-a-supplied-trace-offline.md",
      "role": "examined",
      "excerpt": "TC-1295's aggregate controls retain exact caller text for unique and byte-identical regions and render differing shared regions by the canonical aggregate assembly above, including omitted versus explicit `min_count 1` and equivalent reformatted duration bounds. Both routes use the actual declaration locus for shared regions and retain the original evaluator outcome and aggregate counts. Changing a window, filter, threshold, kind or normalized min_count changes the checked aggregate and is not collapsed by label identity; missing source or invalid spans retain `SourceNotSupplied`, never a cano"
    },
    {
      "id": "FR-501-AC-30",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "AC-29's request with no late-data cutoff reads all three observations and its result writes no `late` member; adding the cutoff changes the result identity; with `HistoryCutoff` and the cutoff together, `once[0,4] holds(p)` reports `MissingHistory` for the activation that reaches before the trace and no observation is `late` because of it."
    },
    {
      "id": "FR-501-AC-31",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "TC-1295 rows 45–47, run over both source and checked-package routes with equal caller sources, preserve the false verdict, `closed-scope` basis and position-1 decision support while rendering a unique atom as exact `holds( p.ok )` with its expression locus, byte-identical shared atoms as that text with the actual claim declaration locus, and differing shared spellings as exact FR-931 canonical `holds(p.ok)` with that declaration locus. Neither shared case chooses a candidate by rank or invents a per-use locus; valid differing text adds no attribution refusal."
    },
    {
      "id": "FR-501-AC-32",
      "path": "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md",
      "role": "examined",
      "excerpt": "TC-1295's timestamped aggregate controls apply FR-283's unique, identical and differing-text rules to named aggregates and decision-support names on both routes, including normalized duration bounds and default min_count. Differing spellings use canonical aggregate assembly and the actual declaration locus, without attribution refusal or a new outcome row; the original evaluator result, basis, support positions and insufficient-data counts are unchanged."
    }
  ]
}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
