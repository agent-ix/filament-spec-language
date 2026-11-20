# QSL-2097 final affected-component gate

Tested source head: `4dcb369d53317c5ed586c1736eb1820c7b8e558a`.

The independent code and gap reviews passed with no findings (SR-1511 and
SR-1512; validation exit 0). This receipt publishes the completed gate; it
does not change the tested source.

The affected component is every test in the changed
`tools/arch-lint/metadata.rs` module, selected by the qualified module name
`metadata::tests::` in the `arch-lint` binary test target. The feature adds
three tests across ten metadata fixtures for FR-356-AC-1: renamed shared
walker dependencies, FSL dependencies disguised by walker aliases, and the
normal CG facade exceptions. Other historical QSL-2097 requirement families
remain unclaimed by this slice.

Before the build, `df -h /` reported 210 GiB available. The command ran from
`/home/peter/dev/.worktrees/luna-qsl-2097`, using its single owned target:

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2097/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p arch-lint --bin arch-lint metadata::tests:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Actual process exit: **0**.

```text
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out; finished in 0.61s
```

The original invocation omitted the separator between Cargo and libtest
arguments:

```sh
CARGO_TARGET_DIR=/home/peter/dev/.worktrees/luna-qsl-2097/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p arch-lint --bin arch-lint metadata::tests:: --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Actual process exit: **1**, before building or executing tests:

```text
error: unexpected argument '--test-threads' found

  tip: a similar argument exists: '--test'
  tip: to pass '--test-threads' as a value, use '-- --test-threads'
```

The corrected invocation executed the previously unrun component. No tests
were repeated because of that argument-parsing correction. No unrelated
workspace, facade, proof, Kani, lint, review, matrix or aggregate checks ran
as part of this final gate. Later publication changes contain only these
review artifacts and this receipt.
