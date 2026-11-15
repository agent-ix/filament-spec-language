---
id: SR-20261115120000000001
title: "Combined review of the FSL completion S1 spec batch across the standard and the language"
type: SpecReview
analysis: base
scope: "agent-ix/filament-specification fsl-spec-batch-s1 @ 2410b70d4 (PR #658) and agent-ix/filament-spec-language fsl-spec-batch-s1 @ ab784ea70, each `git diff origin/main...HEAD`; area tickets S1-01 to S1-17"
review_set: subset
---
# Combined review of the S1 spec batch

## Summary

One reviewer read both commits as one batch. The lenses ran in this order: requirement quality, EARS conformance, integrity and cross-repo consistency, dependencies, scope boundary, evidence and test-matrix coverage, and failure domain. The ticket's four cross-ticket checks were run, and so was every shared-edit point the brief names.

Verdict: CHANGES REQUESTED. There are three blocking findings and nine major ones.

## Method

- `quire validate --scope .` over every changed `spec/**/*.md` file in each repo exits 0. The EARS warnings are listed under FND-017.
- `quire matrix --scope . --format tsv` was run in the language repo. Of the new or changed language criteria, 84 are `untagged`. Each new or changed standard criterion was checked for an `FSpec-` trace tag in the language tree or a tag in the standard's `tests/`; 79 of them have neither.
- Audit-path, probe and ticket-key grep over the added requirement text: clean. The only ids found are inventory ids such as `CF-IR-01`.
- Deferral and narrowing grep (deferred, out of V1, typed unsupported, not-yet-implemented): clean. The FR-230 change removes a `not-yet-implemented` refusal.
- No Python and no logic-bearing shell were added. No stub is keyed to a spec ticket. The G-258 and G-256 stubs are re-keyed to their code tickets.
- `cargo test -p fsl-semantics --features test-support --test it -- --ignored` was run. Seven ignored stubs already pass (FND-012). No other test binary was run.
- Checks that passed:
  - The analyze envelope carries every S1-04 item family and the sixteen FR-352 temporal kinds, with their categories and exits matching FR-285 and FR-301.
  - The FR-233 edits by S1-02 and S1-05 agree.
  - The two authors of FR-301-AC-4 and AC-5 agree.
  - The QSL-2168 parts (FR-680 scaling and FR-700 interval rule) agree.
  - The S1-07 FR-664 `parameters` refusal agrees with S1-06's selections.
  - The integer-division law identities agree across the two repos.
- Register rows: G-255, G-256, G-258 and G-259 to G-265 were traced to merged text. The other primary and secondary rows were spot-checked, not traced exhaustively.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Blocker, S1-05 against S1-02 and the standard. Std FR-540 and lang FR-640 add a `timed` clock configuration row. Std FR-322 still has three row kinds and calls timed the no-row alternative. It sets a timed clause's `clock_name` to `model-time`, a name S1-02 refuses over a trace subject, and it refuses any timed row. Both V2 schemas list three `kind` constants. Fix: add a `timed` `{unit}` row for a timed clause over a trace subject to FR-322's `temporal_bindings` table and to both schemas, with vectors. Limit the no-row rule and `model-time` to timed-model subjects. | FSpec FR-322:3720-3762; FSpec FR-540:33; FR-640:65; schema.json |
| FND-002 | high | Blocker, S1-03 against S1-06. Std FR-520 says analyze values use FR-181 forms (`{"type":"integer","value":"3"}`). Lang FR-621 uses the FSL run value form (`{"integer":"3"}`, AC-7), and lang FR-634 uses FR-181. Fix: choose one spelling. Recommended: FR-181 forms, because FR-520 owns the document and FR-331 counterexamples already use them. Then edit FR-621:116 and FR-621-AC-7. The alternative is to amend FR-520 "Values" and FR-634:49. | FSpec FR-520:75; FR-621:116,157; FR-634:49 |
| FND-003 | high | Blocker, S1-06 against S1-11. One selection has two wire forms. Lang FR-652 has `value_invariants{snapshot, invocation}`, admitted only in `items` and refused as a `clause` selection. Lang FR-702 and FR-312 have `invariants{observation, invocation}` as a `clause` selection. Fix: keep FR-652's form. Delete the `invariants` variant from FR-312 and FR-702, and rewrite FR-702-AC-3 as an `items` request. | FR-652:42-43,91; FR-702:85,103; FR-312:68 |
| FND-004 | medium | Major, S1-03. FR-331 nests `item_kind` inside `temporal`, and std FR-520 agrees. Lang FR-620 and its stubs put `item_kind` at the top level of the item while claiming FR-331's spellings. Fix: nest it in the FR-620 table, AC-1 and the stubs, or state the mapping in both FR-520 and FR-620. | FR-620:56,90; FSpec FR-520:52; FSpec FR-331:135; fsl-analyze/tests/it/analyze_cli.rs:152 |
| FND-005 | medium | Major, S1-03, scope boundary. Std FR-520 leaves the spelling and defaults of the cross-tool document to "FSL FR-620". Fix: move FR-620's request and item member tables into FR-520, and have FR-620 cite them. | FSpec FR-520:50 |
| FND-006 | medium | Major, S1-03 and S1-04. FR-620's `kind` row allows only FR-290 kinds, but FR-634 decides that `simulation` is outside FR-290. Fix: the row reads "an FR-290 kind, or `simulation` (FR-634)". Make the same change in FR-520 "Items". | FR-620:89; FR-634:28 |
| FND-007 | medium | Major, S1-07 against S1-05. FR-665's `handoff.clock_selections` is keyed by declaration name inside `native-compile/1`. FR-640's `clocks` is keyed by role. One request could then give two configurations for one clock, and the handoff package could differ from the compile's (FR-665-AC-1). Fix: delete `clock_selections`; the handoff uses the compile's `clocks`. | FR-665:60; FR-640:80-88 |
| FND-008 | medium | Major, S1-09. Std FR-142 calls `filament.op.quantity.scale` and `scale_div` FR-322 catalog entries. FR-322 and the `OperationIdentity` of both schemas lack them. Fix: add the catalog rows, an AC shaped like FR-322-AC-119, and the schema enum entries. | FSpec FR-142:286; FSpec FR-322:3570 |
| FND-009 | medium | Major, shared file `grammar_drift.rs` (S1-01 and S1-12). `CORRESPONDENCE` has no entries for `object-decl`, `population-member` and `operation-member`, which the S1-12 test maps to `ObjectDeclaration`, `ObjectPopulation` and `ObjectOperation`. The PascalCase fallback yields `ObjectDecl`, so the FR-600-AC-1 stub fails even after the code is written. Fix: add the three entries. | xtask/tests/grammar_drift.rs:68-77,104 |
| FND-010 | high | Major, S1-16, pin antipattern. FR-651 pins "TLA+ distribution 1.8.0" and "SANY 1.8.0". Fix: remove the version and say "accepted by the TLA+ SANY parser". | FSpec FR-651:39,98 |
| FND-011 | medium | Major, all tickets, evidence. Coverage gaps: 84 language ACs are untagged, 79 standard ACs have no trace, and 12 language TCs have no tagged test (TC-1665 to 1667, 1669, 1704, 1706, 1711, 1714, 1743, 1746, 1782, 1931). These have no stub at all: std FR-550 (S1-06); FR-561 to FR-563 (S1-07); FR-640 and FR-641 (S1-15); FR-144-AC-29 and AC-30, FR-145-AC-13 to AC-15, and FR-323-AC-36 and AC-37 (S1-13); FR-643 and FR-644 (S1-05); FR-730 (S1-14); FR-750 (S1-16). Fix: add an ignored failing stub with trace tags for each AC (Coverage). | quire matrix; Coverage |
| FND-012 | medium | Major, S1-10, S1-11 and S1-15. Seven ignored stubs already pass. Fix: remove `#[ignore]` from each, then run every other new ignored stub once. The seven are in `checker_rulings.rs` (`a_clock_parameter_checks_under_the_temporal_layer`, `a_clock_parameter_without_the_temporal_layer_is_gated`, `narrower_rational_results_refuse_unproved_range`, `catalog_steps_over_a_set_check`, `set_steps_outside_the_catalog_refuse`), `probabilistic_declarations.rs` (`a_negative_workload_weight_is_refused`) and `state_clauses.rs` (`a_range_obligation_not_provable_from_the_precondition_still_refuses`). | fsl-semantics/tests/it |
| FND-013 | low | Minor, S1-01 and S1-09, `Some`. FR-375-AC-17 and FR-501 item 4 type `Some(e)` as `Option<E>` with no condition. FR-143 gives the expected `Option<T>`, so `Some(3)` at an `Option<Int[0, 9]>` position has that type. Fix: qualify both with "at a position with no expected option type (FR-143)". | FSpec FR-375:836; FSpec FR-501:72 |
| FND-014 | low | Minor, S1-02. FR-610 says FR-640 reserves `model-steps` and `model-time` from configuration, but FR-640 has no such rule. Fix: add that refusal to FR-640, or drop the claim. | FR-610:70 |
| FND-015 | low | Minor, S1-03, failure domain. `deadline_ms` goes up to `u64::MAX` as a JSON integer, beyond the RFC 8785 precision limit that FR-650 enforces. Fix: cap it at 2^53-1 or use a decimal string. | FR-620:82 |
| FND-016 | low | Minor, S1-07, scope. Std FR-301 cites FSL FR-665 and FR-663 as its requirement source. Fix: state the behaviour in the standard. | FSpec FR-301:109 |
| FND-017 | low | Minor, EARS. Lines with unclassifiable, missing-subject, non-singular or vague statements: FR-098:222, FR-121:190, FR-611:40, FR-627:26, FR-643:145, FR-645:27, FR-651:34, FR-652:28, FR-660:40 and 107, FR-661:26, FR-665:89, FR-710:29, and FSpec FR-520:30, FR-610:33, FR-322:108. Fix: name the subject and use one SHALL per statement. | quire validate |

## Coverage

Untagged language ACs by ticket, from `quire matrix`:

- S1-03: FR-621-AC-4, AC-5 and AC-7; FR-622-AC-2; FR-624-AC-3.
- S1-04: FR-625-AC-5; FR-627-AC-2 and AC-3; FR-628-AC-2 and AC-3; FR-629-AC-2, AC-3 and AC-5; FR-631-AC-2 and AC-3; FR-632-AC-2 and AC-3; FR-633-AC-3; FR-634-AC-2 and AC-3.
- S1-05: FR-640-AC-4; FR-641-AC-4 and AC-5; FR-642-AC-4; FR-643-AC-2 to AC-5; FR-644-AC-1 to AC-4; FR-645-AC-5; FR-312-AC-8.
- S1-06: FR-650-AC-5 and AC-7; FR-651-AC-4; FR-652-AC-1; FR-100-AC-21; FR-106-AC-48.
- S1-07: FR-660-AC-5 and AC-6; FR-661-AC-2 and AC-3; FR-662-AC-1 and AC-2; FR-663-AC-2 and AC-3; FR-664-AC-2; FR-665-AC-2.
- S1-08: FR-670-AC-5 to AC-7; FR-110-AC-12; FR-098-AC-27.
- S1-10: FR-230-AC-63.
- S1-11: FR-702-AC-3; FR-391-AC-8; NFR-130-AC-2.
- S1-12: FR-710-AC-5.
- S1-14: FR-730-AC-1 to AC-3; FR-731-AC-2.
- S1-15: FR-740-AC-2 to AC-4; FR-201-AC-1; FR-203-AC-1, AC-5 and AC-8.
- S1-16: FR-750-AC-1, AC-2 and AC-5; FR-751-AC-2 and AC-3; FR-753-AC-3 and AC-4; FR-754-AC-3; FR-755-AC-3; FR-757-AC-2 and AC-3; FR-282-AC-20.
- Not attributed: FR-345-AC-9.

Untraced standard ACs by ticket:

- S1-01: FR-339-AC-5; FR-500-AC-3; FR-503-AC-2.
- S1-01 and S1-09: FR-375-AC-17.
- S1-05: FR-541-AC-2.
- S1-06: FR-550-AC-1 to AC-3.
- S1-07: FR-301-AC-5; FR-467-AC-1; FR-560-AC-2; FR-561-AC-1 to AC-3; FR-562-AC-1 and AC-2; FR-563-AC-1 and AC-2.
- S1-08: FR-570-AC-5.
- S1-10: FR-415-AC-28.
- S1-11: FR-346-AC-2; FR-376-AC-8 and AC-9.
- S1-12: FR-610-AC-2 and AC-5 to AC-8.
- S1-13: FR-144-AC-29 and AC-30; FR-145-AC-13 to AC-15; FR-323-AC-36 and AC-37; FR-352-AC-10 and AC-11.
- S1-15: FR-187-AC-1; FR-405-AC-5, AC-10 and AC-11; FR-406-AC-6 and AC-7; FR-407-AC-16; FR-411-AC-17; FR-420-AC-8; FR-640-AC-1 to AC-5; FR-641-AC-1 to AC-3.
- S1-16: FR-190-AC-8 to AC-10; FR-192-AC-9 and AC-11; FR-193-AC-22; FR-194-AC-9 and AC-10; FR-297-AC-5; FR-650-AC-2 to AC-6; FR-651-AC-2, AC-4 and AC-5; FR-652-AC-2 to AC-4.
- S1-17: FR-660-AC-1 and AC-5; FR-661-AC-1, AC-2 and AC-4.

## Round 2 (standard 1f2255d23, language 130d27c36)

Verdict: CHANGES REQUESTED. Two medium findings remain open, and both are about stubs.

The round-1 Coverage lists above are superseded by this round's `quire matrix` run. The standard `FR-187-AC-1` in the S1-15 list was a false positive: no such criterion exists.

### Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 | Resolved. FR-322 adds the `timed {unit}` row for trace subjects only, and the timed-model no-row case keeps `model-time`. `schema.json` gains the `timed` alternative. The vectors TN-06 to TN-08, FR-322-AC-450 and the `tc_1164` test are added. |
| FND-002 | Resolved. FR-621 and FR-520 use FR-181 typed forms, and run documents keep FR-650. FR-621-AC-7 and the `analyze_coverage_cli.rs` stub assert `{"type":"integer","value":"3"}`. |
| FND-003 | Resolved. `value_invariants` (FR-652) is the only invariants selection. FR-312, FR-702 and FR-664 are aligned. |
| FND-004 | Resolved. `temporal.item_kind` is used in FR-620, FR-520 and the stubs. |
| FND-005 | Resolved for the members FR-520 now tabulates. FND-021 is a residual from this fix. |
| FND-006 | Resolved. `kind` allows FR-290 kinds or `simulation`. |
| FND-007 | Resolved. The handoff uses the compile's `clocks` member. |
| FND-008 | Resolved. FR-322 defines the scale rows and FR-322-AC-451. The schema enum is deferred to QSL-2256 by lead ruling; see FND-020. |
| FND-009 | Resolved. The three correspondence entries are added. |
| FND-010 | Resolved. The SANY version is removed. |
| FND-011 | Mostly resolved. The remaining gaps are FND-018, FND-019 and FND-022. |
| FND-012 | Resolved. The seven `#[ignore]` attributes are removed. |
| FND-013 | Resolved. FR-375-AC-17 and FR-501 are qualified. |
| FND-014 | Resolved. FR-640 adds `reserved-clock-name` (FR-640-AC-6). |
| FND-015 | Resolved. `deadline_ms` is capped at 2^53-1. |
| FND-016 | Resolved. FR-301 states the behaviour itself. |
| FND-017 | Resolved. The remaining warnings, at FR-098:222, FR-121:190 and FSpec FR-322:108, are on lines already on main. |

### New and residual findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-018 | medium | S1-14. The FR-730 stubs assert only that the loop items are derived. They do not check the decided settlement: `proved` with exploration complete (AC-1), `refuted` with a counterexample that replays (AC-2), and `inconclusive` naming the state limit (AC-3). Fix: run each item through `analyze` and assert its record, counterexample, replay and cause, keeping the stubs ignored under their code ticket. | fsl-analyze/tests/it/loop_proof.rs:67-99; FR-730:135-137 |
| FND-019 | medium | S1-15 and S1-16. FR-203-AC-8 and FR-282-AC-20 still have no tagged test. Both require an exact certificate that `analyze` writes to be accepted by the independent checker. Fix: add an ignored stub to `tools/fsl-probability-check/tests/it/certificate_kinds.rs` with traces for `TC-2065`/`FR-203-AC-8` and for FR-282-AC-20's TC, keyed to the probability code ticket. | FR-203:174; FR-282:132 |
| FND-020 | low | S1-09. FR-322's prose and FR-322-AC-451 say both V2 schemas and the external catalog list the two scale identities. The lead deferred the schema change to QSL-2256, but no stub holds that part. Fix: add an ignored standard test in `tests/checked_package_v2.rs`, keyed QSL-2256 and traced to `TC-1326`/`FR-322-AC-451`, asserting the `OperationIdentity` entries after `filament.op.quantity.pow`. | FSpec FR-322:5004 |
| FND-021 | low | S1-03. FR-520's member table still takes the meaning of `program`, `package`, `models`, `libraries`, `refinement_internals`, `required`, `trace` and `simulation` from FSL requirements (FR-026, FR-027, FR-507, FR-634, FR-636). Fix: cite the standard's native run request (FR-323) and state the `simulation` and `trace` members in FR-520. | FSpec FR-520:50-72 |
| FND-022 | low | S1-03 and S1-07. The stub for the unrouted-verb criteria (FR-624-AC-4 and AC-5, FSpec FR-301-AC-4, FSpec FR-522-AC-1) was removed and now lives only in the driver. These rows stay untagged in both matrices, and TC-1665 has no tagged test. Fix: move AC-4 and AC-5 to the driver's spec, citing them from FR-624, or tag the driver test. Tag the FR-642-AC-5 stub with `TC-1665`. | FR-624:90-91; FSpec FR-301; FSpec FR-522 |
