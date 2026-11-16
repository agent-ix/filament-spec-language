// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-017 "Documented FSL fences" (TC-2242): every FSL fence of
//! `README.md` and `docs/**/*.md` declares the stage it reaches and runs at
//! it; a `reject` fence refuses with exactly its named code and cause; a
//! `pending` fence refuses with the code its expect comment records and
//! fails the check once it passes. The fence extractor agrees with a second,
//! simpler line count. The runner here covers stage `parse`; QSL-2047 adds
//! the `check` and `compile` stages through the compile path.

use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n";

/// The documents the check reads: the README and every Markdown page
/// under `docs/`, in path order.
fn documents() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = vec![root.join("README.md")];
    let mut pending = vec![root.join("docs")];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Whether an info string opens an FSL fence.
fn is_fsl(info: &str) -> bool {
    matches!(info.split_whitespace().next(), Some("ix:native" | "fsl"))
}

/// One FSL fence: its opening line (one-based), info string, text and the
/// line after its closing marker.
struct Fence {
    line: usize,
    info: String,
    text: String,
    after: Option<String>,
}

impl Fence {
    fn tag(&self, key: &str) -> Option<&str> {
        self.info
            .split_whitespace()
            .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
    }
}

/// The FSL fences of `text`, tracking nesting by marker length so a fence
/// inside a longer Markdown-example fence is found too.
fn fences(text: &str) -> Vec<Fence> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut open: Vec<(usize, Option<usize>)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|c| *c == '`').count();
        if ticks < 3 {
            continue;
        }
        let info = trimmed[ticks..].trim();
        if info.is_empty() && open.last().is_some_and(|&(n, _)| ticks >= n) {
            if let Some((_, Some(start))) = open.pop() {
                let fence: &mut Fence = &mut found[start];
                fence.text = lines[fence.line..index].join("\n");
                fence.after = lines.get(index + 1).map(|line| (*line).to_owned());
            }
            continue;
        }
        let slot = is_fsl(info).then(|| {
            found.push(Fence {
                line: index + 1,
                info: info.to_owned(),
                text: String::new(),
                after: None,
            });
            found.len() - 1
        });
        open.push((ticks, slot));
    }
    found
}

/// The second, simpler count: opening fence lines with an FSL info string.
fn line_count(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            let ticks = trimmed.chars().take_while(|c| *c == '`').count();
            ticks >= 3 && is_fsl(trimmed[ticks..].trim())
        })
        .count()
}

/// The first refusal `fence` gives at its declared stage, as
/// `<code>/<cause>`, or `None` when it passes.
fn run_at_stage(fence: &Fence) -> Result<Option<String>, String> {
    let source = if fence.text.trim_start().starts_with("language ") {
        fence.text.clone()
    } else {
        format!("{HEADER}{}", fence.text)
    };
    let parsed = fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:doc-snippets".into(),
        },
        "fence.native",
        source.as_bytes(),
        Limits::default(),
    )
    .map_err(|error| format!("{error:?}"))?;
    if let Some(first) = parsed.diagnostics().first() {
        return Ok(Some(format!(
            "{}/{}",
            first.code.as_str(),
            first.cause.as_str()
        )));
    }
    match fence.tag("stage") {
        Some("parse") => Ok(None),
        Some(stage @ ("check" | "compile")) => {
            Err(format!("the {stage} stage runner is not built yet"))
        }
        other => Err(format!("no declared stage: {other:?}")),
    }
}

/// The verdict of the check over one fence: `Ok` when it behaves as its
/// declaration says, `Err` naming why not.
fn check(fence: &Fence) -> Result<(), String> {
    let outcome = run_at_stage(fence)?;
    match (fence.tag("reject"), fence.tag("pending"), outcome) {
        (Some(_), Some(_), _) => Err("both reject and pending".into()),
        (None, None, None) => Ok(()),
        (None, None, Some(refusal)) => Err(format!("refused {refusal}")),
        (Some(expected), None, Some(refusal)) if refusal == expected => Ok(()),
        (Some(expected), None, other) => Err(format!("expected {expected}, got {other:?}")),
        (None, Some(_), None) => Err("a pending fence passes its stage".into()),
        (None, Some(_), Some(refusal)) => {
            let expected = fence
                .after
                .as_deref()
                .and_then(|line| line.trim().strip_prefix("<!-- expect: "))
                .and_then(|rest| rest.strip_suffix(" -->"))
                .ok_or("a pending fence has no expect comment")?;
            if refusal == expected {
                Ok(())
            } else {
                Err(format!("expected {expected}, got {refusal}"))
            }
        }
    }
}

/// NFR-017-AC-5: the two counts agree, every fence declares one stage, and
/// every fence behaves as declared at that stage.
#[trace("TC-2242", "NFR-017-AC-5")]
#[test]
#[ignore = "QSL-2047: the README and docs fences do not declare their stages yet, and the check and compile stages have no runner"]
fn every_fsl_fence_runs_at_its_declared_stage() {
    let mut total = 0;
    for file in documents() {
        let text = std::fs::read_to_string(&file).expect("a readable document");
        let found = fences(&text);
        assert_eq!(found.len(), line_count(&text), "{}", file.display());
        for fence in &found {
            total += 1;
            check(fence).unwrap_or_else(|why| panic!("{}:{}: {why}", file.display(), fence.line));
        }
    }
    assert!(total > 0, "the README holds FSL fences");
}

/// NFR-017-AC-5: an FSL excerpt in a fence with no FSL info string is a
/// miss: a bare fence whose first line opens an FSL declaration fails.
#[trace("TC-2242", "NFR-017-AC-5")]
#[test]
#[ignore = "QSL-2047: the README's FSL excerpts are in fences with no info string"]
fn fence_extraction_finds_every_fsl_excerpt() {
    const KEYWORDS: [&str; 6] = [
        "type ",
        "invariant ",
        "temporal ",
        "protocol ",
        "function ",
        "language ",
    ];
    let mut bare = Vec::new();
    for file in documents() {
        let text = std::fs::read_to_string(&file).expect("a readable document");
        let lines: Vec<&str> = text.lines().collect();
        let mut inside = false;
        for (index, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("```") {
                if !inside
                    && line.trim() == "```"
                    && lines.get(index + 1).is_some_and(|next| {
                        KEYWORDS.iter().any(|k| next.trim_start().starts_with(k))
                    })
                {
                    bare.push(format!("{}:{}", file.display(), index + 1));
                }
                inside = !inside;
            }
        }
    }
    assert!(
        bare.is_empty(),
        "FSL in fences with no FSL info string: {bare:?}"
    );
}

fn one(markdown: &str) -> Fence {
    let mut found = fences(markdown);
    assert_eq!(found.len(), 1, "{markdown}");
    found.remove(0)
}

/// NFR-017-AC-6: a deliberate syntax error passes only under its exact
/// `reject` code and cause.
#[trace("TC-2242", "NFR-017-AC-6")]
#[test]
#[ignore = "QSL-2047: the doc fence check is not built yet"]
fn reject_fences_refuse_with_the_named_code() {
    let body = "type Digit = Int[0, ;\n";
    let tagged = |info: &str| one(&format!("```{info}\n{body}```\n"));
    assert_eq!(
        check(&tagged(
            "fsl stage=parse reject=invalid_syntax/unexpected-token"
        )),
        Ok(())
    );
    assert!(check(&tagged("fsl stage=parse")).is_err());
    assert!(check(&tagged(
        "fsl stage=parse reject=unknown_profile/unsupported-selection"
    ))
    .is_err());
    assert!(check(&one("```fsl\ntype Digit = Int[0, 9];\n```\n")).is_err());
}

/// NFR-017-AC-7: a pending fence passes while it refuses as its expect
/// comment records, and fails once it passes, names another code, or has no
/// comment.
#[trace("TC-2242", "NFR-017-AC-7")]
#[test]
#[ignore = "QSL-2047: the doc fence check is not built yet"]
fn pending_fences_match_their_current_refusal() {
    let fence = |body: &str, after: &str| {
        one(&format!(
            "```fsl stage=parse pending=QSL-0\n{body}```\n{after}\n"
        ))
    };
    let broken = "type Digit = Int[0, ;\n";
    assert_eq!(
        check(&fence(
            broken,
            "<!-- expect: invalid_syntax/unexpected-token -->"
        )),
        Ok(())
    );
    assert!(check(&fence(
        "type Digit = Int[0, 9];\n",
        "<!-- expect: invalid_syntax/unexpected-token -->"
    ))
    .is_err());
    assert!(check(&fence(
        broken,
        "<!-- expect: unknown_profile/unsupported-selection -->"
    ))
    .is_err());
    assert!(check(&fence(broken, "")).is_err());
}
