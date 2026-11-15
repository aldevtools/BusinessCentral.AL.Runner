//! The fact vector, after the run, of every Html hash cell of the pinned corpus
//! (reports/html-email-body/RxHtmlHashTests.al:192 Hash_WordAlpha_TwoRows, :218 Hash_WordRgba_TwoRows, :395 Bound_OneRow,
//! :433 Bound_ThreeRows, :471 Bound_RowCountChangesHash). `crate::report_render` renders real Html only for a save whose
//! whole vector equals [`HTML_HASH_CELL`]; every other save keeps Part A.

use super::Format;
use super::facts::{
    AnswerForm, DataItems, Decl, Declares, Extensions, Facts, Kind, Layout, LayoutFile, Rows, Selection, Stored,
    Target, Triggers,
};

/// The facts of the cells, each by equality:
/// - the call `Ok := Report.SaveAs(ReportId, '', ReportFormat::Html, OutS)` into a Temp Blob stream (RxHtmlBodyFixtures.al:125):
///   format Html, target a stream, the answer used;
/// - the reports RX Word Alpha (RxHtmlBodyFixtures.al:53), RX Word Rgba (RxLayoutSelFixtures.al:73) and RX Hash Bound
///   (RxHtmlHashTests.al:50): a Word layout that `DefaultRenderingLayout` names in the rendering section, no RDLC layout,
///   one data item, no report extension, no report trigger, not processing only;
/// - the session: no layout of Type Custom, not the company build, not on the ADR-040 allow-list, no layout selection row
///   of the company or the tenant, and none naming the report before or after the run;
/// - the run: one row or more (`RX Html Body`.Rows, RxHtmlBodyFixtures.al:105-116, with 1, 2 or 3), and no `rgba(` in the
///   dataset (the values are `RXROW<n>`, the row number and the Bound expressions).
pub(super) const HTML_HASH_CELL: Facts = Facts {
    format: Format::Html,
    target: Target::Stream,
    answer: AnswerForm::Used,
    kind: Kind::Word,
    layout: Layout::NotRdlc,
    decl: Decl::Rendering,
    declares: Declares::Other,
    file: LayoutFile::Present,
    processing_only: false,
    data_items: DataItems::One,
    extensions: Extensions::None,
    triggers: Triggers::None,
    custom: false,
    company_build: false,
    listed: false,
    selection: Selection::None,
    stored: Stored::None,
    named: Some(false),
    rows: Some(Rows::Some),
    dataset_rgba: Some(false),
};
