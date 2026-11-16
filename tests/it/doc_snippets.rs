// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-017 "Documented FSL fences" (TC-2242): every FSL fence of
//! `README.md` and `docs/**/*.md` declares the stage it reaches, and the
//! fence extractor agrees with a second, simpler line count. These stubs
//! hold the declaration rules; the stage runner lands with the doc fix.

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

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

/// One FSL fence: its file, opening line (one-based) and info string.
struct Fence {
    file: PathBuf,
    line: usize,
    info: String,
}

/// The FSL fences of `text`, tracking fence nesting by marker length so a
/// fence inside a longer Markdown-example fence is found too.
fn fences(file: &Path, text: &str) -> Vec<Fence> {
    let mut found = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let ticks = trimmed.chars().take_while(|c| *c == '`').count();
        if ticks < 3 {
            continue;
        }
        let info = trimmed[ticks..].trim();
        if info.is_empty() && open.last().is_some_and(|&n| ticks >= n) {
            open.pop();
            continue;
        }
        open.push(ticks);
        if is_fsl(info) {
            found.push(Fence {
                file: file.to_owned(),
                line: index + 1,
                info: info.to_owned(),
            });
        }
    }
    found
}

fn all_fences() -> Vec<Fence> {
    documents()
        .iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(file).expect("a readable document");
            fences(file, &text)
        })
        .collect()
}

/// NFR-017-AC-5: every FSL fence declares `stage=parse`, `stage=check` or
/// `stage=compile`, and none carries both `reject=` and `pending=`.
#[trace("TC-2242", "NFR-017-AC-5")]
#[trace("TC-1398", "NFR-017-AC-2")]
#[test]
#[ignore = "QSL-2047: the README and docs fences do not declare their stages yet"]
fn every_fsl_fence_declares_its_stage() {
    let fences = all_fences();
    assert!(!fences.is_empty(), "the README holds FSL fences");
    for fence in &fences {
        let words: Vec<&str> = fence.info.split_whitespace().collect();
        let stages = words
            .iter()
            .filter(|word| matches!(**word, "stage=parse" | "stage=check" | "stage=compile"))
            .count();
        assert_eq!(
            stages,
            1,
            "{}:{} declares no single stage: {}",
            fence.file.display(),
            fence.line,
            fence.info
        );
        let reject = words.iter().any(|word| word.starts_with("reject="));
        let pending = words.iter().any(|word| word.starts_with("pending="));
        assert!(
            !(reject && pending),
            "{}:{}",
            fence.file.display(),
            fence.line
        );
    }
}

/// NFR-017-AC-5: the README's FSL excerpts are FSL fences, so the
/// extractor and a second count of every fence whose body starts with an
/// FSL declaration keyword agree; a bare fence holding FSL is a miss.
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
        for (index, line) in lines.iter().enumerate() {
            if line.trim() == "```"
                && lines
                    .get(index + 1)
                    .is_some_and(|next| KEYWORDS.iter().any(|k| next.trim_start().starts_with(k)))
            {
                bare.push(format!("{}:{}", file.display(), index + 1));
            }
        }
    }
    assert!(bare.is_empty(), "FSL in fences with no FSL info string: {bare:?}");
}
