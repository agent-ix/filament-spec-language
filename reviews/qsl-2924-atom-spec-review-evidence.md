---
id: SR-967175026302197768900474181941935084776
title: "evidence review of PR3549 unwindowed atom as eventually[0,0]"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md; PR3549 5aa557c44af9fbc00c910c661fec8db9b44119c9; ticket QSL-2924"
review_set: subset
---

## Summary

Checked the verification method of FR-201-AC-57 and FR-202-AC-40. Test is the right method (both are executable fixtures with exact expected values). No equality case is needed: one initial state and at most one admitted binding give an atom value 0 or 1 and source thresholds 0 and 1 are refused, so no threshold can equal the value; several `over` instances are outside the bounded subset for both spellings alike. Two traceability/seam nits.

Ticket: QSL-2924

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New ACs carry bare 'Test' with no TC binding; siblings bind TC-636/TC-637 | FR-201:1371, FR-202:613 |
| FND-002 | low | AC-57 mixes the fsl-eval `supports` unit assertion into an end-to-end checker AC | FR-201:1371 |

## Finding detail

- FND-001 (FR-201-AC-57): FR-201-AC-57 and FR-202-AC-40 are verified by bare 'Test' with no TC binding, while the sibling bounded ACs they mirror are bound to TC-636 (FR-201-AC-56) and TC-637 (FR-202-AC-14 tests carry #[trace("TC-637", ...)]). Bind them to TC-636/TC-637 (or a new TC) so the matrix records which procedure owns the atom cases.
- FND-002 (FR-201-AC-57): FR-201-AC-57's last sentence asserts a unit-level fsl-eval API (`QualifiedBoundedMonitor::supports` per profile) inside an end-to-end checker/settlement AC; that check belongs with the fsl-eval translation statement and is a different test seam (fsl-eval unit test vs fsl-probability-check process). Consider a separate AC so a failure localises.

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

Mergeable; low findings optional.

## New findings (disposition pass 1)

Re-checked at head 0ac6d707d3bb705df8cae0e6510008a4b922174e (rebased onto origin/main 9a2555a093).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | AC-75/TC-3195 call private BoundedTranslation::in_profile; name public BoundedTranslation::clause | FR-201:1386 |

- FND-003 (FR-201-AC-75): FR-201-AC-75 and TC-3195 step 5 tell the test to call `BoundedTranslation::in_profile` under the infinite-trace profile, but `in_profile` is a private fn (fsl-eval/src/automaton/bounded.rs:154, `fn in_profile`); a test cannot call it. Its infinite-trace admission is reached through the public `BoundedTranslation::clause` with an infinite-trace origin clause; name that entry.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified at 0ac6d707d3: Over the engines `Coin` model (initially tails, as in its `data/coin-tails.json`; `flip` sets heads or tails with equal weight), with the `ev` profile under eve |
| FND-002 | fixed | verified at 0ac6d707d3: For a checked root `holds(c.heads)` with no temporal operator, `QualifiedBoundedMonitor::supports` and `QualifiedBoundedMonitor::supports_observations` both ret |
