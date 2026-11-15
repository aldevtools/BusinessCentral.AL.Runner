//! The measured shapes of the Html of a Word layout (ADR-040 amendment 2026-11-15, decisions 1 and 4). A save renders real
//! Html only when every fact below equals a fact of the probe cells whose Html, made by the runner's render process, has the
//! SHA-256 BC writes. The cells are the evidence that BC's own merger and converter, run by the runner on the build's own
//! assemblies, give BC's bytes for a layout of that shape. Until a corpus bump pins the probe, each pass runs through the
//! assumption `report-html-rendered-by-bc-converter`.
//!
//! The cells (corpus pull request 1415, merged, not pinned; reports/html-email-body/RxHtmlHashTests.al):
//! Hash_WordAlpha_TwoRows (:192, report `RX Word Alpha`, RxHtmlBodyFixtures.al:53) and Hash_WordRgba_TwoRows (:218, report
//! `RX Word Rgba`, RxLayoutSelFixtures.al:73). Each report has one data item over `RX Row` (RxHtmlBodyFixtures.al:12) with
//! the field columns RowId (Integer) and RowTxt (Text), no caption, no label, no report extension, and a hand-built layout
//! named by DefaultRenderingLayout; each save has two rows. Bound_OneRow (:395), Bound_ThreeRows (:433) and
//! Bound_RowCountChangesHash (:471) save report `RX Hash Bound` (:50), whose hand-built RxHashBound.docx binds all four of
//! its columns: the two field columns and the expressions `Id * 1234.5` (`AutoFormatType = 1`) and `DMY2Date(1, 2, 2026)`;
//! one and three rows.

use super::layout_facts::LayoutFacts;
use crate::dataset::ValueKind;
use al_builtins::value::RtType;
use al_ir as ir;

/// A report column by its source: a table field of the data item, by the field's type, or an expression of the measured type
/// and format properties (RxHtmlHashTests.al:62-63). Any other source has no kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColumnKind {
    IntegerField,
    TextField,
    /// An expression of static type Decimal with `AutoFormatType = 1`, no `AutoFormatExpression` and no `DecimalPlaces`.
    DecimalExpression,
    /// An expression of static type Date with no format property.
    DateExpression,
}

/// A content control bound to the merger's data part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BindingKind {
    /// A repeating section over the rows of the top-level data item.
    RowSection,
    /// A column of that data item, inside that section.
    Column(ColumnKind),
}

/// The facts of one save the gate compares; every list is sorted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SaveShape {
    /// The columns of the report's one data item.
    pub(crate) columns: Vec<ColumnKind>,
    /// The bound content controls of the layout.
    pub(crate) bindings: Vec<BindingKind>,
    /// The rows of the data item in the dataset.
    pub(crate) rows: usize,
}

/// One measured shape: the save's facts must equal these.
#[derive(Debug)]
pub(crate) struct MeasuredShape {
    pub(crate) columns: &'static [ColumnKind],
    pub(crate) bindings: &'static [BindingKind],
    pub(crate) rows: &'static [usize],
}

pub(crate) const SHAPES: &[MeasuredShape] = &[
    // Hash_WordAlpha_TwoRows and Hash_WordRgba_TwoRows: RowId and RowTxt, a row section binding RowTxt, two rows.
    MeasuredShape {
        columns: &[ColumnKind::IntegerField, ColumnKind::TextField],
        bindings: &[BindingKind::RowSection, BindingKind::Column(ColumnKind::TextField)],
        rows: &[2],
    },
    // Bound_OneRow and Bound_ThreeRows: RowId, RowTxt, Amount and Day, a row section binding all four, one or three rows.
    MeasuredShape {
        columns: &[
            ColumnKind::IntegerField,
            ColumnKind::TextField,
            ColumnKind::DecimalExpression,
            ColumnKind::DateExpression,
        ],
        bindings: &[
            BindingKind::RowSection,
            BindingKind::Column(ColumnKind::IntegerField),
            BindingKind::Column(ColumnKind::TextField),
            BindingKind::Column(ColumnKind::DecimalExpression),
            BindingKind::Column(ColumnKind::DateExpression),
        ],
        rows: &[1, 3],
    },
];

/// The BC major and the converter's generator text (`<meta name=\"generator\" content=..>`, which Aspose.Words writes with
/// its version) of a run whose Html equals BC's. The probe's CI ran BC 27.5.46862.58057 and 28.5.54151.58194 (Aspose.Words
/// 26.10.0) and 29.0.54011.55816 (26.8.0, as the pinned 29.0.54011.56262). The runner's render with each of those builds'
/// own assemblies equals BC's SHA-256, length and GetBody SHA-256 in both cells (for 27 and 28 measured locally with
/// `--dotnet-assembly-dir` on those builds' render sets). The pinned 27.5.46862.56249 and 28.5.54151.56267 carry
/// Aspose.Words 26.8.0: their Html has a generator text no cell measured on those majors, and keeps Part A.
#[derive(Debug)]
pub(crate) struct MeasuredBuild {
    pub(crate) major: u16,
    pub(crate) generator: &'static str,
}

pub(crate) const BUILDS: &[MeasuredBuild] = &[
    MeasuredBuild {
        major: 27,
        generator: "Aspose.Words for .NET 26.10.0",
    },
    MeasuredBuild {
        major: 28,
        generator: "Aspose.Words for .NET 26.10.0",
    },
    MeasuredBuild {
        major: 29,
        generator: "Aspose.Words for .NET 26.8.0",
    },
];

/// The parts of the measured layout files (RxWordAlpha.docx and RxWordRgba.docx have the same).
const PARTS: &[&str] = &[
    "[Content_Types].xml",
    "_rels/.rels",
    "customXml/item.xml",
    "customXml/item2.xml",
    "docProps/app.xml",
    "word/_rels/document.xml.rels",
    "word/document.xml",
    "word/settings.xml",
];

/// The element names of the measured layouts' `word/document.xml` (both files together); the values of their attributes
/// and their text are not part of the shape.
const DOCUMENT_ELEMENTS: &[&str] = &[
    "w14:alpha",
    "w14:solidFill",
    "w14:srgbClr",
    "w14:textFill",
    "w15:repeatingSection",
    "w15:repeatingSectionItem",
    "w:alias",
    "w:body",
    "w:color",
    "w:cols",
    "w:dataBinding",
    "w:docGrid",
    "w:document",
    "w:gridCol",
    "w:highlight",
    "w:id",
    "w:p",
    "w:pPr",
    "w:pgMar",
    "w:pgSz",
    "w:r",
    "w:rPr",
    "w:sdt",
    "w:sdtContent",
    "w:sdtPr",
    "w:sectPr",
    "w:shd",
    "w:t",
    "w:tag",
    "w:tbl",
    "w:tblGrid",
    "w:tblPr",
    "w:tblW",
    "w:tc",
    "w:tcPr",
    "w:tcW",
    "w:text",
    "w:tr",
];

/// The element names of the measured layouts' `word/settings.xml`.
const SETTINGS_ELEMENTS: &[&str] = &["w:compat", "w:compatSetting", "w:settings", "w:useFELayout"];

/// Whether every part of the layout and every element of its document and settings parts is one of the measured layouts'.
pub(crate) fn content_is_measured(f: &LayoutFacts) -> bool {
    f.parts.iter().all(|p| PARTS.contains(&p.as_str()))
        && f.document.iter().all(|e| DOCUMENT_ELEMENTS.contains(&e.as_str()))
        && f.settings.iter().all(|e| SETTINGS_ELEMENTS.contains(&e.as_str()))
}

/// Whether `s` equals one row of `table`.
pub(crate) fn shape_is_measured(table: &[MeasuredShape], s: &SaveShape) -> bool {
    table
        .iter()
        .any(|m| m.columns == s.columns.as_slice() && m.bindings == s.bindings.as_slice() && m.rows.contains(&s.rows))
}

/// Whether the run's major and the generator text of `html` equal one row of `table`.
pub(crate) fn build_is_measured(table: &[MeasuredBuild], major: u16, html: &[u8]) -> bool {
    let Some(generator) = generator(html) else {
        return false;
    };
    table.iter().any(|b| b.major == major && b.generator == generator)
}

/// The `content` of the Html's `<meta name="generator" ..>`.
fn generator(html: &[u8]) -> Option<&str> {
    const META: &[u8] = b"<meta name=\"generator\" content=\"";
    let at = html.windows(META.len()).position(|w| w == META)? + META.len();
    let rest = &html[at..];
    std::str::from_utf8(&rest[..rest.iter().position(|b| *b == b'"')?]).ok()
}

/// The facts of a save of report `object` with layout facts `layout` and dataset `dataset`, or `None` when the report or
/// the layout has a fact no measured shape has: more than one data item, a data item without a table, a column without a
/// [`ColumnKind`], a caption, a label, a report extension, a layout not named by DefaultRenderingLayout, or a
/// binding other than a row section over the data item and its columns inside it.
pub(crate) fn save_shape(
    program: &ir::Program,
    object: usize,
    layout: &LayoutFacts,
    dataset: &[u8],
    kinds: &[(String, Option<ValueKind>)],
) -> Option<SaveShape> {
    let o = program.objects.get(object)?;
    let r = o.report.as_ref()?;
    let [item] = r.items.as_slice() else {
        return None;
    };
    let plain = r.extensions.is_empty()
        && r.labels.is_empty()
        && r.caption.is_none()
        && r.default_rendering_layout.is_some()
        && item.parent.is_none();
    if !plain {
        return None;
    }
    let table = program.objects.get(item.table?)?.table.as_ref()?;
    let mut columns = Vec::new();
    let mut tags = Vec::new();
    for c in &item.columns {
        if c.unsupported.is_some() || c.value_unsupported.is_some() || c.include_caption {
            return None;
        }
        let kind = column_kind(table, c, kinds)?;
        columns.push(kind);
        tags.push((crate::reports::word_xml_part::xml_tag(&c.name).ok()?, kind));
    }
    columns.sort();
    let ns = format!(
        "xmlns:ns0='urn:microsoft-dynamics-nav/reports/{}/{}/'",
        crate::reports::word_xml_part::xml_tag(&o.name).ok()?,
        o.id?
    );
    let section = format!(
        "/ns0:NavWordReportXmlPart[1]/ns0:{}",
        crate::reports::word_xml_part::xml_tag(&item.name).ok()?
    );
    let mut bindings = Vec::new();
    for b in &layout.bindings {
        if b.prefixes != ns {
            return None;
        }
        let kind = if b.repeating && b.within.is_empty() && b.xpath == section {
            BindingKind::RowSection
        } else if !b.repeating && b.within == [section.clone()] {
            let (_, kind) = tags
                .iter()
                .find(|(tag, _)| b.xpath == format!("{section}[1]/ns0:{tag}[1]"))?;
            BindingKind::Column(*kind)
        } else {
            return None;
        };
        bindings.push(kind);
    }
    bindings.sort();
    // One data item without children: each of its rows is one `DataItem` element of the dataset (`dataset::Header::write`).
    const ROW: &[u8] = b"<DataItem ";
    let rows = dataset.windows(ROW.len()).filter(|w| *w == ROW).count();
    Some(SaveShape {
        columns,
        bindings,
        rows,
    })
}

/// The [`ColumnKind`] of column `c` of a data item over `table`. An expression's kind is the kind of value it held on every
/// row of the run (`kinds`, from the dataset writer): the expression's static type is not known to the gate.
fn column_kind(table: &ir::Table, c: &ir::ReportColumn, kinds: &[(String, Option<ValueKind>)]) -> Option<ColumnKind> {
    if let Some(field) = c.field {
        let plain = c.decimal_places.is_none() && c.auto_format_type.is_none() && c.auto_format_expression.is_none();
        return match table.fields.get(field)?.ty {
            RtType::Integer if plain => Some(ColumnKind::IntegerField),
            RtType::Text(_) if plain => Some(ColumnKind::TextField),
            _ => None,
        };
    }
    if c.item_field.is_some() || c.decimal_places.is_some() || c.auto_format_expression.is_some() {
        return None;
    }
    let name = crate::dataset::dataset_name(&c.name).ok()?;
    let (_, held) = kinds.iter().find(|(n, _)| *n == name)?;
    match ((*held)?, c.auto_format_type.as_deref().map(str::trim)) {
        (ValueKind::Decimal, Some("1")) => Some(ColumnKind::DecimalExpression),
        (ValueKind::Date, None) => Some(ColumnKind::DateExpression),
        _ => None,
    }
}
