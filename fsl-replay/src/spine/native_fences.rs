// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-662, FR-767: check verified native fences under one caller selection.

use filament_exact::Cancel;
use filament_semantic_value::catalog::Category;
use fsl_foundation::diagnostic::{StageFailure, StageWork, Staged};
use fsl_foundation::{Code, Diagnostic, Source, SourceIdentity};

use super::{DependencyInput, ParseRequest, ProfileCatalogInput, SpineLimits};
use crate::{AdmittedIntegerDivision, OutcomeDiagnostic, OutcomeLocus};

/// The original document and inputs shared by each independently checked fence.
pub struct NativeFenceRequest<'a> {
    /// Original immutable Markdown document with explicit source labels.
    pub original: &'a Source,
    /// Domain packages available to each fence.
    pub packages: &'a fsl_semantics::model::intake::PackageInput,
    /// Libraries available to each fence.
    pub dependencies: &'a DependencyInput,
    /// Optional extension catalog used to parse each native body.
    pub extensions: Option<ProfileCatalogInput<'a>>,
    /// Caller-selected law applied independently to every native body.
    pub integer_division: Option<&'a AdmittedIntegerDivision>,
}

/// Original-document extraction ceilings and native compiler stage ceilings.
#[derive(Clone, Copy, Debug, Default)]
pub struct CheckLimits {
    /// Quire extraction limits over the original document.
    pub extraction: fsl_source::Limits,
    /// S1 through S4 limits applied to each body.
    pub spine: SpineLimits,
}

/// A diagnostic in body coordinates with verified original-document regions.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeFenceDiagnostic {
    /// The original document and extracted unit this diagnostic concerns.
    pub unit: NativeFenceUnit,
    /// Compiler diagnostic retaining its body locus and cause.
    #[serde(flatten)]
    pub diagnostic: OutcomeDiagnostic,
    /// Corresponding source regions in the original Markdown document.
    pub original_regions: Vec<OutcomeLocus>,
}

/// The document and body identity of one native fence (FR-662).
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeFenceUnit {
    /// The original document's explicit labels.
    pub document: SourceIdentity,
    /// The original document's display path.
    pub path: String,
    /// The owning clause heading, absent for a headless fence.
    pub clause: Option<String>,
    /// The derived identity of the verified native body.
    pub body: SourceIdentity,
}

/// The result of checking one native fence, in original document order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFenceResult {
    /// Original Markdown document identity.
    pub document: SourceIdentity,
    /// Original Markdown display path.
    pub path: String,
    /// Heading that owns the fence; absent for a headless native fence.
    pub clause: Option<String>,
    /// Extracted body's explicit source identity.
    pub body: SourceIdentity,
    /// First body line in the original Markdown document.
    pub first_line: usize,
    /// Most severe category reached while checking this body.
    pub category: Category,
    /// Independent checking or extraction diagnostics.
    pub diagnostics: Vec<NativeFenceDiagnostic>,
    /// Number of conflicting bodies when a heading owns multiple native fences.
    pub actual: Option<usize>,
}

impl serde::Serialize for NativeFenceResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        #[derive(serde::Serialize)]
        struct Details {
            actual: usize,
        }
        let mut record = serializer.serialize_struct(
            "NativeFenceResult",
            if self.actual.is_some() { 5 } else { 4 },
        )?;
        record.serialize_field("unit", &self.unit())?;
        record.serialize_field("first_line", &self.first_line)?;
        record.serialize_field("category", self.category.as_str())?;
        record.serialize_field("diagnostics", &self.diagnostics)?;
        if let Some(actual) = self.actual {
            record.serialize_field("details", &Details { actual })?;
        }
        record.end()
    }
}

impl NativeFenceResult {
    fn unit(&self) -> NativeFenceUnit {
        NativeFenceUnit {
            document: self.document.clone(),
            path: self.path.clone(),
            clause: self.clause.clone(),
            body: self.body.clone(),
        }
    }
}

/// Results of checking every native fence in document order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFenceReport {
    /// One result per native fence; other languages are absent.
    pub fences: Vec<NativeFenceResult>,
}

/// A failure of the original document before independent fence checking.
#[derive(Debug, thiserror::Error)]
pub enum FenceCheckCause {
    /// Quire extraction or source-map verification refused the document.
    #[error("{0}")]
    Extraction(Box<Diagnostic>),
}

impl FenceCheckCause {
    /// Stable catalog code of the document's extraction failure.
    pub fn code(&self) -> Code {
        match self {
            Self::Extraction(diagnostic) => diagnostic.code,
        }
    }
}

/// Check every native fence with the request's caller law (FR-767-AC-11).
/// A refused body does not prevent checking other fences.
pub fn check_native_fences(
    request: &NativeFenceRequest<'_>,
    limits: CheckLimits,
    cancel: &Cancel,
) -> Result<Staged<NativeFenceReport>, StageFailure<FenceCheckCause>> {
    if let Some(cause) = cancel.cause() {
        return Err(StageFailure::Cancelled(cause));
    }
    let fences = fsl_source::extract_native_fences(request.original, limits.extraction)
        .map_err(|error| StageFailure::Refused(FenceCheckCause::Extraction(error)))?;
    let mut results = Vec::with_capacity(fences.len());
    let mut work = StageWork::default();
    for fence in fences {
        if cancel.poll() {
            if let Some(cause) = cancel.cause() {
                return Err(StageFailure::Cancelled(cause));
            }
        }
        let body = fence.map.body();
        let mut result = NativeFenceResult {
            document: request.original.identity().clone(),
            path: request.original.path().to_owned(),
            clause: fence.clause.clone(),
            body: body.identity().clone(),
            first_line: fence.first_line,
            category: Category::Success,
            diagnostics: Vec::new(),
            actual: None,
        };
        let extraction_refusal = if fence.clause.is_none() {
            Some((
                Code::MissingDeclaration,
                "missing-name",
                "native fence has no owning clause heading",
            ))
        } else if fence.count > 1 {
            result.actual = Some(fence.count);
            Some((
                Code::InvalidModelBinding,
                "extraction-clause-count",
                "clause heading owns multiple native fences",
            ))
        } else {
            None
        };
        if let Some((code, cause, message)) = extraction_refusal {
            result.category = Category::Refusal;
            result.diagnostics.push(NativeFenceDiagnostic {
                unit: result.unit(),
                diagnostic: OutcomeDiagnostic::new(
                    Some(cause),
                    code.as_str(),
                    Some(OutcomeLocus::Region {
                        source: body.reference().clone(),
                        start: 0,
                        end: 0,
                    }),
                    message,
                ),
                original_regions: original_regions(
                    &fence.map,
                    fsl_foundation::Span { start: 0, end: 0 },
                ),
            });
        } else {
            let parse = ParseRequest {
                source: body.identity(),
                path: body.path(),
                bytes: body.text().as_bytes(),
            };
            let checked = (|| {
                let parsed = if let Some(catalog) = request.extensions {
                    super::parse_selecting_extensions(&parse, catalog, limits.spine.source, cancel)?
                } else {
                    super::parse(&parse, limits.spine.source, cancel)?
                };
                add_work(&mut work, parsed.work());
                let parsed = parsed.into_value();
                let models = super::select(&parsed, request.packages, limits.spine.model, cancel)?;
                add_work(&mut work, models.work());
                let models = models.into_value();
                let name = body.identity().identity.clone();
                match request.integer_division {
                    Some(law) => super::check_with_integer_division(
                        &parsed,
                        name,
                        &models,
                        request.dependencies,
                        law,
                        limits.spine,
                        cancel,
                    ),
                    None => super::check(
                        &parsed,
                        name,
                        &models,
                        request.dependencies,
                        limits.spine,
                        cancel,
                    ),
                }
            })();
            if let Ok(checked) = &checked {
                add_work(&mut work, checked.work());
            }
            if let Err(failure) = &checked {
                if let StageFailure::Cancelled(cause) = failure {
                    return Err(StageFailure::Cancelled(*cause));
                }
                let outcome = crate::OutcomeDocument::from_check(&checked);
                result.category = outcome.category();
                for diagnostic in outcome.diagnostics() {
                    let original_regions = match diagnostic.locus() {
                        Some(OutcomeLocus::Region { start, end, .. }) => usize::try_from(*start)
                            .ok()
                            .zip(usize::try_from(*end).ok())
                            .map(|(start, end)| {
                                original_regions(&fence.map, fsl_foundation::Span { start, end })
                            })
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    result.diagnostics.push(NativeFenceDiagnostic {
                        unit: result.unit(),
                        diagnostic: diagnostic.clone(),
                        original_regions,
                    });
                }
            }
        }
        results.push(result);
    }
    Ok(Staged::new(NativeFenceReport { fences: results }).with_work(work))
}

fn original_regions(
    map: &fsl_foundation::source_map::SourceMap,
    span: fsl_foundation::Span,
) -> Vec<OutcomeLocus> {
    map.map_regions(map.body(), span)
        .unwrap_or_default()
        .into_iter()
        .map(|region| match map.original().render(&region) {
            Some(span) => OutcomeLocus::Located {
                file: map.original().path().to_owned(),
                span,
            },
            None => OutcomeLocus::Region {
                source: region.source().clone(),
                start: region.start(),
                end: region.end(),
            },
        })
        .collect()
}

fn add_work(total: &mut StageWork, stage: StageWork) {
    total.s1 = total.s1.saturating_add(stage.s1);
    total.s2 = total.s2.saturating_add(stage.s2);
    total.i1 = total.i1.saturating_add(stage.i1);
    total.s3 = total.s3.saturating_add(stage.s3);
    total.s4 = total.s4.saturating_add(stage.s4);
    total.e4 = total.e4.saturating_add(stage.e4);
}
