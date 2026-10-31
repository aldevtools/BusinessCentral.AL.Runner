//! The report tables: Report Layout List, Report Metadata and Report Data Items.
//!
//! Pinned by corpus handlers/TestSourceReportLayoutFlags.al and TestPrecompiledReportLayouts.al (the
//! layouts of a report, from its `rendering` section: their format, `ObsoleteState = Pending` as
//! IsObsolete, `ExcelLayoutMultipleDataSheets` as Default / Single / Multiple) and
//! handlers/TestReportMetadataColumns.al (`ProcessingOnly`, `UseRequestPage`, `WordMergeDataItem`, the
//! table of the first data item; and per data item: name, related table, indentation, the field numbers
//! of `RequestFilterFields` and of the sorting key, and a view that ends the sort direction with
//! `ORDER(1)` when it is descending).
//!
//! The layouts of any report come from the program, shells included (the declaration scan reads them).
//! The data items exist only for a report that compiled; the rows of another are poisoned.

use super::mat::Entry;
use super::objects::{Catalog, al_sema_platform_app_id, ident, prop, string_lit, truthy};
use super::{Providers, blank, ids, option, pos, unknown};
use al_builtins::describe::Describe;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use al_ir::{Object, Table};

fn lprop<'a>(l: &'a al_ir::LayoutDecl, name: &str) -> Option<&'a str> {
    l.properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

const FORMATS: &[&str] = &["RDLC", "Word", "Excel", "Custom"];

/// Report Layout List: "Report ID" (1), Name (2), "Layout Format" (7), IsObsolete (16),
/// ExcelLayoutMultipleDataSheets (17), and the columns `layout_row` cites; the key also holds "Runtime Package ID" (5):
/// a non-null guid other than the app id (platform/lt-ug-report-layout-list/UgReportLayoutList.al:101, :135).
pub(super) fn layout_list_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    // A reportextension's layouts are rows of the report it extends, owned by the extension's app
    // (record/TestEventSubscriptionColumnsReportLayoutListContracts.al:131-165).
    // The layouts are in the objects themselves: only the reports and report extensions are made.
    for o in c
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| matches!(o.kind, al_ir::ObjKind::Report | al_ir::ObjKind::ReportExtension))
        .map(|(i, _)| &c.full[i])
    {
        let id = if o.kind == al_ir::ObjKind::Report {
            o.id
        } else {
            o.extends_id
        };
        let Some(id) = id else { continue };
        let pkg = ids::package_id(c.owner(o));
        for l in &o.layouts {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Integer(id as i32);
            r[pos(t, 2)] = Value::Text(l.name.chars().take(250).collect());
            r[pos(t, 5)] = Value::Guid(pkg);
            // The app that owns the report (the layout is published with it; probe batch 5).
            r[pos(t, 11)] = Value::Guid(c.owner(o));
            match layout_row(&c, o, l, t, &mut r, p.language()) {
                Ok(()) => out.push(Entry::ok(r)),
                Err(why) => {
                    let mut k = blank(t);
                    k[pos(t, 1)] = Value::Integer(id as i32);
                    k[pos(t, 2)] = Value::Text(l.name.chars().take(250).collect());
                    k[pos(t, 5)] = Value::Guid(pkg);
                    out.push(Entry::poisoned(k, why));
                }
            }
        }
    }
    out
}

/// "Report Name" (12) of a layout of report `o`: its name when it declares no Caption and no CaptionML, its Caption when it
/// declares one; a CaptionML is not pinned.
fn report_name(t: &TableDef, o: &Object, what: &str) -> Value {
    if prop(o, "CaptionML").is_some() {
        return unknown(t, 12, &format!("{what}, whose report declares a CaptionML"));
    }
    match prop(o, "Caption") {
        None => Value::Text(o.name.chars().take(250).collect()),
        Some(v) => match string_lit(v) {
            Some(caption) => Value::Text(caption.chars().take(250).collect()),
            None => unknown(t, 12, &format!("{what}, whose report's Caption is not a literal")),
        },
    }
}

/// The Report Layout List rows of the user-defined layouts: one per stored Tenant Report Layout row (`tenant`, its schema;
/// `stored`, its rows). Pinned for an RDLC layout of a report (platform/lt-ug-report-layout-list/UgReportLayoutList.al
/// Y17_ReportLayoutList_UserDefinedRow, corpus master 97c29f86): Caption the layout's name, CaptionML blank, Runtime Package
/// ID and Application ID the null guid, Custom Type the null guid, MIME Type blank, Description as stored, Report Name as
/// for a declared layout, Layout Publisher blank, User Defined Yes, ReportIsInstalled Yes, IsObsolete No,
/// ExcelLayoutMultipleDataSheets Default, Layout Status Approved, Layout Subtype Default and blank parts. Any other column, and
/// every column of a layout of another format or of a report the program does not hold, is unknown.
pub(super) fn user_layout_rows(p: &Providers, t: &TableDef, tenant: &TableDef, stored: &[al_data::Row]) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let col = |name: &str| tenant.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    let (Some(report_col), Some(name_col), Some(format_col)) = (col("Report ID"), col("Name"), col("Layout Format")) else {
        return Vec::new();
    };
    let description_col = col("Description");
    let has = |id: u32| t.fields.iter().any(|f| f.id == id);
    stored
        .iter()
        .map(|row| {
            let mut r = blank(t);
            let name: String = match &row[name_col] {
                Value::Text(v) | Value::Code(v) => v.to_string(),
                _ => String::new(),
            };
            let report_id = match row[report_col] {
                Value::Integer(id) => id,
                _ => 0,
            };
            r[pos(t, 1)] = Value::Integer(report_id);
            r[pos(t, 2)] = Value::Text(name.chars().take(250).collect());
            r[pos(t, 5)] = Value::Guid([0; 16]);
            let whose = format!("the user-defined layout '{name}' of report {report_id}");
            let report = c
                .objects
                .iter()
                .enumerate()
                .find(|(_, o)| o.kind == al_ir::ObjKind::Report && o.id == Some(report_id as u32))
                .map(|(i, _)| &c.full[i]);
            let rdlc = matches!(row[format_col], Value::Option(0, _));
            let Some(o) = report.filter(|_| rdlc) else {
                for f in &t.fields {
                    if ![1, 2, 5].contains(&f.id) {
                        r[pos(t, f.id)] = unknown(t, f.id, &format!("{whose}, not an RDLC layout of a report of the program"));
                    }
                }
                return Entry::ok(r);
            };
            r[pos(t, 3)] = Value::Text(name.chars().take(250).collect());
            r[pos(t, 6)] = unknown(t, 6, &whose);
            r[pos(t, 7)] = option(t, 7, 0);
            r[pos(t, 10)] = match description_col.map(|d| &row[d]) {
                Some(Value::Text(v) | Value::Code(v)) => Value::Text(v.chars().take(250).collect()),
                _ => unknown(t, 10, &whose),
            };
            r[pos(t, 11)] = Value::Guid([0; 16]);
            r[pos(t, 12)] = report_name(t, o, &whose);
            r[pos(t, 14)] = Value::Boolean(true);
            r[pos(t, 15)] = Value::Boolean(true);
            for (id, member) in [(18, 0), (19, 0)] {
                if has(id) {
                    r[pos(t, id)] = match &t.fields[pos(t, id)].ty {
                        RtType::Enum(e) => Value::Enum(*e, member),
                        _ => unknown(t, id, &whose),
                    };
                }
            }
            Entry::ok(r)
        })
        .collect()
}

fn layout_row(
    c: &Catalog,
    o: &Object,
    l: &al_ir::LayoutDecl,
    t: &TableDef,
    r: &mut al_data::Row,
    language: i32,
) -> Result<(), String> {
    let what = || format!("layout '{}' of report '{}'", l.name, o.name);
    // BC 29: "Layout Subtype" (19), HeaderFooterPart (20), ThemePart (21)
    // (report/lt-y-report-props/YReportParts.al:75-80). The enum is HeaderFooter 1, Theme 2, Body 3; the parts a layout
    // does not declare are blank. The corpus pins only layouts that declare a Subtype: for one that declares none the
    // three columns stay unknown (a read through a RecordRef is refused, the rest of the row reads), and static reads of
    // them are refused where they are lowered (not in PINNED_COLUMNS).
    // A layout that declares no Subtype reads Default with blank parts (platform/lt-ug-report-layout-list/UgReportLayoutList.al:
    // 119-121, :153-155; platform/lt-aw-fresh-tenant/AwTests.al:228, all RDLC, Word or Excel layouts of a report).
    if t.fields.iter().any(|f| f.id == 19) {
        'subtype: {
            let subtype = match lprop(l, "Subtype").map(ident) {
                None => {
                    r[pos(t, 19)] = match &t.fields[pos(t, 19)].ty {
                        RtType::Enum(e) => Value::Enum(*e, 0),
                        other => return Err(format!("field 19 is a {}, not an enum", other.describe())),
                    };
                    for id in [20, 21] {
                        r[pos(t, id)] = Value::default_for(&t.fields[pos(t, id)].ty);
                    }
                    break 'subtype;
                }
                Some(s) if s.eq_ignore_ascii_case("HeaderFooter") => 1,
                Some(s) if s.eq_ignore_ascii_case("Theme") => 2,
                Some(s) if s.eq_ignore_ascii_case("Body") => 3,
                Some(s) => return Err(format!("{}: Subtype `{s}` is not pinned", what())),
            };
            r[pos(t, 19)] = match &t.fields[pos(t, 19)].ty {
                RtType::Enum(e) => Value::Enum(*e, subtype),
                other => return Err(format!("field 19 is a {}, not an enum", other.describe())),
            };
            for (id, name) in [(20, "HeaderFooterPart"), (21, "ThemePart")] {
                // A part the layout does not declare is blank (YReportParts.al:80).
                r[pos(t, id)] = Value::default_for(&t.fields[pos(t, id)].ty);
                let Some(v) = lprop(l, name) else { continue };
                let part = ident(v);
                // The value is the id of the declaring app, `::` and the layout's name; only a layout of the same object is pinned.
                if !o.layouts.iter().any(|x| x.name.eq_ignore_ascii_case(&part)) {
                    return Err(format!(
                        "{}: {name} `{part}` is not a layout of the same object (not pinned)",
                        what()
                    ));
                }
                let app = c
                    .apps
                    .get(o.app as usize)
                    .filter(|a| !a.id.eq_ignore_ascii_case(al_sema_platform_app_id()))
                    .ok_or_else(|| format!("{}: the app that declares it is not known", what()))?;
                r[pos(t, id)] = Value::Text(format!("{}::{part}", app.id.to_ascii_lowercase()).into());
            }
        }
    }
    let format = lprop(l, "Type").ok_or_else(|| format!("{}: no Type", what()))?;
    let format = FORMATS
        .iter()
        .position(|f| f.eq_ignore_ascii_case(&ident(format)))
        .ok_or_else(|| format!("{}: Type `{format}` is not pinned", what()))?;
    r[pos(t, 7)] = option(t, 7, format as i32);
    // "User Defined" (14): No for an RDLC, a Word and an Excel layout declared in AL (platform/lt-ug-report-layout-list/
    // UgReportLayoutList.al:112, :146; platform/lt-aw-fresh-tenant/AwTests.al:228). A Custom layout is not pinned.
    let pinned_format = format <= 2;
    r[pos(t, 14)] = if pinned_format {
        Value::Boolean(false)
    } else {
        unknown(t, 14, &format!("{}, a {} layout", what(), FORMATS[format]))
    };
    // "Custom Type" (8) the null guid and "MIME Type" (9) blank for an RDLC, Word and Excel layout (UgReportLayoutList.al:105-106,
    // :139-140; AwTests.al:228). A Custom layout is not pinned.
    if !pinned_format {
        for id in [8, 9] {
            r[pos(t, id)] = unknown(t, id, &format!("{}, a Custom layout", what()));
        }
    }
    // The rest is pinned for the RDLC, Word and Excel layouts of a report (UgReportLayoutList.al:94-150, AwTests.al:220-300),
    // not for the layouts of a report extension.
    let of_report = o.kind == al_ir::ObjKind::Report;
    let whose = |why: &str| format!("{}, {why}", what());
    // CaptionML (4): blank for a layout that declares none (UgReportLayoutList.al:103, :137; AwTests.al:228).
    if lprop(l, "CaptionML").is_some() {
        r[pos(t, 4)] = unknown(t, 4, &whose("which declares a CaptionML"));
    }
    // "Report Name" (12): the name of a report that declares no Caption and no CaptionML (UgReportLayoutList.al:110, :144;
    // AwTests.al:228), the Caption of one that declares it (UgReportLayoutList.al Y17_ReportLayoutList_ReportName_WithCaption,
    // corpus master 97c29f86: 'LT Caption Differs'). A CaptionML is not pinned.
    r[pos(t, 12)] = if of_report {
        report_name(t, o, &what())
    } else {
        unknown(t, 12, &whose("a layout of a report extension"))
    };
    // "Layout Publisher" (13): the app's name, " by " and its publisher (UgReportLayoutList.al:111, :145; AwTests.al:228:
    // `AL Language Coverage Tests by AL Language`, the corpus app's name and publisher).
    r[pos(t, 13)] = match c.apps.get(o.app as usize) {
        Some(app) if of_report => Value::Text(format!("{} by {}", app.name, app.publisher).chars().take(250).collect()),
        _ => unknown(t, 13, &whose("a layout of a report extension")),
    };
    // ReportIsInstalled (15): Yes (UgReportLayoutList.al:113, :147; AwTests.al:228).
    r[pos(t, 15)] = if of_report {
        Value::Boolean(true)
    } else {
        unknown(t, 15, &whose("a layout of a report extension"))
    };
    // "Layout Status" (18, BC 28+): Approved for a layout without a Subtype (UgReportLayoutList.al:117, :151; AwTests.al:228).
    // A layout part, a Custom layout and a layout of a report extension are not pinned.
    if t.fields.iter().any(|f| f.id == 18) {
        if !(of_report && pinned_format && lprop(l, "Subtype").is_none()) {
            r[pos(t, 18)] = unknown(
                t,
                18,
                &whose("a layout part, a Custom layout or a layout of a report extension"),
            );
        } else {
            r[pos(t, 18)] = match &t.fields[pos(t, 18)].ty {
                RtType::Enum(e) => Value::Enum(*e, 0),
                other => return Err(format!("field 18 is a {}, not an enum", other.describe())),
            };
        }
    }
    // Caption is the Caption literal, else the layout's name; Description the Summary literal, else the layout's name.
    let literal = |name: &str| -> Result<Option<String>, String> {
        lprop(l, name)
            .map(|v| string_lit(v).ok_or_else(|| format!("{}: {name} `{v}` is not a literal", what())))
            .transpose()
    };
    // A layout without a Caption reads its name (session/PWPlatformTables.al:126-139).
    r[pos(t, 3)] = Value::Text(
        literal("Caption")?
            .unwrap_or_else(|| l.name.clone())
            .chars()
            .take(250)
            .collect(),
    );
    // `SummaryML`: the entry of the session's language, ENU when it has none (report/lt-y-report-props/YReportPropsTests.al:
    // SummaryML_English, SummaryML_German, SummaryML_FrenchFallsBack); other languages are not pinned.
    let summary_ml = match lprop(l, "SummaryML") {
        None => None,
        Some(ml) => {
            let code = match language {
                1033 => "ENU",
                1031 => "DEU",
                1036 => "FRA",
                n => {
                    return Err(format!(
                        "{}: SummaryML in language {n} (only 1033, 1031 and 1036 are pinned)",
                        what()
                    ));
                }
            };
            let entry = super::objects::ml_entry(ml, code).map_err(|e| format!("{}: SummaryML {e}", what()))?;
            match entry {
                Some(e) => Some(e),
                None if code == "FRA" => {
                    super::objects::ml_entry(ml, "ENU").map_err(|e| format!("{}: SummaryML {e}", what()))?
                }
                None => {
                    return Err(format!(
                        "{}: SummaryML without a {code} entry (the fallback is not pinned)",
                        what()
                    ));
                }
            }
        }
    };
    r[pos(t, 10)] = Value::Text(
        literal("Summary")?
            .or(summary_ml)
            .unwrap_or_else(|| l.name.clone())
            .chars()
            .take(250)
            .collect(),
    );
    // Pending is obsolete (TestPrecompiledReportLayouts.al:63-72); Removed is not pinned.
    r[pos(t, 16)] = Value::Boolean(match lprop(l, "ObsoleteState").map(ident) {
        None => false,
        Some(s) if s.eq_ignore_ascii_case("No") => false,
        Some(s) if s.eq_ignore_ascii_case("Pending") => true,
        Some(s) => return Err(format!("{}: ObsoleteState `{s}` is not pinned", what())),
    });
    // The enum "Excel Sheet Configuration": Default, Single Data sheet, Multiple data sheets.
    let sheets = match lprop(l, "ExcelLayoutMultipleDataSheets") {
        None => 0,
        Some(v) if truthy(v) => 2,
        Some(v) if v.eq_ignore_ascii_case("false") => 1,
        Some(v) => return Err(format!("{}: ExcelLayoutMultipleDataSheets `{v}`", what())),
    };
    r[pos(t, 17)] = match &t.fields[pos(t, 17)].ty {
        RtType::Enum(e) => Value::Enum(*e, sheets),
        other => return Err(format!("field 17 is a {}, not an enum", other.describe())),
    };
    Ok(())
}

/// Report Metadata: ID (1), UseRequestPage (4), ProcessingOnly (9), WordMergeDataItem (17),
/// FirstDataItemTableID (18). Needs the data items, so a report nothing compiled is poisoned.
pub(super) fn report_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    // The data items are in the reports themselves: only the reports are made.
    for o in c
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind == al_ir::ObjKind::Report)
        .map(|(i, _)| &c.full[i])
    {
        let Some(id) = o.id else { continue };
        let mut key = blank(t);
        key[pos(t, 1)] = Value::Integer(id as i32);
        let Some(rep) = &o.report else {
            out.push(Entry::poisoned(
                key,
                format!("report '{}' was not compiled, so its data items are not known", o.name),
            ));
            continue;
        };
        let mut r = key.clone();
        r[pos(t, 2)] = Value::Text(o.name.clone().into());
        // Caption: the Caption literal, else the English CaptionML, else the name (metadata/lt-report-metadata-columns/
        // ReportMetadata.al:P175_Meta_Rdlc_Part1, _Multi_, _ProcessingOnly_ and _Word_).
        match super::objects::declared_caption(prop(o, "Caption"), prop(o, "CaptionML"), &o.name) {
            Ok(c) => r[pos(t, 3)] = Value::Text(c.chars().take(250).collect()),
            Err(why) => {
                out.push(Entry::poisoned(key, format!("report '{}': {why}", o.name)));
                continue;
            }
        }
        // UseRequestPage defaults to true, ProcessingOnly to false (TestReportMetadataColumns.al:30-60).
        r[pos(t, 4)] = Value::Boolean(prop(o, "UseRequestPage").is_none_or(truthy));
        r[pos(t, 9)] = Value::Boolean(prop(o, "ProcessingOnly").is_some_and(truthy));
        r[pos(t, 17)] = Value::Text(prop(o, "WordMergeDataItem").map(ident).unwrap_or_default().into());
        // DefaultLayoutName: the layout `DefaultRenderingLayout` names, else the file the Word, RDLC or Excel layout property gives, as
        // written (metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:ReportMetadata_DefaultLayoutName_OwnAndBase:
        // `RdlcFixture`, `StandardSalesInvoice.docx`, `./System/Notifications/NotificationEmail.docx`, blank for a report with neither).
        // DefaultLayout: the kind the property names (`Word` for report 1306), RDLC when it names none.
        let rendering_kind = rep
            .default_rendering_layout
            .as_ref()
            .and_then(|n| rep.rendering_layouts.iter().find(|l| l.name.eq_ignore_ascii_case(n)))
            .map(|l| l.kind.clone());
        let kind_text = prop(o, "DefaultLayout").map(ident).or(rendering_kind);
        if let (Some(col), Some(kind)) = (t.fields.iter().position(|x| x.name == "DefaultLayout"), kind_text) {
            if let Value::Option(_, members) = &r[col] {
                if let Some(n) = members.iter().position(|m| m.eq_ignore_ascii_case(&kind)) {
                    r[col] = Value::Option(n as i32, members.clone());
                }
            }
        }
        if let Some(col) = t.fields.iter().position(|x| x.name == "DefaultLayoutName") {
            let name = prop(o, "DefaultRenderingLayout")
                .map(ident)
                .or_else(|| {
                    ["WordLayout", "RDLCLayout", "ExcelLayout"]
                        .iter()
                        .find_map(|n| prop(o, n))
                        .and_then(string_lit)
                })
                .unwrap_or_default();
            r[col] = Value::Text(name.into());
        }
        // EnableExternalImages (6), the paper sources (12-14), FormatRegion (24) and ExcelLayoutMultipleDataSheets (25) are the
        // properties as declared (report/lt-y-report-props/YReportPropsTests.al:34-160: `EnableExternalImages=Yes`, `PaperSourceFirstPage=1`
        // for Upper, 2 for Lower, 3 for Middle, `FormatRegion=de-DE`, `ExcelLayoutMultipleDataSheets=Yes`; 0, No and blank when not declared).
        // Another paper source is not pinned: the row is refused.
        let mut unpinned = None;
        for (col, name) in [(6, "EnableExternalImages"), (25, "ExcelLayoutMultipleDataSheets")] {
            if t.fields.iter().any(|f| f.id == col) {
                if let Value::Boolean(_) = r[pos(t, col)] {
                    r[pos(t, col)] = Value::Boolean(prop(o, name).is_some_and(truthy));
                }
            }
        }
        for (col, name) in [
            (12, "PaperSourceFirstPage"),
            (13, "PaperSourceDefaultPage"),
            (14, "PaperSourceLastPage"),
        ] {
            let Some(v) = prop(o, name) else { continue };
            let n = match ident(v).to_ascii_lowercase().as_str() {
                "upper" => 1,
                "lower" => 2,
                "middle" => 3,
                other => {
                    unpinned = Some(format!(
                        "report '{}': {name} = {other} (only Upper, Lower and Middle are pinned)",
                        o.name
                    ));
                    continue;
                }
            };
            if t.fields.iter().any(|f| f.id == col) {
                match &r[pos(t, col)] {
                    Value::Integer(_) => r[pos(t, col)] = Value::Integer(n),
                    Value::Option(_, m) => r[pos(t, col)] = Value::Option(n, m.clone()),
                    Value::Enum(e, _) => r[pos(t, col)] = Value::Enum(*e, n),
                    other => unpinned = Some(format!("report '{}': column {col} is a {}", o.name, other.describe())),
                }
            }
        }
        if t.fields.iter().any(|f| f.id == 24) {
            if let Some(v) = prop(o, "FormatRegion") {
                match string_lit(v) {
                    Some(region) => r[pos(t, 24)] = Value::Text(region.into()),
                    None => unpinned = Some(format!("report '{}': FormatRegion = {v}", o.name)),
                }
            }
        }
        if let Some(why) = unpinned {
            out.push(Entry::poisoned(key, why));
            continue;
        }
        // TransactionType (11): UpdateNoLocks for a report that declares none (metadata/lt-report-metadata-columns/
        // ReportMetadata.al:210, :236, :262; ReportMetadataWord.al:70). No corpus test pins the column for a declared
        // TransactionType: it stays unknown there, and a read of it is refused.
        if let Some(col) = t.fields.iter().position(|f| f.id == 11) {
            r[col] = match prop(o, "TransactionType") {
                None => match &r[col] {
                    Value::Option(_, m) => match m.iter().position(|x| x.eq_ignore_ascii_case("UpdateNoLocks")) {
                        Some(n) => Value::Option(n as i32, m.clone()),
                        None => unknown(
                            t,
                            11,
                            &format!("report '{}': TransactionType has no member UpdateNoLocks", o.name),
                        ),
                    },
                    other => unknown(
                        t,
                        11,
                        &format!("report '{}': TransactionType is a {}", o.name, other.describe()),
                    ),
                },
                Some(v) => unknown(
                    t,
                    11,
                    &format!("report '{}', which declares TransactionType = {}", o.name, ident(v)),
                ),
            };
        }
        // EnableExternalAssemblies (8): No for a report that declares none (report/lt-y-report-props/YReportPropsTests.al:45,
        // YHelper.al:226). A declared one (OnPrem only) is pinned by no corpus test: unknown, so a read of it is refused.
        if let (Some(col), Some(v)) = (
            t.fields.iter().position(|f| f.id == 8),
            prop(o, "EnableExternalAssemblies"),
        ) {
            r[col] = unknown(
                t,
                8,
                &format!(
                    "report '{}', which declares EnableExternalAssemblies = {}",
                    o.name,
                    ident(v)
                ),
            );
        }
        // UseSystemPrinter (5): the property as declared, Yes for true and No for false (misc/lt-y13-triage/Y13ReportSystemPrinter.al:
        // 530-543, corpus master). For a report that does not declare it no corpus test pins the column: unknown.
        if let Some(col) = t.fields.iter().position(|f| f.id == 5) {
            r[col] = match prop(o, "UseSystemPrinter") {
                Some(v) if truthy(v) => Value::Boolean(true),
                Some(v) if v.trim().eq_ignore_ascii_case("false") => Value::Boolean(false),
                Some(v) => unknown(
                    t,
                    5,
                    &format!("report '{}', which declares UseSystemPrinter = {}", o.name, v.trim()),
                ),
                None => unknown(
                    t,
                    5,
                    &format!("report '{}', which does not declare UseSystemPrinter", o.name),
                ),
            };
        }
        // BC 29: DefaultHeaderFooterPartName (30) and DefaultThemePartName (31) are the layout names the properties give
        // (report/lt-y-report-props/YReportParts.al:65); blank when the report declares none.
        for (col, name) in [(30, "DefaultHeaderFooterPart"), (31, "DefaultThemePart")] {
            if t.fields.iter().any(|f| f.id == col) {
                r[pos(t, col)] = Value::Text(prop(o, name).map(ident).unwrap_or_default().into());
            }
        }
        let first = rep
            .items
            .iter()
            .find(|i| i.parent.is_none())
            .and_then(|i| i.table)
            .and_then(|t| c.objects[t].id);
        match first {
            Some(tid) => r[pos(t, 18)] = Value::Integer(tid as i32),
            None if rep.items.is_empty() => {}
            None => {
                out.push(Entry::poisoned(
                    key,
                    format!("report '{}': the table of its first data item is not compiled", o.name),
                ));
                continue;
            }
        }
        out.push(Entry::ok(r));
    }
    out
}

/// `sorting(a, b)`, `order(...)` and `where(...)` of a view, each with the text between its brackets.
fn view_segments(view: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let b: Vec<char> = view.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i].is_alphanumeric() {
            i += 1;
        }
        let word: String = b[start..i].iter().collect::<String>().to_ascii_lowercase();
        while i < b.len() && b[i].is_whitespace() {
            i += 1;
        }
        if word.is_empty() || b.get(i) != Some(&'(') {
            return Err(format!("view `{view}`"));
        }
        let (mut depth, mut quoted) = (0, false);
        let open = i;
        while i < b.len() {
            match b[i] {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if i >= b.len() {
            return Err(format!("view `{view}`"));
        }
        out.push((word, b[open + 1..i].iter().collect::<String>()));
        i += 1;
    }
    Ok(out)
}

/// The field numbers a comma separated list of field names names, in order.
fn field_numbers(table: &Table, list: &str) -> Result<Vec<u32>, String> {
    list.split(',')
        .map(|n| {
            let n = ident(n);
            table
                .fields
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case(&n))
                .map(|f| f.id)
                .ok_or_else(|| format!("`{n}` is not a field of the table"))
        })
        .collect()
}

fn join(ids: &[u32]) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Report Data Items: "Report ID" (1), "Data Item ID" (2), Name (3), "Request Filter Fields" (4),
/// "Related Table ID" (5), "Indentation Level" (6), "Sorting Fields" (7), "Data Item Table View" (8).
/// The id is the absolute value of the 32-bit FNV-1a hash of the item's name in UTF-16 (little endian), read as a signed
/// number: `A` 1792377636, `D` 120712215 (|-120712215|), `Kid` 1167510851, `Src` 1369927887 (misc/lt-probes-needed-3/PnC21Metadata.al:
/// 201-249), and the rows come in the order of the ids: Rw, Rw3, Rw2 (report/lt-y-report-props/YReportPropsTests.al:272).
pub(super) fn report_data_item_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    for (oi, _) in c
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind == al_ir::ObjKind::Report)
    {
        // The data items are in the report itself: only the reports are made.
        let o = &c.full[oi];
        let Some(id) = o.id else { continue };
        let Some(rep) = &o.report else {
            let mut k = blank(t);
            k[pos(t, 1)] = Value::Integer(id as i32);
            out.push(Entry::poisoned_prefix(
                k,
                1,
                format!("report '{}' was not compiled, so its data items are not known", o.name),
            ));
            continue;
        };
        for (n, item) in rep.items.iter().enumerate() {
            let Some(item_id) = data_item_id(&item.name) else {
                let mut k = blank(t);
                k[pos(t, 1)] = Value::Integer(id as i32);
                out.push(Entry::poisoned_prefix(
                    k,
                    1,
                    format!("data item '{}' of report '{}': its id is not pinned", item.name, o.name),
                ));
                continue;
            };
            let mut key = blank(t);
            key[pos(t, 1)] = Value::Integer(id as i32);
            key[pos(t, 2)] = Value::Integer(item_id);
            let mut r = key.clone();
            // `modify(Item) { RequestFilterFields = ...; }` of a reportextension adds to the item's list
            // (report/lt-y-report-props/YReportExt.al:231). Two extensions that do so is not pinned.
            let ext_fields: Vec<&String> = c
                .objects
                .iter()
                .enumerate()
                .filter(|(_, e)| e.report_ext_base == Some(oi))
                .filter_map(|(ei, _)| c.full[ei].report_ext.as_ref().filter(|x| x.base == oi))
                .flat_map(|x| x.modifies.iter().filter(|m| m.item == n))
                .filter_map(|m| m.request_filter_fields.as_ref())
                .collect();
            let result = if ext_fields.len() > 1 {
                Err(format!(
                    "data item '{}' of report '{}': two reportextensions set RequestFilterFields (the merge is not pinned)",
                    item.name, o.name
                ))
            } else {
                match ext_fields.first() {
                    Some(list) if item.request_filter_fields.is_none() => {
                        let mut with = item.clone();
                        with.request_filter_fields = Some((*list).clone());
                        data_item_row(&c, o, rep, &with, t, &mut r)
                    }
                    Some(_) => Err(format!(
                        "data item '{}' of report '{}': a reportextension sets RequestFilterFields on an item that has them (the merge is not pinned)",
                        item.name, o.name
                    )),
                    None => data_item_row(&c, o, rep, item, t, &mut r),
                }
            };
            match result {
                Ok(()) => out.push(Entry::ok(r)),
                Err(why) => out.push(Entry::poisoned(key, why)),
            }
        }
    }
    out
}

/// `Data Item ID`: see `report_data_item_rows`. `None` for the one hash whose absolute value overflows (not pinned).
fn data_item_id(name: &str) -> Option<i32> {
    let mut h: u32 = 0x811c_9dc5;
    for unit in name.encode_utf16() {
        for b in unit.to_le_bytes() {
            h ^= u32::from(b);
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    (h as i32).checked_abs()
}

fn data_item_row(
    c: &Catalog,
    o: &Object,
    rep: &al_ir::Report,
    item: &al_ir::ReportItem,
    t: &TableDef,
    r: &mut al_data::Row,
) -> Result<(), String> {
    let what = || format!("data item '{}' of report '{}'", item.name, o.name);
    let table_obj = item
        .table
        .map(|i| &c.objects[i])
        .ok_or_else(|| format!("{}: its table is not compiled", what()))?;
    let table = table_obj
        .table
        .as_ref()
        .ok_or_else(|| format!("{}: its table was not compiled", what()))?;
    r[pos(t, 3)] = Value::Text(item.name.clone().into());
    r[pos(t, 5)] = Value::Integer(table_obj.id.unwrap_or(0) as i32);
    let mut depth = 0;
    let mut at = item.parent;
    while let Some(parent) = at {
        depth += 1;
        at = rep.items[parent].parent;
    }
    r[pos(t, 6)] = Value::Integer(depth);
    // The field numbers of RequestFilterFields in declaration order ('2,5').
    let filter_fields = match &item.request_filter_fields {
        None => String::new(),
        Some(list) => join(&field_numbers(table, list).map_err(|e| format!("{}: RequestFilterFields {e}", what()))?),
    };
    r[pos(t, 4)] = Value::Text(filter_fields.into());
    // The sorting key's field numbers in the key's own order ('5,2'), and the view BC formats from the
    // view: SORTING(FieldN,..) then ORDER(1) when descending, then WHERE for a single `field = const(v)`.
    let mut sorting = String::new();
    let mut view = Vec::new();
    if let Some(text) = &item.view {
        let mut where_text = None;
        let mut descending = false;
        let mut ascending = false;
        for (word, inner) in view_segments(text).map_err(|e| format!("{}: {e}", what()))? {
            match word.as_str() {
                "sorting" => {
                    let ids = field_numbers(table, &inner).map_err(|e| format!("{}: sorting {e}", what()))?;
                    sorting = join(&ids);
                    view.push(format!(
                        "SORTING({})",
                        ids.iter().map(|i| format!("Field{i}")).collect::<Vec<_>>().join(",")
                    ));
                }
                "order" if inner.trim().eq_ignore_ascii_case("descending") => descending = true,
                // An explicit `order(ascending)` writes ORDER(0) (ScReportMetadataViews.al:89).
                "order" if inner.trim().eq_ignore_ascii_case("ascending") => ascending = true,
                "order" => return Err(format!("{}: order({}) is not pinned", what(), inner.trim())),
                "where" => where_text = Some(inner),
                other => return Err(format!("{}: `{other}` in the view is not pinned", what())),
            }
        }
        if descending {
            view.push("ORDER(1)".into());
        } else if ascending {
            view.push("ORDER(0)".into());
        }
        if let Some(w) = where_text {
            view.push(where_const(table, c.full, &w).map_err(|e| format!("{}: where({w}): {e}", what()))?);
        }
    }
    r[pos(t, 7)] = Value::Text(sorting.into());
    r[pos(t, 8)] = Value::Text(view.join(" ").into());
    Ok(())
}

/// `WHERE(Field1=0(1))` for the single `field = const(value)` condition of an Integer field: the Report Data Items
/// table writes the filter type number (0 for a const) in front of the value, unlike a page's source table view
/// (report/lt-y-report-props/YReportExt.al:231, misc/lt-probes-needed-3/PnC21Metadata.al:206, :218). On a Code field
/// `const('')` is `0()` and `filter('A*|B')` is `1(A*|B)` (report/lt-sc-report-metadata-views/ScReportMetadataViews.al:88,
/// :90). Other values and field types are not pinned for this table (an option member's ordinal or name, an enum, text).
fn where_const(table: &Table, _objects: &al_ir::Objects, inner: &str) -> Result<String, String> {
    let (field, rest) = inner.split_once('=').ok_or("not a field = const(value)")?;
    let name = ident(field);
    let f = table
        .fields
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(&name))
        .ok_or("no such field")?;
    where_condition(f.id, &f.ty, rest)
}

/// `WHERE(...)` for field `id` of type `ty` and the condition text after `=` (see `where_const`).
fn where_condition(id: u32, ty: &RtType, rest: &str) -> Result<String, String> {
    let rest = rest.trim();
    let lower = rest.to_ascii_lowercase();
    let (kind, value) = match (lower.strip_prefix("const("), lower.strip_prefix("filter(")) {
        (Some(_), _) => (0, rest[6..].strip_suffix(')')),
        (_, Some(_)) => (1, rest[7..].strip_suffix(')')),
        _ => (0, None),
    };
    let value = value.ok_or("only const(value) or filter(text) is pinned")?.trim();
    // A single-quoted literal without inner quotes.
    let literal = value
        .strip_prefix('\'')
        .and_then(|v| v.strip_suffix('\''))
        .filter(|v| !v.contains('\''));
    match (ty, kind) {
        (RtType::Integer | RtType::BigInteger, 0) if value.parse::<i64>().is_ok() => {
            Ok(format!("WHERE(Field{id}=0({value}))"))
        }
        (RtType::Code(_), 0) if literal == Some("") => Ok(format!("WHERE(Field{id}=0())")),
        (RtType::Code(_), 1) if literal.is_some_and(|l| !l.is_empty()) => {
            Ok(format!("WHERE(Field{id}=1({}))", literal.unwrap_or_default()))
        }
        _ => Err("only a number on an Integer field, const('') and a filter literal on a Code field are pinned".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_splits_into_its_words_and_bracketed_texts() {
        let v = view_segments("sorting(\"No.\", Name) ORDER(descending) where(Kind = const(B), \"A (b)\" = const(1))")
            .unwrap();
        assert_eq!(
            v,
            vec![
                ("sorting".to_string(), "\"No.\", Name".to_string()),
                ("order".to_string(), "descending".to_string()),
                ("where".to_string(), "Kind = const(B), \"A (b)\" = const(1)".to_string()),
            ]
        );
        assert!(view_segments("sorting(a").is_err());
        assert!(view_segments("nonsense").is_err());
    }
}

#[cfg(test)]
mod data_item_id_tests {
    // misc/lt-probes-needed-3/PnC21Metadata.al:204-249.
    #[test]
    fn the_data_item_id_is_the_fnv_hash_of_the_name() {
        let ids: Vec<i32> = ["A", "B", "C", "D", "E", "Kid", "Src"]
            .iter()
            .map(|n| super::data_item_id(n).unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                1792377636, 589499809, 1227196426, 120712215, 758408832, 1167510851, 1369927887
            ]
        );
    }
}

#[cfg(test)]
mod view_tests {
    use al_builtins::value::RtType;

    // report/lt-sc-report-metadata-views/ScReportMetadataViews.al:86-90.
    #[test]
    fn the_where_of_a_code_field_writes_const_blank_and_a_filter_literal() {
        assert_eq!(
            super::where_condition(1, &RtType::Code(Some(20)), "const('')").unwrap(),
            "WHERE(Field1=0())"
        );
        assert_eq!(
            super::where_condition(1, &RtType::Code(Some(20)), "filter('A*|B')").unwrap(),
            "WHERE(Field1=1(A*|B))"
        );
        assert_eq!(
            super::where_condition(3, &RtType::Integer, "const(5)").unwrap(),
            "WHERE(Field3=0(5))"
        );
        // Not pinned: another const of a Code field, a filter of an Integer field.
        assert!(super::where_condition(1, &RtType::Code(Some(20)), "const('X')").is_err());
        assert!(super::where_condition(3, &RtType::Integer, "filter('1..5')").is_err());
    }

    #[test]
    fn the_data_item_ids_of_the_views_probe() {
        let ids: Vec<i32> = ["EmptyCode", "Ascending", "Filtered"]
            .iter()
            .map(|n| super::data_item_id(n).unwrap())
            .collect();
        assert_eq!(ids, [57155483, 1027492269, 1968063604]);
    }
}
