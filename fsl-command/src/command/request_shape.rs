// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1047: what a request document holds, in one place. The request shape
//! of each request format below names its required members, its one-of
//! groups and the values of its example. The same shape reports every
//! missing member at once (`admit`) and writes the example
//! (`request_example`), so a refusal and the example it points a caller at
//! cannot disagree. The table of formats each entry family reads and every
//! message template are in this module.

use std::collections::BTreeMap;

use fsl_foundation::wire_format::WireFormat;
use fsl_foundation::Code;
use serde_json::value::RawValue;

use super::{CompleteRunSelection, ExportSelection, Result, RunCause};

/// The entry families of FR-1047's table of request entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Family {
    /// `run` and its variants.
    Run,
    /// `prepare_run`, `prepare_run_with_overrides`, `read_clause_run_request`.
    RunReader,
    /// `compile` and its variants.
    Compile,
    /// `prepare_analyze` and the analyze entries.
    Analyze,
    /// `monitor`.
    Monitor,
    /// `export_with_limits_and_overrides`, with its selection.
    Export(ExportSelection),
}

impl Family {
    fn name(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::RunReader => "run-reader",
            Self::Compile => "compile",
            Self::Analyze => "analyze",
            Self::Monitor => "monitor",
            Self::Export(_) => "export",
        }
    }

    /// The formats this family reads, in the table's order.
    fn formats(self) -> &'static [WireFormat] {
        match self {
            Self::Run => &[
                WireFormat::RunRequest,
                WireFormat::MonitorRequest,
                WireFormat::ReplayRequest,
            ],
            Self::RunReader => &[WireFormat::RunRequest],
            Self::Compile => &[WireFormat::CompileRequest],
            Self::Analyze => &[WireFormat::AnalyzeRequest],
            Self::Monitor => &[WireFormat::MonitorRequest],
            Self::Export(_) => &[WireFormat::RunRequest],
        }
    }

    /// The family whose reader decodes `format` into its typed request.
    fn reading(format: WireFormat) -> Self {
        match format {
            WireFormat::CompileRequest => Self::Compile,
            WireFormat::AnalyzeRequest => Self::Analyze,
            WireFormat::MonitorRequest => Self::Monitor,
            _ => Self::Run,
        }
    }
}

/// The request formats `request_example` knows, in FR-1047's order.
const EXAMPLE_FORMATS: [WireFormat; 5] = [
    WireFormat::RunRequest,
    WireFormat::CompileRequest,
    WireFormat::AnalyzeRequest,
    WireFormat::MonitorRequest,
    WireFormat::ReplayRequest,
];

fn join(names: impl IntoIterator<Item = &'static str>) -> String {
    names.into_iter().collect::<Vec<_>>().join(", ")
}

fn accepted(family: Family) -> Vec<&'static str> {
    family
        .formats()
        .iter()
        .map(|format| format.as_str())
        .collect()
}

fn not_read_message(family: Family, format: &str) -> String {
    format!(
        "invalid request: format {format} is not read by {name}; {name} reads {reads}",
        name = family.name(),
        reads = join(accepted(family)),
    )
}

fn missing_message(missing: &[Missing], shape: Option<&str>) -> String {
    let entries = missing
        .iter()
        .map(Missing::text)
        .collect::<Vec<_>>()
        .join(", ");
    let noun = if missing.len() == 1 {
        "member"
    } else {
        "members"
    };
    let message = format!("invalid request: missing required {noun} {entries}");
    match shape {
        Some(shape) => format!("{message}: {shape}"),
        None => message,
    }
}

/// FR-1015: the member `export` carries.
const ORDINARY_SHAPE: &str = "export reads a native-run/1 request whose request holds an export object with at least profile and obligations";
/// FR-1022: the member `export_evidence` carries.
const EVIDENCE_SHAPE: &str = "export evidence reads a native-run/1 request whose request holds an export_evidence object with package, obligation, tool and artifact";

/// One missing entry: a required member, or a one-of group none of whose
/// members is present.
#[derive(Debug, PartialEq)]
enum Missing {
    Member(String),
    Group(Vec<String>),
}

impl Missing {
    fn text(&self) -> String {
        match self {
            Self::Member(pointer) => pointer.clone(),
            Self::Group(pointers) => pointers.join(" | "),
        }
    }

    fn detail(&self) -> serde_json::Value {
        match self {
            Self::Member(pointer) => serde_json::Value::String(pointer.clone()),
            Self::Group(pointers) => serde_json::Value::Array(
                pointers
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        }
    }
}

/// The value a member takes in an example, or the members of an object
/// that a reader here checks for completeness.
enum Shape {
    /// A value written as this JSON text; its members, if any, are read by
    /// the format's own reader.
    Leaf(&'static str),
    /// An object whose required members and groups are listed.
    Object(&'static [Entry]),
    /// An array of objects, each with these required members and groups.
    Objects(&'static [Entry]),
}

struct Field {
    name: &'static str,
    shape: Shape,
}

enum Entry {
    /// A member the reader refuses the request for lacking.
    Required(Field),
    /// Members of which exactly one is read; the example carries the first.
    OneOf(&'static [Field]),
    /// A member in scope when present, listed by no absence.
    Optional(Field),
}

const fn required(name: &'static str, shape: Shape) -> Entry {
    Entry::Required(Field { name, shape })
}

const fn field(name: &'static str, shape: Shape) -> Field {
    Field { name, shape }
}

const SOURCE: &[Entry] = &[
    required("file", Shape::Leaf("\"program.fsl\"")),
    required("authority", Shape::Leaf("\"example\"")),
    required("identity", Shape::Leaf("\"example:program\"")),
    required("document", Shape::Leaf("\"Program\"")),
    required("formal_revision", Shape::Leaf("1")),
];

const PROGRAM: &[Entry] = &[required("source", Shape::Object(SOURCE))];

const MODEL: &[Entry] = &[
    required("format", Shape::Leaf("\"semantic-ir/2.0.0\"")),
    required("source", Shape::Object(SOURCE)),
];

const LIBRARY: &[Entry] = &[
    required("identity", Shape::Leaf("\"example:library\"")),
    required("source", Shape::Object(SOURCE)),
];

const CALL: &[Entry] = &[
    required("function", Shape::Leaf("\"main\"")),
    required("arguments", Shape::Leaf("[]")),
];

const PACKAGE_FILE: &str = "{\"file\":\"package.json\"}";

/// `models`: required where the reading requires it, an array of entries.
const fn models() -> Entry {
    required("models", Shape::Objects(MODEL))
}

/// `libraries`: in scope where a document carries it.
const fn libraries() -> Entry {
    Entry::Optional(field("libraries", Shape::Objects(LIBRARY)))
}

const PROGRAM_OR_PACKAGE: Entry = Entry::OneOf(&[
    field("program", Shape::Object(PROGRAM)),
    field("package", Shape::Leaf(PACKAGE_FILE)),
]);

/// The run reading's run members: its `call` / `clause` / `items` group.
const RUN_MEMBERS: &[Field] = &[
    field("call", Shape::Object(CALL)),
    field("clause", Shape::Leaf("{}")),
    field("items", Shape::Leaf("{}")),
];

const RUN: &[Entry] = &[
    models(),
    PROGRAM_OR_PACKAGE,
    Entry::OneOf(RUN_MEMBERS),
    libraries(),
];

/// A run request carrying a member that selects another reading of
/// `native-run/1` lacks no `call` / `clause` / `items` group (FR-1047).
const RUN_SELECTED: &[Entry] = &[models(), PROGRAM_OR_PACKAGE, libraries()];

/// The members that select another reading of `native-run/1` than a call
/// (the export members have their own shapes below).
const SELECTORS: [&str; 5] = [
    "tl_lowering",
    "synthesis",
    "verification_plan",
    "model_based_test",
    "refinement_transfer",
];

const EXPORT: &[Entry] = &[
    PROGRAM_OR_PACKAGE,
    required(
        "export",
        Shape::Leaf("{\"obligations\":[\"Obligation\"],\"profile\":\"filament.output.tla2/v1\"}"),
    ),
    libraries(),
];

const EXPORT_EVIDENCE: &[Entry] = &[
    models(),
    required(
        "export_evidence",
        Shape::Leaf(
            "{\"artifact\":\"proof.txt\",\"obligation\":\"Obligation\",\"package\":{},\"tool\":\"tlc\"}",
        ),
    ),
];

const COMPILE: &[Entry] = &[
    required("program", Shape::Object(PROGRAM)),
    Entry::Optional(field("models", Shape::Objects(MODEL))),
    libraries(),
];

const ANALYZE: &[Entry] = &[
    Entry::OneOf(&[
        field("program", Shape::Object(PROGRAM)),
        field("package", Shape::Leaf("{\"directory\":\"package\"}")),
    ]),
    Entry::Optional(field("models", Shape::Objects(MODEL))),
    libraries(),
];

const CLAUSE: &[Entry] = &[required("name", Shape::Leaf("\"Clause\""))];

const MONITOR: &[Entry] = &[
    models(),
    required("program", Shape::Object(PROGRAM)),
    required("clauses", Shape::Objects(CLAUSE)),
    required(
        "trace",
        Shape::Object(&[required("kind", Shape::Leaf("\"finite\""))]),
    ),
    libraries(),
];

/// A run entry runs a monitor request's `records` file (FR-644).
const MONITOR_RUN: &[Entry] = &[
    models(),
    required("program", Shape::Object(PROGRAM)),
    required("clauses", Shape::Objects(CLAUSE)),
    required(
        "trace",
        Shape::Object(&[required("kind", Shape::Leaf("\"finite\""))]),
    ),
    required("records", Shape::Leaf("\"records.jsonl\"")),
    libraries(),
];

/// The members of FSpec FR-336's replay request, in the reader's order
/// ([`crate::replay_request::MEMBERS`]).
const REPLAY: &[Entry] = &[
    required(crate::replay_request::MEMBERS[0], Shape::Leaf("[]")),
    required(crate::replay_request::MEMBERS[1], Shape::Leaf(NODE_PACKAGE)),
    required(crate::replay_request::MEMBERS[2], Shape::Leaf("[]")),
    required(
        crate::replay_request::MEMBERS[3],
        Shape::Leaf(REPLAY_TARGET),
    ),
    required(
        crate::replay_request::MEMBERS[4],
        Shape::Leaf("{\"assignments\":[],\"kind\":\"input\"}"),
    ),
    required(
        crate::replay_request::MEMBERS[5],
        Shape::Leaf(NODE_OBLIGATION),
    ),
    required(crate::replay_request::MEMBERS[6], Shape::Leaf("\"fsl\"")),
    required(crate::replay_request::MEMBERS[7], Shape::Leaf("[]")),
    required(crate::replay_request::MEMBERS[8], Shape::Leaf("{}")),
    required(crate::replay_request::MEMBERS[9], Shape::Leaf("{}")),
    required(crate::replay_request::MEMBERS[10], Shape::Leaf("null")),
    required(crate::replay_request::MEMBERS[11], Shape::Leaf("null")),
    required(crate::replay_request::MEMBERS[12], Shape::Leaf("[]")),
    required(
        crate::replay_request::MEMBERS[13],
        Shape::Leaf(
            "[{\"authority\":\"example\",\"bytes_path\":\"entry.fsl\",\"identity\":\"entry\"}]",
        ),
    ),
];

fn export_shape(selection: ExportSelection) -> &'static [Entry] {
    match selection {
        ExportSelection::Ordinary => EXPORT,
        ExportSelection::Evidence => EXPORT_EVIDENCE,
    }
}

/// The run reading of a request: a member selecting another reading of
/// `native-run/1` than a call lacks no `call` / `clause` / `items` group.
fn run_shape(request: &BTreeMap<String, &RawValue>) -> &'static [Entry] {
    if SELECTORS.iter().any(|name| request.contains_key(*name)) {
        RUN_SELECTED
    } else {
        RUN
    }
}

/// The request shape `family` reads a document of `format` under.
/// `request` is the document's request members.
fn shape(
    family: Family,
    format: WireFormat,
    request: &BTreeMap<String, &RawValue>,
) -> &'static [Entry] {
    match (format, family) {
        (WireFormat::RunRequest, Family::Export(selection)) => export_shape(selection),
        (WireFormat::RunRequest, Family::RunReader) => run_shape(request),
        (WireFormat::RunRequest, _) => {
            if request.contains_key("export_evidence") {
                EXPORT_EVIDENCE
            } else if request.contains_key("export") {
                EXPORT
            } else {
                run_shape(request)
            }
        }
        (WireFormat::CompileRequest, _) => COMPILE,
        (WireFormat::AnalyzeRequest, _) => ANALYZE,
        (WireFormat::MonitorRequest, Family::Monitor) => MONITOR,
        (WireFormat::MonitorRequest, _) => MONITOR_RUN,
        _ => REPLAY,
    }
}

fn object(raw: &RawValue) -> Option<BTreeMap<String, &RawValue>> {
    serde_json::from_str(raw.get()).ok()
}

/// RFC 6901: a member name as one pointer segment.
fn token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

fn descend(field: &Field, raw: &RawValue, base: &str, out: &mut Vec<Missing>) {
    let base = format!("{base}/{}", token(field.name));
    match field.shape {
        Shape::Leaf(_) => {}
        Shape::Object(entries) => {
            if let Some(members) = object(raw) {
                collect(entries, &members, &base, out);
            }
        }
        Shape::Objects(entries) => {
            if let Ok(items) = serde_json::from_str::<Vec<&RawValue>>(raw.get()) {
                for (index, item) in items.into_iter().enumerate() {
                    if let Some(members) = object(item) {
                        collect(entries, &members, &format!("{base}/{index}"), out);
                    }
                }
            }
        }
    }
}

/// Every missing entry of `entries` in `members`, in member order, at every
/// depth of the in-scope objects `members` holds.
fn collect(
    entries: &[Entry],
    members: &BTreeMap<String, &RawValue>,
    base: &str,
    out: &mut Vec<Missing>,
) {
    for entry in entries {
        match entry {
            Entry::Required(field) => match members.get(field.name) {
                None => out.push(Missing::Member(format!("{base}/{}", token(field.name)))),
                Some(raw) => descend(field, raw, base, out),
            },
            Entry::Optional(field) => {
                if let Some(raw) = members.get(field.name) {
                    descend(field, raw, base, out);
                }
            }
            Entry::OneOf(fields) => {
                let present: Vec<&Field> = fields
                    .iter()
                    .filter(|field| members.contains_key(field.name))
                    .collect();
                if present.is_empty() {
                    out.push(Missing::Group(
                        fields
                            .iter()
                            .map(|field| format!("{base}/{}", token(field.name)))
                            .collect(),
                    ));
                }
                for field in present {
                    if let Some(raw) = members.get(field.name) {
                        descend(field, raw, base, out);
                    }
                }
            }
        }
    }
}

/// FR-1047: refuse a request document whose format `family` does not read,
/// and one that lacks required members, naming all of them, before any file
/// the request names is read. A document that is not JSON, and every other
/// defect, stays with the reader that owns it.
pub(super) fn admit(family: Family, bytes: &[u8]) -> Result<()> {
    let Ok(document) = serde_json::from_slice::<BTreeMap<String, &RawValue>>(bytes) else {
        return Ok(());
    };
    // An `argv` document is an argument document (FR-561, FR-661), not a
    // request envelope.
    if document.contains_key("argv") {
        return Ok(());
    }
    let Some(format) = document.get("format") else {
        let mut absent = vec![Missing::Member("/format".to_owned())];
        if !document.contains_key("request") {
            absent.push(Missing::Member("/request".to_owned()));
        }
        if envelope_defect(&document, bytes, false) {
            return Ok(());
        }
        return Err(missing_members(&absent, None));
    };
    // A string `format` is checked before `request`.
    let Ok(format) = serde_json::from_str::<String>(format.get()) else {
        return Ok(());
    };
    let Some(format) = family
        .formats()
        .iter()
        .copied()
        .find(|known| known.as_str() == format)
    else {
        return Err(RunCause::FormatNotRead {
            message: not_read_message(family, &format),
            accepted: accepted(family),
        });
    };
    let whole_document = matches!(format, WireFormat::RunRequest | WireFormat::ReplayRequest);
    let Some(request) = document.get("request") else {
        if envelope_defect(&document, bytes, whole_document) {
            return Ok(());
        }
        return Err(missing_members(
            &[Missing::Member("/request".to_owned())],
            None,
        ));
    };
    let Some(request) = object(request) else {
        return Ok(());
    };
    let mut missing = Vec::new();
    collect(
        shape(family, format, &request),
        &request,
        "/request",
        &mut missing,
    );
    if missing.is_empty() {
        return Ok(());
    }
    // FR-1047 "A selected source with no run member": the typed reader's own
    // refusals come first, and it then gives [`no_run_member`].
    if missing == [run_group()]
        && request.contains_key("program") != request.contains_key("package")
    {
        return Ok(());
    }
    let selected = match family {
        Family::Export(ExportSelection::Ordinary) => Some(("/request/export", ORDINARY_SHAPE)),
        Family::Export(ExportSelection::Evidence) => {
            Some(("/request/export_evidence", EVIDENCE_SHAPE))
        }
        _ => None,
    };
    let sentence = selected.and_then(|(pointer, sentence)| {
        missing
            .iter()
            .any(|entry| matches!(entry, Missing::Member(found) if found == pointer))
            .then_some(sentence)
    });
    // An unknown, wrong-kind or repeated member is refused for that defect
    // by the reader that owns it, ahead of any missing member.
    if envelope_defect(&document, bytes, whole_document) || another_defect(family, format, &request)
    {
        return Ok(());
    }
    Err(missing_members(&missing, sentence))
}

/// Whether the envelope carries a member beyond `format` and `request`, or
/// the envelope or its `request` object repeats a member name. A repeat
/// deeper in the document belongs to the reader of that subtree, except
/// that the run and replay readers keep some members as plain JSON values
/// that drop a repeat, so `whole_document` checks every object.
fn envelope_defect(
    document: &BTreeMap<String, &RawValue>,
    bytes: &[u8],
    whole_document: bool,
) -> bool {
    document
        .keys()
        .any(|name| !matches!(name.as_str(), "format" | "request"))
        || (whole_document && crate::unique_json::read(bytes).is_err())
        || repeats_a_name(bytes)
        || document
            .get("request")
            .is_some_and(|request| repeats_a_name(request.get().as_bytes()))
}

/// Whether the JSON object in `bytes` repeats a member name at its own level.
fn repeats_a_name(bytes: &[u8]) -> bool {
    struct Names(Vec<String>);
    impl<'de> serde::Deserialize<'de> for Names {
        fn deserialize<D: serde::Deserializer<'de>>(
            decoder: D,
        ) -> std::result::Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Names;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("an object")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> std::result::Result<Names, A::Error> {
                    let mut names = Vec::new();
                    while let Some((name, _)) = map.next_entry::<String, &'de RawValue>()? {
                        names.push(name);
                    }
                    Ok(Names(names))
                }
            }
            decoder.deserialize_map(Visitor)
        }
    }
    let Ok(Names(mut names)) = serde_json::from_slice(bytes) else {
        return false;
    };
    let total = names.len();
    names.sort();
    names.dedup();
    names.len() != total
}

/// Whether the request's typed reader refuses it for a defect other than a
/// missing member (an unknown member or a wrong kind): that refusal belongs
/// to the reader, not to this requirement.
fn another_defect(
    family: Family,
    format: WireFormat,
    request: &BTreeMap<String, &RawValue>,
) -> bool {
    use super::wire;
    use fsl_foundation::serde_object::Object;
    let mut members = request.clone();
    if matches!(family, Family::Export(_)) {
        members.remove("export");
        members.remove("export_evidence");
    }
    let Ok(text) = serde_json::to_string(&members) else {
        return false;
    };
    let error = match (format, family) {
        (WireFormat::RunRequest, Family::Export(_)) if members.contains_key("package") => {
            serde_json::from_str::<Object<wire::Request>>(&text).err()
        }
        (WireFormat::RunRequest, Family::Export(_)) => {
            serde_json::from_str::<Object<wire::CompileRequest>>(&text).err()
        }
        (WireFormat::RunRequest, _) => serde_json::from_str::<Object<wire::Request>>(&text).err(),
        (WireFormat::CompileRequest, _) => {
            serde_json::from_str::<Object<wire::CompileRequest>>(&text).err()
        }
        (WireFormat::AnalyzeRequest, _) => {
            serde_json::from_str::<Object<wire::AnalyzeRequest>>(&text).err()
        }
        (WireFormat::MonitorRequest, _) => {
            serde_json::from_str::<Object<wire::MonitorRequest>>(&text).err()
        }
        _ => None,
    };
    error.is_some_and(|error| !is_missing_member(&error))
}

/// Serde's derived readers report an absent required member as
/// ``missing field `name` ``; every other error is another defect.
fn is_missing_member(error: &serde_json::Error) -> bool {
    error.classify() == serde_json::error::Category::Data
        && error.to_string().starts_with("missing field `")
}

fn missing_members(missing: &[Missing], shape: Option<&str>) -> RunCause {
    RunCause::MissingMembers {
        message: missing_message(missing, shape),
        missing: missing.iter().map(Missing::detail).collect(),
    }
}

/// The missing entry of a request that carries none of [`RUN_MEMBERS`].
fn run_group() -> Missing {
    Missing::Group(
        RUN_MEMBERS
            .iter()
            .map(|field| format!("/request/{}", token(field.name)))
            .collect(),
    )
}

/// FR-1047-AC-10, FR-100-AC-25: the refusal of a request that selects a
/// source and no run member: the run group as its one missing entry, under
/// FR-100's message.
pub(super) fn no_run_member() -> RunCause {
    RunCause::MissingMembers {
        message: RunCause::CompleteRunSelection(CompleteRunSelection::NoCall).to_string(),
        missing: vec![run_group().detail()],
    }
}

/// The refusal of a reader handed a document whose format is not its own.
pub(super) fn not_read(expected: WireFormat, format: &str) -> RunCause {
    let family = Family::reading(expected);
    RunCause::FormatNotRead {
        message: not_read_message(family, format),
        accepted: accepted(family),
    }
}

/// Why [`request_example`] returned no document.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RequestExampleError {
    /// The format is not one of the five request formats.
    #[error(
        "unknown request format {format}; request formats are {}",
        .known.join(", ")
    )]
    UnknownFormat {
        /// The format asked for.
        format: String,
        /// The request formats, in FR-1047's order.
        known: Vec<&'static str>,
    },
    /// An export selection was given for a format other than `native-run/1`.
    #[error("an export selection reads native-run/1, not {format}")]
    SelectionNeedsRunFormat {
        /// The format asked for.
        format: String,
    },
}

impl RequestExampleError {
    /// The catalogued code of this refusal: `invalid-request`.
    pub fn code(&self) -> Code {
        Code::InvalidRequest
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn write_object(entries: &[Entry], depth: usize, out: &mut String) {
    out.push_str("{\n");
    let fields: Vec<&Field> = entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Required(field) => Some(field),
            Entry::OneOf(fields) => fields.first(),
            Entry::Optional(_) => None,
        })
        .collect();
    for (index, field) in fields.iter().enumerate() {
        out.push_str(&indent(depth + 1));
        out.push_str(&format!("\"{}\": ", field.name));
        match field.shape {
            Shape::Object(inner) => write_object(inner, depth + 1, out),
            Shape::Objects(_) => out.push_str("[]"),
            Shape::Leaf(text) => {
                let value: serde_json::Value =
                    serde_json::from_str(text).expect("a request example leaf is valid JSON text");
                let pretty = serde_json::to_string_pretty(&value).expect("a JSON value serializes");
                out.push_str(&pretty.replace('\n', &format!("\n{}", indent(depth + 1))));
            }
        }
        out.push_str(if index + 1 == fields.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str(&indent(depth));
    out.push('}');
}

/// FR-1047: the minimal valid request document of `format`, or of its export
/// reading under `selection`.
///
/// # Errors
///
/// [`RequestExampleError::UnknownFormat`] for a format that is no request
/// format; [`RequestExampleError::SelectionNeedsRunFormat`] for a selection
/// with any format but `native-run/1`.
pub fn request_example(
    format: &str,
    selection: Option<ExportSelection>,
) -> std::result::Result<String, RequestExampleError> {
    let Some(known) = EXAMPLE_FORMATS
        .iter()
        .copied()
        .find(|known| known.as_str() == format)
    else {
        return Err(RequestExampleError::UnknownFormat {
            format: format.to_owned(),
            known: EXAMPLE_FORMATS.iter().map(|known| known.as_str()).collect(),
        });
    };
    let entries = match (known, selection) {
        (WireFormat::RunRequest, Some(selection)) => export_shape(selection),
        (_, Some(_)) => {
            return Err(RequestExampleError::SelectionNeedsRunFormat {
                format: format.to_owned(),
            })
        }
        (WireFormat::RunRequest, None) => RUN,
        (WireFormat::CompileRequest, None) => COMPILE,
        (WireFormat::AnalyzeRequest, None) => ANALYZE,
        (WireFormat::MonitorRequest, None) => MONITOR,
        (_, None) => REPLAY,
    };
    let mut out = format!("{{\n  \"format\": \"{}\",\n  \"request\": ", known.as_str());
    write_object(entries, 1, &mut out);
    out.push_str("\n}");
    Ok(out)
}

/// The one node of the replay example's package, and the obligation that
/// names it.
const NODE_PACKAGE: &str = "{\"lock\":{\"sources\":[{\"authority\":\"example\",\"identity\":\"entry\"}]},\"name\":\"example\",\"semantic_graph\":{\"nodes\":[{\"check_mode\":\"linked\",\"declaration\":{\"qualified_name\":[\"entry\"]},\"node_id\":{\"ordinal\":0,\"package\":\"example\",\"path\":[\"entry\"]},\"node_tag\":\"function\"}]}}";
const REPLAY_TARGET: &str = "{\"arguments\":false,\"members\":{\"function\":\"entry\"},\"occurrence_key\":{\"node_id\":{\"ordinal\":0,\"package\":\"example\",\"path\":[\"entry\"]},\"ordinal\":\"0\",\"role\":\"declaration\"},\"over\":null}";
const NODE_OBLIGATION: &str = "{\"model\":null,\"node_id\":{\"ordinal\":0,\"package\":\"example\",\"path\":[\"entry\"]},\"ordinal\":0}";

#[cfg(test)]
mod tests {
    use super::is_missing_member;

    /// Serde words an absent required member as `missing field`; this pins
    /// the wording `is_missing_member` classifies on.
    #[test]
    fn a_missing_member_is_told_apart_from_other_defects() {
        #[derive(Debug, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Probe {
            #[allow(dead_code)]
            needed: u8,
        }
        let absent = serde_json::from_str::<Probe>("{}").unwrap_err();
        assert!(is_missing_member(&absent));
        let unknown = serde_json::from_str::<Probe>(r#"{"needed":1,"z":1}"#).unwrap_err();
        assert!(!is_missing_member(&unknown));
        let wrong = serde_json::from_str::<Probe>(r#"{"needed":"x"}"#).unwrap_err();
        assert!(!is_missing_member(&wrong));
    }
}
