# Merge gate recipe

Run `cargo xtask merge-gate-recipe <changed-path>...` from the workspace for a
PR head that already contains current `origin/main`. Pass changed paths relative
to the workspace root. The command checks each name filter with a filtered
`cargo test -- --list`; a filter with no matches fails closed.

The output is one command per line, with target `--no-run` compilation commands
first, name-filtered test commands next, any external driver note after those,
and `cargo check --locked --workspace --all-targets` last. Run the printed
commands in that order. The recipe itself only lists tests; it does not execute
them. For spec-only changes it prints `NO CARGO: spec-only change.`

| Changed paths | Local controls | Consumer controls |
| --- | --- | --- |
| `fsl-package/src/emit/**` | `fsl-package` lib `emit::` tests (skip the 100k case) | `fsl-analyze` IT filters `monitor_`, `timed_`, `hyper`, `probabilistic`; `fsl-command` IT `run_requirement_items`; root `tc_736` |
| `fsl-analyze/src/<engine>/**` | Touched library module | Matching analyze IT engine filter; shared outcome modules route all four engine filters |
| `fsl-replay/src/**` | Touched replay library module | `fsl-command` IT `dependency_handoff`, `run_requirement_items`, `run_text_witness`; analyze IT `probabilistic_replay`, `bounded_probabilistic_replay`, `package_probabilistic` |
| `fsl-inspect/src/**` | `fsl-inspect` render tests | `fsl-command` IT `run_text_witness` |
| `fsl-command/src/**` | IT modules selected by changed area | Driver pin changes also require driver targets `frontend_cli`, `binary_process`, `cli_contract` |
| `fsl-eval/src/**`, `fsl-semantics/src/**` | Touched library module | Analyze IT `timed_` and matching root IT controls |
| One or more test files only | Exactly the changed test modules or named tests | If a changed diff adds/removes `#[ignore]`, xtask `ignored_reason_tests::` |
| `xtask/src/**`, `xtask/tests/**` | Touched xtask module or exact tests | None |
| `Cargo.toml`, `Cargo.lock` | Workspace check | All emitter consumers: analyze's four engine filters and command `run_requirement_items` |
| `spec/**` only | No Cargo commands | None |
| Other crate source or integration modules | Touched library or integration module | No inferred consumers |

Changes to `fsl-package/src/reader_protocol_projection.rs`,
`fsl-package/src/checked_v2.rs`, or `fsl-forms/src/spans.rs` also run the
large-input control below. Emitter changes run it too:

```text
/usr/bin/time -v cargo test --locked -p fsl-package --lib -- emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack --exact --test-threads=1
```

This control runs alone and serially. The regular emitter module command skips
`a_function_body_100000_deep_projects_on_a_small_stack`.
