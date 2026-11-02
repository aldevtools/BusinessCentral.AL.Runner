//! `Media.ExportStream` of the Layout of a Report Layout List row (`Report.LayoutListExportStream#1`, al-sema call.rs).
//!
//! The bytes are those of the layout's file as the app's folder holds it, the answer true, the row unchanged (HasValue stays
//! No), the same bytes on a second export (report/lt-y17-layout-export/Y17LayoutExportTests.al:70 RDLC, :88 Word, :129 Excel,
//! :146 twice). The layout is one of the report's rendering section, one of its legacy RDLCLayout, WordLayout and ExcelLayout
//! properties (Y17LayoutExportTests.al:105), or one a reportextension declares in its rendering section: a Word, RDLC or Excel
//! layout, a name with a dot in it, on an own report and on Base Application's report 1306 (report/lt-y67-layout-ext/
//! Y67LayoutExtExport.al:134 Export_ExtensionLayouts_OwnReport, :187 Export_ExtensionLayout_BaseApplicationReport).
//!
//! The compiler writes the data set part of a layout into the app's folder at build (Y17LayoutExportTests.al:5-9): the folder
//! file is what BC exports. A Word file whose data set part names another report than the one it belongs to was not written by
//! a build of this report (the Y67 files name report 93617, BC exported the rewritten bytes): it is refused, since the rewrite
//! is not reproduced.

use crate::{Session, Signal};
use al_builtins::value::Value;
use al_ir as ir;
use std::io::Read;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// A layout found for a row: the app that holds its file, its name, the file's path in the app and its type (`word`, ...).
struct Found {
    app: usize,
    name: String,
    path: String,
    kind: String,
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
        let Some(bytes) = engine
            .program
            .apps
            .get(found.app)
            .and_then(|a| a.layout_files.iter().find(|(p, _)| *p == found.path))
            .map(|(_, b)| b)
        else {
            return unsupported(format!(
                "Media.ExportStream of the Layout of a Report Layout List row: the layout file {} of layout '{}' is not held by its app (a Microsoft app read from its package keeps no layout files)",
                found.path, found.name
            ));
        };
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
                al_builtins::value::LibraryValue::OutStream(blob, _) => al_builtins::stream::write_bytes(blob, bytes),
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
}

/// A layout of the rendering section of report `id` by its name, or one of the legacy RDLCLayout, WordLayout and ExcelLayout
/// properties by its LayoutFile as written (Y17LayoutExportTests.al:105 Export_LegacyProperties: the same bytes as the file).
fn own_layout(program: &ir::Program, id: i32, name: &str) -> Option<Found> {
    let o = program
        .objects
        .iter()
        .find(|o| o.kind == ir::ObjKind::Report && o.report.is_some() && o.id.map(|i| i as i32) == Some(id))?;
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
fn zip_entries(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
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
