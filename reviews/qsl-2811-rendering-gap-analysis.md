---
id: SR-967175026302197768900474181941935084726
title: gap-analysis of evaluator rendering follow-up
type: SpecReview
analysis: gap-analysis
scope: agent-ix/filament-spec-language; QSL-2811 PR3481; fsl-eval/src/value/expression/{evaluate,causes}.rs;
  fsl-semantics/src/model/clock_reference/identity.rs; replay/foundation adapters
  and clock controls
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-062
  type: references
- target: ix://agent-ix/filament-spec-language/FR-230
  type: references
- target: ix://agent-ix/filament-spec-language/FR-096
  type: references
---

## Summary

Frozen head: `16496acf5d9f80a1dbf861c03d1cdc88ff6b86a2`; PR #3481; Ticket: QSL-2811. Reviewed the three-file delta from merged parent `20e2fa9faffc298c3230a1efbf527ba43b7d7208`, plus the actual replay/foundation call chain and unchanged clock boundary controls. No branch edits, builds, tests, repository executables or test jobs were run. Installed Quoin/Quire read-only source analysis was permitted. Exact runtime model identity is unavailable.


## Verdict

**CONDITIONAL** — no scoped source findings; final Titan checks and compilation remain pending. Global matrix diagnostics and incomplete repository-wide reverse census prevent a repository assurance PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Source evidence

The two causes have one defining owner in existing `expression/causes.rs`. `unavailable_clock_origin` stores `ReferenceOriginLocus` and `clock_identity_halt` stores the allocation `usize`; neither invokes catalog field rendering. `Machine::run` returns `Evaluation { FamilyEvaluated(result), ... }` at evaluate.rs:1835 and 1871 without consuming fields. After `execute(...)` returns at replay call.rs:1331, `convert_outcome` calls `convert_outcome_resolving` (2472/2478), which calls `fsl_eval::value::refusal_record` (2484). The adapter at evaluate.rs:106-124 delegates to foundation diagnostic.rs:778-784; that is where `catalog_fields()` is invoked. The relocation therefore retains the existing deferred field-rendering phase, rather than introducing an eager rendering helper to evade the scan.

`Decimal::try_from(&Integer)` belongs to the existing layer-3 canonical identity type and is used by the evaluator. It rejects negative integers with existing `IdentityError::Malformed`, retains exact canonical integer digits (confirmed against the pinned Integer Display implementation), and avoids a machine-integer ceiling for selectors. The evaluator keeps the existing `NoElement` mapping for negative selectors. The path position alone uses checked usize-to-u64 conversion and existing `From<u64>`. Encoding remains the same `PlaceIdentity::encode`/canonical writer with the same byte ceiling; declared maximum validation compares digit counts and digit strings. This owner conversion constructs a canonical identity representation, rather than reading checked source text or serializing a diagnostic in the evaluation hook. No custom digit renderer or second codec was added.

New tests call real cause/adaptation and identity owner APIs. They assert exact catalog code/fields and retained origin/requested data, and zero, negative, 2^128 (beyond u128), complete identity decode equality, exact-byte success, and one-byte-short typed canonical failure with bound/required fields. The nested test imports now resolve the formerly missing types and BTreeMap by source inspection. No scanner, allow entry, existing test assertion, workflow or manifest changed. No new unsafe, concurrency, source stub, vendored representation or runtime panic was introduced by this delta.


## Scope

- `FR-062-AC-6`; role: examined; spec/functional/FR-062-implement-checked-family-contract.md; excerpt: Every other family's evaluation hook, invoked on a checked node built only from checked input, returns without reading any CST, token or display string.
- `FR-096-AC-7`; role: examined; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; excerpt: For each `CatalogCoded` cause in the key table (every row but the thirteen kernel `Refusal` rows, which `kernel_refusal_record` maps and AC-8 checks), `catalog_fields()` holds exactly the keys the table lists for it.
- `FR-230-AC-43`; role: context_only; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; excerpt: In FR-230-AC-24's model, `ref(self.x)` of `Call1` and of `Call2` have different place ids, `ref(self.q[0])` and `ref(self.q[1])` have different place ids, `ref(self.m[k])` over a `Map` value refuses `ill_typed`/`type-mismatch`, and `ref(self.x) = ref(other.x)` is false for two calls and true for one call.
- `FR-230-AC-45`; role: context_only; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; excerpt: A raw snapshot `Reference<Clock>` using the delimiter vector of AC-43 with an uppercase hex letter, with `J` replaced by the hex of `[["slot","01","0"]]`, with whitespace in decoded JSON, or with any extra component refuses `invalid_runtime_input`/`invalid-value`.
- `FR-230/Place identity`; role: examined; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; excerpt: All integers below are JSON **strings** in grammar
  `0 | [1-9][0-9]*`, never JSON numbers, signed values, exponent notation or
  zero-padded decimals. An unbounded upper cardinality is JSON `null`.
- `FR-230/Slot indices`; role: examined; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; excerpt: `J` is the array `[ ["slot", pathPosition, index], ... ]`, one entry
  per `collection` entry of `T`, in that entry's order. `pathPosition` is
  its zero-based position in the canonical array `T`, not an S3/S4 id;
  `index` is the concrete nonnegative positional selector. Cardinality and
  presence validation use the declared model and existing run bounds.

## Coverage

Titan evidence is supplied by the lead, not rerun by this reviewer: at source-equivalent `f7d27151a9c976f4bb800540474212bd856f0231`, root FR-062 scan, evaluator renaming invariance, typed Decimal unit and ordinary-owner Source generation each passed one selected test. Two evaluator library checks and workspace compilation were blocked by missing nested test imports, now repaired in frozen head; final two named checks, nonignored raw-clock refusal control and compilation remain pending. Formatting diff check passed; full formatter, Clippy, deny and runtime suites were not run under the explicit Titan-only/name-filtered restriction.

`actual_clock_references_roundtrip_as_data_through_source_and_bare` remains ignored for tracked QSL-2178, exactly as in merged parent; an ignored selection is not a pass. Source resolve_clock unconditionally refuses missing-route-neutral-place-identity, and Bare/source-declaration projection refuses missing-type-representation. The existing nonignored `raw_clock_data_has_explicit_source_and_bare_refusals` genuinely exercises both refusal routes. Ordinary-owner positive generation uses `ref(call.x)` without selectors. Numeric conversion is directly unit-tested; no indexed Source/Bare end-to-end integration coverage was established. This review does not assert complete FR-230-AC-43/45 acceptance or raw transported Clock support.


Reconciliation: Quoin 0.36.8 evidence matrix exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. This matches the exact reported full-repository size failure; installed executable `/home/peter/.npm-global/bin/quoin`. Fallback `/home/peter/.npm-global/bin/quire` 0.52.0 (engine 0.62.2), `quire matrix --scope /home/peter/dev/worktrees/qsl-2811-evaluator-rendering --format json`, exited 0. Saved raw matrix in scratchpad. No run evidence was read by the fallback. Matrix: 5020 tagged, 1081 untagged, 68 tagged-by-ignored-test, 107 method-without-symbol, 15 retired. All four criterion IDs examined/contextualized here are tagged; both newly added test symbols bind in the computed matrix. Tagging alone does not establish full criterion behavior.

Reverse-gap inventory in the assigned production delta: two relocated typed causes and one new typed Decimal conversion, all owned by FR-062, FR-096 and FR-230; zero introduced unowned behaviors or hollow source/test stubs found. No coverage measurement was supplied, so no numerical coverage claim is made. The repository-wide matrix was read, but a complete reverse-gap/source census over unrelated repository code was not performed. Consequently this is a scoped delta audit, not a repository assurance PASS; the broad existing matrix diagnostics remain unresolved by this PR.

Plan completion: not assessed

Optional semantic gap-analysis expansion: skipped (not requested). Code-review's required scoped contract/test reasoning is recorded separately. No spec-review or optional spec subanalysis applies because no normative specification or plan changed.
