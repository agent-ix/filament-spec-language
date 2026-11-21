---
id: SR-202611210022442313144108538
title: "QSL-2750 spec-review/base contract review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2750 PR3346; spec/functional/FR-643 and FR-644 contract delta"
review_set: subset
---

## Summary

FAIL for the frozen spec-only delta 49f9e7a18c..cf3bd8ba8ba2605a9284558847211fef6b6948ad in PR3346. The blank line before AC-13 ends each acceptance table. AC-13 and AC-14 in both files parse as a paragraph, not table rows, so rendered criteria and table-based extraction lose the new contract. Remove the two intervening blank lines to keep all four criteria in their existing tables.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The blank line before AC-13 ends each acceptance table. AC-13 and AC-14 in both files parse as a paragraph, not table rows, so rendered criteria and table-based extraction lose the new contract. Remove the two intervening blank lines to keep all four criteria in their existing tables. | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:419; spec/functional/FR-644-stream-observations-through-the-monitor-command.md:327 |

## Examined contract

The borrowed missing slice reuses FR-641's TimestampedMissingValuation rather than introducing another atom identity. Entries name the accepted zero-based observation index, not a counted call or stamp; batch atom admission remains authoritative. Timestamped absence reaches aggregate MissingValuation, while valid complete samples still pass ordinary admission. The library refuses semantic invalidity atomically; the stream reader refuses malformed JSON and non-timestamped presence at request stage, including an empty list. Omitted and empty timestamped lists remain equivalent. Retained-window absence survives progress and checkpoint/resume; preexisting finality and limits apply unchanged.

| User need | Requirement | Stakeholder need | New verification obligations |
| --- | --- | --- | --- |
| US-033 | FR-643 | StR-001 via US-033 traces_to | AC-13 batch/prefix/checkpoint parity; AC-14 invalid positions/atoms/kinds and rollback |
| US-033 | FR-644 | StR-001 via US-033 traces_to | AC-13 stream parity and empty/omitted equivalence; AC-14 strict reader/recovery |

No external CLI/API lookup, registry tie-break, pagination, scaffold or authentication assumption is introduced. Atomicity, selection identity and evaluation purity are explicit. The changed semantics have observable oracles and do not alter FR-194's existing aggregate behavior. Existing ledger and checkpoint references describe runtime protocol state, not repository tracking. No new dependency owner, duplicate semantic implementation, file-tracking mechanism or external assurance claim is introduced.

## Validation and limits

Focused quire validation of both changed files exits 0, with one preexisting agentless-passive warning on the unchanged usage paragraph. Markdown-it CommonMark with table enabled independently parses each new pair as paragraph_open, proving FND-001. The current base checklist's six coverage dimensions were checked in the new criteria: positive/negative, kind and integer boundaries, malformed input, rollback, progress and checkpoint transitions are specified. New implementation trace bindings and runtime evidence are intentionally pending this contract-first review; no claim of implemented acceptance is made.

A repository-wide ambient matrix command emitted extensive existing extraction/trace diagnostics; its output was not used as a clean coverage assertion. This is a bounded contract review, not a full traceability or reverse-source audit. No Cargo, aggregate gate or implementation test ran. No applicable AssuranceProfile exists under spec; direct review applies. Base and integrity were selected by the dispatching owner; other spec lenses and language review are not claimed. The later implementation needs its independent Rust/code and gap pass.

## Review data

```yaml
clean: false
scope:
- id: FR-643-AC-13
  path: spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md
  role: examined
  excerpt: 'Feed FR-194-AC-17''s timestamped observations one at a time, marking the
    aggregate atom absent with `{position: 18, atom: A}` on observation 18, then assert
    its closed progress complete through 300000. Each pending report equals the corresponding
    batch prefix with the same `missing` entries; the final report is `indeterminate`
    with `MissingValuation { position: 18, atom: A }`, never `invalid-value` or a
    quantile over the other values. A checkpoint immediately after observation 18,
    resumed before observation 19 and the progress assertion, gives the same reports
    and ledger as the uninte'
- id: FR-643-AC-14
  path: spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md
  role: examined
  excerpt: At next observation index 1, a missing entry whose position is 0 or 2,
    or whose atom is not a `holds` or aggregate node admitted by the selected clauses'
    batch run, refuses `invalid_runtime_input`/`invalid-value` naming `missing`. A
    nonempty missing slice on a `Finite`, `Progressed` or `Timed` session refuses
    likewise. Each refusal leaves checkpoint bytes and observation/count-call indices
    unchanged; the corrected observation is accepted at index 1. An empty slice is
    accepted wherever the same observation without absences is accepted.
- id: FR-644-AC-13
  path: spec/functional/FR-644-stream-observations-through-the-monitor-command.md
  role: examined
  excerpt: 'Stream FR-643-AC-13''s timestamped observations with `missing: [{"position":
    18, "atom": A}]` on observation 18. Events and final summary match the session
    and batch reports, including `MissingValuation { position: 18, atom: A }` and
    exit 11. An omitted `missing` and `missing: []` give identical events and summary
    for the same timestamped stream.'
- id: FR-644-AC-14
  path: spec/functional/FR-644-stream-observations-through-the-monitor-command.md
  role: examined
  excerpt: 'Between good timestamped observations, supply records with `missing: null`,
    a non-array value, entries with absent, unknown, duplicate or null members, negative,
    fractional, string or overflowing-u64 `position` or `atom`, and a duplicate top-level
    `missing`. Each yields one stage-`request` `invalid-request` refusal event naming
    the member and changes no session state; subsequent good records give the events
    and final clause reports of the stream without the refused records. A `missing`
    member on a progress record, or on a non-timestamped observation even as `[]`,
    is likewise refused '
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  check: trace
  artifact_id: FR-643-AC-13
  related_ids:
  - FR-643-AC-14
  - FR-644-AC-13
  - FR-644-AC-14
  path: spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md
  lines: 419-421
  excerpt: 'Feed FR-194-AC-17''s timestamped observations one at a time, marking the
    aggregate atom absent with `{position: 18, atom: A}` on observation 18, then assert
    its closed progress complete through 300000. Each pending report equals the corresponding
    batch prefix with the same `missing` entries; the final report is `indeterminate`
    with `MissingValuation { position: 18, atom: A }`, never `invalid-value` or a
    quantile over the other values. A checkpoint immediately after observation 18,
    resumed before observation 19 and the progress assertion, gives the same reports
    and ledger as the uninte'
  finding: The blank line before AC-13 ends each acceptance table. AC-13 and AC-14
    in both files parse as a paragraph, not table rows, so rendered criteria and table-based
    extraction lose the new contract. Remove the two intervening blank lines to keep
    all four criteria in their existing tables.
  method: spec-review/base
```
