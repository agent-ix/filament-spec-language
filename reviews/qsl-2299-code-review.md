---
id: SR-2026111612172400000001
title: "code review of PR 2751 compile output file"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 2751 head f9612564d3fc94f7c088c8aadd66cf70d62ab152; fsl-command/src/command.rs, fsl-command/src/command/analyze.rs, fsl-command/src/command/output.rs, fsl-command/src/command/wire.rs, fsl-command/tests/it/request_overrides.rs, tests/it/compile_package_output.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-810
    type: references
---

## Summary

Ticket: QSL-2299. Code review with the rust-review lane (rust-style loaded) of `git diff origin/main...HEAD`. The PR adds the compile request's closed `output` member, `RequestOverrides::output`, the `OutputGivenTwice` and `OutputExists` refusals, and `write_new_file`, which writes a sibling temporary file and hard-links it to the output path. The refusal mapping and the never-overwrite rule are correct. Two defects are in `write_new_file`'s failure handling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `write_new_file` removes the temporary path unconditionally, even when `create_new` failed because that path already existed. It then deletes a file this call did not create. The temporary name carries only the process id, so two compiles in one process writing to the same output collide. The second call's `create_new` fails, and it deletes the first call's temporary file. The first call's `hard_link` then fails `NotFound`, so both calls fail `io-error` and neither writes the file, where one should succeed and the other should refuse `OutputExists`. A stale temporary file left by an earlier process with a reused pid also makes the call fail `io-error` (`AlreadyExists` naming the output path, which does not exist). Fix: remove the temporary file only after `create_new` succeeded, and make the name unique per call, for example pid plus a process-wide `AtomicU64` counter, retrying on `AlreadyExists` for the temporary file. | fsl-command/src/command.rs:864-872 |
| FND-002 | medium | Where the filesystem has no hard links (FAT32/exFAT volumes, some SMB/network mounts), `std::fs::hard_link` fails (`Unsupported`/`PermissionDenied`), so every `output` write there fails `io-error`, although the target is absent and FR-810 requires the write. Fix: when `hard_link` fails with a kind other than `AlreadyExists`, fall back to opening the target itself with `create_new(true)`, writing and syncing, and removing the target on any write error, since this call created it. This keeps never-overwrite through `O_EXCL`, and a partial file can remain only after a crash. Add a test for the fallback branch, for example by factoring the link step behind a function that the test can make fail. | fsl-command/src/command.rs:871 |
| FND-003 | low | FR-810 "Behavior" says the command "writes a sibling temporary file and renames it into place", but the code hard-links instead. That is the correct choice, because `rename` replaces an existing target and would break never-overwrite, but now the spec and the code disagree. Fix: reword the FR-810 bullet to "links it into place without replacing an existing path" (or "renames it with no-replace semantics"). | spec/functional/FR-810-write-the-checked-package-to-a-file-from-a-compile-request.md:84 |

## Verdict

REQUEST CHANGES for FND-001 and FND-002. The rest of the diff was checked and is correct:

- Never overwrite: the `symlink_metadata` pre-check is advisory only. `link(2)` makes the atomic decision and fails `EEXIST` on any existing entry, including a dangling symlink or a directory. The `AlreadyExists` error is mapped to `OutputExists` only when the path exists. A file created between the pre-check and the link therefore still refuses, and the TOCTOU window cannot overwrite. The target never holds partial bytes, because it appears only through the link, after `write_all` and `sync_all`.
- Cleanup: the temporary file is removed after a write, sync or link failure and after success. Apart from FND-001, the only leftover is an orphaned hidden temporary file after a crash.
- Refusals: `OutputGivenTwice` and `OutputExists` map to `Code::InvalidRequest` (category exit 20), envelope stage `request` (output.rs), and outcome-document `last_stage` null with one code diagnostic, the same as `ClocksGivenTwice`. This matches FR-810 and FR-287-AC-11. The `OutputGivenTwice` message names `output` and `--output`, and `OutputExists` names the path. `{"package": ""}`, `{"path": "x"}` and `{"package": 3}` are refused during request decoding, before any source is read.
- Path resolution: the member is joined to the request file's parent directory (`with_request_limits` passes `path.parent()`; `""` for a bare file name resolves against the working directory), and an absolute `package` replaces it. This is what FR-810 "Inputs" states. The override path is used as given, and the driver resolves it against the working directory.
- Write ordering: the write runs only after `complete` returns the package, so every compile refusal writes nothing.
- AC-1 stub rewrite: `compile` is `compile_outcome(..).map(|o| o.package_bytes)`, so a single `compile_outcome` whose `package_bytes()` equals the written file and whose document has `artifacts: []` checks the same bytes AC-1 names. AC-3's test also calls `compile` over a member request. The rewrite is faithful.
- Internal faults: none can be reached from a request. An output path with no file name (`.`, `..`, `/`) is refused as `io-error` `InvalidInput`, not as an internal failure.

Evidence: `cargo test --test it compile_package_output` (3 passed, 1 ignored: AC-5, blocked on QSL-2193); `cargo test -p fsl-command --test it request_overrides` (4 passed); `cargo clippy -p fsl-command --all-targets -- -D warnings` clean. No Python or logic-bearing shell in the diff.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The fallback's write-failure branch is untested. `write_exclusively` removes the target it created when `write_all`/`sync_all` fails. That is the branch that keeps FR-810's "leave no partial file" promise on filesystems without links, and no test reaches it. The coder calls it untestable, but it is not. The same seam that injects `link` can inject the writer: give `write_new_file_linking` (or `write_exclusively`) a write step such as `impl Fn(&mut File, &[u8]) -> io::Result<()>`, then pass one that writes some bytes and returns an error, and assert that the call returns `io-error` naming the path and that the directory is empty. | fsl-command/src/command/output_file.rs:100-111 |
| FND-005 | low | A failure while staging the temporary file is routed into the fallback. `staged.and_then(\|()\| link(..))` merges a `write_all`/`sync_all` error on the temporary file with a link error. A failed staging write (for example `ENOSPC` or `EIO`) therefore retries the whole write directly on the target, and the call reports the fallback's error instead of the original one. That contradicts the doc comment ("When `link` fails for another reason ..."). Fix: when staging fails, remove the temporary file and return `io(error)` before calling `link`. Only a link error should fall back. | fsl-command/src/command/output_file.rs:52-63 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | output_file.rs creates the temporary file with `create_new`, named pid plus an `AtomicU64` counter, and retries up to 64 times on `AlreadyExists`. Only the call that created a temporary file removes it, and a failed `create_new` never reaches the remove. A sequential double-write test and an 8-thread one-winner test pass. When all 64 attempts are used up, `create_temporary` returns `AlreadyExists`, which maps to the typed `RunCause::Io` (`io-error`, exit 20), not a fault. |
| FND-002 | fixed | A link failure other than `AlreadyExists` falls back to `create_new` on the target, then write and sync, removing the target if that write fails. `AlreadyExists` from the link or from the fallback gives `OutputExists`. The link step is injected; the tests for a forced `Unsupported` link and for a target that appears mid-fallback pass. FND-004 records the untested removal branch. |
| FND-003 | fixed | FR-810 line 84 now reads "links it into place without replacing an existing path, or, where the filesystem cannot link, creates the target exclusively and removes it again if that write fails". |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      temporary_name.push(format!(
          ".{}.{}.tmp",
          std::process::id(),
          NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
      ));
      ...
          Ok(file) => return Ok((temporary, file)),
          Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = error,
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |-
      Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(exists()),
      Err(_) => write_exclusively(path, bytes).map_err(|error| {
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      sibling temporary file and links it into place without
      replacing an existing path, or, where the filesystem cannot link, creates
      the target exclusively and removes it again if that write fails.
```

Round 2 evidence at head 2819774c8: `cargo test -p fsl-command --lib output_file` (4 passed), `cargo test --test it compile_package_output` (3 passed, 1 ignored), `cargo test -p fsl-command --test it request_overrides` (4 passed), `cargo clippy -p fsl-command --all-targets -D warnings` and `cargo fmt --check` clean. Round 2 verdict: REQUEST CHANGES for FND-004.
