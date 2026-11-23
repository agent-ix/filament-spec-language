# Merge gate recipe

Run `cargo xtask merge-gate-recipe <changed-path>...` from the workspace for a
PR head that already contains current `origin/main`. Pass changed paths relative
to the workspace root. The command fetches `origin/main`, verifies ancestry, and
prints both commit IDs in its header. It checks each name filter with normal and
ignored `cargo test -- --list` output; an explicit filter with no non-ignored
matches fails closed. A local library-module filter with no matches is reported
as a note.

The output is one command per line, with target `--no-run` compilation commands
first, name-filtered test commands next, any external driver note after those,
and `cargo check --locked --workspace --all-targets` last. Run the printed
commands in that order. The recipe itself only lists tests; it does not execute
them. For spec-only changes it prints `NO CARGO: spec-only change.`

| Changed paths | Local controls | Consumer controls |
| --- | --- | --- |
| `fsl-package/src/emit/**` | `fsl-package` lib `emit` tests, one thread, skip 100k tests and the 2/1000 nested-form stress test | Analyze IT filters `monitor_`, `timed_`, `hyper`, `probabilistic`; command IT `run_requirement_items`; each large emitter and replay control runs alone with timing enabled |
| `fsl-analyze/src/analyze/exact.rs`, `exact/**` | Touched library module | Analyze IT `probabilistic` and `timed_`; every `fsl-probability-check` IT module |
| `fsl-analyze/src/exact_probabilistic*` | Touched library module | Analyze IT `probabilistic` and `timed_`; every `fsl-probability-check` IT module |
| `fsl-semantics/src/check/probabilistic*`, `check/mod.rs` | Touched library module | Analyze IT `timed_`, `timed_within_200`, `timed_within_201`; every `fsl-probability-check` IT module |
| `tools/fsl-probability-check/**` | None | `tests/it/<module>.rs` runs that module; the IT root, fixtures and source run every module |
| `fsl-analyze/src/zone_check/**` | Touched library module | Analyze IT `timed_`, plus the focused `timed_within_200` and `timed_within_201` regressions |
| `fsl-analyze/src/analyze/request.rs`, `fsl-analyze/src/analyze.rs` | Touched library module | The four analyze IT engine filters plus the source/package parity modules `analyze_package_selection`, `analyze_request_wire`, `analyze_package_provider`, `analyze_composition_cli` |
| `fsl-analyze/src/analyze/document.rs`, `fsl-analyze/src/analyze/document/**` | Touched library module | As `request.rs` |
| Other `fsl-analyze/src/**` | Touched library module | All four analyze IT engine filters: `monitor_`, `timed_`, `hyper`, `probabilistic` |
| `fsl-replay/src/**` | Touched replay library module | `fsl-command` IT `dependency_handoff`, `run_requirement_items`, `run_text_witness`; analyze IT `probabilistic_replay`, `bounded_probabilistic_replay`, `package_probabilistic` |
| `fsl-eval/src/simulation/**` | Touched library module | Analyze IT `protocol_bounded_lasso`, `race_freedom`, `native_correlation` (concrete protocol state keys, message ordinals, delivery identities) |
| `fsl-replay/src/certificate*`, `probability_certificate*`, `exact_probabilistic*`, `spine*` | Touched replay library module | The `fsl-replay/src/**` consumers plus every `fsl-probability-check` IT module |
| `fsl-replay/src/lib.rs`, `fsl-replay/Cargo.toml`, `product_action.rs`, `identity.rs`, `limits.rs`, `proof_result*`, `outcome*`, `request*`, `statistical*`, `model_check/limits*` | Touched replay library module (crate fallback for `lib.rs` and the manifest) | The `fsl-replay/src/**` consumers plus every `fsl-probability-check` IT module; these are the replay files the certificate checker and `probability_certificate` import |
| `fsl-inspect/src/**` | `fsl-inspect` render tests | `fsl-command` IT `run_text_witness` |
| `fsl-command/src/**` | Touched library module plus IT modules selected by explicit command-area rules | Driver pin changes also require driver targets `frontend_cli`, `binary_process`, `cli_contract` |
| Known `fsl-eval` and `fsl-semantics` paths | Touched library module | Explicit table of root/consumer IT filters; unknown paths add analyze `timed_` and a note |
| One or more test files only | Exactly the changed test modules or named tests | If a changed diff adds/removes `#[ignore]`, xtask `ignored_reason_tests::` |
| `xtask/**` | Whole xtask `--lib` target | None |
| `Cargo.toml`, `Cargo.lock` | Workspace check | All emitter consumers: analyze's four engine filters and command `run_requirement_items` |
| `spec/**` only | No Cargo commands | None |
| Crate `lib.rs`, `main.rs`, or root `mod.rs` | Whole `--lib` for small crates; union of crate rules for large crates | Crate-specific consumer routes |
| `tests/fixtures/**`, `tests/support/**` | All integration-test modules in the crate | — |
| `README.md`, `docs/**`, `Makefile`, `.github/**` | Note only; workspace check if combined with code | — |
| Other crate source or integration modules | Touched library or integration module | No inferred consumers |

Changes to `fsl-package/src/reader_protocol_projection.rs`,
`fsl-package/src/checked_v2.rs`, or `fsl-forms/src/spans.rs` also run the
large-input control below. Emitter changes run it too. Emitter module runs are
serial because parallel runs exceeded the build queue memory limit. The isolated
controls include the 100k emitter cases, the nested-form stress case, and the
three `fsl-replay` TC-736 large-input tests:

```text
/usr/bin/time -v cargo test --locked -p fsl-package --lib -- emit::tests::protocol_locations::a_function_body_100000_deep_projects_on_a_small_stack --exact --test-threads=1
```

Each control runs alone with `/usr/bin/time -v`. The regular emitter command
uses `--test-threads=1 --skip 100000 --skip
every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels`.
