---
id: SR-599012
title: "Code review — QSL-2937 PR #3576 FSL result texts"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...eebfca09d9 (32 files); FR-865 AC-9..24, FR-866 AC-8..10, FR-286 AC-29/38/39/40, FR-810-AC-6, FR-1047-AC-9"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

## Summary

Ticket: QSL-2937. Code review with the rust-review lane folded in, of PR #3576 at head eebfca09d9 (base 7ca2bd11ab), implementing merged spec PR #3563. Read only, with no cargo run (the host was at load 47, so no build and no tests). Checked against the brief:

1. **Text from the document alone.** The `packaged` lines come from `packaged_result_lines` reading `wire["result"]["package"]` (fsl-inspect/src/render.rs:378-402). There is no `render_text` or side channel on a compile outcome. Stage words come from `stage_word` over the wire label. Clean.
2. **Producers.** `selection` is set at every `missing_import`/`missing-selection` construction site that reaches an `OutcomeDiagnostic` in production:
   - `model`: CompileRefusal::Intake, the per-entry intake diagnostics, ReplayRefusal::DomainPackages, the RunError model-intake refusal, AssemblyCause::UnadmittedModel and `check_missing_model_help`.
   - `library`: ImportRefusal::MissingSelection and AssemblyCause::UnsuppliedImport.
   - `sibling-unit`, `function-owner`, `byte-provision` (which also gains cause `missing-selection`) and `extension-definition` (ExtensionRefusal::MissingDependency).
   - `source`: AnalyzeRefusal::Provision.
   - `package-reference`: ReplayRefusal::Package/PackageWithIdentity, AnalyzeRefusal::Package and RunCause::AnalyzePackage.
   - Plugin replay's `UnresolvedDependency` stays without a selection and keeps the pair's catalog hint.

   Three listed producers have no production path to an `OutcomeDiagnostic`:
   - `LibraryRefusal::MissingImport`: `resolve_libraries` has test callers only.
   - `PackageError::MissingDefinition`: reached only through `link_bundle` in tests and `ProfileCheckFailure::Closure`, which no command maps to a diagnostic.
   - `CallSiteRefusal::UnknownImport`: see FND-001.

   A grep of every `Code::MissingImport` and `"missing_import"` site found no missed producer. The 3-argument `hint_for` leaves every other hint unchanged: each old row gets `selection: None`, every old caller passes `None`, and the count goes from 23 to 32 rows, the 9 new ones being the selection rows.
3. **Stage words.** The table matches FR-865 exactly, `I2` maps to `imports`, an unknown label is returned unchanged, and the function is re-exported at the crate root.
4. **The `packaged` result.**
   - `output` is `directory.join(member.package)`, where `directory` is `path.parent()`. For a bare `c.json` that parent is `""`, so no join happens.
   - `--output` is reported as given, using `to_string_lossy`.
   - `handoff` is `directory.join(handoff.directory)`.
   - The member is set only after the writes succeed. Both members are `null` when nothing is written, and the model-unit path passes `handoff: None` (it refuses a handoff anyway).
   - The coder's `native_join` note is not a defect. FR-286-AC-39 names FR-665-AC-1's handoff, which carries `native_join`, and FR-665 makes `native_join` required (`write_compile_handoff` refuses without it). FR-810-AC-6 is the output-only case.
5. **FR-866 messages.** The text is exact, identities are listed in source order and deduplicated, the no-models tail is right, and `--library` keeps its old message.
6. **FR-1047-AC-9.** `export::require_object_request` runs only on the export entry and only for a `native-run/1` envelope. It writes the exact message `invalid clause request member /request: expected an object` and a `null` details object. The run, monitor and replay entries keep the old `with_request_limits` path.
7. **Schema.**
   - The `selection` enum lists the nine kinds.
   - The `packaged` rule is closed: `bytes` is a non-negative integer, and `output` and `handoff` are each a string or `null`.
   - `missing` and `accepted` were already on main from #3561.
   - The outcome schema is FSL's own (`OUTCOME_SCHEMA_1`), and consumers validate against those same bytes. No pinned FSpec copy of `filament-outcome/1` exists, so the additive members cannot break an external validator in this repo.
8. **AC-16 driver half.** There is no driver code and nothing tags or claims FR-865-AC-16.
9. **`jsonschema` dev-dependency.** It is `0.17.1` with feature `draft202012`, the same spec as the workspace root and six other crates. The Cargo.lock change is one line inside fsl-replay's own dependency list, so no new crate enters the lock.

The commits also rustfmt some lines outside the change (re-export lists, one test assertion), with no behaviour change. The PR body is stale in three places (known; it cannot be edited).

## Verdict

**CONDITIONAL.** No high finding.
- FND-001 is a medium spec/code mismatch: the `unimported-library` producer writes no `OutcomeDiagnostic` in production, and the PR satisfies AC-19 through a public constructor that only a test calls. A spec steward decision is needed: keep the constructor and correct the spec's producer wording, or wire a real diagnostic path.
- FND-002 and FND-003 are low robustness gaps in the new helper surface.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `CallSiteRefusal::UnknownImport` never becomes an `OutcomeDiagnostic` in production, since every call-site refusal settles as an item `TerminalValue::Declined`. The PR adds a public `OutcomeDiagnostic::from_call_site_refusal` that only `imported_function_site.rs` calls, so AC-19 and the `unimported-library` row of AC-40 are verified on a path no command takes. This contradicts FR-286's "producers are every producer of the pair that writes an OutcomeDiagnostic" | fsl-replay/src/outcome.rs:2652-2662; fsl-replay/src/call_site.rs:528-561; fsl-replay/src/proof_result.rs:1523-1545; fsl-replay/tests/imported_function_site.rs:357 |
| FND-002 | low | `with_selection` sets `selection` whatever the diagnostic's code and cause, so a producer whose cause is not `missing-selection` would write a `selection` with no hint, breaking FR-286's rule. ReplayRefusal::Package passes the native reader's cause through unnormalized, unlike AnalyzeRefusal::Package and RunCause::AnalyzePackage, which force `missing-selection`. The pinned contract-ir sites all set `MissingSelection`, so the rule holds today but nothing asserts it, and no test covers the replay or run package-reference producers | fsl-replay/src/outcome.rs:2516-2524; fsl-replay/src/execute.rs:735-739; fsl-analyze/src/analyze.rs:1199-1212; fsl-command/src/command.rs:1673-1684 |
| FND-003 | low | The new `CallSiteRefusal::cause` returns `None` for `Compile`, `ModelIntake`, `Import` and `Dependency`, while `code()` delegates to the inner refusal. A `from_call_site_refusal` diagnostic of a wrapped `missing_import` refusal therefore pairs the inner code with a `null` cause, losing both its hint and its selection | fsl-replay/src/call_site.rs:528-548 |

FND-001: A `grep` for `CallSiteRefusal` outside tests finds only `call_site.rs`, `application.rs`, `proof_result.rs`, `outcome.rs` (the new constructor) and the `lib.rs` re-export. FR-121-AC-35's refusal reaches users as a declined item record, which FR-286 "Missing selections" says carries no `selection`. Either the spec's `unimported-library` row should name the producer as reachable only through the library constructor (and AC-19 be reworded), or a command path should project the refusal as a document diagnostic. That is a spec-steward call, not a coder fix.

FND-002: Add a guard in `with_selection` (`self.code == "missing_import" && self.cause.as_deref() == Some("missing-selection")`), or normalize the cause the way the analyze and command paths do. Add one replay `Package` producer assertion.

FND-003: Delegate `cause()` to the wrapped refusal's cause where `code()` delegates, or confine the constructor to `UnknownImport`.

## Coverage

- FR-865 "Stage words", the `packaged` text rules, and the "Missing selections" producer sentence: examined; clean apart from FND-001.
- FR-286 "Compile results", "Missing selections", "Fix hints" (3-key lookup), "Published schema" (`selection`, `packaged`): examined; FND-001 and FND-002.
- FR-866 unselected `--model` and `--library` messages: examined; clean.
- FR-810-AC-6: examined; clean.
- FR-1047-AC-9: examined; clean.
- FR-665 `native_join`: context_only (handoff precondition); not a defect.
- Rust lane:
  - No new `unwrap` or `expect` in production code.
  - `refusals[0]` matches the existing indexing of the non-empty `Refusals`.
  - No `unsafe` and no async.
  - `bytes: usize` is serialized as a JSON integer.
  - Gates were not run (host load; per brief).
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Deferred to QSL-2962, which exists in Backlog and cites QSL-2937 code-review FND-001: imported-call refusals never reach an `OutcomeDiagnostic` in production. That needs a spec-steward decision; the code meets the spec as written |
| FND-002 | fixed | At head 86c05adc80, `with_selection` writes `selection` and the keyed hint only when `code == missing_import` and `cause == missing-selection` (fsl-replay/src/outcome.rs:2519-2531). `ReplayRefusal::cause` returns `missing-selection` for a `missing_import` `Package`/`PackageWithIdentity` (execute.rs:754-760). `a_missing_import_package_refusal_writes_the_package_reference_selection` covers reader causes None, `invalid-value` and `missing-selection` on both variants. `a_selection_is_written_only_on_the_missing_selection_pair` covers the guard. `the_package_reference_producer_text_holds_its_hint` covers `RunCause::AnalyzePackage` |
| FND-003 | fixed | `Compile`, `ModelIntake`, `Import` and `Dependency` carry the `cause` and `selection` that `From<CompileRefusal>` reads once before its match (call_site.rs:710-760), and `cause()`/`selection()` return them. `wrapped_call_site_refusals_keep_their_cause_selection_and_hint` uses real Import, Dependency and Extension refusals. The ModelIntake case is built by hand. That is acceptable because the From arm passes the same two shared bindings as the tested arms, so no intake-specific mapping is left untested. The xtask pattern change to `..` is correct, and no other code in the workspace or in filament-driver matches these variants by shape |
