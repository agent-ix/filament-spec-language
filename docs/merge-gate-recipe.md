# Merge gate recipe

For ordinary PRs, use a clean tool checkout on `origin/main` history, separate
from the per-PR checkout. It can lag unrelated `origin/main` commits. Run
Cargo there before invoking the recipe executable:

```text
CARGO_INCREMENTAL=0 cargo build --locked -p xtask
```

Cargo's normal freshness checks reuse the existing build and rebuild only when
the tool checkout's inputs require it. The tool checkout may lag unrelated
`origin/main` changes; it must be on the current main history. Invoke its
executable and explicitly select the PR checkout. Pass changed paths relative
to the selected workspace root:

```text
target/debug/xtask merge-gate-recipe --workspace /path/to/pr-worktree --preflight <changed-path>...
```

The same explicit workspace selection is required when calling from elsewhere:

```text
/path/to/tool-checkout/target/debug/xtask merge-gate-recipe --workspace /path/to/pr-worktree --preflight <changed-path>...
```

The command requires a clean tool checkout and clean relevant source paths in
the selected PR checkout. It fetches current `origin/main` in both checkouts,
requires the tool checkout `HEAD` to be an ancestor of its fetched main, and
requires the selected PR `HEAD` to contain its fetched main. It uses Git's
changed-path output to refuse reuse when `Cargo.toml`, `Cargo.lock`, `.cargo`, `rust-toolchain.toml`,
`xtask/Cargo.toml`, `xtask/build.rs`, or `xtask/src/**` differ between the tool
checkout and PR head. It prints the PR head and its verified `origin/main`
revision in the recipe header. If the xtask build inputs differ, update the
tool checkout to a revision containing those changes and build there before
retrying.

For a tooling PR that changes xtask recipe or build inputs, build and invoke
from the same clean PR checkout instead. Cargo's freshness check builds the
PR's executable in that checkout, and the command checks the PR's relevant
build paths are clean and verifies its runtime `origin/main` ancestry. It
skips the separate stable-tool comparison, which would correctly reject the
new tooling changes:

```text
CARGO_INCREMENTAL=0 cargo build --locked -p xtask
target/debug/xtask merge-gate-recipe --workspace /path/to/this-pr-worktree --preflight <changed-path>...
```

By default the command only prints the recipe and does not invoke Cargo. Add
`--preflight` before the paths to check each name filter with normal and
ignored `cargo test -- --list` output; an explicit filter with no non-ignored
matches then fails closed. A local library-module filter with no matches is
reported as a note.

The output is one command per line, with target `--no-run` compilation commands
first, named-filter test commands next, any external driver note after those,
and `cargo check --locked --workspace --all-targets` last. Every test command
has a named module or test filter; the recipe never runs an unfiltered library
or crate test target. Run the printed commands in that order. The recipe itself
only lists tests; it does not execute them. For spec-only changes it prints
`NO CARGO: spec-only change.`

`fsl-analyze` integration tests are split across the `it` (analysis and package
API), `it_monitor_protocol`, `it_probabilistic`, `it_model_check`,
`it_refinement`, and `it_refinement_cx1` (the one heavy CX-1 refinement
liveness test, isolated) Cargo targets. A normal
`cargo test -p fsl-analyze` runs every target. The recipe reads every target
root. An analyzer target-root change, or a change to a crate's own shared
fixture, selects every module of the crate. A low-level library root or crate
manifest change also selects its named downstream consumer controls. A changed
module file, at any depth,
runs that module in each target that declares it; a helper-only module (no test
in it or below it) runs every module of each target that declares it. A consumer
filter selects every target containing a matching test. The command fails, and
prints no recipe, when no target declares a changed module file or a filter
matches no test.

A file under the workspace root's `tests/support/` or `tests/fixtures/` runs
every module of each integration target, in any package, that declares it as a
module. A file no target declares (data a test reads, a module only library code
includes, or a deleted file) runs every root-package module, with a note; its
other readers are not inferred.

| Changed paths | Local controls | Consumer controls |
| --- | --- | --- |
| `fsl-package/src/emit/**` | `fsl-package` lib `emit` tests, one thread, skip 100k tests and the 2/1000 nested-form stress test | Analyze engine filters `monitor_`, `timed_`, `hyper`, `probabilistic`; command `run_requirement_items`; selected package reader modules; TC-736 and other large controls run alone with timing enabled |
| `fsl-package/src/**` outside `emit/**` | Touched library module | `fsl-eval` `checked_route_owner`; `fsl-replay` `emit_admit_agreement` and the TC-736 library control; analyzer `analyze_package_selection`; command `run_requirement_items`; probability-check `roundtrip`; root `run_items` |
| `fsl-foundation/src/**` | Touched library module | Identity/source controls in `fsl-cst` `cst_identity`, `fsl-source` `quire_source`, `fsl-forms` `source_grammar_vectors`, `fsl-semantics` `identity_golden_vectors`, `fsl-package` `source_object_identity_rows_public`, `fsl-command` `check_source_identity`, `fsl-replay` `source_diagnostic_outcome` and root `layer_crate_reexports` |
| `fsl-foundation/src/lib.rs`, `fsl-foundation/Cargo.toml` | Named top-level library modules | Same identity/source controls as `fsl-foundation/src/**` |
| `fsl-source/src/**` | Touched library module | `fsl-replay` `source_diagnostic_outcome`, command `extraction_request_refusals` and root `formal_source` |
| `fsl-source/src/lib.rs`, `fsl-source/Cargo.toml` | Named top-level library modules | Same source controls as `fsl-source/src/**` |
| `fsl-cst/src/**` | Touched library module | `fsl-forms` `source_grammar_vectors`, package `checked_ir_surface`, replay `compile_package_facade` and `compile_text_literals`, and `fsl-cst` `complete_cst` and `complete_grammar` |
| `fsl-cst/src/lib.rs`, `fsl-cst/Cargo.toml` | Named top-level library modules | Same CST controls as `fsl-cst/src/**` |
| `fsl-forms/src/**` | Touched library module | `fsl-semantics` `type_decisions`, evaluator `union_value_profile`, analyzer `package_forms`, command `fr311_s3_forms` and root `forms_package_contracts` |
| `fsl-forms/src/lib.rs`, `fsl-forms/Cargo.toml` | Named top-level library modules | Same forms controls as `fsl-forms/src/**` |
| `fsl-analyze/src/analyze/exact.rs`, `exact/**` | Touched library module | `fsl-analyze` filters `probabilistic` and `timed_`; every `fsl-probability-check` IT module |
| `fsl-analyze/src/exact_probabilistic*` | Touched library module | `fsl-analyze` filters `probabilistic` and `timed_`; every `fsl-probability-check` IT module |
| `fsl-semantics/src/check/probabilistic*`, `check/mod.rs` | Touched library module | `fsl-analyze` filters `timed_`, `timed_within_200`, `timed_within_201`; every `fsl-probability-check` IT module |
| `tools/fsl-probability-check/**` | None | `tests/it/<module>.rs` runs that module; the IT root, fixtures and source run every module |
| `fsl-analyze/src/zone_check/**` | Touched library module | `fsl-analyze` filter `timed_`, plus the focused `timed_within_200` and `timed_within_201` regressions |
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
| One or more test files only | The changed test modules or named tests; every module of each declaring target for a helper-only module | If a changed diff adds/removes `#[ignore]`, xtask `ignored_reason_tests::` |
| `xtask/src/<module>.rs` | The changed xtask library module | None |
| `xtask` crate root or manifest | Named top-level xtask library modules | None |
| `Cargo.toml`, `Cargo.lock` | Workspace check | All named low-level consumers and emitter controls, plus the TC-736 replay control |
| `spec/**` only | No Cargo commands | None |
| Crate `lib.rs`, `main.rs`, or root `mod.rs` | Named top-level library modules | Crate-specific consumer routes |
| `<crate>/tests/fixtures/**`, `<crate>/tests/support/**` | All integration-test modules in the crate | — |
| Root `tests/fixtures/**`, `tests/support/**` | All root-package modules when the root target declares the file, or, with a note, when no target does | All modules of each other declaring target, in any package |
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
