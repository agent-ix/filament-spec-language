---
id: SR-967180
title: "Code review — QSL-3301 keep only an ordinary identifier"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs, fsl-cst/tests/it/reserved_declaration_names.rs; PR #3914 at 13127f026a9639c2b406a8337c41685394ef4dfb; QSL-3301"
review_set: subset
---

## Summary

Ticket: QSL-3301. Code review (with the rust-review lane) of PR #3914, head `13127f026a9639c2b406a8337c41685394ef4dfb`, scope `git diff origin/main...HEAD`. The PR changes the head of the collection form `only(x in c: e)` in `CollectionCall` from `x("only")` (an `Exact` terminal) to `literal("only")`, and adds one test that parses `only` and `seq` in every native name position, a protocol step binder, and a call and name reference in a function body.

## Verdict

**Mergeable at this head.** `complete_reserved_words` collects only `Exact` terminals with an identifier spelling, so the change takes `only` out of the reserved inventory. A diff of identifier-spelled `x(...)` terminals between origin/main and the head shows `only` as the only change. The lexer table (`fsl-cst/src/token.rs`) never had `only`, so its token kind is `Identifier` before and after. That keeps the fsl-forms `collection_call` binder offset (`usize::from(query == Only)`) and the formatter call-parenthesis rule correct. `Literal` matches by spelling exactly as `Exact` does, so `only(x in (c): e)` (CollectionCall) and `only(x in P where c: e)` (PopulationCall, already a literal) parse as before. Expression has no `in` operator, so no identifier call `only(...)` can be shadowed by the form. Malformed forms keep their farthest failure past the head, so diagnostics and recovery stay the same. The one intended change is that the "`only` is a reserved word" refusal is gone. The PR matches FSpec FR-376-AC-8 and the shared grammar ("`only` and `seq` are matched as keyword spellings only in these productions and remain identifiers elsewhere").

Sibling form words: `seq` and the population `only` were already literals. `count`, `sum`, `exists` and `all` are reserved by the spec. `map`, `filter` and `forall` are reserved, and `collect`, `flatMap`, `flatten`, `dropFirst`, `fold` and `reduce` are in the complete-V1 inventory. No other spec-listed contextual word (memory-order words, `undefined`/`empty`/`refused`, `population`/`operation`/`receiver`, `observe`, `recommend`, `where`, guarantee/composition words) is an `Exact` terminal, except `on`, which the common inventory reserves.

Not run: the memory gate (MemAvailable of at least 30 GB inside the lead-prob cargo lock) refused twice, at 27.9 GB and 19.2 GB, so no cargo test ran. The review is from source. The coder reported the five ticket tests 5 passed / 0 failed at this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new test is tagged FSpec-FR-376-AC-8, but AC-8 states only that `only` and `seq` are ordinary identifiers "in a function body". Most of the test's positions (record, enum, union, tuple and type-alias names, profile and import aliases, and the protocol send binder that was the actual regression) are governed by the shared grammar's "remain identifiers elsewhere" sentence, which no AC carries. The binding is only partly correct. | fsl-cst/tests/it/reserved_declaration_names.rs:130-161 |

## Coverage

- Examined: grammar.rs CollectionCall, PopulationCall, Primary ordering, `complete_reserved_words`, `Terminal::Literal` matching (parser.rs), the lexer reservation table (token.rs), fsl-forms `collection_call`, format.rs call-parenthesis, and the new test.
- Existing tests that cover the form after the change (not run): fsl-forms `collection_only_retains_its_binder_in_the_expression_form`, fsl-cst `population_queries_preserve_population_names_filters_and_projection_ranges`, fsl-eval `clock_reference_set_aggregators_use_the_canonical_member_order`.
- Tests run: none (memory gate refused).
