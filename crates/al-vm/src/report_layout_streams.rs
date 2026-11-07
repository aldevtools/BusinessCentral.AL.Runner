//! `Report.RdlcLayout`, `WordLayout` and `ExcelLayout` of a report with a rendering section (reports/runtime/TrlResourceRows.al).

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::Value;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

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
        if !self.ret_used || r.processing_only || !r.extensions.is_empty() || legacy || r.rendering_layouts.is_empty() {
            return None;
        }
        let want = kind.to_ascii_lowercase();
        let of_kind: Vec<_> = r.rendering_layouts.iter().filter(|l| l.kind == want).collect();
        if of_kind.len() > 1 {
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
