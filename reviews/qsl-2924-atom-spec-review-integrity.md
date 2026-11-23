---
id: SR-967175026302197768900474181941935084774
title: "integrity review of PR3549 unwindowed atom as eventually[0,0]"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md; PR3549 5aa557c44af9fbc00c910c661fec8db9b44119c9; ticket QSL-2924"
review_set: subset
---

## Summary

Checked consistency of the FR-201 profile bullet, fsl-eval bullet, FR-202 pointer and the two new ACs, and hand-verified their numbers. The equivalence is normative once in the FR-201 profile bullet with the fsl-eval bullet as its implementation obligation (per the ruling); FR-202 points to FR-201 with no duplicate normative text; no profile, objective member or document field is added; the 'unsupported forms remain unsupported' clause (FR-201:160-162) is consistent because the atom is now inside the bounded subset rather than encoded into it. Numbers check: tails max 0 <= 1/2 proved, empty targets, rank 0; heads min 1 >= 1/2 proved; 1/2 at a decided key fails the target-1/terminal-0 Exact condition -> InequalityFails; R = 1 > 1 - 1/2, A = 1 > 1/2, opposite-verdict parity per FR-202. FSpec FR-407:94-121 and FR-411:73-76 agree; under the infinite-trace profile a root atom is not a reachability form and stays refused `unbounded-event` (FR-407:110-117). AC ids FR-201-AC-57 and FR-202-AC-40 do not collide with open PRs (#3546 adds FR-202-AC-36..39, 41, 42 and skips 40). Ticket: QSL-2924.

Ticket: QSL-2924

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-202-AC-40 'empty accepting path over tails fails replay agreement' has no defined input or distinct outcome | FR-202:613 |
| FND-002 | low | Heads-initial fixture need not stated; only coin-tails data exists | FR-201:1371 |
| FND-003 | low | 'decided at its activating position by FSpec FR-407' overstates FR-407; cite FR-407 + FR-091 | FR-201:182-190 |

## Finding detail

- FND-001 (FR-202-AC-40): 'an empty accepting path offered over the tails initial state fails replay agreement' has no defined input: `ProbabilityWitnessPath` carries only `steps`, and replay derives each path's verdict (FR-202 'the closed decision is derived by replay'). An empty path over tails is the same data as the rejecting path already covered, and under `<=` it yields parity, not a distinct agreement failure. Name the actual input (e.g. a witness whose `initial_key` is the accepting decided key over the tails state: replay refuses, no token) and the claim it is offered for, or drop the clause.
- FND-002 (FR-201-AC-57): The heads-initial variant is used for proofs (FR-201-AC-57) and an accepting-path refutation (FR-202-AC-40), but filament-examples engines has only data/coin-tails.json and neither AC says the test must author a heads-initial fixture (as FR-202-AC-14's test sets the initial predicate true). State the fixture need, e.g. 'a fixture of the Coin model with its one coin initially heads'.
- FND-003 (FR-201): 'decided at its activating position by FSpec FR-407' cites more than FR-407 says: FR-407 'Events' defines a bounded event's truth over positions from its activating position up to its horizon h but never states that an operator-free atom has h = 0. The reading follows from FR-407 with the FR-091 position semantics (and agrees with FR-411's 'decided within h + 1 positions'); cite that chain rather than attributing the conclusion to FR-407 alone.

## Scope and evidence

No applicable enforced AssuranceProfile; direct subset review (integrity, ears, evidence, failure-domain) per the dispatching brief. Spec-only PR; no cargo run. Code grounding read at origin/main in the PR worktree: fsl-eval/src/automaton/qualified_bounded.rs, bounded.rs, counters.rs, translate.rs; fsl-analyze/src/analyze/exact.rs; fsl-replay witness/bounded.rs. FSpec read at filament-specification origin/main.

- FR-201 (examined): even when the underlying model graph is cyclic. An origin-activated event-position claim whose event is a bare `holds(P)` with no temporal operator (an unwindowed atom claim, decided at its activating position by FSpec FR-407) SHALL be read as `eventually[0,0] holds(P)` and admitted to this subset under that reading. Its admission, objective, targets, optimization, values, ranks, rejection rules and verdict SHALL be those of the explicit `eventually[0,0] holds(P)` spelling of the same claim; only the claim identity and the checked formula context carried in its keys differ. No profile, obje...
- FR-201 (examined): than copy EN-5's monitor. It SHALL translate a root `holds(P)` with no temporal operator under the event-position false-extension profile as `eventually[0,0] holds(P)`: `QualifiedBoundedMonitor::supports` and its bounded-shape classification SHALL admit it as the eventually kind at horizon 0, and initialization, step, canonical monitor-state key and terminal false-extension SHALL behave as for that explicit spelling. The checked formula SHALL NOT be rewritten, and no surrogate formula AST is introduced. Under any other profile the translation SHALL continue not to admit the atom.
- FR-201-AC-57 (examined): Over the engines `Coin` model (initially tails; `flip` sets heads or tails with equal weight), with the `ev` profile under every scheduler on origin, `probability <= 0.5 [ holds(c.heads) ]` is admitted as `eventually[0,0] holds(c.heads)`: position 0 decides rejection, the maximum is `ExactValue{0}`, `targets` is empty, the initial key has value 0 and rank 0, `fsl-probability-check` accepts the `reachability/v1` `Exact` document with side `AtMost`, and the item settles `proved`. Over the variant whose initial state is heads, `probability >= 0.5 [ holds(c.heads) ]` decides acceptance at posit...
- FR-202 (examined): The same native entry SHALL additionally admit origin-activated untimed `eventually[0,h] holds(P)` and `always[0,h] holds(P)` (FR-201's bounded subset, including its reading of an unwindowed atom claim `holds(P)` as `eventually[0,0] holds(P)`, which replay applies unchanged), with finite nonnegative event-position horizon h, one initial state and at most one admitted binding, for minimum `AtLeast`,
- FR-202-AC-40 (examined): Over the engines `Coin` model (initially tails), with the `ev` profile under every scheduler on origin, `probability >= 0.5 [ holds(c.heads) ]` (value 0) is refuted through the bounded checked-subject entry by one empty rejecting path of mass 1 and an empty policy: `R = 1 > 1 - 1/2`, and replay reproduces `refuted` with its token. The same empty rejecting path offered for `probability <= 0.5 [ holds(c.heads) ]` is an opposite-verdict path and yields replay parity without a token, and an empty accepting path offered over the tails initial state fails replay agreement. Over the variant whose ...
- FR-201 (context_only): FR-412's open, nonempty `objective` object. This profile does not narrow the other objective obligations above and below; unsupported forms SHALL remain explicitly unsupported rather than be encoded as this profile.
- FR-202-AC-14 (context_only): At horizon 0, initially true P gives an empty accepting path of mass 1 refuting AtMost 1/2, while initially false P gives an empty rejecting path refuting AtLeast 1/2; opposite-verdict empty paths yield parity. At horizon 2, a terminal initially false P closes by false extension with mass 1 and no fabricated stutter step, refuting AtLeast 1/2. A live undecided prefix after one failure does not refute; after two failures horizon rejection does, even though another model action is enabled. An initial true P path extended by an operation cannot reproduce.

## Verdict

Mergeable once the medium findings are fixed.

## New findings (disposition pass 1)

Re-checked at head 0ac6d707d3bb705df8cae0e6510008a4b922174e (rebased onto origin/main 9a2555a093).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | AC-40 'same counterexample offered for <=' yields parity is false: keys/item carry the >= claim, so replay refuses | FR-202:676 |

- FND-004 (FR-202-AC-40): FR-202-AC-40 (and TC-3196 step 2) now says 'the same counterexample offered for `probability <= 0.5 [ holds(c.heads) ]` ... yields replay parity'. A `ProbabilisticCounterexample` carries the `>=` claim's item identity and complete keys, whose formula-monitor context includes the claim occurrence (FR-201: only the claim identity and checked formula context differ; FR-202-AC-13: rebinding to another claim occurrence prevents reuse). Offered unchanged, replay refuses it (`root_key != witness.initial_key`, fsl-replay witness/bounded.rs), so the stated parity outcome cannot occur. FR-202-AC-14's own test rebuilds item and keys for the opposite claim (`rekey`, fsl-analyze/tests/it/bounded_probabilistic_replay.rs:298-300) before getting parity. Say 'the same empty rejecting path, in a counterexample whose item and keys are rebuilt for the `<=` claim'.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified at 0ac6d707d3: A counterexample over the tails subject whose `initial` is the heads state and whose `initial_key` is its accepting decided key is refused, with no token. |
| FND-002 | fixed | verified at 0ac6d707d3: Over a fixture the test authors, the same `Coin` model with its one coin initially heads (the examples supply no such fixture) |
| FND-003 | fixed | verified at 0ac6d707d3: The reading rests on FSpec FR-407's bounded event, whose truth depends on positions from its activating position up to its horizon, and on FSpec FR-091's positi |
