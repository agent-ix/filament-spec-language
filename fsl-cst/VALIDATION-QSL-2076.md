---
type: status
---
# Temporal grammar test validation

The new temporal grammar cases passed focused checks. The single final component
gate at `3d0775109ebb335f32596f697463d2949200b499` exited 101 in an existing
formatter unit test; it did not reach integration or doc tests. Those two
unrun segments were then executed once separately. The original failure log
is retained, and this receipt does not report a full component pass.

## Commands and inputs

All commands used the existing ticket worktree target, `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`. Free space before the gate was 213 GiB.

```sh
cargo test --locked -p fsl-cst -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
cargo test --locked -p fsl-cst --test it -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
cargo test --locked -p fsl-cst --doc -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Only the two previously unrun segments followed the failed original command.
The 15 passing unit cases and known failing unit were not rerun. No workspace,
product-binary, lint, formatter or review confidence gate followed.

## Unit disposition

The unit segment recorded 15 passes and one failure:
`format::tests::a_population_bound_ends_its_own_line`. Actual output joins
`M::orders[8]`; the existing required oracle is `M::orders [8]`, likewise for
`people [2]` and `staff [0]`.

FSL FR-003's canonical bracket rule requires a space outside its type-bound,
index, collection-value and temporal/window exceptions. FSpec's
`population-bound` production is none of these exceptions. The formatter
therefore has a product defect; the expected strings remain unchanged. The
unit is explicitly pending under QSL-2679 and excluded from coverage. Its
production formatter remains unchanged in this PR.

Local logs are `.agent/qsl-2076-final-cst.log`,
`.agent/qsl-2076-final-it.log` and `.agent/qsl-2076-final-doc.log`.

## Segmented results

| Segment | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Original unit segment, exit 101 | 15 | 1 | 0 |
| Previously unrun integration segment, exit 101 | 268 | 12 | 11 |
| Previously unrun doc segment, exit 0 | 1 | 0 | 0 |
| Total actual recorded results | 284 | 13 | 11 |

All seven new near-miss functions passed in the integration segment. Its
171.28-second run also recorded these existing failures:

- `complete_cst::header_recovery_resumes_at_the_next_declaration`
- `complete_grammar::complete_source_diagnostics_carry_their_catalogued_typed_cause`
- `complete_grammar::contextual_complete_words_remain_eligible_identifiers`
- `complete_grammar::every_named_production_has_positive_boundary_and_located_negative_vectors`
- `decimal_constructor::a_numeric_token_that_is_no_signed_int_or_uint_is_an_invalid_token`
- `nesting_levels::a_work_budget_refusal_names_the_work_limit`
- `nesting_levels::nested_trailing_comma_typo_recovery_points_at_the_offending_comma`
- `protocol_memory_refinement::a_history_row_refuses_a_non_literal_initial_value_at_its_first_token`
- `readme_snippets::readme_protocol_excerpt_parses_and_native_fences_parse`
- `reserved_declaration_names::lowercase_tuple_still_refuses_as_a_record_name`
- `selection::an_edition_other_than_1_draft_refuses_unknown_edition_at_its_literal`
- `syntax_messages::long_expected_sets_use_a_short_category_phrase`

The original unit segment took 39.53 seconds. The doc segment passed its one
compile-fail example in 0.03 seconds. No failed segment was repeated. The
existing integration failures and 11 ignored cases remain visible; none is
changed into passing coverage by this PR.

## Confined failure dispositions

The original gate and its totals above remain unchanged. Only the reported
failing functions were considered; no component suite or previously passing
group was repeated.

Four stale tests were corrected and passed exact-name checks:

- Work-budget refusal now selects an explicit `s1.work_units` ceiling on valid
  source, as FR-255 requires, instead of assuming one malformed expression has
  quadratic work under default limits.
- Nested typo recovery now distinguishes FR-762's whole-declaration recovery
  region from the exact diagnostic token locus required by FSpec FR-302.
- README fence recognition accepts the current `stage=parse` info-string
  metadata while continuing to parse the actual authoritative example bytes.
- The bounded expected-set message case now reaches a genuine expression
  choice; a brace after an otherwise complete unit expects end of source,
  rather than a large declaration-alternative set.

Seven required functions preserve their original oracles and are explicitly
pending under six product owners:

| Owner | Required behavior |
| --- | --- |
| QSL-2680 | FR-762-AC-9 malformed record CST after header recovery |
| QSL-2681 | FR-762-AC-2 dollar-token lexical classification |
| QSL-2682 | FR-091-AC-58 invalid decimal numeric classification |
| QSL-2683 | Contextual collection names and their history-literal locus |
| QSL-2684 | FR-762 single-error recovery without reserved-name cascades |
| QSL-2685 | FSpec FR-500 unsupported-edition refusal precedence |

The exact pinned FSpec revision is
`f32bca6c7937e0652742b2bbe27a52419bf5a0ae`. Its complete reserved inventories
admit lowercase `set`, `bag` and `orderedSet`; an existing implementation test
that refuses these names supplies no normative authority. The history case
therefore keeps its original bracket locus and shares QSL-2683. Its temporary
changed-oracle pass is not accepted coverage.

The production-vector function has a grounded `ReferenceLiteral` inventory
entry and original executable reference-literal input. Its pre-existing strict
right-boundary location assertion remains unresolved: targeted receipts name
`RoundingMode` and `TextProfile`, but did not print actual diagnostics/spans.
The original strict assertion is retained, and no speculative prefix-span
alternative or per-production exception remains. No product defect is inferred
from those omitted data.

Confined logs are `.agent/qsl-2076-disposition-failing-names.log` and
`.agent/qsl-2076-disposition-production-{exact,mode,token}.log`. The first
unqualified `--exact` command selected zero tests and supplies no pass evidence.
No strict pending case was reprobed. This checkpoint awaits a confined review
of the corrections and unresolved production-vector contract.

## Reviewer-directed token mutation check

The confined correction now retains the last significant original CST token
span for each production occurrence and requires its end to match the
production's end. The boundary vector replaces that whole lexeme with
same-width spaces and a final `#`, preserving the existing strict
boundary-covering diagnostic assertion. No prefix exception, production
whitelist, parser change or additional original-source parse was introduced.

Only the changed qualified production-vector function ran once with the same
target, profiles, single thread and required skips. It exited **101** after
0.94 seconds; the warm compile took 2.16 seconds. Exact local receipt:
`.agent/qsl-2076-vector-fix.log`. No other test or gate followed.

The actual failure was `CaseArm`. Its generated source was:

```fsl
language "ix:native" edition "1-draft";
profile Complete = "filament.value.complete/v1";
union Shape { Empty, Circle(Integer), Rect(Integer, Integer), } function area using Complete(s: Shape): Integer pure { case s { Circle(r): r# Shape::Rect(w, h): w * h; Empty: 0; } }
```

The actual diagnostic was `invalid_syntax` / `unexpected-token`, byte span
`[231, 236)`, message `expected quoted string`, at the following `Shape`.
Replacing the one-byte arm terminator yielded adjacent `r#`; this is the
retained failure context, not a new parser bug claim. The strict location
assertion remains unchanged. No additional trial change or runtime was made.
