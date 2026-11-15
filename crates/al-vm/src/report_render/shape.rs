//! The measured reports and datasets of the Html of a Word layout, and the vocabulary of the measured layouts (ADR-040
//! amendment 2026-11-15, decisions 1 and 4, and its Notes of 2026-11-15). A save renders real Html only when every input the
//! runner feeds BC's merger and converter equals the probe cells' (owner decision of 2026-11-15, "gate inputs, layout
//! free"): the report, its columns and the row count here, the call in `super::call`, the fact vector in
//! `report_save::html_cell`. The cells are the evidence that BC's own merger and converter, run by the runner on the
//! build's own assemblies, give BC's bytes. Until a corpus bump pins the probe, each pass runs through the assumption
//! `report-html-rendered-by-bc-converter`.
//!
//! The cells (corpus pull request 1415, merged, not pinned; reports/html-email-body/RxHtmlHashTests.al):
//! Hash_WordAlpha_TwoRows (:192, report `RX Word Alpha`, RxHtmlBodyFixtures.al:53) and Hash_WordRgba_TwoRows (:218, report
//! `RX Word Rgba`, RxLayoutSelFixtures.al:73). Each report has one data item over `RX Row` (RxHtmlBodyFixtures.al:12) with
//! the field columns RowId (Integer) and RowTxt (Text), and a hand-built layout named by DefaultRenderingLayout; each save
//! has two rows. Bound_OneRow (:395), Bound_ThreeRows (:433) and Bound_RowCountChangesHash (:471) save report
//! `RX Hash Bound` (:50), whose hand-built RxHashBound.docx binds the two field columns and the expressions `Id * 1234.5`
//! (`AutoFormatType = 1`) and `DMY2Date(1, 2, 2026)`; one and three rows.
//!
//! The cells of corpus pull request 1419 (merged, not pinned; reports/html-email-body/RxHtmlWidenTests.al), each on the
//! Bound report's columns with two rows: Plain_TwoRows (:394, the bound layout), Styles_ArialParagraphStyle (:440),
//! Theme_FontScheme (:482), Numbering_BulletList_NoTabs (:524) and Header_FromSectPr (:568), each the bound layout plus one
//! part; Caption_RxCap_PlainBoundLayout (:614, a report `Caption`, the title). BlankZero_IntegerZero (:664) measures a column
//! over an Integer field with `BlankZero = true`; its layout binds another namespace than the report's, so that save itself
//! keeps Part A (`save_shape`).

use super::layout_facts::LayoutFacts;
use crate::dataset::ValueKind;
use al_builtins::value::RtType;
use al_ir as ir;

/// A report column by its source: a normal table field of the data item, by the field's type and properties, or an
/// expression of the measured syntax, value kind and format properties (RxHtmlHashTests.al:62-63). Any other source has no
/// kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColumnKind {
    /// An Integer field that states no property (`RX Row`.Id, RxHtmlBodyFixtures.al:18; `RX Zero Row`.Qty,
    /// RxHtmlWidenTests.al:43).
    IntegerField,
    /// A Text field that states no property (`RX Row`.Txt, RxHtmlBodyFixtures.al:19).
    TextField,
    /// An expression that held a Decimal on every row, with `AutoFormatType = 1`, no `AutoFormatExpression` and no
    /// `DecimalPlaces`.
    DecimalExpression,
    /// An expression that held a Date on every row, with no format property.
    DateExpression,
    /// An Integer field whose only property is `BlankZero = true` (`RX Zero Row`.QtyField, RxHtmlWidenTests.al:44). The value
    /// is what the dataset writer writes for it; BC renders the 0 it held as `0`, as the column over a field without the
    /// property (BlankZero_IntegerZero, RxHtmlWidenTests.al:705).
    BlankZeroIntegerField,
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
    // Bound_OneRow and Bound_ThreeRows: RowId, RowTxt, Amount and Day, one or three rows; two rows in every cell of
    // RxHtmlWidenTests.al but BlankZero_IntegerZero (Html.Rows(2), :403, :445, :487, :529, :573, :619).
    MeasuredShape {
        columns: &[
            ColumnKind::IntegerField,
            ColumnKind::TextField,
            ColumnKind::DecimalExpression,
            ColumnKind::DateExpression,
        ],
        rows: &[1, 2, 3],
    },
    // BlankZero_IntegerZero (RxHtmlWidenTests.al:664): RowTxt, QtyField (BlankZero) and QtyPlain, one row.
    MeasuredShape {
        columns: &[
            ColumnKind::IntegerField,
            ColumnKind::TextField,
            ColumnKind::BlankZeroIntegerField,
        ],
        rows: &[1],
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

/// The parts of the measured layout files: those of RxWordAlpha.docx, RxWordRgba.docx and RxHashBound.docx (corpus pull
/// request 1415), and the one part each layout of corpus pull request 1419 (merged, not pinned;
/// reports/html-email-body/RxHtmlWidenTests.al) adds to them: a styles part (RxWdStyles.docx, Styles_ArialParagraphStyle
/// :440), a theme part (RxWdTheme.docx, Theme_FontScheme :482), a numbering part (RxWdNumbering.docx,
/// Numbering_BulletList_NoTabs :524) and a header part (RxWdHeader.docx, Header_FromSectPr :568).
const PARTS: &[&str] = &[
    "[Content_Types].xml",
    "_rels/.rels",
    "customXml/item.xml",
    "customXml/item2.xml",
    "docProps/app.xml",
    "word/_rels/document.xml.rels",
    "word/document.xml",
    "word/header1.xml",
    "word/numbering.xml",
    "word/settings.xml",
    "word/styles.xml",
    "word/theme/theme1.xml",
];

/// The relationship types of the measured layouts' `_rels/.rels` and `word/_rels/document.xml.rels`, each with an internal
/// target (no `TargetMode`): the four of corpus pull request 1415's layouts, then the styles, theme, numbering and header
/// relationships of RxWdStyles.docx, RxWdTheme.docx, RxWdNumbering.docx and RxWdHeader.docx (corpus pull request 1419).
const RELATIONSHIPS: &[&str] = &[
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header",
];

/// The element names of the measured layouts' `word/document.xml` (all of them together). Of corpus pull request 1419's
/// layouts: `w:pStyle` (RxWdStyles.docx), `w:rFonts` with theme font attributes (RxWdTheme.docx), `w:numPr`, `w:ilvl` and
/// `w:numId` (RxWdNumbering.docx), `w:headerReference` in `w:sectPr` (RxWdHeader.docx).
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
    "w:headerReference",
    "w:highlight",
    "w:id",
    "w:ilvl",
    "w:numId",
    "w:numPr",
    "w:p",
    "w:pPr",
    "w:pStyle",
    "w:pgMar",
    "w:pgSz",
    "w:r",
    "w:rFonts",
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

/// The element names of RxWdStyles.docx's `word/styles.xml`: document defaults with a font size, and paragraph styles with a
/// name, a base style, `w:qFormat` and run properties of font, bold and size (corpus pull request 1419, :440; the
/// `font-family` values BC writes are the style's font and `'Times New Roman'`, RxHtmlWidenTests.al:473).
const STYLES_ELEMENTS: &[&str] = &[
    "w:b",
    "w:basedOn",
    "w:docDefaults",
    "w:name",
    "w:qFormat",
    "w:rFonts",
    "w:rPr",
    "w:rPrDefault",
    "w:style",
    "w:styles",
    "w:sz",
];

/// The element names of RxWdTheme.docx's `word/theme/theme1.xml`: a color scheme, a font scheme and a format scheme
/// (corpus pull request 1419, :482; the theme fonts the document names resolve to the scheme's typefaces,
/// RxHtmlWidenTests.al:515).
const THEME_ELEMENTS: &[&str] = &[
    "a:accent1",
    "a:accent2",
    "a:accent3",
    "a:accent4",
    "a:accent5",
    "a:accent6",
    "a:bgFillStyleLst",
    "a:clrScheme",
    "a:cs",
    "a:dk1",
    "a:dk2",
    "a:ea",
    "a:effectLst",
    "a:effectStyle",
    "a:effectStyleLst",
    "a:fillStyleLst",
    "a:fmtScheme",
    "a:folHlink",
    "a:fontScheme",
    "a:hlink",
    "a:latin",
    "a:ln",
    "a:lnStyleLst",
    "a:lt1",
    "a:lt2",
    "a:majorFont",
    "a:minorFont",
    "a:schemeClr",
    "a:solidFill",
    "a:srgbClr",
    "a:sysClr",
    "a:theme",
    "a:themeElements",
];

/// The element names of RxWdNumbering.docx's `word/numbering.xml`: one abstract numbering of one bullet level with an
/// indent, and one numbering instance (corpus pull request 1419, :524). No tab stop (`w:tabs`), no label suffix
/// (`w:suff`) and no run properties of the label: BC writes the marker, then nine `&#xa0;` and no tab character
/// (RxHtmlWidenTests.al:560, :563).
const NUMBERING_ELEMENTS: &[&str] = &[
    "w:abstractNum",
    "w:abstractNumId",
    "w:ind",
    "w:lvl",
    "w:lvlJc",
    "w:lvlText",
    "w:multiLevelType",
    "w:num",
    "w:numFmt",
    "w:numbering",
    "w:pPr",
    "w:start",
];

/// The element names of RxWdHeader.docx's `word/header1.xml`: paragraphs of plain runs of text (corpus pull request 1419,
/// :568; BC writes them in a `-aw-headerfooter-type:header-primary` div before the body, RxHtmlWidenTests.al:609).
const HEADER_ELEMENTS: &[&str] = &["w:hdr", "w:p", "w:r", "w:t"];

/// Each XML part under `word/` with the element names the measured layouts use in it.
const PART_ELEMENTS: &[(&str, &[&str])] = &[
    ("word/document.xml", DOCUMENT_ELEMENTS),
    ("word/settings.xml", SETTINGS_ELEMENTS),
    ("word/styles.xml", STYLES_ELEMENTS),
    ("word/theme/theme1.xml", THEME_ELEMENTS),
    ("word/numbering.xml", NUMBERING_ELEMENTS),
    ("word/header1.xml", HEADER_ELEMENTS),
];

/// The namespace declarations of the measured layouts' XML parts under `word/` (prefix, URI): the document and settings
/// parts, and the DrawingML namespace of RxWdTheme.docx's theme part (the styles, numbering and header parts declare only
/// `w`).
const NAMESPACES: &[(&str, &str)] = &[
    ("a", "http://schemas.openxmlformats.org/drawingml/2006/main"),
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

/// Whether the layout is made only of what the measured layouts are made of: their parts, relationship types with internal
/// targets, the element names of each of their XML parts under `word/` ([`PART_ELEMENTS`]), and their namespace
/// declarations (so each prefix of those names stands for the namespace it stands for there). The structure is free (owner
/// decision of 2026-11-15, "gate inputs, layout free"): any number, order and nesting of those elements, any text and
/// attribute values, any content control with or without a binding the reader understands (`save_shape`). What this keeps
/// on Part A includes every layout feature that makes BC's server do more than merge and convert, or that the merger or
/// converter resolves from outside the file (ADR-040, Notes of 2026-11-15): pictures (an image part, `w:drawing`,
/// `w:pict`, `w:object`, a picture control), footer parts and references (`w:footerReference`; BC 29 composes header-footer
/// layouts for a body layout), header parts other than `word/header1.xml`, an external target (`TargetMode="External"`: a
/// linked image, a hyperlink, an attached template), fields that include other content (`w:fldSimple`, `w:instrText`:
/// INCLUDEPICTURE, INCLUDETEXT, LINK), `w:altChunk` and `w:subDoc`, embedded fonts and the font table part, and tabs
/// (`w:tab`, `w:tabs`, `w:ptab`, a numbering label suffix `w:suff`; tabs take their widths from font metrics). BC's
/// converter saves with `HtmlSaveOptions(SaveFormat.Html) { ExportImagesAsBase64 = true }`: `ResolveFontNames` and
/// `ExportFontResources` stay false, so font names go into the CSS as written.
pub(crate) fn content_is_measured(f: &LayoutFacts) -> bool {
    f.parts.iter().all(|p| PARTS.contains(&p.as_str()))
        && f.relationships
            .iter()
            .all(|r| RELATIONSHIPS.contains(&r.kind.as_str()) && !r.external)
        && f.elements.iter().all(|(part, names)| {
            PART_ELEMENTS
                .iter()
                .find(|(p, _)| p == part)
                .is_some_and(|(_, vocabulary)| names.iter().all(|e| vocabulary.contains(&e.as_str())))
        })
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

/// The title BC hands the merger for report `o` (its caption, ADR-040 amendment 2026-11-15, context step 3), when the
/// report's object properties are those of a measured report, by name and value (RxHtmlBodyFixtures.al:55-57,
/// RxLayoutSelFixtures.al:75-77, RxHtmlHashTests.al:52-54): `UsageCategory = None`, `UseRequestPage = false` and
/// `DefaultRenderingLayout` naming its one layout; the title is then the report name (Plain_TwoRows,
/// RxHtmlWidenTests.al:431 with :53). Or those three and `Caption` with a value that is exactly one text literal that is not
/// empty (the lowered value holds no label attribute; `Comment`, `Locked` and `MaxLength` do not change the text): the
/// title is that text (`RX Wd Caption`,
/// RxHtmlWidenTests.al:190; Caption_RxCap_PlainBoundLayout :648). No translation applies: the call is in session language
/// 1033 (`super::call::MEASURED_CALL`) and the gate in `super` requires that the report's app ships no translation file of
/// that language. `None` for every other set (`CaptionML` among them).
fn measured_title(o: &ir::Object, layout: &str) -> Option<Title> {
    let mut props: Vec<(String, &str)> = o
        .properties
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim()))
        .collect();
    props.sort();
    let caption = match props.iter().position(|(k, _)| k == "caption") {
        Some(at) => Some(caption_literal(props.remove(at).1)?),
        None => None,
    };
    let measured = [
        ("defaultrenderinglayout", layout.to_ascii_lowercase()),
        ("usagecategory", "none".to_string()),
        ("userequestpage", "false".to_string()),
    ];
    let three = props.len() == measured.len()
        && props
            .iter()
            .zip(&measured)
            .all(|(p, m)| p.0 == m.0 && p.1.eq_ignore_ascii_case(&m.1));
    three.then(|| match caption {
        Some(text) => Title { text, caption: true },
        None => Title {
            text: o.name.clone(),
            caption: false,
        },
    })
}

/// The title BC hands the merger, and whether it is the report's `Caption` (a text a translation file could change) rather
/// than its name.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Title {
    pub(crate) text: String,
    pub(crate) caption: bool,
}

/// The text of a `Caption` value that is exactly one text literal that is not empty (`'Rx Cap'`, quotes doubled inside);
/// `None` for anything more or less.
fn caption_literal(v: &str) -> Option<String> {
    let text = crate::platform::objects::string_lit(v)?;
    (!text.is_empty() && format!("'{}'", text.replace('\'', "''")) == v).then_some(text)
}

/// The title of report `o` ([`measured_title`]) when it is as the measured reports are, beyond its columns: its object
/// properties ([`measured_title`]); its rendering section
/// declares one layout with only `Type = Word` and `LayoutFile` (so no `Subtype`: a layout of subtype Default, which BC 29
/// does not compose with a header-footer or theme layout, ADR-040 amendment 2026-11-15, context step 4); no request page, no
/// label, no report extension of any kind; one data item over a table, with no `DataItemTableView`, link, request filter
/// fields, `MaxIteration`, `PrintOnlyIfDetail`, `CalcFields` or trigger.
pub(super) fn report_title(program: &ir::Program, o: &ir::Object, r: &ir::Report) -> Option<Title> {
    let [item] = r.items.as_slice() else {
        return None;
    };
    let [layout] = o.layouts.as_slice() else {
        return None;
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
    let title = measured_title(o, &layout.name)?;
    let measured = r.default_rendering_layout.as_deref() == Some(layout.name.as_str())
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
        && item.on_post.is_none();
    measured.then_some(title)
}

/// The facts of a save of report `object` with layout facts `layout` and dataset `dataset`, and the title BC hands the
/// merger, or `None` when the report is not as the measured reports are ([`report_title`]), a column has no
/// [`ColumnKind`], or a content control binds anything but a row section over the data item or one of its columns inside
/// that section.
pub(crate) fn save_shape(
    program: &ir::Program,
    object: usize,
    layout: &LayoutFacts,
    dataset: &[u8],
    kinds: &[(String, Option<ValueKind>)],
) -> Option<(SaveShape, Title)> {
    let o = program.objects.get(object)?;
    let r = o.report.as_ref()?;
    let title = report_title(program, o, r)?;
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
    Some((SaveShape { columns, rows }, title))
}

/// The [`ColumnKind`] of column `c` of a data item over `table`. A field column names a normal field of the data item by
/// itself (not a FlowField or FlowFilter), with no column format property, and the field's properties (a table extension's
/// `modify` included) are none, or for an Integer field only `BlankZero = true`; an expression column has the syntax of the measured `Id * 1234.5` and `DMY2Date(1, 2, 2026)` (operators or a call with an
/// argument list, `SourceSyntax::Other`) and names no field whose properties the dataset reads (no `places_table`, no
/// `caption_table`). An expression's kind is the kind of value it held on every row of the run (`kinds`, from the dataset
/// writer): its static type is not known to the gate.
pub(super) fn column_kind(
    table: &ir::Table,
    c: &ir::ReportColumn,
    kinds: &[(String, Option<ValueKind>)],
) -> Option<ColumnKind> {
    if let Some(field) = c.field {
        let f = table.fields.get(field)?;
        let plain = c.decimal_places.is_none()
            && c.auto_format_type.is_none()
            && c.auto_format_expression.is_none()
            && f.stored
            && f.calc.is_none()
            && f.calc_refusal.is_none()
            && !f.flow_filter;
        let blank_zero = matches!(f.properties.as_slice(), [(k, v)]
            if k.eq_ignore_ascii_case("BlankZero") && v.trim().eq_ignore_ascii_case("true"));
        return match f.ty {
            RtType::Integer if plain && f.properties.is_empty() => Some(ColumnKind::IntegerField),
            RtType::Integer if plain && blank_zero => Some(ColumnKind::BlankZeroIntegerField),
            RtType::Text(_) if plain && f.properties.is_empty() => Some(ColumnKind::TextField),
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
