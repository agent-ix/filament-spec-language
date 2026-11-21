---
id: SR-2026112117074738624700
title: "Code review (Rust lane) — QSL-2739 main reds: package-route composition guard and default-item oracles"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3348; branch terra/missing-engines-forms/qsl-2739-main-reds frozen 79aa75be18b07aab639f8078caa3d4c674826ac5 over base 660a550b40b068df5337155009e009299654f3b1; fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/tests/it/analyze_composition_cli.rs, fsl-analyze/tests/it/analyze_request_wire.rs, spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md"
review_set: subset
---

## Summary

Ticket: QSL-2739. Reviewed the frozen four-file diff (+83/-43) that clears the five
`fsl-analyze --test it` reds on main. The diff has two parts. The product fix makes
`unnamed_composition` detect an admitted `component_composition` node directly, and
`resolve_names` uses the same fact. The other part corrects three stale default-item
oracles and two stale composition-package oracles. Code-review and rust-review (lane)
were run together in this one artifact. The product logic is correct and narrowly scoped. The only
finding is low-severity doc-comment drift.

## Verdict

**CONDITIONAL**: one low finding (doc comments). The behaviour, oracles and scope are
sound. Runtime qualification has not been done yet: no test was run by the author or the
reviewer, and the seven exact cases are queued on Draco.

What was checked, and is clean:

- **Product predicate (`package_run.rs:1885-1892`).** `crate::claim_family::is_component_composition` is the existing decoded
  family predicate (`claim_family.rs:56`). Nothing is derived from a path, region or
  body. No synthetic name, IR, wire or API change, and no shared `AnalysisView::unnamed_claims`
  change. This matches the audit's narrower owner boundary. The statically traced chain is: the producer
  writes the composition as a `Declaration` occurrence with `declaration: None`, and
  `analysis_view.rs:204-233` `unnamed_claims` requires a `Claim` occurrence. That confirms the old
  guard could never fire.
- **No silent drop and no over-refusal of known items.** With no `items`, the whole
  request is refused before `default_selections` (`package_run.rs:298-310`), so the
  composition cannot be silently dropped. With explicit `items`, `resolve_names` still resolves `temporal_clause_names`,
  `claim_names`, `declared_names` and model aliases before it consults `unnamed`
  (`package_run.rs:792-838`). The flag only changes the final fallthrough from
  `unknown-claim` to `PendingFamily::UnnamedClaim`, so names that ordinary rows carry are unaffected.
  A package with no composition node keeps `unknown-claim`, so FR-850-AC-5 and AC-7 over
  Counter are unchanged. The conservatism is applied only when an unread composition is
  present. It is documented in FR-1044 "Current system behavior" and invents no name guess.
- **Rendered oracles (renderer inspected).** `PendingPackageFamily::detail`
  (`package_run.rs:78-112`) yields `": the composition rule has no package form yet at {node:?}"`
  for the default request (node `Some`). For a named item (node `None`) it yields
  `": items[0] names the unnamed-claim family, which has no package form yet"`.
  `fsl-command/src/command/analyze.rs:352-356` maps Pending to `AnalyzeNotYetImplemented`. The
  `#[error]` text at `fsl-command/src/command.rs:1047-1049` embeds `{detail}`. `command.rs:1222`
  gives code `unsupported_construct`, `command.rs:1543-1553` gives one diagnostic with cause
  `not-yet-implemented` and `locus` is `None` for both families. `output.rs:331` gives the request stage.
  The precedent `analyze_package_selection.rs:831` asserts `last_stage` null for a request-stage refusal.
  The oracles are honest and require no earlier `invalid_package`.
- **Local control.** `{"claim": "Local"}` resolves by its `temporal_clause_names` row
  before `unnamed` is consulted. `UnreadModelForm` (`fsl-package/src/model_input_form.rs:19-34`)
  has no composition form, so `PackageModelInput::of` is not expected to refuse. The `proved`
  and exit 0 expectation matches the source route's `Local` (`analyze_composition_cli.rs:877-878`),
  but it has not been run.
- **Default-item oracles.** `CONTRACT` (`tests/it/counter.rs:32-34`) prepends `pre CanInc`
  and `post Inc`, and `counter_request_with` (`analyze_cli_support.rs:230-236`) writes it first.
  FR-620 ("one item per checked claim of the program in source order across all declaration
  families", lines 125-129) and FR-489 (line 55, operation `pre`/`post` routes to
  `contract-checker`) support `CanInc, Inc, <body claims>` with deadlock-freedom at index 4.
  The prefix now names the full authored claim set rather than skipping items, and the
  library `Early` exclusion is retained.
- **Rust lane** (rust-style baseline, testing reference, repo AGENTS.md). Borrowed `&dyn AnalysisView`,
  `Option` for absence, no new `unwrap`/indexing panic on caller input in production code.
  `names[..4]` slicing in tests panics with a clear location if fewer items are written,
  which is acceptable for a test. No `pub` surface change, no Tokio, no unsafe and no wire change.
  Tests live in `tests/it/` modules per AGENTS.md. The new and modified current-behaviour
  tests carry no `#[trace]`: they back non-normative "Current system behavior" prose, not an
  AC, and deliberately carry no FR-1044-AC-10 parity tag. `the_package_route_settles_the_same_items`
  stays `#[ignore]` with a tracked reason.
- **Duplication/vendoring and ceremony.** None. `assert_package_pending` replaces
  `assert_package_refused` rather than duplicating it. No pins, ledgers or manifests.
- **Gates.** Per the dispatch brief and the user's focused-test policy, the reviewer ran no Cargo
  commands. The author reports `cargo check --locked -j4 -p fsl-analyze --test it`
  exit 0 and `rustfmt --check` clean on the three Rust files. Clippy, the test run and runtime
  qualification are pending on Draco and are not claimed here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `unnamed_composition`'s name and doc ("which no name table row carries") now state a property the function no longer checks, and `resolve_names`' doc still lists only invariant/refinement/state-graph as the unnamed families that suppress `unknown-claim`, omitting composition | fsl-analyze/src/analyze/document/package_run.rs:1882 |

FND-001 detail. After the change, `unnamed_composition` returns any admitted
`component_composition` node, whether or not a name row carries it. That is
correct only because the producer writes no composition name row today. If composition
name rows land (FSpec#797), the default guard is still right, because the family has no package
form. But the `unnamed` flag in `resolve_names` would then refuse a genuinely unknown
`Typo` as `unnamed-claim` instead of FR-850's `unknown-claim`, and nothing in the code
or its doc would say so. The `resolve_names` doc (`package_run.rs:694-702`) enumerates the claim
kinds whose presence suppresses `unknown-claim` and omits composition, so the doc no longer
matches the code at line 717. Suggested fix: name the presence fact for what it is (for example
`composition_node`) or state that the producer writes no composition name row yet, and add the
composition to the `resolve_names` doc list.
