//! A Pdf save writes a placeholder PDF (owner decision 2026-11-08, ADR-040 amendment "A placeholder PDF").
//!
//! `Report.SaveAs(.., ReportFormat::Pdf, ..)`, `Report.SaveAsPdf` and `TestRequestPage.SaveAsPdf` of a report whose
//! default layout is RDLC or Word run the report exactly as the Xml path does (request parameters, triggers, data items,
//! errors) and write [`placeholder_pdf`] where BC writes its rendering. The bytes are not BC's: only that the output
//! exists, is a PDF and is not empty is meant to match. The path marks the assumption `report-pdf-placeholder`, so a
//! test that passes through it is a flagged pass.
//!
//! Microsoft tests that need it (none reads the bytes): `Library - Post Inventory to G/L`.PostInvtCostToGL calls
//! `Post Inventory Cost to G/L`.SaveAsPdf (SCM-Costing, SCM-Manufacturing, SCM-Assembly, SCM); the request page
//! handlers of Report Layout, ERM Account Schedule II and ERM Purchase Reports III call `SaveAsPdf`;
//! ReportCatalogTests CustomerStatementReport checks the file is not empty (`CheckFileNotEmpty`).

use crate::reports::{SaveTarget, Schema};
use crate::{Session, Signal};
use al_builtins::value::Value;

/// What a rendering save writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SaveOutput {
    /// The Xml dataset the run wrote.
    Dataset,
    /// Nothing; the target is marked as pending (ADR-040 allow-list, and the company build).
    Discard,
    /// [`placeholder_pdf`].
    Placeholder,
}

/// A small valid PDF: the `%PDF-1.4` header, a catalog, a page tree and one blank Letter page, with a correct
/// cross-reference table.
pub fn placeholder_pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>",
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{body}\nendobj\n", i + 1));
    }
    let xref = out.len();
    out.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for at in offsets {
        out.push_str(&format!("{at:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));
    out.into_bytes()
}

/// The events of codeunit "Reporting Triggers" BC raises around a render (reports/layouts-saveas, events/platform-
/// triggers/PeVTests.al:721 `saveas static html`: SubstituteReport, SelectReportLayoutCode, HasCustomLayout,
/// SelectedBuiltinLayoutType, ApplicationReportMergeStrategy; :750 OnDocumentReady after a render), and the ones
/// Base Application answers for a render (FetchReportLayoutByCode, OnIntermediateDocumentReady, GetCompanyMetadata).
/// This runtime raises none of them for a placeholder.
const LAYOUT_EVENTS: &[(&str, &str)] = &[
    ("substitutereport", "SubstituteReport"),
    ("selectreportlayoutcode", "SelectReportLayoutCode"),
    ("hascustomlayout", "HasCustomLayout"),
    ("selectedbuiltinlayouttype", "SelectedBuiltinLayoutType"),
    ("applicationreportmergestrategy", "ApplicationReportMergeStrategy"),
    ("ondocumentready", "OnDocumentReady"),
    ("onintermediatedocumentready", "OnIntermediateDocumentReady"),
    ("fetchreportlayoutbycode", "FetchReportLayoutByCode"),
    ("getcompanymetadata", "GetCompanyMetadata"),
];

/// The integration events Base Application's ReportManagement raises from its subscribers to [`LAYOUT_EVENTS`]: a
/// subscriber to one of them would see the call.
const REPORT_MANAGEMENT_EVENTS: &[&str] = &[
    "onaftersubstitutereport",
    "onselectreportlayoutcode",
    "onafterhascustomlayout",
    "onafterdocumentready",
    "onafterintermediatedocumentready",
    "onfetchreportlayoutbycode",
];

const REPORT_MANAGEMENT: &str = "ReportManagement";

/// Base Application's table "Report Layout Selection" and codeunit "Design-time Report Selection".
const REPORT_LAYOUT_SELECTION: u32 = 9651;
const DESIGN_TIME_REPORT_SELECTION: u32 = 9654;
/// The platform table Tenant Report Layout Selection.
const TENANT_REPORT_LAYOUT_SELECTION: u32 = 2_000_000_233;

impl Session {
    /// Puts the output of a rendering save that ran to its end where the caller asked for it.
    pub(crate) fn write_save_output(
        &mut self,
        target: SaveTarget,
        output: SaveOutput,
        schema: &Schema,
        dataset: &[u8],
    ) -> Result<(), Signal> {
        use al_builtins::assumptions::{Assumption, assume};
        match (output, target) {
            (SaveOutput::Dataset, SaveTarget::Stream(blob)) => al_builtins::stream::write_bytes(&blob, dataset),
            (SaveOutput::Dataset, SaveTarget::File(name)) => self.write_save_file(&name, dataset)?,
            (SaveOutput::Placeholder, SaveTarget::Stream(blob)) => {
                al_builtins::stream::write_bytes(&blob, &placeholder_pdf());
                assume(Assumption::ReportPdfPlaceholder);
            }
            (SaveOutput::Placeholder, SaveTarget::File(name)) => {
                self.write_save_file(&name, &placeholder_pdf())?;
                assume(Assumption::ReportPdfPlaceholder);
            }
            (SaveOutput::Discard, SaveTarget::Stream(blob)) => {
                // The dataset is dropped; the buffer is marked and any look at it is refused.
                blob.borrow_mut().render_pending = Some(schema.name().into());
                assume(Assumption::ReportRenderDiscarded);
            }
            (SaveOutput::Discard, SaveTarget::File(name)) => {
                self.render_pending_files
                    .insert(crate::ctx::pending_key(&name), schema.name().into());
                assume(Assumption::ReportRenderDiscarded);
            }
        }
        Ok(())
    }

    /// Writes a file a save produced; a write over a path that was pending ends the mark, as a write by AL does.
    pub(crate) fn write_save_file(&mut self, name: &str, bytes: &[u8]) -> Result<(), Signal> {
        al_builtins::native::Ctx::fs_write(self, name, bytes).map_err(Signal::unsupported)
    }

    /// Whether a Pdf save of the report writes the placeholder: its default layout is RDLC or Word, and no subscriber the
    /// test can see listens to an event BC raises around the render. Such a subscriber refuses the save, and so does one
    /// the compile left out (other than Base Application's ReportManagement, which picks a layout from data: a layout
    /// selection other than the report's RDLC or Word layout refuses too, [`Session::layout_selection`]).
    pub(crate) fn pdf_placeholder(&self, schema: &Schema) -> Result<bool, Signal> {
        let report = schema.report();
        if !matches!(report.default_layout_kind.as_str(), "rdlc" | "word") || report.layout_file_missing {
            return Ok(false);
        }
        if let Some(who) = self.layout_event_subscriber() {
            return Err(Signal::unsupported(format!(
                "Report.SaveAs of \"{}\" as Pdf with {who} (BC raises the layout events before it renders; this runtime \
                 writes a placeholder PDF and raises none of them)",
                schema.name()
            )));
        }
        if let Some(what) = self.layout_selection(schema)? {
            return Err(Signal::unsupported(format!(
                "Report.SaveAs of \"{}\" as Pdf while {what} (BC may render another layout, and an Excel one writes no \
                 output; this runtime writes a placeholder PDF only for the report's own RDLC or Word layout)",
                schema.name()
            )));
        }
        Ok(true)
    }

    /// A layout selection BC follows when it renders that may pick something other than an RDLC or Word layout of the report,
    /// as text for a refusal. Base Application's ReportManagement answers SelectedBuiltinLayoutType and SelectReportLayoutCode
    /// from table 9651 "Report Layout Selection" and from the SingleInstance codeunit 9654 "Design-time Report Selection"
    /// (ReportManagement.Codeunit.al, Base Application 29); the platform reads Tenant Report Layout Selection. What BC does
    /// with each selection is not probed, so any selection but a built-in RDLC or Word one, or a tenant row naming an RDLC or
    /// Word layout the report declares, refuses (reports/layouts-saveas/AMReportOutputTests.al:257: a Pdf of an Excel layout
    /// writes nothing).
    fn layout_selection(&self, schema: &Schema) -> Result<Option<String>, Signal> {
        let report = schema.report();
        let Some(id) = self.engine.program.objects[schema.object()].id else {
            return Ok(None);
        };
        let has_kind =
            |kind: &str| report.default_layout_kind == kind || report.rendering_layouts.iter().any(|l| l.kind == kind);
        let field = |t: &al_data::TableDef, name: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
        let text = |v: &Value| match v {
            Value::Text(s) | Value::Code(s) => s.to_string(),
            _ => String::new(),
        };
        let cx = self.cx.clone();
        if let Some(t) = self
            .table_def(REPORT_LAYOUT_SELECTION)
            .filter(|t| t.name.eq_ignore_ascii_case("Report Layout Selection"))
        {
            if let (Some(r), Some(c), Some(k)) = (field(&t, "Report ID"), field(&t, "Company Name"), field(&t, "Type"))
            {
                for row in self.all_rows(&cx, &t)? {
                    if !matches!(row[r], Value::Integer(n) if n as i64 == id as i64)
                        || !text(&row[c]).eq_ignore_ascii_case(&self.cx.company)
                    {
                        continue;
                    }
                    // Option 0 "RDLC (built-in)", 1 "Word (built-in)".
                    let builtin = match &row[k] {
                        Value::Integer(0) | Value::Option(0, _) => has_kind("rdlc"),
                        Value::Integer(1) | Value::Option(1, _) => has_kind("word"),
                        _ => false,
                    };
                    if !builtin {
                        return Ok(Some("a Report Layout Selection row selects a layout for it".into()));
                    }
                }
            }
        }
        if let Some(t) = self.table_def(TENANT_REPORT_LAYOUT_SELECTION) {
            if let (Some(r), Some(n)) = (field(&t, "Report ID"), field(&t, "Layout Name")) {
                for row in self.all_rows(&cx, &t)? {
                    if !matches!(row[r], Value::Integer(n) if n as i64 == id as i64) {
                        continue;
                    }
                    let name = text(&row[n]);
                    let known = report
                        .rendering_layouts
                        .iter()
                        .any(|l| l.name.eq_ignore_ascii_case(&name) && matches!(l.kind.as_str(), "rdlc" | "word"));
                    if !known {
                        return Ok(Some(format!(
                            "Tenant Report Layout Selection selects layout \"{name}\", not an RDLC or Word layout of it"
                        )));
                    }
                }
            }
        }
        let program = &self.engine.program;
        let design_time = program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Codeunit)
                && o.id == Some(DESIGN_TIME_REPORT_SELECTION)
                && o.name.eq_ignore_ascii_case("Design-time Report Selection")
        });
        if let Some(inst) = design_time.and_then(|oi| self.singletons.get(&oi)) {
            if inst.borrow().globals.iter().any(|v| !text(v).is_empty()) {
                return Ok(Some("Design-time Report Selection holds a selected layout".into()));
            }
        }
        Ok(None)
    }

    /// The first subscriber to a layout event that refuses a placeholder, as text for a refusal.
    fn layout_event_subscriber(&self) -> Option<String> {
        let events = &self.engine.events;
        let program = &self.engine.program;
        // A subscriber the test cannot see (it names neither the subscriber's codeunit nor a SingleInstance codeunit the
        // subscriber holds) cannot tell whether the event ran, as for CaptionClassTranslate (`Registry::unraised_subscriber`).
        let test_object = self.test_object_read();
        let test = test_object.and_then(|t| program.objects.get(t));
        for (key, name) in LAYOUT_EVENTS {
            if let Some((_, sub)) = events.unraised_subscriber("reporting triggers", Some(key), test, test_object) {
                return Some(format!("a subscriber to {name} ({sub})"));
            }
            let key = (crate::events::REPORTING_TRIGGERS, key.to_string());
            if let Some(m) = events.missing.get(&key).and_then(|list| {
                list.iter()
                    .find(|m| !m.subscriber.contains(&format!("\"{REPORT_MANAGEMENT}\"")))
            }) {
                return Some(format!("a subscriber to {name} ({})", m.subscriber));
            }
        }
        REPORT_MANAGEMENT_EVENTS.iter().find_map(|e| {
            events
                .unraised_subscriber("reportmanagement", Some(e), test, test_object)
                .map(|(_, sub)| format!("a subscriber to ReportManagement's {e} ({sub})"))
        })
    }
}
