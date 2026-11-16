---
id: NFR-017
org: agent-ix
title: "Build a clean clone with locked dependencies and run the documented examples"
type: NFR
quality_attribute: portability
relationships:
  - target: ix://agent-ix/filament-spec-language/US-037
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/NFR-013
    type: references
---
# NFR-017: Build a clean clone with locked dependencies and run the documented examples

## Statement

When the `make clean-clone-check` gate runs at a commit, the gate shall
clone that commit into a new directory, build it with
`cargo build --locked --workspace` under an empty `CARGO_HOME`, run the
examples the README documents, and fail if any step fails or needs a
sibling checkout, a local patch or a repository that cannot be cloned
anonymously.

## Scope

The README's Usage section promises an outside user that a clone builds with
only a Rust toolchain at the `rust-version` in `Cargo.toml`. This gate keeps
that promise at every commit it runs on.

### The gate

`make clean-clone-check` takes `REPO` (default: this repository's own
directory) and `REV` (default `HEAD`). It makes every temporary directory
under one new directory it creates in `TMPDIR`, and:

1. clones `REPO` with `git clone --no-local` into a new directory and checks
   out `REV`; it refuses, before building, when the clone directory or any
   of its ancestors holds a `.cargo/config.toml` or `.cargo/config`, because
   Cargo would read it;
2. sets `CARGO_HOME` and `CARGO_TARGET_DIR` to new, empty directories, so no
   registry cache, git checkout or build artifact of the host is reused
   (NFR-013);
3. runs `cargo build --locked --workspace` in the clone;
4. checks the sources: every package `cargo metadata --locked` reports is in
   the clone, from crates.io, or from a git repository under
   `https://github.com/agent-ix/` that `git ls-remote` reads with
   `GIT_TERMINAL_PROMPT=0` and no credential helper; a path dependency or
   `[patch]` entry outside the clone fails this step, naming it;
5. runs the README's example commands in the clone and checks their outputs:
   `cargo run --locked --example config_version_fixtures -- <tmp>/config-version`
   writes the case directories `healthy-parent`, `violating-parent`,
   `dangling-parent`, `incomplete-population` and `cycle`;
   `examples/config-version/requests.sh <tmp>/config-version/healthy-parent ParentOrder`
   writes `run.json` and `compile.json` there;
   `cargo run --locked --example emit_checked_v2 -- --package-name switch --model examples/switch-transition/switch.semantic-ir.json examples/switch-transition/switch.native`
   exits 0 and its stdout, saved to `<tmp>/package.json`, reads back through
   FSL's I2 reader with the switch model; and
   `cargo test --locked --test it walkthrough_requests_reproduce_the_documented_exit_codes`
   passes;
6. writes a report naming each step and its result, and the first failing
   step with its cause, to stdout;
7. removes the directory it created in `TMPDIR`, whether it passes or fails.

### Documented FSL fences

The README and the pages under `docs/` show FSL source to readers, and a
reader copies it. Every such excerpt is checked by the integration test
`doc_snippets`, which runs in `make ci` and in the clean clone's test step,
so a documented excerpt either does what it claims or says why not.

- An **FSL fence** is a fenced code block in `README.md` or a `docs/**/*.md`
  file whose info string's first word is `ix:native` or `fsl`. FSL source in
  those files SHALL be written in an FSL fence; a fence with no info string
  holds no FSL. A fence nested inside a longer fence (a Markdown example) is
  an FSL fence too.
- Each FSL fence's info string SHALL carry `stage=parse`, `stage=check` or
  `stage=compile`, the stage it reaches, and MAY carry one of
  `reject=<code>/<cause>` or `pending=<ticket>`, never both.
- A fence whose text has no `language` line is an excerpt. The test runs it
  after the header `language "ix:native" edition "1-draft";`, and the stage
  is the one the excerpt reaches with that header alone.
- A fence with no `reject` or `pending` SHALL pass its declared stage. A
  `reject=<code>/<cause>` fence SHALL refuse at or before its declared stage
  with exactly that code and cause; it documents a refusal on purpose.
- A `pending=<ticket>` fence is an excerpt that does not yet reach its
  declared stage because of a known gap. The line right after its closing
  fence marker is the Markdown comment `<!-- expect: <code>/<cause> -->`, the
  refusal it gives today. The test SHALL fail when it refuses with another
  code or cause, and when it passes its stage, so the fix that closes the
  gap removes the `pending` tag and the comment in the same change. A
  failing excerpt is declared, never deleted from the page or skipped.
- The test reads the Markdown with the standard library, calls no external
  tool, and identifies a fence by its file and opening line, never by a hash
  or digest of its text. It SHALL fail when an FSL fence declares no stage,
  and when its fence count differs from a second, simpler count of
  opening fence lines with an FSL info string.

### Out of scope

The examples step runs only the README examples; `make ci`'s conformance and
qualification targets are not part of an outside user's build. They read the
standard's vectors through the pinned Cargo dependency (FR-760), never through
a sibling checkout, so they too run from a clean clone.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Failed steps of `make clean-clone-check` at the head of `main` | 0 | 0 | Test |
| Package sources outside the clone, crates.io and anonymously readable `agent-ix` repositories | 0 | 0 | Test |
| Temporary directories left after a gate run | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-017-AC-1 | `make clean-clone-check` passes at the head of `main`, and its report shows steps 1 to 5 passed: the locked build exits 0, and every package source is the clone, crates.io, or an anonymously readable `https://github.com/agent-ix/` git repository. | Test (TC-1397) |
| NFR-017-AC-2 | In the gate's step 5, `config_version_fixtures` writes the case directories `healthy-parent`, `violating-parent`, `dangling-parent`, `incomplete-population` and `cycle`; `requests.sh` writes `run.json` and `compile.json` in `healthy-parent`; `emit_checked_v2` exits 0 and writes package bytes that FSL's I2 reader reads back with the switch model; and the walkthrough test passes with the README's exit codes 0, 10, 20, 22 and 10. | Test (TC-1398) |
| NFR-017-AC-3 | With `REPO` a scratch repository: a commit whose `Cargo.lock` lacks a dependency `Cargo.toml` declares fails at step 3 with Cargo's `--locked` refusal; a commit whose `Cargo.toml` adds a path dependency outside the repository fails, naming it; a commit that removes `examples/config-version/requests.sh` fails at step 5; and a run whose `TMPDIR` has a `.cargo/config.toml` in an ancestor of the clone refuses at step 1. Each report names the failing step. | Test (TC-1397) |
| NFR-017-AC-4 | Run with `TMPDIR` set to a new empty directory, after a passing run and after each AC-3 failing run that directory is empty again. | Test (TC-1397) |
| NFR-017-AC-5 | `doc_snippets` finds every FSL fence of `README.md` and `docs/**/*.md`, its count equal to a second line scan's count of opening fence lines whose info string starts with `ix:native` or `fsl`, and every fence with no `reject` or `pending` passes its declared stage; a fence with no `stage=` fails the test, naming its file and line. | Test (TC-2242) |
| NFR-017-AC-6 | A fence holding a deliberate syntax error tagged `stage=parse reject=invalid_syntax/unexpected-token` passes; the same fence without `reject` fails, and with `reject=unknown_profile/unsupported-selection` fails. | Test (TC-2242) |
| NFR-017-AC-7 | A `pending=<ticket>` fence whose `<!-- expect: ... -->` comment names its current refusal passes; the same fence when it passes its stage fails the test, and so does one whose comment names a different code, or that has no comment. | Test (TC-2242) |

## Verification

TC-1397 runs the gate and its failure cases; TC-1398 checks the examples'
outputs from the gate's report. Rust tests carry `#[trace("TC-...",
"NFR-017-AC-...")]` attributes (NFR-005).

## Dependencies

- README Usage: the documented commands.
- [NFR-013](NFR-013-isolate-native-ci-feature-artifacts.md): a gate's own artifact directory.

## References

- QSL-12: the Layer 5 outside-user path.
