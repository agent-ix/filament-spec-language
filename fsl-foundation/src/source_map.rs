// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-004: checked body-to-document byte correspondence; no Markdown or wire decoding.
use crate::source::provenance::SourceRegion;
use crate::source::{LocatedSpan, Source, SourceIdentity, Span};
use alloc::{boxed::Box, string::String, vec, vec::Vec};

/// The one language tag a mapped (I3 extracted) body may declare, decoded
/// once from the caller's fence tag (ADR-012 section 9 source-lexing edge).
/// Both native joins of an extracted body -- the root crate's
/// `mapped::compile` and `fsl_replay::spine::run_clause` (FR-109) -- decode
/// the tag here, so they refuse the same fences.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeLanguage;

impl NativeLanguage {
    /// `Some` exactly for the tag `ix:native`.
    #[ix_trace_rs::string_edge]
    pub fn of(tag: &str) -> Option<Self> {
        (tag == "ix:native").then_some(Self)
    }
}

/// Hard ceiling on selected correspondence segments.
pub const MAX_SEGMENTS: usize = 50_000;

/// Equal byte regions in the extracted body and its original document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Segment {
    /// Half-open region of the extracted body.
    pub body: Span,
    /// Corresponding half-open region of the original document.
    pub original: Span,
}

/// Maximal body ranges whose corresponding original bytes are contiguous.
#[derive(Clone, Copy, Debug)]
struct RegionGroup {
    body: Span,
    original: Span,
}

/// Only explicitly enabled layout deletions are accepted. No inserted or rewritten bytes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Layout {
    /// Permit deletion of leading spaces/tabs at original line boundaries.
    pub strip_indentation: bool,
    /// Permit deletion of CR immediately before a retained LF.
    pub normalize_crlf: bool,
    /// Permit deletion of the selected region's final newline.
    pub drop_final_newline: bool,
}

/// Verified immutable correspondence bound to exact original and body sources.
#[derive(Clone, Debug)]
pub struct SourceMap {
    original: Source,
    body: Source,
    region: Span,
    segments: Vec<Segment>,
    region_groups: Vec<RegionGroup>,
    layout: Layout,
    #[cfg(test)]
    region_lookup_steps: std::cell::Cell<usize>,
    #[cfg(test)]
    region_index_build_steps: usize,
}

/// Cause-specific location and message for a [`SourceMap::verify`] or
/// [`SourceMap::map_span`] refusal, before `diagnostic` (later in this
/// layer's order) maps it onto a stable `Code`. `source_map` does not
/// construct a `Diagnostic` (ADR-011 §6.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceMapError {
    pub(crate) cause: SourceMapErrorCause,
    pub(crate) source: SourceIdentity,
    pub(crate) path: String,
    pub(crate) span: LocatedSpan,
    pub(crate) message: String,
}

/// Why a [`SourceMap::verify`] or [`SourceMap::map_span`] refused its input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceMapErrorCause {
    /// The selected segment count exceeds the caller's budget.
    SegmentBudget,
    /// Anything else: an invalid region, segment, correspondence or coverage.
    InvalidMap,
}

impl SourceMap {
    /// [`SourceMap::verify`] with the refusal's typed cause retained, before
    /// `diagnostic` maps it onto a stable code (`SourceMap::verify` itself is
    /// implemented there; see `diagnostic.rs`).
    pub(crate) fn verify_typed(
        original: Source,
        body: Source,
        region: Span,
        segments: Vec<Segment>,
        layout: Layout,
        segment_limit: usize,
    ) -> Result<Self, Box<SourceMapError>> {
        let fail = |cause, message: &str| {
            Box::new(SourceMapError {
                cause,
                source: original.identity().clone(),
                path: original.path().into(),
                span: original
                    .locate(Span { start: 0, end: 0 })
                    .expect("internal offsets are UTF-8 boundaries"),
                message: message.into(),
            })
        };
        if segments.len() > segment_limit.min(MAX_SEGMENTS) {
            return Err(fail(
                SourceMapErrorCause::SegmentBudget,
                "source-map segment budget exhausted",
            ));
        }
        if original.slice(region).is_none() {
            return Err(fail(
                SourceMapErrorCause::InvalidMap,
                "invalid original source region",
            ));
        }
        if original.identity() == body.identity() && original.text() != body.text() {
            return Err(fail(
                SourceMapErrorCause::InvalidMap,
                "original and extracted body reuse one set of source labels for different bytes",
            ));
        }
        let mut body_cursor = 0;
        let mut original_cursor = region.start;
        for segment in &segments {
            let Some(body_bytes) = body.slice(segment.body) else {
                return Err(fail(
                    SourceMapErrorCause::InvalidMap,
                    "invalid body segment",
                ));
            };
            let Some(original_bytes) = original.slice(segment.original) else {
                return Err(fail(
                    SourceMapErrorCause::InvalidMap,
                    "invalid original segment",
                ));
            };
            if body_bytes.is_empty()
                || segment.body.start != body_cursor
                || segment.original.start < original_cursor
                || segment.original.end > region.end
            {
                return Err(fail(
                    SourceMapErrorCause::InvalidMap,
                    "segments must cover the body once in original order inside the selected region",
                ));
            }
            if body_bytes != original_bytes {
                return Err(fail(SourceMapErrorCause::InvalidMap, "mapped bytes differ"));
            }
            if !allowed_gap(
                &original,
                Span {
                    start: original_cursor,
                    end: segment.original.start,
                },
                region.end,
                false,
                layout,
            ) {
                return Err(fail(
                    SourceMapErrorCause::InvalidMap,
                    "mapping discards bytes outside the selected layout policy",
                ));
            }
            body_cursor = segment.body.end;
            original_cursor = segment.original.end;
        }
        if body_cursor != body.text().len()
            || !allowed_gap(
                &original,
                Span {
                    start: original_cursor,
                    end: region.end,
                },
                region.end,
                true,
                layout,
            )
        {
            return Err(fail(
                SourceMapErrorCause::InvalidMap,
                "mapping omits body bytes or discards an unapproved source suffix",
            ));
        }
        let mut region_groups: Vec<RegionGroup> = Vec::new();
        #[cfg(test)]
        let mut region_index_build_steps = 0;
        for segment in &segments {
            #[cfg(test)]
            {
                region_index_build_steps += 1;
            }
            if let Some(previous) = region_groups
                .last_mut()
                .filter(|group| group.original.end == segment.original.start)
            {
                previous.body.end = segment.body.end;
                previous.original.end = segment.original.end;
            } else {
                region_groups.push(RegionGroup {
                    body: segment.body,
                    original: segment.original,
                });
            }
        }
        Ok(Self {
            original,
            body,
            region,
            segments,
            region_groups,
            layout,
            #[cfg(test)]
            region_lookup_steps: std::cell::Cell::new(0),
            #[cfg(test)]
            region_index_build_steps,
        })
    }
    /// Original source whose identity and bytes were checked at construction.
    pub fn original(&self) -> &Source {
        &self.original
    }
    /// Extracted source whose full content the segments cover.
    pub fn body(&self) -> &Source {
        &self.body
    }
    /// Selected original region enclosing the correspondence.
    pub fn region(&self) -> Span {
        self.region
    }
    /// Verified segments in body/original order.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    /// Explicit layout deletions admitted by this map.
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Map a span from the exact body source. Discontiguous regions stay separate;
    /// concatenating their original bytes reproduces the body region exactly.
    /// A zero-width boundary selects the following segment, except EOF uses the last end.
    pub(crate) fn map_span_typed(
        &self,
        source: &Source,
        span: Span,
    ) -> Result<Vec<LocatedSpan>, Box<SourceMapError>> {
        Ok(self
            .map_bytes(source, span)?
            .into_iter()
            .map(|region| {
                self.original
                    .locate(region)
                    .expect("verified map subregion")
            })
            .collect())
    }

    /// ADR-013 C-21: an embedded-body span to its document regions, each
    /// under the document's own `RawSourceRef` (FR-001), in document order.
    /// Discontiguous regions stay separate, exactly as [`SourceMap::map_span`]
    /// maps them; a zero-width body span maps to one empty region. Refuses a
    /// span of another source, and one that is not a body span, with
    /// `None`.
    pub fn map_regions(&self, source: &Source, span: Span) -> Option<Vec<SourceRegion>> {
        self.map_bytes(source, span)
            .ok()?
            .into_iter()
            .map(|region| self.original.region(region))
            .collect()
    }

    /// Maps a body span only when it corresponds to one contiguous original
    /// region. This has the same result as filtering [`Self::map_regions`]
    /// to exactly one region, without materializing every region for a
    /// discontiguous span.
    pub fn map_region(&self, source: &Source, span: Span) -> Option<SourceRegion> {
        if !self.matches_body(source) || self.body.slice(span).is_none() {
            return None;
        }

        if span.start == span.end {
            let first = self.region_groups.partition_point(|group| {
                self.record_region_lookup_step();
                group.body.end <= span.start
            });
            let byte = self.region_groups.get(first).map_or_else(
                || {
                    self.region_groups
                        .last()
                        .map_or(self.region.start, |group| group.original.end)
                },
                |group| group.original.start + span.start - group.body.start,
            );
            return self.original.region(Span {
                start: byte,
                end: byte,
            });
        }

        let first = self.region_groups.partition_point(|group| {
            self.record_region_lookup_step();
            group.body.end <= span.start
        });
        let last = self.region_groups.partition_point(|group| {
            self.record_region_lookup_step();
            group.body.end < span.end
        });
        if first != last || first == self.region_groups.len() {
            return None;
        }
        let group = self.region_groups[first];
        self.original.region(Span {
            start: group.original.start + span.start - group.body.start,
            end: group.original.start + span.end - group.body.start,
        })
    }

    /// The original byte ranges of a body span.
    fn map_bytes(&self, source: &Source, span: Span) -> Result<Vec<Span>, Box<SourceMapError>> {
        let fail = |message: &str| {
            Box::new(SourceMapError {
                cause: SourceMapErrorCause::InvalidMap,
                source: self.body.identity().clone(),
                path: self.body.path().into(),
                span: self
                    .body
                    .locate(Span { start: 0, end: 0 })
                    .expect("internal offsets are UTF-8 boundaries"),
                message: message.into(),
            })
        };
        if !self.matches_body(source) {
            return Err(fail(
                "span belongs to a different source label, path or bytes",
            ));
        }
        if self.body.slice(span).is_none() {
            return Err(fail("invalid body span"));
        }
        let first = self.segments.partition_point(|segment| {
            self.record_region_lookup_step();
            segment.body.end <= span.start
        });
        if span.start == span.end {
            let byte = self.segments.get(first).map_or_else(
                || {
                    self.segments
                        .last()
                        .map_or(self.region.start, |segment| segment.original.end)
                },
                |segment| segment.original.start + span.start - segment.body.start,
            );
            return Ok(vec![Span {
                start: byte,
                end: byte,
            }]);
        }
        let mut regions: Vec<Span> = Vec::new();
        for segment in &self.segments[first..] {
            self.record_region_lookup_step();
            if segment.body.start >= span.end {
                break;
            }
            let start =
                segment.original.start + span.start.max(segment.body.start) - segment.body.start;
            let end = segment.original.start + span.end.min(segment.body.end) - segment.body.start;
            if let Some(previous) = regions.last_mut().filter(|previous| previous.end == start) {
                previous.end = end;
            } else {
                regions.push(Span { start, end });
            }
        }
        Ok(regions)
    }

    fn matches_body(&self, source: &Source) -> bool {
        source.same_document(&self.body)
            || (source.identity() == self.body.identity()
                && source.path() == self.body.path()
                && source.text() == self.body.text())
    }

    #[cfg(test)]
    fn record_region_lookup_step(&self) {
        self.region_lookup_steps
            .set(self.region_lookup_steps.get().saturating_add(1));
    }

    #[cfg(not(test))]
    #[inline(always)]
    fn record_region_lookup_step(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Source, SourceIdentity};
    use ix_trace_rs::trace;

    const SIZES: [usize; 3] = [5_000, 10_000, 20_000];

    fn source(identity: &str, path: &str, text: &str) -> Source {
        Source::read_typed(
            SourceIdentity::new("agent-ix", identity),
            path,
            text.as_bytes(),
            text.len(),
        )
        .unwrap_or_else(|_| panic!("test source should be valid"))
    }

    fn indented_map(lines: usize) -> (SourceMap, Source) {
        let mut body_text = String::with_capacity(lines * 2);
        let mut original_text = String::with_capacity(lines * 4);
        let mut segments = Vec::with_capacity(lines);

        for line in 0..lines {
            if line > 0 {
                original_text.push_str("  ");
            }
            let body_start = body_text.len();
            let original_start = original_text.len();
            body_text.push_str("x\n");
            original_text.push_str("x\n");
            segments.push(Segment {
                body: Span {
                    start: body_start,
                    end: body_text.len(),
                },
                original: Span {
                    start: original_start,
                    end: original_text.len(),
                },
            });
        }

        let body = source("region-body", "unit.native", &body_text);
        let original = source("region-document", "unit.md", &original_text);
        let original_end = original_text.len();
        let map = SourceMap::verify_typed(
            original,
            body.clone(),
            Span {
                start: 0,
                end: original_end,
            },
            segments,
            Layout {
                strip_indentation: true,
                ..Layout::default()
            },
            lines,
        )
        .unwrap_or_else(|_| panic!("the indented source map should verify"));
        (map, body)
    }

    fn current_single_region(map: &SourceMap, body: &Source, span: Span) -> Option<SourceRegion> {
        let mut regions = map.map_regions(body, span)?;
        (regions.len() == 1).then(|| regions.remove(0))
    }

    fn ceil_log2(value: usize) -> usize {
        (usize::BITS - value.saturating_sub(1).leading_zeros()) as usize
    }

    #[test]
    #[trace("NFR-141-AC-19")]
    fn nested_single_region_lookups_do_not_rescan_every_segment() {
        let mut previous_steps: Option<usize> = None;
        for lines in SIZES {
            let (map, body) = indented_map(lines);
            assert!(map.region_index_build_steps <= lines);
            let mut total_steps = 0usize;

            for prefix_lines in 1..=lines {
                map.region_lookup_steps.set(0);
                let span = Span {
                    start: 0,
                    end: prefix_lines * 2,
                };
                let region = map.map_region(&body, span);
                let steps = map.region_lookup_steps.get();
                total_steps = total_steps.saturating_add(steps);

                if prefix_lines == 1 {
                    let region = region.expect("one source-map segment resolves to one region");
                    assert_eq!((region.start(), region.end()), (0, 2));
                } else {
                    assert!(
                        region.is_none(),
                        "an indentation gap makes a multi-line prefix discontiguous"
                    );
                }
            }

            let per_lookup_bound = 2 * ceil_log2(lines) + 4;
            let total_bound = lines.saturating_mul(per_lookup_bound);
            assert!(
                total_steps <= total_bound,
                "single-region lookup did {total_steps} segment steps for {lines} nested spans; bound is {total_bound}"
            );
            if let Some(previous) = previous_steps {
                assert!(
                    total_steps <= previous.saturating_mul(3),
                    "doubling segments grew lookup work from {previous} to {total_steps}"
                );
            }
            previous_steps = Some(total_steps);
        }
    }

    #[test]
    #[trace("NFR-141-AC-19")]
    fn single_region_lookup_matches_multi_region_mapping_at_boundaries() {
        let (map, body) = indented_map(12);
        for start_line in 0..=12 {
            for end_line in start_line..=12 {
                let span = Span {
                    start: start_line * 2,
                    end: end_line * 2,
                };
                assert_eq!(
                    map.map_region(&body, span),
                    current_single_region(&map, &body, span),
                    "mapping differs for body span {span:?}"
                );
            }
        }
    }
}

fn allowed_gap(
    source: &Source,
    gap: Span,
    region_end: usize,
    trailing: bool,
    layout: Layout,
) -> bool {
    let bytes = source.text().as_bytes();
    let mut at = gap.start;
    while at < gap.end {
        match bytes[at] {
            b' ' | b'\t' if layout.strip_indentation && at < source.indentation_end(at) => {
                // Skip the entire allowed indentation prefix, keeping validation linear.
                at = gap.end.min(source.indentation_end(at));
            }
            b'\r'
                if layout.normalize_crlf
                    && at + 1 < region_end
                    && bytes.get(at + 1) == Some(&b'\n') =>
            {
                at += 1
            }
            b'\n' if trailing && layout.drop_final_newline && at + 1 == region_end => at += 1,
            // Dropping the final CRLF pair does not require normalization of interior newlines.
            b'\r'
                if trailing
                    && layout.drop_final_newline
                    && at + 2 == region_end
                    && gap.end == region_end
                    && bytes.get(at + 1) == Some(&b'\n') =>
            {
                at += 2
            }
            _ => return false,
        }
    }
    true
}
