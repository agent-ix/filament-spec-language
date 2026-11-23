---
id: SR-3594001
title: "spec-review of FR-204-AC-18 given delays racing under unequal weights (filament-spec-language PR 3594)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@505fbf2d161f07d040d647541f69434cbb4c94dc; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md (FR-204-AC-18, Worked example: given delays racing under weights); spec/test-cases/TC-3204-given-delays-race-under-unequal-weights.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3204
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: references
---

# Spec review of FR-204-AC-18 (filament-spec-language PR 3594)

## Summary

Ticket: QSL-2884. One spec-review pass over `git diff origin/main...505fbf2d`.
It covers the new FR-204-AC-18 row, the new section "Worked example: given
delays racing under weights" and the new TC-3204. The oracles were ADR-026
SD-1 to SD-3, FR-204's Behavior bullets on races under a workload, FR-196's
race-edge label rule and canonical order, FR-201's rejection table and its
Emission rules, and FR-921 and FR-923.

Sub-analyses: integrity (SR-3594002) and criterion-strength (SR-3594003, done
by hand because no Jev client is installed). EARS was not run: the diff
changes no FR, NFR or StR requirement statement, only an AC row, a worked
example and a TC.

Every number was recomputed by hand from `Trio`. The conditioned laws are A
1/2 at 1 or 2, B 1/2 at 2 or 3, and C 1/4 at 2 and 3/4 at 3. Every support
lies inside the window `[0, 3]`, so conditioning changes nothing. In units of
1/192:

- Draws: the four draws with A=1 come to 12+36+12+36 = 96. Then (2,3,3) = 36,
  (2,2,3) = 36, (2,3,2) = 12 and (2,2,2) = 12, for a total of 192.
- The AB tie with weights 1 and 2 splits 12/24. The AC tie with weights 1 and
  3 splits 3/9. The ABC tie splits 2/4/6.
- A = 96+36+12+3+2 = 149, so 149/192. B = 24+4 = 28, so 7/48. C = 9+6 = 15,
  so 5/64. A at 2 ms = 36+12+3+2 = 53, so 53/192. The edge masses sum to 192.
- With even ties, A = 160 (5/6), B = 22 (11/96) and C = 10 (5/96). With ties
  taken by canonical order, A = 1 and B = C = 0.
- Three-way orders: ABC 1/6·2/5 = 1/15, ACB 1/10, BAC 1/3·1/4 = 1/12, BCA 1/4,
  CAB 1/2·1/3 = 1/6 and CBA 1/3. They sum to (4+6+5+15+10+20)/60 = 1.
- The claims: 7/48 ≈ 0.146 ≥ 0.13, 5/64 ≈ 0.078 ≥ 0.07, 149/192 ≈ 0.776 ≤ 0.8,
  and 1/2 ≥ 0.5 at equality. All four are proved. The even checker refutes
  the b, c and a claims (0.115, 0.052, 0.833), and so does the canonical-order
  checker (0, 0, 1).

Every number in the diff is correct.

Examined, clean:

- **Edge label.** Each edge is labelled `Identity(A)`. This matches FR-204
  ("each race edge carrying `Identity(r)` with `r` the first racing identity
  in canonical order") and FR-196 ("labels each race edge `Identity(r)` with
  `r` its first racing identity"). A races in every draw.
- **Action set.** There is no `FreeDelays` or `WorkloadResolve` action. This
  is correct for an all-given state: FR-204 gives `WorkloadResolve` only to
  an all-free state and `FreeDelays` only to a mixed state.
- **Tie order.** SD-3 says ties are drawn "without replacement by ADR-024
  PM-2's weights and then uniformly within an operation". The weighted
  first-pick marginals used here follow it.
- **Rejection rule.** FR-201's table checks the per-state conditions before
  the threshold. EN-5 emits an `Exact` certificate from an exact value. In
  the rebuilt product, the even or canonical document differs only at the
  initial state, because every post-race state has `phase = Done` and nothing
  races. So the initial state is the first failing state, and the rule is
  `InequalityFails`, not `ThresholdMissed`. FR-921 is the applicable timed
  checker, and it defers to FR-201. FR-923 covers the every-scheduler `tie`
  rows and does not apply under `Skew`. FND-001 records the one caveat.
- **Gates.** `quire validate` on both changed files exits 0. FR-204-AC-18 and
  TC-3204 are new and unique on origin/main and on every remote branch. The
  only branch that carries TC-3204 is `QSL-2884-spec`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-204-AC-18's certificate clause does not name the certificate kind or the item. `InequalityFails` follows only for an `Exact` certificate, where 11/96 ≠ 7/48 fails the equality at the initial state. For a `Lower` (or FR-1042 value-bounds) document, 11/96 ≤ 7/48 = 14/96 and 0 ≤ 7/48 both satisfy the lower inequality. The conditions then hold, and FR-201's table gives `ThresholdMissed` instead. The clause "a certificate whose values order ties evenly ... or by canonical order" also leaves open whether both documents are tested. Fix: write it as "the `t.b` item's `Exact` certificate with its values computed with even ties (initial `11/96`), and the one with ties taken in canonical order (initial `0`), are each rejected `InequalityFails` at the initial state's `ProbabilisticProductState`". | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:253 | wrong-requirement |
| FND-002 | medium | SD-3's "then uniformly within an operation" branch has no criterion. AC-18 gives each of A, B and C its own operation, as the worked example's last sentence admits. An implementation that gives every identity of an operation that operation's full weight would pass every FR-204 AC, and so would one that splits the weight. It should be added in this PR rather than as a follow-up ticket, because it is the same tie rule. Fix: add one AC with an operation that has two racing identities (for example a receiver-parameterised `B`), tied with `A`, whose exact value differs between "operation weight, then uniform" and "full weight per identity". | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:298-303 | missing-requirement |
| FND-003 | medium | The without-replacement order beyond the first pick cannot be observed. Every operation in `Trio` sets `Done`, so only the first tied identity fires. The worked example's six permutation probabilities (ABC 1/15 … CBA 1/3) feed no checked value. An implementation that draws only the winner by weight and orders the rest evenly, or by canonical order, passes AC-18. Fix: in the same new AC as FND-002, or in a `Trio` variant whose operations leave `phase = Idle` and set only their flags, check one claim that depends on the second position, such as "`b` set before `c` at the same instant", whose exact value comes from the 1/15 … 1/3 table. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:296-303 | missing-requirement |
| FND-004 | medium | TC-3204 does not cover all of AC-18. Its procedure and Expected Results submit only the even-ties certificate (`11/96`). AC-18 also requires the canonical-order certificate (`b`: `0`) to be rejected `InequalityFails`, and no step does that. Fix: add the canonical-order certificate, with its rejection, to the Test Procedure and Expected Results, and name the certificate kind (see FND-001). | spec/test-cases/TC-3204-given-delays-race-under-unequal-weights.md:19,27 | correct-requirement-no-evidence |
| FND-005 | low | The worked example calls the value `P(a within 2 ms) = 149/192`, but AC-18's claim is `eventually[0 ms, 3 ms] holds(t.a)`. The value is the same, because A only fires at 1 or 2 ms, but the window does not match the claim. Fix: write `P(a within 3 ms)`. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:290 | wrong-requirement |
| FND-006 | low | One sentence does not parse: "the identities' weights are distinct operations here, so no operation's weight is shared among tied identities". Fix: "each racing identity is its own operation here, so no operation's weight is shared among tied identities". | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:301-303 | wrong-requirement |

## Verdict

Not mergeable as is. The arithmetic is correct throughout. The edge label, the
action set and the rejection rule (for an `Exact` certificate) are consistent
with FR-196, FR-201, FR-204 and ADR-026 SD-3. The certificate clause needs the
kind stated (FND-001), and TC-3204 needs the second certificate (FND-004).
FND-002 and FND-003 are real SD-3 coverage gaps. A single extra AC can close
both. The lead may defer them to a follow-up ticket instead, with the reason
recorded. FND-005 and FND-006 are wording.

## New findings (disposition pass 1)

Round 1 reviewed `git diff 505fbf2d..b84df6c5`, head b84df6c58ff0d9168560d21411f0d1456df6ec57. The new content is FR-204-AC-19 with TC-3205 (`Pair`), FR-204-AC-20 with TC-3206 (`Chain`), and the AC-18 and TC-3204 fixes. Every number was recomputed by hand, and every one is correct:

- **`Pair`, weighted.** With G = 2 ms, `fire` takes 4/5 split uniformly, so F1 2/5, F2 2/5 and G 1/5, each times 1/2. With G = 3 ms, F1 and F2 get 1/2 each, times 1/2. That gives F1 = 1/5 + 1/4 = 9/20, F2 = 9/20 and G = 1/10. The masses sum to 1, and G at 3 ms never wins.
- **`Pair`, undivided weight (4, 4, 1).** F1 = 2/9 + 1/4 = 17/36 and G = 1/18.
- **`Pair`, even ties.** F1 = 1/6 + 1/4 = 5/12.
- **`Pair` thresholds.** `>= 0.45` holds at 9/20 and 17/36 and fails at 5/12. `<= 0.45` holds at 9/20 and 5/12 and fails at 17/36. So the pair pins the value exactly.
- **`Chain`, orders.** XYZ 1/15, XZY 1/10, YXZ 1/12, YZX 1/4, ZXY 1/6, ZYX 1/3, summing to 60/60. `last` = X is 7/12, Y is 4/15 and Z is 3/20.
- **`Chain`, wrong checkers.** First pick weighted, rest canonical: last = X 0, Y 1/2, Z 1/2. Even: 1/3 each. Canonical: Z = 1.
- **`Chain` thresholds.** `>= 0.58` (7/12 ≈ 0.583) and `<= 0.2` (0.15) are each refuted by all three wrong checkers.

AC-19 and AC-20 are falsifiable. They are as bundled as AC-18, so SR-3594002's FND-001 applies to them too, and its accepted-no-change disposition covers them. TC-3205 and TC-3206 follow the TC convention. TC-3205 and TC-3206 are unique: no other remote branch carries them, and FR-204-AC-19 and AC-20 do not appear on origin/main. `quire validate` exits 0.

On AC-19's first-pick pin: every reading of SD-3 gives F1, F2 and G 2/5, 2/5 and 1/5 at the first pick, so AC-19 is sound. It is an acceptable stopping point for this PR. The open question, the weight a remaining identity carries after a pick, is an ambiguity in ADR-026 itself (FND-008), not a missing criterion.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-007 | medium | ADR-026 SD-3 takes every tied identity "in that order at the same instant, each only when it is still enabled at its turn". FR-204 and FR-196 state no product representation for such a multi-step tie. FR-204 line 139 says "each race outcome SHALL be one edge, delaying by the winning amount and taking the winning identity". FR-196 line 676 says "a given-delay race is one edge with its exact probability, delay and discrete endpoint". Read literally, `Chain` takes only the winner, and Y and Z redraw 2 ms at the new state, so `n = 3` is not reached by 3 ms and AC-20's values would be 0. AC-20 therefore states no edge list and leaves the `InequalityFails` locus out. AC-12 already relied on this ("`A`, still enabled, sets `b`"), so the gap predates this PR, and AC-20 agrees with the ADR. Recommendation: defer to a ticket, "FR-204/FR-196: product representation of a same-instant multi-identity tie (ADR-026 SD-3 ordered takes)". It should state: an outcome is a tie order; the edge delays once and applies the still-enabled identities in that order to one post-key; one trace position per fired identity at the same time stamp; the edge label; and the certificate locus. Then pin AC-20's edges and locus. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:139,255; spec/functional/FR-196-build-the-probabilistic-product.md:676 | missing-requirement |
| FND-008 | medium | ADR-026 SD-3 ("without replacement by ADR-024 PM-2's weights and then uniformly within an operation") is ambiguous for an operation with several identities beyond the first pick. After F1 of `fire` is taken, F2's weight against G could be the operation weight (4 vs 1), the divided weight the `Pair` worked example uses (2 vs 1), or nothing at all, because the operation's identities stay consecutive (operations ordered first, then identities uniformly inside each). All three readings agree on AC-19's first pick, so AC-19 is sound but cannot decide between them. The worked example's phrase "`F1` and `F2` carry 2 each" reads as a ruling on later picks too. Recommendation: defer to an ADR-026 SD-3 clarification ticket ("SD-3: order of the identities of one operation within a tie"), which needs an owner ruling. Then add an AC with a `Pair` variant whose identities stay enabled. | spec/decisions/ADR-026-dense-time.md:238; spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:318-331 | wrong-requirement |
| FND-009 | low | `Pair` declares `delay Pair::fire ~ discrete { 2 ms: 1 };` for the operation it calls `Node::fire`. FR-204 elsewhere names a class operation with its class, as in AC-17's `delay M::Call::send`. Fix: write `delay Pair::Node::fire ~ discrete { 2 ms: 1 };`. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:324 | wrong-requirement |
| FND-010 | low | FR-204-AC-20 says "`last = Y` has value `4/15`" but names no claim, threshold or item, so the value cannot be observed as a verdict. TC-3206 says only "decide … the `last = Y` value". Fix: state it as an item, for example `probability >= 0.26 [eventually[0 ms, 3 ms] holds(c.n = 3 and c.last = Y)]` settles `proved`, `ExactValue{4/15}`, or drop it. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:255 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | b84df6c5: AC-18 names the `t.b` item's `Exact` certificates (even ties `11/96`, canonical order `0`), each rejected `InequalityFails`, "and the test submits both". The worked example explains why a `Lower` document would meet `ThresholdMissed` instead. |
| FND-002 | fixed | b84df6c5: FR-204-AC-19 and TC-3205 (`Pair`, `fire` 4 shared by F1 and F2, G 1) pin the within-operation division at the first pick (9/20, 9/20, 1/10). The undivided and even checkers are rejected. Later picks are recorded as FND-008. |
| FND-003 | fixed | b84df6c5: FR-204-AC-20 and TC-3206 (`Chain`, three still-enabled identities) make the order after the first pick observable: 7/12, 4/15 and 3/20, with all three wrong checkers refuted. |
| FND-004 | fixed | b84df6c5: TC-3204's procedure submits both `Exact` certificates (`11/96` and `0`), and Expected Results rejects both. |
| FND-005 | fixed | b84df6c5: the worked example gives `P(a) = 149/192` over `[0 ms, 3 ms]` and notes that `[0 ms, 2 ms]` gives the same. |
| FND-006 | fixed | b84df6c5: the sentence now reads "`A`, `B` and `C` are three distinct operations of one identity each, so no operation's weight is shared among tied identities here". |
