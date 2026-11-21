---
id: SR-60054
title: "code-review (Rust lane) of filament-spec-language PR 3267, QSL-2600 model-unit words stay contextual"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3267 head b51b397210 base b1df987a24; fsl-cst/src/grammar.rs, fsl-cst/tests/it/model_unit.rs"
review_set: subset
---

## Summary

Ticket: QSL-2600. Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD` (2 files, +72/-25). Read only: no build, no test run. Parser semantics were checked against fsl-cst/src/parser.rs: `Choice` is ordered and always tries the next alternative on failure; `Repeat` with `commit_on_progress` fails the whole repeat when a failed iteration got past its start; diagnostics use a whole-call farthest failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-710 Grammar still says every model-unit grammar word is reserved except `population`, `interface` and `part`. After this PR 23 more words are contextual. The arch ruling is not written into FR-710, and no AC states that these words are identifiers outside their declaration. | spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:71 |
| FND-002 | medium | No test uses these words as identifiers inside a model unit, which is what the `backtracking_star` change and the ordered body choices exist for. Untested examples: a field named `occurs` or `states` in an event or machine body (`event E { occurs: Timestamp; occurs occurs; }`), a field `step` next to a step declaration in a process, declaration names such as `object store` or `machine process`, and port and member names `out`/`inout`. The new test covers only complete-V1 record fields, parameters and one expression. | fsl-cst/tests/it/model_unit.rs:135 |
| FND-003 | low | The new test is tagged `FR-710-AC-3`, but AC-3 is "one declaration of every form compiles to an admitted document". The test checks none of that, so it inflates AC-3's binders. Retag it to the AC that FND-001 adds. | fsl-cst/tests/it/model_unit.rs:133 |
| FND-004 | low | Editor completion builds its keyword candidates from `base_reserved_spellings()`, which collects only `Exact` and `Literal` terminals. The model-unit keywords `machine`, `process`, `store`, `namespace`, `port`, `connection`, `allocate`, `consumes`, `emits`, `inverse`, `subsets`, `members`, `term`, `persists`, `occurs`, `states`, `open`, `out` and `inout` are no longer offered. | src/complete/editor.rs:151 |

## Scope and evidence

- fsl-cst/src/grammar.rs: examined, all 16 hunks. Each switched word was checked for ambiguity where its declaration is also valid:
  - Model-unit top level: every `domain-decl` alternative starts with a keyword, and none starts with an identifier. There is no ambiguity.
  - Object, process, store and interface bodies: `DomainField` needs `ident ':'` right away, and every member declaration needs `keyword ident`. A field named `operation` or `step` parses as a field. `operation f()` and `step s: T;` fail `DomainField` at the second token, and the ordered choice then tries the next alternative. The choice returns a match, so the committing star never sees a failure.
  - Event and machine bodies: the terminators `occurs x;` and `states (` are not members. With a plain `star`, `DomainField` now consumes `occurs`/`states` as an identifier and fails one token later, which commits the repeat and refuses. `backtracking_star` is therefore required. The existing EVERY_FORM test (AC-3) already exercises it, because `occurs timestamp;` and `states (Open, Shut);` would refuse under `star`.
  - Error reporting: diagnostics come from the whole-call `farthest_failure`. A malformed member in an event or machine body is still reported at the same deepest token. The alternatives tried after backtracking sit at earlier positions, so they do not change the reported position or its expected set. No test asserts an old error for these words: `syntax_messages.rs` checks `first`/`bounded`, `state_graph_claim.rs` checks `eventually`/`fair`, and `every_declarative_reserved_word_is_refused_as_an_identifier` reads the computed set.
- #3202 attribution, checked with `git show 8f71444d8a^:fsl-cst/src/grammar.rs`: none of the 23 words, including `inout`, `open` and `out`, had an `x(...)` terminal before #3202, and each had exactly one or two on main. The "nine already reserved" claim is correct for domain, extends, identity, redefines, relationship, ordered, unique, event and transition, but incomplete: `symmetric` was also reserved before #3202 and stays reserved.
- complete_grammar: the PR adds no productions, so `ACCEPTED_PRODUCTIONS` is unchanged, and the `model_unit` corpus entry is unaffected. fsl-cst has no changes on main since the base.
- monitor_root I2WorkUnits failures: the CST tree holds only production matches, and a terminal of kind `Identifier` or `ContextualWord` lowers to the same token. A complete-V1 fixture therefore parses to the same tree as it did before #3202. The overrun is reported by the checked-package writer (`Write(OutputLimit(.. I2WorkUnits ..))` at `/checked_term_data/body_locations/..`), not by S1. It cannot come from this diff. It is pre-existing and was hidden behind the #3202 parse refusal; commits after #3202 that touch the emitter include #3226, #3227 and #3238. `actual: 1000001` against a bound of 1000000 is the meter reporting the first unit over the bound, not an off-by-one. This needs its own owner and is not a finding against this PR.
- rust-review lane: declarative grammar edits only, with no unsafe, no panics, no new allocation paths and no API change. The test uses plain iterator code and has a real oracle: before this PR every word was refused with "is a reserved word", and both admissibility and the byte round-trip are asserted. No finding.

## Verdict

The grammar change is sound: no switched word creates an ambiguity in any position where its declaration is valid, and `backtracking_star` is the right fix for the event and machine terminators. Two medium findings must be fixed in this PR. FND-001 is the FR-710 spec text and an AC. FND-002 is model-unit-internal identifier coverage. FND-003 and FND-004 are low.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new Grammar text says the ten listed words are reserved and "every other model-unit word" is contextual. Read literally, that covers `record`, `map`, `invariant`, `pre`, `post`, `in`, `on`, `when`, `from` and `to`. These appear in the model-unit EBNF, stay `x(...)` reserved terminals, and are in neither list. Only the parenthetical enumeration limits the claim. | spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:73 |

## Dispositions

Disposition pass 1, checked at frozen head 6f7bd3b1c8 by reading the code. No build or test was run.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-710 Grammar now names the ten reserved words and lists all 26 contextual words, which match exactly the 26 `contextual(...)` terminals in fsl-cst/src/grammar.rs:438-746. FR-710-AC-10 states the identifier behaviour. The residual literal-reading gap is FND-005. |
| FND-002 | still-open | The new test `model_unit_words_are_identifiers_inside_model_unit_bodies` (fsl-cst/tests/it/model_unit.rs:188) writes `step first: begin;`. `first` is an `x("first")` terminal of complete-V1 (grammar.rs:2492, Compensation) and a vocabulary keyword (token.rs:174), so `complete_reserved_words` reserves it. The model unit is parsed inside `CompleteUnit` with that same reserved set (grammar.rs:383, parser.rs:2272), and `Terminal::Identifier` refuses reserved spellings (parser.rs:2711-2714). syntax_messages.rs:100 already asserts that `first` is refused as an identifier. The process body therefore refuses and the test's `is_admissible` assertion fails, so the in-body coverage is not delivered. Fix: rename the step, for example `step go: begin;`. |
| FND-003 | fixed | The test is retagged `#[trace("FR-710-AC-10", "TC-1942")]`. |
| FND-004 | fixed | `contextual_spellings()` collects every `ContextualWord` terminal of complete-V1, and `analyze_document` adds the result as `GrammarToken` candidates. tests/it/complete_editor.rs asserts that `machine`, `step`, `population` and `occurs` are offered. |
| FND-002 | fixed | Disposition pass 2, checked at frozen head 54517da8bd by reading the code; no build or test was run. The body test now writes `process Flow {{ step: Integer; step go: begin; }}` (fsl-cst/tests/it/model_unit.rs:191). Neither `go` nor `begin` is an `x(...)` or `literal(...)` terminal of complete-V1 or a token.rs keyword, so `complete_reserved_words` does not reserve them and the step name parses as an identifier. Execution of the test is pending the lead's draco focused run. |
| FND-005 | fixed | Disposition pass 2, at 54517da8bd. FR-710 Grammar now reads: "The contextual model-unit words are `abstract`, `allocate`, ... `subsets` and `term`. Each is a keyword only where its declaration or clause begins, and an ordinary identifier everywhere else, including inside model-unit bodies." The "every other model-unit word" claim is gone, so the reserved words outside both lists are no longer described as contextual. |
