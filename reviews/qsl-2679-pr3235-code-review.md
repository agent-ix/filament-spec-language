---
id: SR-20261121010922497829000
title: "Code review — QSL-2679 batch PR #3235 parser and formatter backlog"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3235 diff (batch QSL-2679, 2680, 2681, 2682, 2683, 2684, 2685, 2687, 2688, 2621, 2599): fsl-cst/src/{format,grammar,parser}.rs, fsl-cst/tests/it/*, fsl-forms/tests/it/value_forms.rs, fsl-replay/tests/dispatch_link_compile.rs, fsl-semantics/src/check/hyper_clause.rs, tests/corpus/refinement/remaining/*, tests/corpus/values/fn-param-named-set-2006.expected.json, tests/it/*"
review_set: subset
evaluated_revision: "c742cd667549301b142b4dd645a64f6f2c04ed2a"
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Ticket: QSL-2679. This review covers the whole eleven-ticket batch in PR #3235 at head `c742cd66`. It was read from the diff, with context from `git show` of the head and of FSpec `origin/main`. Nothing was built or run. The diff has no Python and no logic-bearing shell.

## Verdict

**FAIL — one fix round needed.** Two mediums and one low. No high: no valid input now parses differently, no input is newly admitted against the spec, and no test was bent.

- **QSL-2683 (collection words).** These flips follow the spec, so they are not bent tests. FSpec `shared-grammar.md:81-84` says "a spelling that neither the common inventory above nor the complete-V1 inventory below lists is an ordinary `ident` in every `1-draft` position". Lowercase `set`/`bag`/`orderedSet` are in neither list (lines 64-78 and 94-108). Only `Set`/`Bag`/`OrderedSet` are reserved. `collection-value` (line 471-474) uses them only as quoted terminals before `[`. QSL-2006's audit leaves "reserved or contextual" open, and QSL-2683 states the contextual reading. So the three flipped tests and `fn-param-named-set-2006` (now S4 success) are correct. On ambiguity: `set(...)` is a call, because `CollectionValue` needs `[`. `set[e]` with a binder named `set` parses as the constructor, because `Primary` tries `CollectionValue` before a name with a postfix index. The spec does not settle that, and no test pins it (FND-003).
- **QSL-2685 (edition/language).** For a string literal it matches FR-500-AC-1: `unknown_edition`/`unsupported-selection`, located at the literal, sole diagnostic. A valid header is not affected, because `selection_code` fires only when the parse fails at token 1 or 3. However, the preflight also fires on a non-string token in that slot (FND-001).
- **QSL-2687.** A lexical error elsewhere in the unit can now hide a real selection-identity refusal from the prelude. A parse error already did that before this PR ("the parser's refusal already locates the failure"). The unit stays refused, and FR-762 only governs syntax errors. Not a defect.
- **QSL-2680 / QSL-2684.** Recovered `Declaration` nodes are inserted only when there are recoveries. The new walk-back and name-slot rules sit only on the failure path. A valid unit has neither, so it cannot gain a skipped region. The new test shows the next declaration still parses.
- **QSL-2682.** `failure_cause` is used on both diagnostic paths that compute a cause. The missing-`;` insertion path keeps `unexpected-token`, and that is right for that error.
- **QSL-2688.** Both cut messages now match FR-762 "The diagnostic bound" word for word. The old assertion that the message contains no `s1.diagnostics` contradicted FR-762-AC-6, so changing it was a correction.
- **QSL-2621.** The span is now the whole `Text` token, for both the space and the comment variants.
- **QSL-2599.** The ruling is confirmed: QSL-2599 arch ruling 4 says "An empty align-skip list parses; FR-172 refuses it at S3", and FSpec `shared-grammar.md:813-814` now has the optional list. FR-172 "Align skip" and AC-9 require `unsupported_construct`/`expression-form`, S3, exit 21, located at `align skip {}`. The new source-route test asserts exactly that. The request path change matches the move out of `pending/`.
- **QSL-2679 (formatter).** A population bound's `[` keeps its space, and the test asserts that formatting twice gives the same output.

## Scope and skill provenance

The code-review lane and the rust-review lane ran by reading the diff. `quoin gap-analysis` was not run, because the brief allows no worktree and no build. A manual check of the changed tests against their ACs was done instead: FR-500-AC-1, FR-762-AC-2/3/4/6/9, FR-172-AC-9, FSpec FR-339-AC-4/5 and FR-003-AC-3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The selection preflight fires for any token in the language or edition slot. `edition ;`, `edition 1;` and `language ;` used to give `invalid_syntax` through header recovery. They now give `unknown_edition`/`unknown_language`. FR-500 covers only an edition *string*. | fsl-cst/src/parser.rs:519-520 |
| FND-002 | medium | QSL-2681 is left ignored. Its catalog vector belongs to FSL and is not FSpec text. Renaming the reserved field `value` to an ordinary name keeps the `(code, cause)` oracle and lets the test run. The ignore reason also names a test that does not exist (`invalid_dollar_token_is_a_lexical_refusal`). | fsl-cst/tests/it/complete_grammar.rs:832,854 |
| FND-003 | low | With `set` now a legal binder, `set[e]` parses as a set constructor rather than as an index of the binder. Nothing pins that precedence. | fsl-cst/src/grammar.rs:1552 |

## Findings detail

- **FND-001.** **Fix:** gate `selection_preflight` on `tokens.get(failure.position)` being a `Kind::Text` token. Then a missing or non-string edition or language still goes through header recovery as `invalid_syntax`. Add one vector (`edition 1;`) to the parser_backlog test.
- **FND-002.** **Fix:** in this PR, change the invalid-token vector to `record Broken { datum: Integer; } $`, remove the `#[ignore]`, and close QSL-2681 on that.
- **FND-003.** **Fix:** add one cst/forms assertion that `function f using v(set: Sequence<Integer>): Set<Integer> pure { set[1] }` yields a `CollectionValue`. That documents the precedence.
