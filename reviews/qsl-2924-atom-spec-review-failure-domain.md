---
id: SR-967175026302197768900474181941935084773
title: "failure-domain review of PR3549 unwindowed atom as eventually[0,0]"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md; PR3549 5aa557c44af9fbc00c910c661fec8db9b44119c9; ticket QSL-2924"
review_set: subset
---

## Summary

Checked the new FR-201 reading and fsl-eval translation obligation against fsl-eval/src/automaton/qualified_bounded.rs and bounded.rs at origin/main. The admission is implementable without rewriting the formula: under Counting::Every a deadline counter with slack 0 expires at position 0 on a false letter (counters.rs DeadlineCounter::step), so both spellings decide at position 0, `state()` never needs `bounded_deadline` for the atom, and keys differ only in the checked-formula context. Two scoping gaps remain.

Ticket: QSL-2924

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Only `supports` named; EN-5 gates on `supports_observations` first, so AC-57's proved is unreachable as written | FR-201:294-302 |
| FND-002 | medium | Shared bounded-shape classification widening is unscoped: leaks atoms into bounded_shape/closure_shape/infinite-trace keying | FR-201:294-302 |

## Finding detail

- FND-001 (FR-201): FR-201's fsl-eval bullet names only `QualifiedBoundedMonitor::supports`, but EN-5's admission (fsl-analyze/src/analyze/exact.rs:1049) first gates on `supports_observations` and returns not-yet-implemented when it is false. An implementer widening only `supports`, as written, leaves the atom unsupported and FR-201-AC-57's `proved` settlement unreachable; FR-202's undefined-observation entry also uses `supports_observations`. Name both predicates (or state the admission at the shared classification with both consumers).
- FND-002 (FR-201): 'its bounded-shape classification' is fsl-eval's shared `bounded_formula_shape`, which also feeds `bounded_shape` (smt/query.rs:283 BoundedMltl item admission), `closure_shape` (FR-338 EN-1 closure keying under `on each` and infinite-trace origin) and `BoundedTranslation::in_profile` (admits the infinite-trace profile). Widening it to a root `holds(P)` silently admits atoms there too; 'under any other profile the translation SHALL continue not to admit the atom' constrains only the qualified translation. State that the reading is confined to the qualified probability path (origin, event-position false-extension) and that FR-338 closure keying, `bounded_shape` and `closure_shape` are unchanged, or specify their new behaviour.

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
