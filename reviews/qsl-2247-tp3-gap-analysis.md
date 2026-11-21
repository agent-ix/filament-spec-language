---
id: SR-2026112115155546054652101
title: "gap-analysis (planless) of the QSL-2247 TP-3 monitor-configuration closure certifier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..d487f66c2ef9ce85d17454b52b1fe83a570d8549 (10 files) against FR-338, FR-127, FR-1020, FR-1030; QSL-2247 partial; PR agent-ix/filament-spec-language#3330"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
---

## Summary

Ticket: QSL-2247. This is a planless gap analysis. The computed matrix came from `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2) at the frozen head d487f66c. It was then interpreted for the criteria this diff touches: FR-338, FR-127, FR-1020 and FR-1030.

Repository-wide matrix at head: 4818 tagged, 1120 untagged, 104 method-without-symbol, 81 tagged-by-ignored-test and 13 retired criteria. The untagged FR-338/FR-127 criteria (for example FR-338-AC-1, AC-24, AC-27; FR-127-AC-3, AC-5) predate this diff, and the diff does not change their behaviour. They are recorded here as context, not as findings of this PR.

Changes in the diff's own criteria:
- FR-1030-AC-1 and FR-1020-AC-1 were tagged only by an `#[ignore]`d test before this change. They are now tagged by the un-ignored `a_step_for_step_refinement_transfers_previous_and_interval_clauses_proved`. That test has not been run.
- FR-1030-AC-2 and AC-5 stay tagged by `previous_transfers_only_through_a_step_for_step_refinement`. Its PrevStutter refutation and its `CasRefinesCounter` part are unchanged.
- FR-127-AC-1 gains two binders, the past and interval closure tests.

Stub, coverage-inflation and reverse-gap checks over the 10 changed files:
- The diff adds no stub, placeholder return, no-op trait method on a live path, or `todo!`.
- `SafetyAutomaton::configuration`'s default `Invariant` error is a typed refusal for explicit automata (`&Gba`), not a stub.
- The one reverse gap is FND-001 below.

## Verdict

**CONDITIONAL.** No touched criterion is untagged, and there is no high finding. Two lows: the new TP-3 certificate behaviour has no FR-338 acceptance criterion of its own, and FR-1030's statement text describes the pre-change TP-3 outcome without naming which classes stay uncertified. Runtime evidence for every binder named above is still pending, because the analysis ran no tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Reverse gap: FR-338's state-algebra row "TP-1 / TP-3 / deadlock / invariant → `["safety", branches]`" now has a production producer and checker for TP-3 (`Translation::Monitored`, `run_monitored`, `certificate()`'s Monitored arm). Unlike TP-4 (AC-23/AC-24/AC-25) and bounded `on each` (AC-28), no FR-338 acceptance criterion states the TP-3 vector: accepted certificate, distinct past/window keys, and the `SuccessorMissing`/`InitialMissing`/`BadState` negatives. The new tests therefore carry only `TC-525`/`FR-127-AC-1` tags, and the matrix cannot show TP-3 certification as covered or uncovered. Route an FR-338 AC addition through the spec owner (a spec change, not this code PR). Do not weaken any existing AC. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:317; fsl-analyze/tests/it/closure_certificate.rs:708,775; fsl-eval/tests/it/automaton_key.rs:1328 |
| FND-002 | low | FR-1030's statement says that "a past or interval (TP-3) clause no core checker certifies settles `failed`" and that "certifying the TP-3 forms … remain scope of QSL-2247". As a conditional, the first clause is still literally true. But the current behaviour is now that TP-3 on origin over an ordinary model subject is certified, which makes FR-1030-AC-1 satisfiable, while TP-3 `on each` and TP-3 over protocol subjects remain typed unproved (`failed`). A reader of the historical sentence cannot tell which classes remain. A current-behaviour clarification naming the remaining classes is needed, through the spec owner. This is not a weakening, and no AC changes. | spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md:134-141 |

## Coverage

- Criteria examined: FR-338-AC-1..AC-28 status rows (context); FR-127-AC-1 (examined, tagged); FR-1020-AC-1, AC-2 (examined, tagged); FR-1030-AC-1..AC-5 (examined, tagged; AC-1 newly un-ignored).
- Matrix source: computed (`quire matrix`); no run evidence ingested (`quoin matrix` evidence not consulted, because no runs exist for this head).
- Stub / inflation scan: changed production files, clean.
- Reverse gap: FND-001.
- Semantic review (intent↔test↔code): not run as a separate opt-in pass. The code-review artifact SR-2026112115155546054652100 records the intent↔test checks relevant to this diff.
- Plan completion: not assessed

## New findings (disposition pass 1)

Pass-1 source: PR #3330 head 943172bbeda2f23bd42754443278491dae142602 on base 01f289375f7a818f03919820396f333b1052bf89, fix commit only. No matrix rerun. The bindings below were read from the diff's `#[trace]` tags.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-338-AC-29's final sentence promises that a TP-3 clause "over a protocol subject … proof carries an empty [certificate] and settles `failed`". None of its four tagged binders runs a protocol proof or settles one. `a_tp3_clause_is_monitored_only_under_an_explicit_ordinary_subject` shows only that `instances_for(.., Protocol, ..)` refuses `NotTranslated` at item level, and `an_on_each_past_or_interval_proof_never_settles_proved` covers `on each` only. The protocol half of the AC is therefore tagged but not demonstrated. Add a protocol-subject TP-3 `Holds` with an empty certificate that settles `Failed`, or narrow the sentence to what is tested. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:1127; fsl-analyze/tests/it/closure_certificate.rs:960 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-338-AC-29 was added (TC-525) and restates the existing TP-3 behaviour. It is traced from the past, interval, BadState, explicit-kind, `on each` and key-body tests. A protocol over-promise in its last sentence is FND-003, and an undefined-origin wording conflict is in SR-2026112115155546054652102. |
| FND-002 | fixed | The FR-1030 statement now says that TP-3 `on origin` over an ordinary model subject is certified and settles `proved`, and that `on each` and protocol settle `failed`. The `ClosureStutterLoop` wording is kept, the remaining QSL-2247 scope is narrowed accordingly, and no AC changed. This is consistent with FR-1030-AC-1 (proved) and AC-5 (PrevStutter refuted). It is a clarification, not a weakening. |
