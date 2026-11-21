---
id: SR-202611211854272
title: "failure-domain review of FSL PR3373 QSL-2739 provider answers, forged witnesses and guard identity"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d; FR-1044 Registered provider, Outputs, AC-11..AC-14; FR-530 Settlement; upstream FSpec #792 5423fcf482 FR-331 core-guard and counterexamples, FR-492 Interface mismatch; FSL FR-290 Replay context"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This review looks for the ways the routed composition path in PR #3373 could get a result wrong: a forged or malformed provider witness, a guard that the identity rules let through, degenerate compositions, and order-dependence.

These parts are sound:
- **Producer-only cause.** FSL writes `provider-answer-unconfirmed`, and a cause a provider supplies is refused at `/cause/type` on every kind (FR-1044-AC-13 = FSpec FR-331-AC-65, FR-492-AC-10).
- **Mismatch versus unproved result.** A wrong supplied result is never treated as a mismatch.
- **Assumed clauses.** A non-empty assumed clause is never run as the clause alone.

The defects are in how a rejected `refuted` answer is classified, and in two identity and degenerate cases that have no criterion behind them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1044 Behavior "Interface mismatch" and AC-14 settle a malformed `interface-mismatch` witness `inconclusive` `replay-refused`/`unknown_required_feature`. That covers a witness with an extra member, a `reason` spelled other than `absent`/`weaker`, or (by implication) a missing member. Upstream gives these shape defects to the schema and the reader, not to replay. FSpec FR-331-AC-66: "it refuses … an `interface-mismatch` missing a member, with another `reason` or with an extra member". So a plugin `result` frame fails its stage schema, and FR-462 settles the item `failed` `tool-failure` `malformed-body` (the FR-331 pattern at line 956). A linked provider's entry is refused by the reader `invalid_result` at the member. The PR's choice contradicts the upstream outcome for those cases. Fix: split the rejection. (a) A shape defect follows FR-331-AC-66 and the reader: plugin → `failed` `malformed-body`; linked → `invalid_result`/`unknown-member` or `invalid-value` at the member. (b) Only a closed, well-formed witness that does not match keeps the disposition FND-002 settles. (c) An assignment counterexample on a composition keeps `replay-refused`/`unknown_required_feature`. Adjust AC-14's variant list to match. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:326-332,407 |
| FND-002 | medium | Needs architecture triage; the code is not settled here. Upstream does not say what the producer does with a closed, well-formed `interface-mismatch` whose values differ from the recomputed first mismatch (wrong `component`, `interface`, `premise` or `reason`, or any witness for a declaration with no mismatch). The author's note 2 chose `replay-refused`/`unknown_required_feature`, but the authority it cites does not license that choice. FSL FR-290 "Replay context" (not FSpec FR-290, which has no such section) uses `replay-refused` + `unknown_required_feature` for a **non-replayable** item. FSL FR-290 Behavior (lines 504–506) sends a counterexample that replay **does not reproduce** to `replay_parity`, and a refusal from replay itself to `replay_refused` with that refusal's own code. A recomputation that runs and disagrees is "does not reproduce", which points to `replay-parity`. It is not an `unknown_required_feature` (unsupported construct) case. FSpec FR-331-AC-63 is the analogue for other kinds: a reader refuses a body whose witness root or deciding node disagrees. That would be a third outcome. Do not invent a new cause. Root should take this to architecture to pick one of: (i) `replay-parity`, by FSL FR-290 precedent; (ii) a reader refusal, by the FR-331-AC-63 analogy; (iii) keep `replay-refused`, with FSL FR-290 amended to own it. Then FR-1044's cause, its citation and AC-14 should be made to match the ruling. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:326-332; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:332-376,504-506 |
| FND-003 | medium | The key identity confusion has no end-to-end criterion. FR-1044-AC-8 builds a request in which item 4 (`Capped`, slot key assumptions `[]`, from `Sys`) and item 5 (`Capped`, assumptions `[0]`, from `SysAGRely`) have **equal** `request` members. Item 4 is `proved` and item 5 is `unsupported`. In that request, a provider `proved` for `SysAGRely` must become `provider-answer-unconfirmed` with `premise` 1, and must never stand because of item 4. AC-12 runs its guard cases on single-composition requests and leaves this one to a seam-level phrase ("another assumption array"). Read literally, the upstream matching rule (FR-331:921-925: the held item's identity members equal the key's item members, and the key's `assumptions` equal the derived `k`th array) would match item 4. FSL is safe only because it matches by the request index of the typed slot key (`CompositionClaim::required`). Fix: add to AC-12 the AC-8 request (`Sys`, `Sys`, `SysAGRely`) with the provider answering `proved` for every composition. `Sys`'s compositions stand, and `SysAGRely` settles `provider-answer-unconfirmed` `premise` 1. Also fix AC-12's opening, which reads "answering `proved` for `SysAG`" but then uses `SysAGRely`'s assumed `Capped`. Name the composition each case answers for. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:401,405; fsl-analyze/src/analyze/document/composition_item.rs:433-460 |
| FND-004 | low | Licensed, but no criterion covers it. A composition with no `component-guarantee` entry keeps the provider's `proved` (FR-1044:312-314). Upstream FR-331's core-guard licenses this: "When every entry has a matching held item settled `proved`, the answer stands" holds vacuously, and FR-491's composed claim is then the empty conjunction. It is not a false proof. FR-491 static checks allow such a declaration: two components, no `guarantee`, so no `rely`/`depends` is possible, and only `environment`/`supplies` members remain. But no FR-1044 acceptance criterion states it, and the existing test `a_proof_with_no_guarantee_to_check_is_refused` (document.rs:3913) flips. Fix: add the zero-entry case to AC-12's seam sentence as "stands", so the changed test has a criterion behind it. No upstream cause is needed. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:312-314,405; fsl-analyze/src/analyze/document.rs:3910-3921 |
| FND-005 | low | The reason-major order on the **routed** admission is not exercised. AC-14 checks one `absent` witness on `SysAbsent`, and the `weaker` and absent-over-lower-weaker cases exist only on the planner (FR-530-AC-9). FSL's recomputation decides admission on the routed path. A routed recomputation that returned the lowest-slot mismatch whatever its reason would pass AC-14. Fix: state in AC-14 (a seam-level test is enough, since author note 4 says the `weaker` CLI fixture is prose-only) that the routed recomputation is the same one FR-530-AC-9 runs, and that a provider witness naming the slot-3 `weaker` refuses where slot 4 is `absent`. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:407 |
| FND-006 | low | Info only, an upstream note for root/architecture. It does not block FSL. FSpec FR-331's core-guard matching sentence (lines 921–925) compares the key's `assumptions` with the composition's own derived `k`th array. On the routed path that is the same array by construction. The rule therefore does not separate two held items with equal request identities and different typed contexts inside one request (FND-003). The intent ("one proven context is never relabelled as another") and FSL's index-by-typed-key reading are sound. Consider an upstream clarification that the held item is the one derived for that entry's typed slot key. | upstream FSpec FR-331-backend-provider-envelope.md:913-946 |

## Verdict

**Changes required:** FND-001 to FND-005. FND-002 needs architecture triage through root before the author settles the cause. FND-006 is an info-only upstream note.

Examined and clean:
- **Guard scope.** The guard applies to every `proved` composition that carries `premises`, including a request that holds no component items.
- **Producer-only cause.** The cause carries only `premise` and no certification, and it is neither `replay-refused` nor `premise-unproved`.
- **Admitted mismatch.** It settles `refuted`/`decisive-counterexample` with the witness built, not replayed. It carries no certification and no conclusion, and the premise records are kept.
- **No provider, no decision.** With no provider registered the recomputation never decides and the item stays `unsupported`.
- **Unproved results stay unproved.** A present, not-weaker interface whose premise did not prove is never a mismatch (AC-14 last sentence, FR-530-AC-2/AC-9).

## Scope and evidence

These are spec-level findings. Proposed tests were neither built nor run. The bounded code-seam check read document.rs:3807-3925 (`gate_compositions` and its tests), composition_item.rs:433-460 (`premise_items`, where `required` is per selection, deduplicated by `ItemKey` including assumptions) and provider.rs:319-353 (`CompositionTarget`, which has premises only).

- FR-1044 Registered provider (lines 282-332), Outputs (199-203), AC-8, AC-11..AC-14: examined.
- FR-530 Settlement and outcomes: examined.
- FSL FR-290 "Replay context" and Behavior 498-506: context_only.
- FSpec FR-331 core-guard, AC-63, AC-65, AC-66; FR-492 Interface mismatch: context_only.

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new Malformed witness bullet sends a missing or extra member, or a bad reason, to the FSpec FR-331 readers (AC-66: plugin schema, linked invalid_result at the member), and FSL SHALL NOT use its own cause. The assignment counterexample keeps replay-refused/unknown_required_feature under FSL FR-290 Replay context. AC-14 keeps the two paths in separate clauses. |
| FND-002 | fixed | Applied per the architecture ruling. A closed, well-formed witness that differs in component, interface, premise or reason, or that answers a declaration with no mismatch, is a reader refusal invalid_result/invalid-value at /counterexample (FSpec FR-331-AC-63), and the item is never settled refuted. It is distinct from the shape failures and from the assignment path. FSL FR-290 is not amended. AC-14 matches: the witness is built, not replayed. |
| FND-003 | fixed | AC-12 adds the AC-8 request (Sys twice plus SysAGRely), all answered proved. Both Sys stand. SysAGRely is provider-answer-unconfirmed with premise 1, because Capped at index 5 has the same wire request as item 4 but a different typed context and is unsupported. Each case names the composition it answers for. |
| FND-004 | fixed | AC-12's seam sentence now says a proved answer for a composition with no component-guarantee entry stands. That is licensed by FSpec FR-331's core-guard (vacuous) and is not a false proof. |
| FND-005 | fixed | The routed Behavior reuses the FR-530-AC-9 recomputation. AC-14's seam case puts weaker at slot 3 and absent at slot 4: the slot-4 witness is accepted and the slot-3 witness is refused at /counterexample. It is seam-level only, with no invented CLI weaker fixture. |
| FND-006 | no_change_needed | Info-only upstream note, not a defect. FSL's typed-slot-key reading is kept, and AC-12 now exercises it. No upstream edit and no ticket. |
