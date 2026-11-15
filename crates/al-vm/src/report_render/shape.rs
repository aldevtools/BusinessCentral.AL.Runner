//! The measured reports and datasets of the Html of a Word layout, and the vocabulary of the measured layouts (ADR-040
//! amendment 2026-11-15, decisions 1 and 4, and its Notes of 2026-11-15). A save renders real Html only when every input the
//! runner feeds BC's merger and converter equals the probe cells' (owner decision of 2026-11-15, "gate inputs, layout
//! free"): the report, its columns and the row count here, the call in `super::call`, the fact vector in
//! `report_save::html_cell`. The cells are the evidence that BC's own merger and converter, run by the runner on the
//! build's own assemblies, give BC's bytes; the pinned corpus holds them.
//!
//! The cells (pinned corpus, reports/html-email-body/RxHtmlHashTests.al):
//! Hash_WordAlpha_TwoRows (:192, report `RX Word Alpha`, RxHtmlBodyFixtures.al:53) and Hash_WordRgba_TwoRows (:218, report
//! `RX Word Rgba`, RxLayoutSelFixtures.al:73). Each report has one data item over `RX Row` (RxHtmlBodyFixtures.al:12) with
//! the field columns RowId (Integer) and RowTxt (Text), and a hand-built layout named by DefaultRenderingLayout; each save
//! has two rows. Bound_OneRow (:395), Bound_ThreeRows (:433) and Bound_RowCountChangesHash (:471) save report
//! `RX Hash Bound` (:50), whose hand-built RxHashBound.docx binds the two field columns and the expressions `Id * 1234.5`
//! (`AutoFormatType = 1`) and `DMY2Date(1, 2, 2026)`; one and three rows.

use super::layout_facts::LayoutFacts;
use crate::dataset::ValueKind;
use al_builtins::value::RtType;
use al_ir as ir;

/// A report column by its source: a table field of the data item, by the field's type, or an expression of the measured
/// syntax, value kind and format properties (RxHtmlHashTests.al:62-63). Any other source has no kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColumnKind {
    IntegerField,
    TextField,
    /// An expression that held a Decimal on every row, with `AutoFormatType = 1`, no `AutoFormatExpression` and no
    /// `DecimalPlaces`.
    DecimalExpression,
    /// An expression that held a Date on every row, with no format property.
    DateExpression,
}

/// The facts of one save the gate compares with [`SHAPES`]: the dataset the merger gets. The layout's structure is not
/// compared (owner decision of 2026-11-15, "gate inputs, layout free"; `content_is_measured` keeps what is not structure).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SaveShape {
    /// The columns of the report's one data item, sorted.
    pub(crate) columns: Vec<ColumnKind>,
    /// The rows of the data item in the dataset.
    pub(crate) rows: usize,
}

/// One measured dataset: the save's facts must equal these.
#[derive(Debug)]
pub(crate) struct MeasuredShape {
    pub(crate) columns: &'static [ColumnKind],
    pub(crate) rows: &'static [usize],
}

pub(crate) const SHAPES: &[MeasuredShape] = &[
    // Hash_WordAlpha_TwoRows and Hash_WordRgba_TwoRows: RowId and RowTxt, two rows.
    MeasuredShape {
        columns: &[ColumnKind::IntegerField, ColumnKind::TextField],
        rows: &[2],
    },
    // Bound_OneRow and Bound_ThreeRows: RowId, RowTxt, Amount and Day, one or three rows.
    MeasuredShape {
        columns: &[
            ColumnKind::IntegerField,
            ColumnKind::TextField,
            ColumnKind::DecimalExpression,
            ColumnKind::DateExpression,
        ],
        rows: &[1, 3],
    },
];

/// The BC major and the converter's version (Aspose.Words, which writes it into the Html as
/// `<meta name="generator" content="Aspose.Words for .NET <version>">`) of a run whose Html equals BC's. The probe's CI ran BC
/// 27.5.46862.58057 and 28.5.54151.58194 (Aspose.Words 26.10.0) and 29.0.54011.55816 (26.8.0, as the pinned 29.0.54011.56262).
/// The runner's render with each of those builds' own assemblies equals BC's SHA-256, length and GetBody SHA-256 in every cell
/// (for 27 and 28 measured locally with `--dotnet-assembly-dir` on those builds' render sets). The pinned 27.5.46862.56249 and
/// 28.5.54151.56267 carry Aspose.Words 26.8.0, a version no cell measured on those majors: their saves keep Part A.
#[derive(Debug)]
pub(crate) struct MeasuredBuild {
    pub(crate) major: u16,
    pub(crate) converter: [u16; 3],
}

pub(crate) const BUILDS: &[MeasuredBuild] = &[
    MeasuredBuild {
        major: 27,
        converter: [26, 10, 0],
    },
    MeasuredBuild {
        major: 28,
        converter: [26, 10, 0],
    },
    MeasuredBuild {
        major: 29,
        converter: [26, 8, 0],
    },
];

/// The parts of the measured layout files (RxWordAlpha.docx, RxWordRgba.docx and RxHashBound.docx have the same).
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

/// The relationship types of the measured layouts' `_rels/.rels` and `word/_rels/document.xml.rels`, each with an internal
/// target (no `TargetMode`).
const RELATIONSHIPS: &[&str] = &[
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
];

/// The element names of the measured layouts' `word/document.xml` (the three files together).
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

/// The namespace declarations of the measured layouts' document and settings parts (prefix, URI).
const NAMESPACES: &[(&str, &str)] = &[
    ("m", "http://schemas.openxmlformats.org/officeDocument/2006/math"),
    ("mc", "http://schemas.openxmlformats.org/markup-compatibility/2006"),
    ("o", "urn:schemas-microsoft-com:office:office"),
    (
        "r",
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    ),
    ("sl", "http://schemas.openxmlformats.org/schemaLibrary/2006/main"),
    ("v", "urn:schemas-microsoft-com:vml"),
    ("w", "http://schemas.openxmlformats.org/wordprocessingml/2006/main"),
    ("w10", "urn:schemas-microsoft-com:office:word"),
    ("w14", "http://schemas.microsoft.com/office/word/2010/wordml"),
    ("w15", "http://schemas.microsoft.com/office/word/2012/wordml"),
    ("wne", "http://schemas.microsoft.com/office/word/2006/wordml"),
    (
        "wp",
        "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
    ),
    (
        "wp14",
        "http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing",
    ),
    (
        "wpc",
        "http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas",
    ),
    (
        "wpg",
        "http://schemas.microsoft.com/office/word/2010/wordprocessingGroup",
    ),
    ("wpi", "http://schemas.microsoft.com/office/word/2010/wordprocessingInk"),
    (
        "wps",
        "http://schemas.microsoft.com/office/word/2010/wordprocessingShape",
    ),
];

/// The element names of the measured layouts' `word/settings.xml`.
const SETTINGS_ELEMENTS: &[&str] = &["w:compat", "w:compatSetting", "w:settings", "w:useFELayout"];

/// Whether the layout is made only of what the measured layouts are made of: their parts, relationship types with internal
/// targets, the element names of their document and settings parts, and their namespace declarations (so each prefix of
/// those names stands for the namespace it stands for there). The structure is free (owner decision of
/// 2026-11-15, "gate inputs, layout free"): any number, order and nesting of those elements, any text and attribute values,
/// any content control with or without a binding the reader understands (`save_shape`). What this keeps on Part A includes
/// every layout feature that makes BC's server do more than merge and convert, or that the merger or converter resolves
/// from outside the file (ADR-040, Notes of 2026-11-15): pictures (an image part, `w:drawing`, `w:pict`, `w:object`, a
/// picture control), header and footer parts (`w:headerReference`, `w:footerReference`), an external target
/// (`TargetMode="External"`: a linked image, a hyperlink, an attached template), fields that include other content
/// (`w:fldSimple`, `w:instrText`: INCLUDEPICTURE, INCLUDETEXT, LINK), `w:altChunk` and `w:subDoc`, embedded fonts and the
/// font, style, numbering and theme parts (no probe cell measured them), and tabs (`w:tab`; tabs and numbering labels take
/// their widths from font metrics). BC's converter saves with `HtmlSaveOptions(SaveFormat.Html) { ExportImagesAsBase64 =
/// true }`: `ResolveFontNames` and `ExportFontResources` stay false, so font names go into the CSS as written.
pub(crate) fn content_is_measured(f: &LayoutFacts) -> bool {
    f.parts.iter().all(|p| PARTS.contains(&p.as_str()))
        && f.relationships
            .iter()
            .all(|r| RELATIONSHIPS.contains(&r.kind.as_str()) && !r.external)
        && f.document.iter().all(|e| DOCUMENT_ELEMENTS.contains(&e.as_str()))
        && f.settings.iter().all(|e| SETTINGS_ELEMENTS.contains(&e.as_str()))
        && f.namespaces
            .iter()
            .all(|(p, u)| NAMESPACES.contains(&(p.as_str(), u.as_str())))
}

/// Whether `s` equals one row of `table`.
pub(crate) fn shape_is_measured(table: &[MeasuredShape], s: &SaveShape) -> bool {
    table
        .iter()
        .any(|m| m.columns == s.columns.as_slice() && m.rows.contains(&s.rows))
}

/// Whether the converter assembly's version (`[major, minor, build, revision]`) is the one a cell measured on `major`: read
/// before the render, so a save on a build no cell measured does not render only to drop the Html (`build_is_measured`
/// still decides on the Html itself).
pub(crate) fn converter_is_measured(table: &[MeasuredBuild], major: u16, version: [u16; 4]) -> bool {
    table
        .iter()
        .any(|b| b.major == major && [b.converter[0], b.converter[1], b.converter[2], 0] == version)
}

/// Whether the run's major and the converter version the generator text of `html` names equal one row of `table`.
pub(crate) fn build_is_measured(table: &[MeasuredBuild], major: u16, html: &[u8]) -> bool {
    let Some(generator) = generator(html) else {
        return false;
    };
    table.iter().any(|b| {
        let [x, y, z] = b.converter;
        b.major == major && generator == format!("Aspose.Words for .NET {x}.{y}.{z}")
    })
}

/// The `content` of the Html's `<meta name="generator" ..>`.
fn generator(html: &[u8]) -> Option<&str> {
    const META: &[u8] = b"<meta name=\"generator\" content=\"";
    let at = html.windows(META.len()).position(|w| w == META)? + META.len();
    let rest = &html[at..];
    std::str::from_utf8(&rest[..rest.iter().position(|b| *b == b'"')?]).ok()
}

/// The object properties of each measured report by name and value (RxHtmlBodyFixtures.al:55-57, RxLayoutSelFixtures.al:75-77,
/// RxHtmlHashTests.al:52-54): `UsageCategory = None`, `UseRequestPage = false` and `DefaultRenderingLayout` naming its one
/// layout. Neither `Caption` nor `CaptionML`: BC passes the report's caption to the merger as the title (ADR-040 amendment
/// 2026-11-15, context step 3), which for these reports is the name the runner passes; no translation applies in the
/// session language 1033 of the call (`super::call::MEASURED_CALL`; ADR-053 step 1).
fn properties_are_measured(o: &ir::Object, layout: &str) -> bool {
    let mut props: Vec<(String, String)> = o
        .properties
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_ascii_lowercase()))
        .collect();
    props.sort();
    let measured = [
        ("defaultrenderinglayout", layout.to_ascii_lowercase()),
        ("usagecategory", "none".to_string()),
        ("userequestpage", "false".to_string()),
    ];
    props.len() == measured.len() && props.iter().zip(&measured).all(|(p, m)| p.0 == m.0 && p.1 == m.1)
}

/// Whether report `o` is as the measured reports are, beyond its columns: [`properties_are_measured`]; its rendering section
/// declares one layout with only `Type = Word` and `LayoutFile` (so no `Subtype`: a layout of subtype Default, which BC 29
/// does not compose with a header-footer or theme layout, ADR-040 amendment 2026-11-15, context step 4); no request page, no
/// label, no report extension of any kind; one data item over a table, with no `DataItemTableView`, link, request filter
/// fields, `MaxIteration`, `PrintOnlyIfDetail`, `CalcFields` or trigger.
fn report_is_measured(program: &ir::Program, o: &ir::Object, r: &ir::Report) -> bool {
    let [item] = r.items.as_slice() else {
        return false;
    };
    let [layout] = o.layouts.as_slice() else {
        return false;
    };
    let mut layout_props: Vec<String> = layout.properties.iter().map(|(k, _)| k.to_ascii_lowercase()).collect();
    layout_props.sort();
    let word = layout
        .properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Type") && v.trim().eq_ignore_ascii_case("Word"));
    let extended = program
        .objects
        .iter()
        .any(|x| x.kind == ir::ObjKind::ReportExtension && x.extends_id.is_some() && x.extends_id == o.id);
    properties_are_measured(o, &layout.name)
        && r.default_rendering_layout.as_deref() == Some(layout.name.as_str())
        && layout_props == ["layoutfile", "type"]
        && word
        && r.rendering_layouts.len() == 1
        && r.use_request_page == Some(false)
        && !r.has_request_page
        && r.labels.is_empty()
        && r.extensions.is_empty()
        && !extended
        && r.format_region.is_none()
        && item.parent.is_none()
        && item.table.is_some()
        && item.view.is_none()
        && item.link.is_empty()
        && item.link_item.is_none()
        && item.request_filter_fields.is_none()
        && item.max_iteration.is_none()
        && !item.print_only_if_detail
        && item.calc_fields.is_empty()
        && item.on_pre.is_none()
        && item.on_after_get.is_none()
        && item.on_post.is_none()
}

/// The facts of a save of report `object` with layout facts `layout` and dataset `dataset`, or `None` when the report is
/// not as the measured reports are ([`report_is_measured`]), a column has no [`ColumnKind`], or a content control binds
/// anything but a row section over the data item or one of its columns inside that section.
pub(crate) fn save_shape(
    program: &ir::Program,
    object: usize,
    layout: &LayoutFacts,
    dataset: &[u8],
    kinds: &[(String, Option<ValueKind>)],
) -> Option<SaveShape> {
    let o = program.objects.get(object)?;
    let r = o.report.as_ref()?;
    if !report_is_measured(program, o, r) {
        return None;
    }
    let item = r.items.first()?;
    let table = program.objects.get(item.table?)?.table.as_ref()?;
    let mut columns = Vec::new();
    let mut tags = Vec::new();
    for c in &item.columns {
        if c.unsupported.is_some()
            || c.value_unsupported.is_some()
            || c.include_caption
            || c.option_caption_ml.is_some()
        {
            return None;
        }
        columns.push(column_kind(table, c, kinds)?);
        tags.push(crate::reports::word_xml_part::xml_tag(&c.name).ok()?);
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
    let understood = layout.bindings.iter().all(|b| {
        b.prefixes == ns
            && ((b.repeating && b.within.is_empty() && b.xpath == section)
                || (!b.repeating
                    && b.within == [section.clone()]
                    && tags.iter().any(|tag| b.xpath == format!("{section}[1]/ns0:{tag}[1]"))))
    });
    if !understood {
        return None;
    }
    // One data item without children: each of its rows is one `DataItem` element of the dataset (`dataset::Header::write`).
    const ROW: &[u8] = b"<DataItem ";
    let rows = dataset.windows(ROW.len()).filter(|w| *w == ROW).count();
    Some(SaveShape { columns, rows })
}

/// The [`ColumnKind`] of column `c` of a data item over `table`. A field column names a field of the data item by itself; an
/// expression column has the syntax of the measured `Id * 1234.5` and `DMY2Date(1, 2, 2026)` (operators or a call with an
/// argument list, `SourceSyntax::Other`) and names no field whose properties the dataset reads (no `places_table`, no
/// `caption_table`). An expression's kind is the kind of value it held on every row of the run (`kinds`, from the dataset
/// writer): its static type is not known to the gate.
fn column_kind(table: &ir::Table, c: &ir::ReportColumn, kinds: &[(String, Option<ValueKind>)]) -> Option<ColumnKind> {
    if let Some(field) = c.field {
        let plain = c.decimal_places.is_none() && c.auto_format_type.is_none() && c.auto_format_expression.is_none();
        return match table.fields.get(field)?.ty {
            RtType::Integer if plain => Some(ColumnKind::IntegerField),
            RtType::Text(_) if plain => Some(ColumnKind::TextField),
            _ => None,
        };
    }
    let expression = c.source_syntax == ir::SourceSyntax::Other
        && c.item_field.is_none()
        && c.places_table.is_none()
        && c.places_field.is_none()
        && c.caption_table.is_none()
        && c.caption_field.is_none()
        && c.decimal_places.is_none()
        && c.auto_format_expression.is_none();
    if !expression {
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
