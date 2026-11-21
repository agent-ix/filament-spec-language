---
id: SR-2026112022042101
title: "code-review of PR 3223 zero-argument qualified calls and mode-switch declarations (QSL-2620, QSL-2626)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3223 head 86516c48c7e82292c90030105ed05e9adc1da8e8; fsl-cst/src/grammar.rs, fsl-cst/src/cst.rs, fsl-forms/src/value.rs, fsl-forms/src/task_set.rs, fsl-forms/src/dispatch.rs, fsl-semantics/src/check/task_set/mode_switch.rs, fsl-semantics/src/check/refusal.rs, fsl-semantics/src/check/mod.rs, fsl-replay/src/spine.rs and the changed tests"
review_set: subset
---

## Summary

Ticket: QSL-2620 (QSL-2626 rides the same PR). PR filament-spec-language#3223, +~1000 lines in 23 files. This is a diff-only code review with the rust-review lane folded in. The checks against FSpec FR-339-AC-1/AC-4 and FSL FR-249-AC-13/AC-16 were done by hand in this pass. No build or test run: the review read the diff and the head blobs only.

The review looked hardest at whether any existing input now parses differently. The parser is an ordered-choice (PEG) interpreter: a `Choice` frame tries its alternatives in order, truncates the failed attempt and takes the first match (fsl-cst/src/parser.rs `Frame::Choice`). So moving `EnumValue` and inserting `QualifiedName '(' ')'` can only change a parse where an alternative between the old and new `EnumValue` position now matches first.

## Verdict

MERGEABLE after the one low below, which the ruling assigns to this PR's fix round. No high or medium findings.

What was checked and holds:

- Alternatives between `TupleValue` and the new position (`present|value|deref|ref|pre (`, `convert|allInstances <`, `( expr )`, `PopulationCall` behind its `Peek` guard, `CollectionCall`, `lookup <`, `size`, `contains|removeOne (`, `Text (`, `reaches (`, `ident args`) all need `(` or `<` right after their first token, or a reserved keyword. None of them can match a prefix `A :: b`, so every input `EnumValue` matched before still reaches `EnumValue` and matches the same span.
- The new alternative matches only `QualifiedName ( )`. A nonempty argument list fails at the first argument, and `TupleValue` (tried earlier) owns it, so `Q::f(3, a.b)` stays a `TupleValue`. An unqualified `f()` is taken by the earlier `ident args`. A bare `Q::f` fails at `(` and falls through to `EnumValue`.
- Whenever the new alternative consumes past its start and then fails, a later alternative (`EnumValue` or `QualifiedName`) still matches at the same position, so the `Primary` outcome is unchanged. No grammar sequence puts `(` directly after an expression (the only `Expression`/`(` adjacencies are `when (` and `holds (`, with `(` first), so no input where `Q::f` was followed by a legitimate `()` can be re-read as a call.
- `ScrutineePrimary` and `HyperPrimary` are copies of `Primary` and receive the same alternative in the same place. `HyperPrimary` adds `StepLabel` and `stutter` at the front, unaffected.
- S2 lowering: `Primary` whose first item is a `QualifiedName` node with a `(` token is only the new alternative (`TupleValue`, `RecordValue`, `EnumValue` and `ReferenceLiteral` arrive as their own child nodes; `protocol Q` starts with a token). It lowers to `Shape::Call(name)` with no operands, the same shape `TupleValue` lowers to.
- Reserved words: `complete_reserved_words` collects only `Exact` identifier terminals. `mode` and `on` were already `Exact` on main (2 and 20 uses), and `hi` is a `Literal`, so the PR adds no globally reserved word. `hi` was already a `Literal` in `task`. The tests show a field named `hi` still parses.
- `ModeSwitchDeclaration` sits at declaration level next to `ReleaseDeclaration`. Hybrid `InitialEntry`/`HybridMode` are reached only inside a `hybrid` body, and the test parses both kinds in one unit.
- S3 `check_mode_switches` resolves the task set the way `check_releases` does (first `Taskset` declaration of the name), resolves the operation through the same `ProbabilisticInput::operation(.., seen_by_task_set(position))`, and keys duplicates on `(task_set, operation key)`. Codes and causes match FR-249-AC-13: `missing_declaration`/`missing-name` naming `Nope` or `C::nope` at their spans, `ambiguous_declaration`/`ambiguous-name`, and `conflicting_declaration`/`duplicate-declaration` at the second declaration's span. Each cause has a span, code, tag and spine message.
- S1 refusals: the six malformed declarations of FR-249-AC-16 are refused `invalid_syntax`/`unexpected-token` at `lo`, `on`, `on`, `Plant`, `;` and `.` with the right widths. The test helper `refused` asserts both code and cause.
- Formatter: no formatter code changed. The tests show `Store::construct ( )` prints as `Store::construct()`, the split `mode` declaration prints on one line, a second format is unchanged, and the stripped token stream is preserved. `formatter_identity` compiles original and formatted sources to equal packages.
- The PR adds no Python and no logic-bearing shell.

Out of scope, as accepted: FR-249-AC-14/AC-15 and the release-backed marker of FR-249-AC-13 belong to QSL-2240. The AC-13 tests use the `ConfigVersion`/`Ctl` fixture rather than FSpec FR-419-AC-14's `C`/`MC`. Each AC-13 clause under this PR is still asserted, so this is not raised.

Not verified without a build: that the head tests pass, and the PR's claim that its 5+1 red tests (`complete_grammar` inventory, `interface`/`population`/`part` reservations) fail on main too. No control run was made.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-249-AC-16 says `mode MC hi on C::go_hi` with no `;` fails with `invalid_syntax`/`unexpected-token` at the end of input. The ruling is that the parser reports `unexpected-end` there, as it does everywhere. The test asserts only the code and span for this case, not the cause. | spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md:271, fsl-cst/tests/it/production_value_forms.rs:1277-1293 |

Fix:

- FND-001: change the AC-16 sentence so the missing-`;` case reads `invalid_syntax`/`unexpected-end` at the end of input, and add `CompleteCause::UnexpectedEnd` to that test's assertion.
