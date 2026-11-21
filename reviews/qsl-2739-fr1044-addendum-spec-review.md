---
id: SR-202611211049161
title: "base spec review addendum of FR-1044 provider forwarding and current system behavior (QSL-2739 PR3287)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md lines 253-289 only (diff 773e661d31..6ff10e952a660f72c8452dd1294e286619ea4818)"
review_set: subset
---

## Summary

Ticket: QSL-2739. Scoped addendum after the contract cleared at 1513bee. Only two added parts of FR-1044 at 6ff10e952a were reviewed: the provider-forwarding Behavior bullet (lines 253-261) and the "Current system behavior" section (lines 263-289). Each was checked against FSL FR-290, FR-635, FR-850, FR-940 and FR-621, against the arch rulings in the lead-supplied ruling file, and against the read-only open-paths evidence. The original contract and ACs were not re-reviewed.

The current-behavior section matches the actual boundaries. Handoff admission may refuse `missing_import` because the dependency closure is not persisted. The `not-yet-implemented` exit 21 is stated only for an admitted package. There is no source-recompile fallback. The `unnamed-claim` guard is the existing package-route family. Text rendering belongs to the driver, which agrees with FR-621 "Text rendering" ("the driver prints"). AC-7, AC-10 and the text fixture stay as written and open.

The forwarding bullet has one normative contradiction, which needs a decision, and two mechanical wording defects. The AC-7 current-behavior bullet names a route that does not exist.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new forwarding bullet settles a registered composition provider's `proved` verdict as `proved`, with no premise condition and with no premise result sent to the provider. FR-1044 lines 241-243 say FSL "SHALL NOT settle the composition `proved`" when a component-guarantee premise does not settle `proved`. So under AC-9's request (Capped `refuted`), or AC-5's (Capped `unsupported`), with a registered plugin answering `proved`, the two SHALL statements contradict. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:257 |
| FND-002 | medium | "`proved` carries the provider's own certification" contradicts FR-635 (lines 150 and 172-174) and FR-290 (lines 75-77 and table rows 89-93). FSL assigns the certification: `trusted` for a third-party plugin and `uncertified` for a linked first-party provider, never the provider's claimed one. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:257 |
| FND-003 | low | The provider outcomes omit their closed causes. FR-635 line 151 gives `unknown` as `inconclusive` `solver-inconclusive`. FR-290 "Replay context" (lines 366-372) gives a non-replayable item's counterexample as `inconclusive` `replay-refused`, and a composition item is such an item. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:257 |
| FND-004 | low | The AC-7 current-behavior bullet says the fixed-point item "is written ... only for a checked package that carries the lattice". No public route reaches that path. The AC-10 bullet in the same section refuses every package handoff that declares compositions, and no adopted artifact member carries a lattice (FSpec FR-176/FR-491 state only the condition; the open-paths evidence names the checker's lattice setter as test-only). | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:271 |

## Verdict

Not clear. FND-001 needs a lead/arch decision. FND-002 to FND-004 are mechanical.

FND-001 rules in conflict, at 6ff10e952a:
- FR-1044 lines 241-243: "When a component-guarantee premise does not settle `proved`, FSL SHALL return that premise's own record and SHALL NOT settle the composition `proved`."
- FR-1044 lines 256-260: "`proved` carries the provider's own certification ... FSL sends no premise result and no component proof with it."
- No legal provider-path cause exists for "premise not proved". FSpec FR-331 lines 842-849 make `premise-unproved` planner-only: "No provider's entry, plugin or in-process, carries one".

Bounded options (no provider algorithm):
- **(a), recommended.** Conservative finish: FSL keeps a provider's `proved` for the composition only when every component-guarantee premise item of that composition in the same request settled `proved`. Otherwise it settles the composition `inconclusive` with an FSL-only cause, alongside FR-290's FSL-only `replay-parity`/`replay-refused` causes. Arch names the cause, or reuses an existing FSL-only spelling.
- **(b).** Narrow lines 241-243 to FSL's own settlement, so a plugin's `proved` stands as `trusted`. This accepts a composition proof that no premise result backs in PR1, and it is not recommended.

Mechanical fixes:
- **FND-002:** "`proved` with FR-635's certification (`trusted` for a plugin, `uncertified` for a linked first-party provider)".
- **FND-003:** "`unknown` is `inconclusive` `solver-inconclusive`; a `refuted` verdict is `inconclusive` `replay-refused` (FR-290 non-replayable item)".
- **FND-004:** "no public request reaches the `fixed-point` item: the source route meets no lattice and the package route refuses compositions (below)".

Clean and examined:
- Forwarding: negotiation to a registered composition backend (FSpec FR-348-AC-3), and the item carrying its derived `premises` keys and `options` through the generic external-provider request with no premise result or proof. This matches the arch ruling (generic forwarding plus premises transport, no built-in algorithm). Refutation and wire questions are left to PR2.
- AC-7 bullet: the absent source lattice declaration and the standard ownership agree with FSpec FR-176 lines 39-45 and FR-491 lines 113-120.
- AC-10 bullet:
  - Both refusals are whole-request and conditional. `missing_import` arises at admission; `not-yet-implemented` (exit 21) applies only over an admitted package, so there is no unconditional exit-21 promise.
  - There is no recompile fallback (FR-940).
  - `unnamed-claim` is the pre-existing package-route pending family (present at 773e661), not a new code.
  - The package-consumer API lane owns the package form.
- Text bullet: consistent with FR-621 "Text rendering" and with the FR-621-AC-6 test, which checks only the members the driver renders from.
- No AC is weakened. AC-1 to AC-10 and the fixture's "machine and text mode" are byte-unchanged in this diff.

## Scope and evidence

Spec-only. I ran no Cargo or runtime, and runtime on 6ff10e952a is queued and not green. `quire validate` on FR-1044 at 6ff10e952a: clean.

- FR-1044 provider-forwarding bullet, lines 253-261: examined.
- FR-1044 "Current system behavior" section, lines 263-289: examined.
- FR-1044 Behavior lines 241-243: context_only.
- FR-635 lines 140-176, FR-290 lines 66-93 and 332-372, FR-850 lines 240-247, FR-940 line 594, FR-621 "Text rendering": context_only.
- FSpec FR-331 lines 842-849, FR-176 lines 39-45, FR-491 lines 113-120: context_only.
- Lead-supplied arch ruling and read-only open-paths evidence: context_only, treated as routed decisions and evidence, not normative text.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Lines 276-277 call replay-refused 'FSpec FR-331's cause for a result FSL does not accept'. FSpec FR-331 (origin/main f9d88c77) lines 856-858 define it more narrowly, as a core-only cause 'that only the core's own replay or Kani run settles: ... replay-refused (FR-399, and a bound contract-counterexample a reader cannot re-execute, FR-485)'. A premise gate is not a replay. Under the arch rule (report the exact line and use the nearest existing allowed cause, add no cause), replay-refused is still the nearest allowed cause. premise-unproved is excluded for an item that reaches a provider (FR-331 lines 842-849), and hypotheses-not-established is FR-468's invariant-hypothesis cause (line 778). Mechanical fix: replace the parenthesis with 'the nearest existing FSL-only (core-only) cause; FSpec FR-331 lines 856-858 define it for the core's replay, and its use for a refused provider proof is a standard gap recorded as remaining work: QSL-2739'. Keep the cause and the code. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:275 |
| FND-006 | low | Lines 262-264 cite 'FSpec FR-290 "Replay context"' and 'FSpec FR-635', but those are FSL requirements: FR-290 'Settle plugin results as typed terminal records', with Replay context at lines 332-376, and FR-635 'Route analyze items to external providers'. FSpec FR-290 is the protocol claim kind, which has no Replay context, and FSpec has no FR-635. Fix: drop 'FSpec' from both citations. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:262 |
| FND-007 | medium | 'An item sends its options only to a backend it names (FR-967)' contradicts FSL FR-243 lines 52-53, where a provider option is 'negotiated against each candidate's manifest like every other provider option (FR-635, FR-075)'. FR-967 governs only an item that names a backend. Read literally, the derived fixed-point item, which never names a backend, could never forward its profile options, contradicting the same bullet's 'send it ... its options'. Fix: 'its options are negotiated as for any provider item (FR-967 for a named backend, FR-243/FR-635 against the candidates' manifests otherwise)'. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:256 |

## Dispositions

Round 1, fix commit cf48c388fc3fe694ec6a367771f0ad8a2982a325 over 6ff10e952a; verified against the committed text.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | round 2, fix commit 19ff988cb9 |
| FND-006 | fixed | round 2, fix commit 19ff988cb9 |
| FND-007 | fixed | round 2, fix commit 19ff988cb9 |
