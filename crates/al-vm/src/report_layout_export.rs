//! `Media.ExportStream` of the Layout of a Report Layout List row (`Report.LayoutListExportStream#1`, al-sema call.rs).
//!
//! The bytes are those of the layout's file as the app's folder holds it, the answer true, the row unchanged (HasValue stays
//! No), the same bytes on a second export (reports/layouts-saveas/Y17LayoutExportTests.al:70 RDLC, :88 Word, :129 Excel,
//! :146 twice). The layout is one of the report's rendering section, one of its legacy RDLCLayout and WordLayout properties
//! (Y17LayoutExportTests.al:105), or one a reportextension declares in its rendering section: a Word, RDLC or Excel
//! layout, a name with a dot in it, on an own report and on Base Application's report 1306 (reports/layouts-saveas/
//! Y67LayoutExtExport.al:134 Export_ExtensionLayouts_OwnReport, :187 Export_ExtensionLayout_BaseApplicationReport).
//!
//! The compiler rewrites the data set section of a layout file at every build and the package holds the rewritten bytes
//! (Y17LayoutExportObjects.al:6-8); the corpus files are as the compiler left them, so the folder file is what BC exports.
//! A legacy RDLCLayout or WordLayout file of an app folder exports only when it is as the compiler writes it for the report
//! (`legacy_file_as_compiled`); a legacy ExcelLayout file is refused (no corpus case exports one). A Word file of a rendering
//! section or a reportextension whose data set part names another report than the one it belongs to was not written by a build
//! of this report (the Y67 files name report 93617, BC exported the rewritten bytes): it is refused, since the rewrite is not
//! reproduced.

use crate::{Session, Signal};
use al_builtins::native::unsupported;
use al_builtins::value::Value;
use al_ir as ir;
use std::io::Read;

/// A layout found for a row: the app that holds its file, its name, the file's path in the app, its type (`word`, ...) and
/// the report object (its index) when a legacy RDLCLayout, WordLayout or ExcelLayout property of the report names it.
struct Found {
    app: usize,
    name: String,
    path: String,
    kind: String,
    legacy: Option<usize>,
}

impl Session {
    pub(crate) fn report_layout_list_export(
        &mut self,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let Some(Value::Record(row)) = receiver else {
            return unsupported("Media.ExportStream of a Report Layout List Layout without its row");
        };
        let t = self.table(&row)?;
        let field = |id: u32| t.fields.iter().position(|f| f.id == id);
        let (report_id, name) = {
            let r = row.borrow();
            let id = field(1).and_then(|p| match r.fields.get(p) {
                Some(Value::Integer(n)) => Some(*n),
                _ => None,
            });
            let name = field(2).and_then(|p| match r.fields.get(p) {
                Some(Value::Text(n) | Value::Code(n)) => Some(n.to_string()),
                _ => None,
            });
            (id, name)
        };
        let engine = self.engine.clone();
        let found = report_id.zip(name.as_deref()).and_then(|(id, name)| {
            own_layout(&engine.program, id, name).or_else(|| extension_layout(&engine.program, id, name))
        });
        let Some(found) = found else {
            return unsupported(format!(
                "Media.ExportStream of the Layout of a Report Layout List row that names no RDLC, Word or Excel layout of a report's rendering section or of a reportextension's (report {}, layout '{}'; not pinned)",
                report_id.unwrap_or(0),
                name.unwrap_or_default()
            ));
        };
        let app = engine.program.apps.get(found.app);
        let held = app
            .and_then(|a| a.layout_files.iter().find(|(p, _)| *p == found.path))
            .map(|(_, b)| std::borrow::Cow::Borrowed(b.as_slice()));
        // A Microsoft app read from its package keeps its layout files in the package and serves one when it is asked for.
        // Microsoft's own tests need it (MailServiceTest.Codeunit.al: `CustomReportLayout.InitBuiltInLayout(ReportID,
        // Type::Word)`, and many more in Tests-Report/ReportLayoutTest): nothing pins that BC exports the source file's bytes
        // unchanged, since the compiler may rewrite the file at build (Y17LayoutExportTests.al:5-9), so the path is flagged.
        let served = held.is_none().then(|| {
            let bytes = app?.layout_source.as_ref()?.0.file(&found.path)?;
            al_builtins::assumptions::assume(
                al_builtins::assumptions::Assumption::PackageLayoutFileIsTheCompiledLayout,
            );
            Some(std::borrow::Cow::Owned(bytes))
        });
        let from_folder = held.is_some();
        let Some(bytes) = held.or(served.flatten()) else {
            if found.legacy.is_some() && found.kind == "excel" {
                return unsupported(format!(
                    "Media.ExportStream of the layout '{}' of the legacy ExcelLayout property of report {}: the runner does not read the file of this property from an app's folder (no corpus case exports one; not pinned)",
                    found.name,
                    report_id.unwrap_or(0)
                ));
            }
            return unsupported(format!(
                "Media.ExportStream of the Layout of a Report Layout List row: the layout file {} of layout '{}' is not held by its app (the runner read no file at the path as written under the app's folder, and no package of the app serves it)",
                found.path, found.name
            ));
        };
        let bytes: &[u8] = &bytes;
        if let Some(object) = found.legacy.filter(|_| from_folder)
            && !self.legacy_file_as_compiled(object, report_id.unwrap_or(0), &found.kind, bytes)
        {
            return unsupported(format!(
                "Media.ExportStream of the {} layout '{}' of report {}: its file is not shown to be as the compiler writes it for the report, and the compiler rewrites it at build (the rewritten bytes are not reproduced)",
                found.kind,
                found.name,
                report_id.unwrap_or(0)
            ));
        }
        if found.kind == "word"
            && let Some(other) = word_part_report(bytes).filter(|id| Some(*id) != report_id)
        {
            return unsupported(format!(
                "Media.ExportStream of the Word layout '{}' of report {}: its file's data set part names report {other}, so the compiler rewrites it at build (the rewritten bytes are not reproduced)",
                found.name,
                report_id.unwrap_or(0)
            ));
        }
        match args.first() {
            Some(Value::Library(_, h)) => match &*h.borrow() {
                al_builtins::value::LibraryValue::OutStream(blob, ..) => al_builtins::stream::write_bytes(blob, bytes),
                _ => {
                    return unsupported(
                        "Media.ExportStream of a Report Layout List Layout into something not an OutStream",
                    );
                }
            },
            _ => {
                return unsupported(
                    "Media.ExportStream of a Report Layout List Layout into something not an OutStream",
                );
            }
        }
        Ok(Value::Boolean(true))
    }

    /// Whether `bytes`, the folder file of a legacy RDLCLayout or WordLayout property of report `report_id` (object `object`), is as the
    /// compiler writes it for the report, so that the rewrite at build leaves it unchanged and it is what the package holds
    /// (Y17LayoutExportObjects.al:6-8; Y17LayoutExportTests.al:105-124 exports the files alc wrote, unchanged).
    /// RDLC: its one DataSets element is the one Y17LayoutExportTests.al:83 shows (`rdlc_data_sets_as_compiled`) and its
    /// Fields list names the report's columns as `Report.RdlcLayout` writes them (rdlc_fields.rs: the rewrite gives the same
    /// bytes). Word: its `customXml` entries are `customXml/item.xml` and `customXml/item2.xml` and nothing else
    /// (Y17LayoutExportTests.al:124), the plain and the extended data set part the compiler writes for the report
    /// (`compiled_word_part`). Any other kind, and any shape these readers refuse, is not shown: false.
    fn legacy_file_as_compiled(&mut self, object: usize, report_id: i32, kind: &str, bytes: &[u8]) -> bool {
        match kind {
            "rdlc" => {
                std::str::from_utf8(bytes).is_ok_and(rdlc_data_sets_as_compiled)
                    && self
                        .rdlc_stream_with_dataset_fields(object, bytes)
                        .is_ok_and(|b| b.as_deref() == Some(bytes))
            }
            "word" => {
                let Some(entries) = zip_entries(bytes) else {
                    return false;
                };
                let parts: Vec<&(String, Vec<u8>)> =
                    entries.iter().filter(|(n, _)| n.starts_with("customXml/")).collect();
                let part = |name: &str| parts.iter().find(|(n, _)| n == name).and_then(|(_, d)| utf16_text(d));
                let (Some(plain), Some(extended)) = (part("customXml/item.xml"), part("customXml/item2.xml")) else {
                    return false;
                };
                parts.len() == 2
                    && self.compiled_word_part(report_id, false).is_ok_and(|t| t == plain)
                    && self.compiled_word_part(report_id, true).is_ok_and(|t| t == extended)
            }
            _ => false,
        }
    }
}

/// Whether the one `<DataSets>` element of the RDLC `text` is the one the compiler writes (Y17LayoutExportTests.al:83, the
/// file whose bytes :7 gives for Y17LayR2.rdl too): one
/// `DataSet_Result` data set with a non-empty Fields list and the query of the `DataSource` data source, CRLF line ends,
/// two spaces per level from the indent of `<DataSets>`.
fn rdlc_data_sets_as_compiled(text: &str) -> bool {
    let found: Vec<usize> = text.match_indices("<DataSets").map(|(i, _)| i).collect();
    let [at] = found[..] else { return false };
    let Some(len) = text[at..].find("</DataSets>") else {
        return false;
    };
    let block = &text[at..at + len + "</DataSets>".len()];
    let i = &text[text[..at].rfind('\n').map_or(0, |p| p + 1)..at];
    let head = format!("<DataSets>\r\n{i}  <DataSet Name=\"DataSet_Result\">\r\n{i}    <Fields>\r\n");
    let tail = format!(
        "\r\n{i}    </Fields>\r\n{i}    <Query>\r\n{i}      <DataSourceName>DataSource</DataSourceName>\r\n{i}      <CommandText />\r\n{i}    </Query>\r\n{i}  </DataSet>\r\n{i}</DataSets>"
    );
    block.len() > head.len() + tail.len() && block.starts_with(&head) && block.ends_with(&tail)
}

/// The bytes of the Word layout report object `object` renders with when none is named, as its app holds the file: the layout
/// `DefaultRenderingLayout` names, or with `DefaultLayout = Word` the file of its `WordLayout` property (own_layout).
/// `None` for any other default, or a file the app does not hold (a Microsoft app read from its package).
pub(crate) fn default_word_layout(program: &ir::Program, object: usize) -> Option<Vec<u8>> {
    let o = program.objects.get(object)?;
    let r = o.report.as_ref()?;
    let id = o.id? as i32;
    let name = match &r.default_rendering_layout {
        Some(name) => name.clone(),
        None => {
            let prop = |n: &str| {
                o.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| v)
            };
            if !prop("DefaultLayout").is_some_and(|v| v.trim().eq_ignore_ascii_case("Word")) {
                return None;
            }
            crate::recmeta::string_literal(prop("WordLayout")?)?
        }
    };
    let found = own_layout(program, id, &name).filter(|f| f.kind == "word")?;
    let app = program.apps.get(found.app)?;
    app.layout_files
        .iter()
        .find(|(p, _)| *p == found.path)
        .map(|(_, b)| b.clone())
}

/// A layout of the rendering section of report `id` by its name, or one of the legacy RDLCLayout, WordLayout and ExcelLayout
/// properties by its LayoutFile as written (Y17LayoutExportTests.al:105 Export_LegacyProperties: the row of an RDLCLayout and
/// a WordLayout, named with the file as written; the export of an ExcelLayout file is refused).
fn own_layout(program: &ir::Program, id: i32, name: &str) -> Option<Found> {
    let (object, o) = program
        .objects
        .iter()
        .enumerate()
        .find(|(_, o)| o.kind == ir::ObjKind::Report && o.report.is_some() && o.id.map(|i| i as i32) == Some(id))?;
    let r = o.report.as_ref()?;
    if let Some(l) = r
        .rendering_layouts
        .iter()
        .find(|l| l.name == name && ["rdlc", "word", "excel"].contains(&l.kind.as_str()))
    {
        return Some(Found {
            app: o.app as usize,
            name: l.name.clone(),
            path: l.path.clone(),
            kind: l.kind.clone(),
            legacy: None,
        });
    }
    o.properties.iter().find_map(|(k, v)| {
        let kind = [("RDLCLayout", "rdlc"), ("WordLayout", "word"), ("ExcelLayout", "excel")]
            .iter()
            .find(|(p, _)| k.eq_ignore_ascii_case(p))?
            .1;
        let file = crate::recmeta::string_literal(v).filter(|f| f == name)?;
        Some(Found {
            app: o.app as usize,
            name: file.clone(),
            path: layout_file_path(&file),
            kind: kind.to_string(),
            legacy: Some(object),
        })
    })
}

/// A Word, RDLC or Excel layout named `name` that a reportextension of report `id` declares in its rendering section: its file
/// is held by the extension's app (Y67LayoutExtExport.al:134-185, :187-200).
fn extension_layout(program: &ir::Program, id: i32, name: &str) -> Option<Found> {
    program
        .objects
        .iter()
        .filter(|o| o.kind == ir::ObjKind::ReportExtension && o.extends_id.map(|i| i as i32) == Some(id))
        .find_map(|o| {
            let l = o.layouts.iter().find(|l| l.name == name)?;
            let prop = |n: &str| {
                l.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| v.trim().to_string())
            };
            let kind = prop("Type")?.to_ascii_lowercase();
            if !["rdlc", "word", "excel"].contains(&kind.as_str()) {
                return None;
            }
            let file = crate::recmeta::string_literal(&prop("LayoutFile")?)?;
            Some(Found {
                app: o.app as usize,
                name: l.name.clone(),
                path: layout_file_path(&file),
                kind,
                legacy: None,
            })
        })
}

/// `file` (a `LayoutFile` as written) relative to the app folder: `/` separators, `.` and `..` resolved.
pub(crate) fn layout_file_path(file: &str) -> String {
    let file = file.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for c in file.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

/// The report id the data set part (`customXml/item*.xml`, UTF-16 `NavWordReportXmlPart`) of a Word file names in its namespace
/// (`urn:microsoft-dynamics-nav/reports/<name>/<id>/`), or `None` when the file has no such part or is not a zip archive.
fn word_part_report(bytes: &[u8]) -> Option<i32> {
    for (name, data) in zip_entries(bytes)? {
        if !(name.starts_with("customXml/item") && name.ends_with(".xml")) {
            continue;
        }
        let text = utf16_text(&data)?;
        let at = text.find("urn:microsoft-dynamics-nav/reports/")?;
        let ns = &text[at..];
        let ns = &ns[..ns.find('"')?];
        let id = ns.trim_end_matches('/').rsplit('/').next()?;
        return id.parse().ok();
    }
    None
}

fn utf16_text(data: &[u8]) -> Option<String> {
    let body = data.strip_prefix(&[0xFF, 0xFE])?;
    let units: Vec<u16> = body.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16(&units).ok()
}

/// The entries of a zip archive read through its central directory: (name, uncompressed bytes) of the stored and deflated
/// entries. `None` when the bytes are not a zip archive this reader understands.
pub(crate) fn zip_entries(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let u16_at = |i: usize| bytes.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |i: usize| {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let end = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| u32_at(i) == Some(0x0605_4b50))?;
    let count = u16_at(end + 10)?;
    let mut at = u32_at(end + 16)?;
    let mut out = Vec::new();
    for _ in 0..count {
        if u32_at(at)? != 0x0201_4b50 {
            return None;
        }
        let method = u16_at(at + 10)?;
        let size = u32_at(at + 20)?;
        let (n, m, k) = (u16_at(at + 28)?, u16_at(at + 30)?, u16_at(at + 32)?);
        let local = u32_at(at + 42)?;
        let name = String::from_utf8_lossy(bytes.get(at + 46..at + 46 + n)?).into_owned();
        at += 46 + n + m + k;
        let start = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
        let raw = bytes.get(start..start + size)?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut d = Vec::new();
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut d).ok()?;
                d
            }
            _ => continue,
        };
        out.push((name, data));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_resolved_from_the_app_root() {
        assert_eq!(layout_file_path("./report/x/../y/A.docx"), "report/y/A.docx");
        assert_eq!(layout_file_path("a\\b.rdl"), "a/b.rdl");
    }

    #[test]
    fn a_file_that_is_no_zip_names_no_report() {
        assert_eq!(word_part_report(b"not a zip"), None);
    }
}
