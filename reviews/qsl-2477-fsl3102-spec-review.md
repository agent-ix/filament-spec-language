---
id: SR-2026111921293000000002
title: "spec-review of QSL-2477 and QSL-2482 outcome schemas, canonical bytes and fix hints"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3102 at d08c7eaa; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md (diagnostics member, Serialization bullets, Fix hints, Canonical encoding and key order, Published schema, FR-286-AC-25..30), spec/functional/FR-644-stream-observations-through-the-monitor-command.md (Published schemas, FR-644-AC-12), spec/functional/FR-865-render-every-outcome-document-as-text-the-cli-default.md (diagnostic lines, FR-865-AC-8), TC-3110..TC-3115; context: fsl-replay/src/outcome.rs (OutcomeDiagnostic, OutcomeDocument Serialize, to_bytes), fsl-analyze/src/analyze/document.rs write_run/item_document, fsl-command/src/command.rs outcome_document, fsl-command/src/command/monitor.rs, fsl-foundation/Cargo.toml, quire-canonical v0.3.1 docs/encoding.md, ix-cli-kit FR-006, filament-driver FR-002/FR-025, FR-963, FR-990, FR-1031, FR-650"
review_set: subset
---

## Summary

Spec review of FSL PR #3102. Tickets: QSL-2477 (published JSON Schemas, key order) and QSL-2482 (diagnostic fix hints). The PR publishes `urn:filament:outcome:1` with a golden corpus, two monitor schemas, makes `OutcomeDocument::to_bytes` RFC 8785, adds an optional diagnostic `hint` from a static catalog in `fsl-foundation`, and writes it as `  hint:` in FR-865's text form. `quire validate` and `make check-spec-validation` pass (exit 0). The only grammar warnings on touched files are at FR-286 line 329 and FR-865 line 149, both outside the diff. TC-3110..TC-3115, FR-286-AC-25..30, FR-644-AC-12 and FR-865-AC-8 are free on origin/main (highest TC there is TC-3084) and in all 27 other open PRs.

## Examined scope

- Closed outcome schema against the real writer. `OutcomeDiagnostic` serializes `construct`, `text`, `declaration` and `profile` on forms refusals (outcome.rs:1912-1918, 2023-2026), which neither the diagnostic member list nor the member-order sentence names (FND-001). An `execute` outcome with `requirements` writes a root `registry` (outcome.rs:2287-2289, FR-963), and analyze items carry FR-990/FR-1032 members (`obligations`, `closure`, `global`, `projections`, `recording`). The schema bullet sources neither (FND-002). (examined)
- Canonical bytes and numbers. Outcome counts, bounds, positions and limits are decimal strings and no outcome member is a float. quire-canonical refuses integers past 2^53 only, so RFC 8785 is compatible with the documents. `to_bytes` is today `serde_json::to_vec` in struct order, so this is new behaviour, as intended. (examined)
- Scope of the canonical rule. Analyze documents are assembled as `serde_json::Value` in fsl-analyze `write_run` (aggregate, registry), and monitor events/summaries and command documents are `Value`s in fsl-command. None goes through `OutcomeDocument::to_bytes`, yet AC-25 requires FR-520-AC-1's analyze golden file to equal "the `to_bytes` output" (FND-005). (examined)
- Permutation invariance against first-defect refusals (FR-1031) (FND-004). (examined)
- RFC 8785 against FR-865 and the driver. The text form is independent of JSON bytes, so there is no conflict. Driver FR-002-AC-9 encodes through ix-cli-kit FR-006, which sorts keys by Rust `String` order (UTF-8 bytes), not UTF-16 code units. The "identity" claim holds only for keys without both supplementary and U+E000..U+FFFF characters (FND-010). Driver FR-025-AC-2 (binary lines equal the canonical encodings FSL hands back) stays consistent. (examined)
- Monitor schemas. The event schema text treats `disposition` as FR-267's clause-run disposition. The refusal events in the AC-2 and AC-6 streams in the corpus have `clause` `null`, `basis` `unavailable`, and a request-refusal or FR-643 refusal record as `disposition` (monitor.rs:179-216) (FND-006). Today the summary writes `"ledger": null` for a finite stream (monitor.rs:803), against existing FR-644 text. AC-12 will expose this, so it is a code gap and not a finding on this diff. (examined)
- Hint placement. `hint` goes on `diagnostics` entries only, and the item `diagnostic` (FSpec FR-331, a closed external wire) carries none. This is sound and stated with its reason. (examined)
- Hint catalog. All 24 pairs exist in the code (every cause string is produced for its code, e.g. `missing_import`/`missing-selection` in fsl-cst). `invalid-request` and `io-error` diagnostics have no cause (command.rs:1283-1318). The table is enumerated, so FR-286-AC-30 can fail. `catalog_pairs()` placed in fsl-foundation cannot reach the typed causes of fsl-cst, fsl-semantics, fsl-route or fsl-replay, which depend on fsl-foundation (FND-003). The `(invalid-request, None)` row covers far more than the "unknown option" class (FND-008). The schema's hint bounds disagree with the stated byte/control-character form (FND-009). (examined)
- ACs. FR-286-AC-25..AC-27 and AC-29..AC-30, FR-644-AC-12 and FR-865-AC-8 can each fail. AC-25's "every operation" clause conflicts with the corpus naming rule (FND-007). The blank-label clauses of FR-286-AC-28 and FR-865-AC-8 are conditional on a row the table does not have (FND-012). (examined)
- TC-3110..TC-3115 follow their ACs step by step. (examined)

## Verdict

Not mergeable. Three high findings must be fixed in this PR:
- The closed diagnostic must list its forms-refusal members.
- The schema's member sources must cover FR-963's execute `registry` and FR-990/FR-1032's analyze item members.
- `catalog_pairs()`/`check_hint_catalog` must be placed where the typed causes are visible, or `known` must come from a catalog fsl-foundation can see.

The medium findings should be fixed in the same round: the permutation scope, the canonical rule bound to one writer, the monitor refusal event shape, AC-25's coverage clause, and the broad `invalid-request` hint row.

Sound as written: hint-on-diagnostics-only, the versioning rule (/1 for additive optional members, /2 for anything an existing reader cannot read), RFC 8785 with the documents' string numbers, and the FR-865 hint line order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Closed diagnostic schema omits `construct`, `text`, `declaration`, `profile`, which `OutcomeDiagnostic` writes on forms refusals; the Description and the member-order sentence list only cause, code, details, hint, label, locus, message | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:81-84 |
| FND-002 | high | Schema member sources omit the `execute` root `registry` (FR-963) and analyze item members from FR-990/FR-1032; the closed root and items reject real documents, and the corpus coverage rule does not force them in | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:547-565 |
| FND-003 | high | `catalog_pairs()` in fsl-foundation's diagnostic module cannot enumerate typed causes defined in fsl-cst, fsl-semantics, fsl-route and fsl-replay, which depend on fsl-foundation; AC-29's Ok case and UnknownPair check are unimplementable as placed | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:462-467 |
| FND-004 | medium | "for a refusal naming one defect, every permutation gives byte-equal output" is false for first-defect refusals chosen in request order (FR-1031) | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:375-377 |
| FND-005 | medium | The canonical rule is bound to `OutcomeDocument::to_bytes`, but analyze, monitor and command documents are `serde_json::Value`s written outside it; AC-25 demands FR-520-AC-1's `to_bytes` output, which does not exist | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:516-529 |
| FND-006 | medium | Monitor refusal events (clause null, basis unavailable, disposition a request-refusal object or FR-643 record) are not FR-267 dispositions; the schema text gives no shape for them though AC-2/AC-6 streams are in the corpus | spec/functional/FR-644-stream-observations-through-the-monitor-command.md:253-268 |
| FND-007 | medium | AC-25 requires a golden document of every operation, but the corpus holds one file per document an FR-286 AC builds and no FR-286 AC builds select, check_fences, lower, generate, prove, inspect or render | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:616 |
| FND-008 | medium | The `(invalid-request, None)` row matches every cause-less invalid-request diagnostic (malformed request JSON, output exists, clocks given twice, clause request, ...), so one text written for "an unknown option or a malformed option value" misdirects most of them | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:505-506 |
| FND-009 | low | Hint form says 1-200 bytes and no control character; schema `maxLength` 200 counts code points and the pattern excludes only U+0000-U+001F, so the schema admits texts the catalog check refuses | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:566-567 |
| FND-010 | low | "re-encoding through ix-cli-kit's json module is the identity": ix-cli-kit FR-006 sorts keys by UTF-8 byte order, RFC 8785 by UTF-16 code units; they differ for user-keyed maps (FR-650 record `fields`) mixing supplementary and U+E000-U+FFFF keys | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:524-529 |
| FND-011 | low | "Every object it describes is closed ... a member the type writes and the schema does not list fails" does not say how user-keyed maps (FR-650 record `fields`, counterexample assignments) are described | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:559-560 |
| FND-012 | low | FR-286-AC-28's and FR-865-AC-8's blank-label clauses ("the hint its pair's row gives or none", "when its pair has a row") cannot fail on hint presence, since the table has no `invalid_source_identity`/`blank-label` row | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:619 |
