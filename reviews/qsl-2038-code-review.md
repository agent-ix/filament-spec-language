---
id: SR-2026111518064800000004
title: "Code review — QSL-2038 source corpus runner"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2038; PR #2722; tests/it/main.rs, tests/it/source_corpus.rs, xtask/src/test_suites.rs"
review_set: subset
---

## Summary

Reviewed the QSL-2038 ticket acceptance criteria and the full PR diff against refreshed `origin/main` (`451b61205`). The PR head (`e483e670`) is based at `e0e0843e9`; main advanced by one commit that adds other module registrations to `tests/it/main.rs`, without changing the source-corpus registration or xtask guard. GitHub reports the PR mergeable. The runner exercises the public compile/run APIs and focused checks pass, but the public-path assertion is self-referential, the make-ci removal guard is not actually tested, the area catalog is duplicated, and request sidecars are not bound to their paired source.

## Verdict

**FAIL** — two high-severity findings violate the duplication policy and the required CI-removal guard; two additional test-soundness findings can let the corpus validate the wrong path or source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The seven corpus area names are maintained twice in `AREAS` and `SOURCE_CORPUS_AREAS`; the catalogs can drift, causing the runner and empty-area guard to cover different areas. | tests/it/source_corpus.rs:9-17; xtask/src/test_suites.rs:190-198 |
| FND-002 | high | The tests exercise the empty-area helper directly but never prove that deleting the corpus test selection makes `make ci` fail. Removing `mod source_corpus;` leaves the integration target nonempty and the xtask guard still passes, contrary to QSL-2038's negative selection criterion. | tests/it/main.rs:35; tests/it/source_corpus.rs:590-595; xtask/src/test_suites.rs:1614-1623 |
| FND-003 | medium | `corpus_runner_uses_public_entry_point` searches its own source, so the string literals in its assertions keep the check passing if the runner's actual public compile/run calls are removed; its import-only guard also misses fully qualified internal calls. | tests/it/source_corpus.rs:576-587 |
| FND-004 | medium | A request sidecar is executed directly without verifying that its `program.source.file` names the paired `.native` file; a sidecar can therefore make the test pass while the corpus program itself is never sent through `fsl_command`. | tests/it/source_corpus.rs:158-163, 167-181 |



## Rebase refresh

Rechecked PR #2722 at head `1851dba35a06655cfa9a8f7893f96b5197a0aa4d` against the fetched current `origin/main` `d71c0a8dfde0076cfbc4fbe13985eb2baddd7839` (the PR branch was based on `40f224a4d12017022b0e78256d3d1d87609d9820`). The newer main adds two unrelated monitor-spec commits; the PR-specific source and test changes are unchanged, and the root test file still registers `source_corpus`. GitHub reports the PR mergeable. All four original code findings remain fixed; no new code-review defects were found. No aggregate `make ci` was run in this reviewer refresh.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Runner and xtask guard both read the shared `tests/corpus/areas.txt` catalog. |
| FND-002 | fixed | `RunParser::close` records every required corpus test missing from actual `tests/it/main.rs` suite output; the regression tests cover no selection. |
| FND-003 | fixed | The `syn::visit::Visit` check parses the runner and finds actual calls to both `fsl_command::command::compile` and `run`, while rejecting internal crate paths. |
| FND-004 | fixed | `execute_request` calls `require_paired_program`, which requires at least one selected source to canonicalize to the paired `.native` file. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |
      const AREA_CATALOG: &str = include_str!("../corpus/areas.txt");
      const SOURCE_CORPUS_AREAS: &str = include_str!("../../tests/corpus/areas.txt");
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |
      self.findings.extend(required_source_corpus_test_findings(&id, &tests));
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |
      calls.visit_file(&syntax);
      assert!(calls.compile, "runner must call fsl_command::command::compile");
      assert!(calls.run, "runner must call fsl_command::command::run");
  - fnd: FND-004
    outcome: fixed
    after_excerpt: |
      require_paired_program(request, paired_program, &request_json)?;
```
