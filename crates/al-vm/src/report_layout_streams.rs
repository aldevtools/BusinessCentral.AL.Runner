//! `Report.RdlcLayout`, `WordLayout` and `ExcelLayout` of a report with a rendering section (reports/runtime/TrlResourceRows.al).

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;

impl Session {
    /// The bytes of the RDLC layout file report `object` names with `DefaultRenderingLayout`, when it declares no
    /// `RDLCLayout`, `WordLayout`, `ExcelLayout` or `DefaultLayout` property and the app holds the file.
    pub(crate) fn default_rdlc_layout_file(&self, object: usize) -> Option<Vec<u8>> {
        let o = &self.engine.program.objects[object];
        let r = o.report.as_ref()?;
        let prop = |name: &str| {
            o.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim().trim_matches('"').to_string())
        };
        if ["RDLCLayout", "WordLayout", "ExcelLayout", "DefaultLayout"]
            .iter()
            .any(|p| prop(p).is_some())
        {
            return None;
        }
        let default = prop("DefaultRenderingLayout")?;
        let layout = r
            .rendering_layouts
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(&default))
            .filter(|l| l.kind == "rdlc")?;
        self.engine
            .program
            .apps
            .get(o.app as usize)?
            .layout_files
            .iter()
            .find(|(p, _)| *p == layout.path)
            .map(|(_, b)| b.clone())
    }

    /// The result of `Report.<kind>Layout` used, on a report with a rendering section and neither a legacy layout property
    /// nor a reportextension (reports/runtime/TrlResourceRows.al:99-146 WordLayout_Stream_VsFile,
    /// RdlcAndExcelLayout_Stream_VsFile, LayoutOfWrongKind_Returned): the one layout of the asked kind gives its file's bytes
    /// and Yes; no layout of the kind gives No for Word and Excel, and Yes with an empty stream for RDLC. Two layouts of a
    /// kind are not pinned. `None`: another shape, the caller decides.
    pub(crate) fn rendering_layout_stream(
        &self,
        object: usize,
        kind: &str,
        stream: Option<&Value>,
    ) -> Option<Result<Value, Signal>> {
        let o = &self.engine.program.objects[object];
        let r = o.report.as_ref()?;
        let has = |name: &str| o.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
        let legacy = ["RDLCLayout", "WordLayout", "ExcelLayout", "DefaultLayout"]
            .iter()
            .any(|p| has(p));
        if r.processing_only || !r.extensions.is_empty() || legacy || r.rendering_layouts.is_empty() {
            return None;
        }
        let want = kind.to_ascii_lowercase();
        let of_kind: Vec<_> = r.rendering_layouts.iter().filter(|l| l.kind == want).collect();
        // As a statement, only the one layout of the asked kind: BC fills the stream without an error, and a Tenant Report
        // Layout row imports it (reports/layouts-saveas/UserDefinedLayoutAppIdAndNames.al:146-157 RDLC, Word and Excel of a
        // report whose default is Word; UserDefinedWordExcelLayoutRows.al:60, :94; reports/runtime/Y6TenantReportLayoutRows.al:
        // 35-89 for the default RDLC layout gives the file). No layout of the kind as a statement is not pinned.
        if of_kind.len() > 1 || (!self.ret_used && of_kind.len() != 1) {
            return None;
        }
        let path = of_kind.first().map(|l| l.path.clone());
        let app = o.app as usize;
        Some(self.rendering_layout_into(object, kind, stream, app, path))
    }

    fn rendering_layout_into(
        &self,
        object: usize,
        kind: &str,
        stream: Option<&Value>,
        app: usize,
        path: Option<String>,
    ) -> Result<Value, Signal> {
        let Some(stream) = stream else {
            return unsupported(format!("Report.{kind}Layout without a stream"));
        };
        let created = || {
            unsupported(format!(
                "Report.{kind}Layout into a stream that was created (what it does to the stream is not pinned)"
            ))
        };
        let Some(path) = path else {
            // No layout of the kind.
            if kind == "RDLC" {
                return if al_builtins::stream::create_empty_in(stream) {
                    Ok(Value::Boolean(true))
                } else {
                    created()
                };
            }
            return if al_builtins::stream::mark_not_initialized(stream) {
                Ok(Value::Boolean(false))
            } else {
                created()
            };
        };
        let file = self
            .engine
            .program
            .apps
            .get(app)
            .and_then(|a| a.layout_files.iter().find(|(p, _)| *p == path).map(|(_, b)| b.clone()));
        let Some(file) = file else {
            // As a statement the call fills the stream without an error on BC (reports/layouts-saveas/
            // UserDefinedLayoutAppIdAndNames.al:146-157, UserDefinedWordExcelLayoutRows.al:60, :94): the compiler generated the
            // file, whose bytes the stream carries unread. Only an app built from source: an app read from a package holds its
            // layout files there (`App::layout_source`), and a file the runner does not hold there was not generated.
            let from_source = self
                .engine
                .program
                .apps
                .get(app)
                .is_some_and(|a| a.layout_source.is_none());
            if !self.ret_used && from_source {
                let mark = al_builtins::stream::layout_file_generated_mark(kind);
                return if al_builtins::stream::create_in_pending(stream, &mark) {
                    Ok(Value::Boolean(true))
                } else {
                    created()
                };
            }
            return unsupported(format!(
                "Report.{kind}Layout of a layout whose file the app does not hold"
            ));
        };
        let bytes = if kind == "RDLC" {
            self.rdlc_layout_stream(object, file)?
        } else {
            file
        };
        if al_builtins::stream::create_in_of(stream, bytes) {
            Ok(Value::Boolean(true))
        } else {
            created()
        }
    }
}
