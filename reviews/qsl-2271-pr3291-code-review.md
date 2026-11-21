---
id: SR-20261121003291001
title: code-review review of dependency artifact handoff
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language; spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md,
  spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md,
  spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md, spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md,
  spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
review_set: subset
---

## Summary

Frozen 515cb5ec8514357124c4a83dc98311f502cd1a8a; base c64dc297; PR filament-spec-language#3291; ticket QSL-2271. Five-file specification amendment only. No Cargo, runtime tests, implementation coverage, matrix census or full-repository gap audit was run. No applicable AssuranceProfile exists (no frontmatter type: AssuranceProfile under spec/). The dispatch authorizes the subset below and the runtime dependency manifest itself; no repository file-tracking manifest, duplicated graph, digest or pin was added. Approval here is specification review only; implementation follows this spec merge and pending emitter PR #3283.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-517-AC-1
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '`write(&request_for_R, dir)` creates exactly the paths of the Outputs
    table, `inventory.json` included (with no `mutations/` for `corpus = None`), and
    `checked-package.json` equals the bytes `emit_checked_v2::emit` returns for the
    same request.'
- id: FR-517-AC-2
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: The package read from `checked-package.json` with `read_checked_package`
    and evidence assembled from the written files is admitted, and `Selection::check`
    passes; the sidecar's clause ids, names, requirement entries, activation forms,
    profile selections, definition authority and `TemporalBinding` rows, those of
    clauses and those of compensation templates, equal the values read from the checked
    unit held in memory by the writer.
- id: FR-517-AC-27
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: A root importing `test/z`, which imports `test/a`, writes the manifest
    vector of "The dependency artifact closure" and exactly one artifact/witness triple
    in each indexed directory. Each triple is admitted with that package's own direct
    dependencies and witnesses, and the root is admitted with the retained closure.
    Reordering the caller's library provisions does not change any written byte. A
    unit with no imports writes the required manifest with an empty list and no dependency
    directory.
- id: FR-517-AC-28
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: For a diamond whose root imports `test/left` and `test/right`, both importing
    `test/shared`, the manifest contains three unique rows ordered `test/left`, `test/right`,
    `test/shared`, with directories `dependencies/0`, `dependencies/1`, `dependencies/2`;
    the shared package and its witnesses occur once. Two writes give equal trees.
    A conflicting `test/shared` selection is refused under the existing dependency
    conflict rule and leaves no output directory, never a tree containing one arbitrarily
    chosen shared artifact.
- id: FR-517-dependency-statement-1
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: ' The writer SHALL persist the unique recursive closure of the root''s
    selected library packages from the same checked acquisition as the root, including
    a library reached by more than one import path only once. Each library''s artifact
    SHALL be its checked-package emission, and its `identities.json` and `equality.json`
    SHALL be derived from that library''s own final emission by the same rules as
    the root''s witness files. A file with no equality applications still has the
    three empty equality lists. The writer SHALL NOT substitute library source for
    these artifacts or recompile a library to recove'
- id: FR-517-dependency-statement-2
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '`dependencies.json` SHALL be UTF-8 JSON, pretty-printed with two-space
    indent, members in the order below, ending in one newline. Equal checked inputs
    SHALL produce byte-identical manifests and dependency trees. Its top-level object
    and every row SHALL be closed: missing, repeated and unknown members are refused.'
- id: FR-517-dependency-statement-4
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'Each row SHALL contain exactly `identity` and `directory`, in that order.
    `identity` SHALL be the non-empty selected library identity used by `lock.dependency_selections`,
    with no normalization, revision or version. For the row at zero-based position
    `n`, `directory` SHALL be exactly `dependencies/<n>`, where `<n>` is the unsigned
    decimal position with no leading zero except `0`. It is relative to the handoff
    directory; an identity is never used as a filesystem path. The only artifact paths
    in that directory are `checked-package.json`, `identities.json` and `equality.json`.
    The manifest SHALL '
- id: FR-517-dependency-statement-7
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'This is manifest order, not admission order. A root with no library selections
    SHALL write the same object with `"dependencies": []` and no `dependencies/` subdirectories.
    A missing manifest is not an empty closure and SHALL be refused; there is no source
    or legacy-format fallback. A repeated identity, a second artifact for one selected
    identity, an artifact not reachable from the root''s lock, or an omitted reachable
    identity SHALL NOT be admitted. A diamond whose branches select the same admitted
    library shares its one row; branches requiring conflicting admitted content under
    one identity SH'
- id: FR-517-dependency-statement-8
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'A bytes-only consumer SHALL load the closure and witnesses under [FR-850](FR-850-analyze-a-selected-checked-package-from-its-bytes.md)
    "Dependency artifact admission". The output directory''s paths are confined relative
    paths: no absolute path, `.` or `..` component, backslash, symbolic link or non-regular
    artifact file is admitted. The writer''s existing atomic write and no-partial-directory
    rules apply to the complete dependency tree.'
- id: FR-850-AC-12
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: A compiled root calls `test/z::answer`, which calls `test/a::answer` returning
    Integer `42`. Running the root function from source and from its FR-517 directory
    returns `42` on both routes. Its Counter temporal claim `always holds(c.value
    <= z::answer() - 40)` is analyzed with the same models/documents/items on both
    routes and gives equal item records, counterexamples and reproduced replays except
    the request selection. Deleting all unit/library source files and the handoff's
    `sources/` before the package run and analyze/replay changes neither result. Removing
    `dependencies.json` refuses `inva
- id: FR-850-AC-13
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: FR-517-AC-28's diamond is admitted once per unique library and settles
    the source route's result. A missing transitive artifact refuses before an item
    runs; a manifest entry for a foreign or conflicting library refuses `conflicting-definition`;
    an unreachable row, a duplicate identity, reversed rows, an unknown row member,
    a nonindexed path, `../escape`, an absolute path and a selected symlink/non-regular
    file each refuse by the dependency admission table at their own file or pointer.
    A lock cycle refuses `definition-cycle`; none reaches source or an engine.
- id: FR-850-AC-14
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: For AC-12's transitive handoff, removal of either dependency's `identities.json`
    or `equality.json` refuses `missing-file` naming that indexed file even when the
    equality lists would be empty. Swapping non-empty witness files between distinct
    admitted libraries refuses their original identity/equality join error at the
    foreign file, before any item runs; no dependency receives the root's witness.
    The unchanged handoff completes.
- id: FR-850-AC-15
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: For AC-12's handoff, compute `B` as the bytes of exactly the root, manifest,
    unique dependency package/witness files and selected model/runtime files read.
    At `dependent_bytes = B` intake completes; at `B - 1` it settles `incomplete`,
    `resource_exhausted`, exit `22`, naming `dependent_bytes` and its configured bound.
    A handoff whose manifest expands the selected-file total from 64 to 65 stops at
    the same phase naming the shared file limit before opening file 65. A diamond
    charges its shared triple once. For strict-read/native-join work and storage,
    an allowance equal to the completed closure p
- id: FR-850-AC-16
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: A library in AC-12's closure selects domain document `test/library-domain`,
    which the root does not select, and the request supplies both that document and
    the root's domain documents. Admission binds each document to the selecting package
    and completes without an extraneous-model refusal. Removing the library-only document
    refuses `missing_import`/`missing-selection` naming its identity; replacing it
    by content the library selection refuses preserves that reader refusal at the
    dependency package. No witness from the root is substituted.
- id: FR-850-dependency-statement-1
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: ' The directory loader shared by `native-analyze/1`, `native-run/1` directory
    selections ([FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md))
    and their bytes-only counterexample replays SHALL consume FR-517''s dependency
    manifest and artifact closure. This changes no `{file}` or `{file, equality}`
    request shape and adds no request-level dependency member. A bare-file selection
    with no supplied dependency evidence SHALL keep its existing strict-reader `MissingImport`
    refusal; it SHALL NOT search neighboring handoff files, sources or artifact directories
    to repair th'
- id: FR-850-dependency-statement-2
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- The loader SHALL require `dependencies.json`, even when the root selects   no
    library. It SHALL parse the closed `filament.checked-handoff.dependencies/1`   format
    of FR-517 before admitting any item. Rows SHALL have unique identities   in the
    specified order and their exact indexed directory. No additional   path, edge,
    source, digest or version member is admitted. - The loader SHALL admit each library
    through the existing strict checked-package   reader with its selected domain
    documents and already admitted direct library   dependencies. It SHALL perform
    admission bottom-up, with independ'
- id: FR-850-dependency-statement-3
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: Every manifest and artifact path SHALL be resolved beneath the selected
    handoff using FR-517's confined-file rules. The loader SHALL reject absolute paths,
    noncanonical indexed paths, `.` or `..` components, backslashes, symbolic links
    at any component and non-regular artifact files before reading their content.
    It SHALL NOT traverse an unlisted dependency directory to discover artifacts.
- id: FR-850-dependency-statement-4
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'The following failures SHALL refuse the whole request before any item
    or engine runs, with category refusal, exit `20`, `items` empty and `last_stage`
    `null`:'
- id: FR-850-dependency-statement-5
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '| Failure | Code / cause | Retained location | | --- | --- | --- | | Missing
    manifest or selected dependency package/witness file | `invalid_package` / `missing-file`
    | exact relative file path | | Non-JSON manifest, missing/unknown/repeated member
    or wrong member kind | `invalid_package` / `malformed-file` | `dependencies.json`
    and first defective JSON pointer | | Another manifest `format` | `invalid_package`
    / `unknown-format` | `dependencies.json`, `/format`, and the supplied string |
    | Repeated identity, wrong order, noncanonical/unsafe path or unreachable manifest
    row | `invalid_package` '
- id: FR-850-dependency-statement-6
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'The loader SHALL use one invocation-wide intake budget: every root, manifest,
    dependency package and witness file read counts once by its full size toward `dependent_bytes`,
    in addition to request-selected model and runtime documents. It SHALL count every
    such selected file once toward FR-026''s shared 64-selected-file ceiling, including
    the manifest and required dependency witness files; a diamond''s shared triple
    is counted once. It SHALL check a prospective file count before opening the next
    file and cap each read by the remaining byte allowance. These intake stops retain
    FR-026''s `resource_e'
- id: FR-850-dependency-statement-7
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: All strict package reads and native joins for the root and closure SHALL
    consume one finite remaining work allowance for the invocation, without resetting
    it per dependency. Native identity joins SHALL share the `NativeJoinBudget` and
    logical storage allowance of `i2.work_units` and `i2.native_join_storage` (FR-531,
    FR-255). Strict-reader work is charged once by its existing read schedule and
    native-join work once by its join schedule; this rule adds no duplicate charge
    for an operation already accounted by that schedule. A denied charge retains its
    original setting, bound, attempted count and
- id: TC-1312-procedure-5-27
  path: spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '| 27 | 27 | Write an import-free unit, then a root importing `test/z`
    which imports `test/a`; write again with reversed library provision order | required
    empty manifest first; then the FR-517 manifest vector, two indexed artifact/witness
    triples, strict admission with each own supply, byte-identical trees across provision
    order |'
- id: TC-1312-procedure-5-28
  path: spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '| 28 | 28 | Write a root importing `test/left` and `test/right`, each
    importing `test/shared`; then supply conflicting shared content | three sorted
    unique manifest rows, the shared triple once, each dependency admitted before
    its importer; conflict refuses and leaves no directory |'
- id: TC-2410-procedure-5-6
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 6. Compile a root importing `test/z` as `z`, whose `answer` calls `test/a::answer`,
    returning Integer `42`. The root's function returns `z::answer()` and its Counter
    temporal claim is `always holds(c.value <= z::answer() - 40)`. Run its function
    and analyze its claim from source and from the written handoff. Remove source
    files and `sources/`, then repeat package execution and counterexample replay.
- id: TC-2410-procedure-5-7
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 7. Compile a diamond sharing `test/shared`. Check its one shared artifact
    triple; remove a transitive artifact, supply a foreign/conflicting artifact, and
    mutate manifest order, uniqueness, closure membership, member shape and canonical
    paths. Exercise a symlink, non-regular file and lock cycle.
- id: TC-2410-procedure-5-8
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 8. Remove each dependency witness in turn, including an empty equality
    file, then swap non-empty witnesses between distinct packages; retain the unchanged
    directory as a positive control.
- id: TC-2410-procedure-5-9
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 9. Measure the total files/bytes of the selected root and closure once,
    then run at the exact byte allowance and one below; test totals 64 and 65 files
    and exact/one-short strict-read/native-join work/storage allowances.
- id: TC-2410-procedure-5-10
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 10. Add a domain selection used only by a library in that closure, supplying
    its matching document beside the root's documents; repeat with that document missing
    and with content the dependency's recorded selection refuses.
- id: TC-2410-procedure-7-6
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 6: function value `42` on both routes and equal source/package
    item records/counterexamples/reproduced replays, including after source deletion;
    no source compile or source provision is consulted.'
- id: TC-2410-procedure-7-7
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 7: the diamond''s shared triple is read/admitted once; every invalid
    variant refuses before any item runs with the exact dependency-admission code/cause
    and its own file/pointer. A missing manifest refuses even for an import-free unit.'
- id: TC-2410-procedure-7-8
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 8: missing indexed witness files refuse `missing-file`; foreign
    witness joins retain the original refusal at the dependency''s file, never joined
    against the root; the positive control completes.'
- id: TC-2410-procedure-7-9
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 9: exact allowances complete; one-short bytes/file-count stops
    are intake `incomplete`/`resource_exhausted`, exit 22; read/join stops retain
    the original `i2.*` setting and typed refusal, with no per-dependency budget reset.
    Shared diamond files count once.'
- id: TC-2410-procedure-7-10
  path: spec/test-cases/TC-2410-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- Step 10: the library-only domain document is admitted against that library;
    its removal refuses missing selection, and changed content retains the dependency
    reader''s refusal, never an extraneous-root-model refusal or a substituted witness.'
- id: FR-1039-amended-context-6
  path: spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md
  role: examined
  excerpt: '- `package`: exactly one of `{"file": P}` or `{"file": P, "equality":
    E}`,   `P` the path of a checked-package v2 file (FR-945) and `E` the path of   that
    package''s equality witness file (an FR-531 equality file, the   `equality.json`
    an FR-517 handoff writes), or `{"directory": D}`, the   path of an FR-517 handoff
    directory holding `checked-package.json`,   `identities.json`, when present `equality.json`,
    and the required   `dependencies.json` with its package/witness closure (FR-517;
    FR-850   "Dependency artifact admission"), each path   resolved against the request
    file''s directory, each fi'
- id: FR-1039-amended-context-10
  path: spec/functional/FR-1039-run-every-native-run-member-over-a-selected-checked-package.md
  role: examined
  excerpt: '- The request reader SHALL refuse, before it opens any selected file,
    a   request carrying neither `program` nor `package` (`invalid-request` naming   `program`),
    both (naming `program`), or `package` beside `libraries`,   `refinement_internals`
    or `integer_division` (naming that member), each at   stage `request`, exit 20.
    - FSL SHALL take the edition, profiles, dependency selections and laws from   the
    admitted package''s `lock`, so a call evaluates `div` and `rem` under   the law
    the package recorded (FR-767), and SHALL resolve every name a   member selects
    only through the package''s name ro'
- id: FR-517-amended-context-12
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '| Path | Content | | --- | --- | | `checked-package.json` | the exact
    bytes `emit_checked_v2::emit` returns | | `dependencies.json` | the required dependency
    artifact manifest, `filament.checked-handoff.dependencies/1`, as "The dependency
    artifact closure" states; present with an empty list when the root imports no
    library | | `dependencies/<n>/checked-package.json`, `dependencies/<n>/identities.json`,
    `dependencies/<n>/equality.json` | one checked package and its own FR-531 witness
    files per manifest row; no source is needed to admit these artifacts | | `sources/<label>`
    | the unit and librar'
- id: FR-517-amended-context-21
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '### The dependency artifact closure'
- id: FR-517-amended-context-22
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: The writer SHALL persist the unique recursive closure of the root's selected
    library packages from the same checked acquisition as the root, including a library
    reached by more than one import path only once. Each library's artifact SHALL
    be its checked-package emission, and its `identities.json` and `equality.json`
    SHALL be derived from that library's own final emission by the same rules as the
    root's witness files. A file with no equality applications still has the three
    empty equality lists. The writer SHALL NOT substitute library source for these
    artifacts or recompile a library to recover
- id: FR-517-amended-context-24
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '| Member | Content | | --- | --- | | `format` | the string `filament.checked-handoff.dependencies/1`
    | | `dependencies` | an array of the entire unique recursive library artifact
    closure, ordered lexicographically by the UTF-8 bytes of each selected library
    identity |'
- id: FR-850-amended-context-2
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: A `native-analyze/1` request MAY select an existing `filament.checked-package/v2`
    artifact through its `package` member instead of a `program` source (FSpec FR-520).
    The selection names the FR-517 handoff directory that holds the package. When
    it does, FSL SHALL admit the directory's `checked-package.json` with the strict
    checked-package reader, attach the identity rows of its `identities.json` and,
    when present, the equality rows of its `equality.json`, admit the dependency artifact
    closure of `dependencies.json` with each package's own witness files, and build
    from those files alone the anal
- id: FR-850-amended-context-4
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'A caller who already holds a checked package, such as a lane that receives
    a handoff directory or a CI step that compiled once and analyzes many times, has
    the package and may not have the source. Recompiling a provisioned source and
    comparing bytes would make the source mandatory, and would make the package selection
    a second spelling of the program selection. The root v2 package carries its own
    analysis facts: the package identity, the formula graph, the total source map,
    each named declaration''s `declaration.qualified_name`, the `temporal_clause_names`
    rows, the profile and clock rows, the '
- id: FR-850-amended-context-6
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: 'The package route''s reader projection (`BarePackage::world()`, the bare
    walker and observation admission) needs the identity witness: the selected declaration
    and field identity rows and the final source rows that join the package''s nodes
    to their exact identities. FR-531 makes the package bytes carry no such rows and
    admits them only from the handoff''s `identities.json` (and the equality rows
    only from its `equality.json`), by no other route. A selection of the package
    file alone therefore cannot build the projection, and the package forms of the
    source-checked evaluators (FR-940) could not r'
- id: FR-850-amended-context-10
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- The request''s `package` member `{directory}`: the path of an FR-517   handoff
    directory, resolved against the request document''s directory. The   analysis
    reads from it `checked-package.json`, `identities.json`, when   present `equality.json`,
    the required `dependencies.json`, and the package   and witness triple of each
    manifest row (FR-517 "The dependency artifact   closure"). It reads no source,
    selection sidecar or other handoff file.   Each file read is counted toward aggregate
    `dependent_bytes` (FR-026). - The request''s `models`: the domain package documents
    the root or any   dependenc'
- id: FR-850-amended-context-15
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: The directory loader shared by `native-analyze/1`, `native-run/1` directory
    selections ([FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md))
    and their bytes-only counterexample replays SHALL consume FR-517's dependency
    manifest and artifact closure. This changes no `{file}` or `{file, equality}`
    request shape and adds no request-level dependency member. A bare-file selection
    with no supplied dependency evidence SHALL keep its existing strict-reader `MissingImport`
    refusal; it SHALL NOT search neighboring handoff files, sources or artifact directories
    to repair the
- id: FR-850-amended-context-26
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '  The equality rows are the producer''s witness for every `=`, `==` and
    `!=`   application (FR-533 "Equality slots"). That includes the `=` of a pre-
    or   postcondition, which the package form of a state clause reads into the   model-check
    input (FR-940 "Package forms of the source-checked   evaluators"). The bare route
    derives no witness from the package and   guesses no decision, so without the
    file it cannot build the state form   of any operation whose state clauses hold
    an equality. Settling the items   anyway would give a verdict the source route
    does not give. The source   route always h'
- id: FR-850-amended-context-28
  path: spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md
  role: examined
  excerpt: '- FSL SHALL build the analysis input from the admitted package value as   FR-940
    states: a `BarePackage` (FR-531) over the read `CheckedPackageV2`,   with the
    supplied domain packages bound to its model selections and the   handoff''s own
    identity/equality rows attached, retaining each dependency''s   admitted package
    and its separate supply ("Dependency artifact admission"   and "Admission"), read
    by the   engines through the sealed `AnalysisView`. It is never a `CheckedPackage`:   FR-087
    R-10 keeps checked typestate to S3 and S4. The source route reads   each fact
    of FR-531''s table through the'
```

## Verdict

PASS: no source changes or independent code-review defects.

## Disposition review round 1

Reviewed the two requirement/test-case fixes from 515cb5ec8514357124c4a83dc98311f502cd1a8a to db5f47ed1da1b100c15d011434ba101ce35d9c66. Original findings and original verdict remain unchanged above. Both original findings are fixed; no regression or additional finding was identified in the bounded fix diff. Supplemental AC-13 specifies raw repeated JSON keys, malformed/missing/unknown/wrong-kind fields, unknown format, omitted selection versus missing selected file, and exact pointers. Supplemental AC-15 distinguishes aggregate transitive replay bytes from initial intake, tests root-fitting P versus exact closure total R, preserves item-local typed refusal and a proved sibling, and keeps source controls before source deletion. TC-2410 explicitly executes these cases.

Current specification verdict: PASS; no open review findings. Merge remains held for the QSL-2717 producer handoff under the lead's architecture priority. This review does not release that hold or claim implementation/test completion. No Cargo or runtime tests ran.

No original findings required dispositions for this method. No new findings.
