//! Report properties a call checks before it starts.
//!
//! `ExcelLayoutMultipleDataSheets = true` (al-runner issue 7836): the corpus probe measured that the property changes nothing
//! in some calls (assumption excel-layout-multiple-data-sheets, reports/layouts-saveas/ExcelMultipleDataSheetsReport.al); they
//! go on as the same report without it ([`Session::excel_sheets_admits`]). Every other run of such a report, and every use of
//! a variable of it, refuses where it starts ([`Session::excel_sheets_refusal`]), as before the probe.
//!
//! The declaration refusals of a SaveAs: [`save_as_declaration_refusals`].

use al_builtins::assumptions::{Assumption, hit};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, Value};

use crate::Session;

/// The report builtins that run a report or read its layout: each runs with the admission of its own call
/// ([`Session::report_call`]).
const ENTRIES: &[al_ir::BuiltinId] = &[
    ids::REPORT_RUN_1,
    ids::REPORT_RUN_2,
    ids::REPORT_RUN_3,
    ids::REPORT_RUNMODAL_1,
    ids::REPORT_RUNMODAL_2,
    ids::REPORT_RUNMODAL_3,
    ids::REPORT_EXECUTE_1,
    ids::REPORT_EXECUTE_2,
    ids::REPORT_EXECUTE_3,
    ids::REPORT_PRINT_1,
    ids::REPORT_PRINT_2,
    ids::REPORT_RUNREQUESTPAGE_1,
    ids::REPORT_RUNREQUESTPAGE_2,
    ids::REPORT_SAVEAS_1,
    ids::REPORT_SAVEAS_2,
    ids::REPORT_SAVEASEXCEL_1,
    ids::REPORT_SAVEASEXCEL_2,
    ids::REPORT_SAVEASHTML_1,
    ids::REPORT_SAVEASHTML_2,
    ids::REPORT_SAVEASPDF_1,
    ids::REPORT_SAVEASPDF_2,
    ids::REPORT_SAVEASWORD_1,
    ids::REPORT_SAVEASWORD_2,
    ids::REPORT_SAVEASXML_1,
    ids::REPORT_SAVEASXML_2,
    ids::REPORT_WORDXMLPART_1,
    ids::REPORT_WORDXMLPART_2,
    ids::REPORT_DEFAULTLAYOUT_1,
    ids::REPORT_DEFAULTLAYOUT_2,
    ids::REPORT_EXCELLAYOUT_1,
    ids::REPORT_EXCELLAYOUT_2,
    ids::REPORT_RDLCLAYOUT_1,
    ids::REPORT_RDLCLAYOUT_2,
    ids::REPORT_WORDLAYOUT_1,
    ids::REPORT_WORDLAYOUT_2,
    ids::REPORT_VALIDATEANDPREPARELAYOUT_1,
    ids::REPORT_VALIDATEANDPREPARELAYOUT_2,
];

impl Session {
    /// Every report builtin call: one of [`ENTRIES`] runs with [`State::excel_sheets_admitted`](crate::state) set to the
    /// report it admits ([`Session::excel_sheets_admits`], `None` for any other call) and restored after it, whatever it
    /// returns; the other calls (`CurrReport` inside a run) keep the running call's.
    pub(crate) fn report_call(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
        running: super::Inst,
    ) -> Result<Value, Signal> {
        if !ENTRIES.contains(&id) {
            return self.report_builtin(id, receiver, args, running);
        }
        let admitted = self.excel_sheets_admits(id, args);
        let outer = std::mem::replace(&mut self.report.excel_sheets_admitted, admitted);
        let r = self.report_builtin(id, receiver, args, running);
        self.report.excel_sheets_admitted = outer;
        r
    }

    /// The report declaring `ExcelLayoutMultipleDataSheets = true` that the call `id` with `args` runs as measured, by
    /// equality on each fact of the probe (reports/layouts-saveas/ExcelMultipleDataSheetsReport.al, corpus pull request
    /// 1368): the call ([`measured_call`]), the report and its extensions ([`measured_report`]) and no SubstituteReport
    /// subscriber. Every run of it under assumption excel-layout-multiple-data-sheets.
    fn excel_sheets_admits(&self, id: al_ir::BuiltinId, args: &[Value]) -> Option<usize> {
        let objects = &self.engine.program.objects;
        let Some(Value::Integer(n)) = args.first() else {
            return None;
        };
        let object = report_numbered(objects, *n)?;
        let report = objects[object].report.as_ref()?;
        let substituted = self
            .engine
            .events
            .is_subscribed(crate::events::REPORTING_TRIGGERS, "substitutereport");
        let measured = report.excel_multiple_data_sheets
            && measured_report(report)
            && !substituted
            && measured_call(id, args, &report.default_layout_kind);
        (measured && hit(Assumption::ExcelLayoutMultipleDataSheets)).then_some(object)
    }

    /// Why report `object` (`r`) cannot be used now: it declares `ExcelLayoutMultipleDataSheets = true` and the running
    /// report call is not the measured call of it ([`Session::excel_sheets_admits`]). Checked where every run and every use
    /// of a report variable starts (`Session::report_schema`), as the run refusals of the declaration are.
    pub(super) fn excel_sheets_refusal(&self, object: usize, r: &al_ir::Report) -> Option<String> {
        (r.excel_multiple_data_sheets && self.report.excel_sheets_admitted != Some(object)).then(|| {
            "ExcelLayoutMultipleDataSheets = true, outside the measured calls (the static Report.SaveAs(Id, '', Xml or Excel, \
             OutStream) and Report.Run or RunModal(Id, false) of a report with no layout, or one Word or Excel rendering layout, \
             and no report extension; not pinned)"
                .to_string()
        })
    }
}

/// The object index of the report numbered `n`.
fn report_numbered(objects: &al_ir::Objects, n: i32) -> Option<usize> {
    let n = u32::try_from(n).ok()?;
    objects
        .heads()
        .iter()
        .position(|o| o.is_kind(al_ir::ObjKind::Report) && o.id == Some(n))
}

/// The report facts of the probe's reports 76406 to 76408 (ExcelMultipleDataSheetsReport.al), each by equality: not
/// ProcessingOnly, no `RDLCLayout`, `WordLayout` or `ExcelLayout` property, no run-changing extension, and either no layout
/// at all or one `rendering` layout of type Word or Excel that is the `DefaultRenderingLayout`.
fn measured_report(r: &al_ir::Report) -> bool {
    let layouts = match r.rendering_layouts.as_slice() {
        [] => r.default_rendering_layout.is_none() && r.default_layout_kind.is_empty(),
        [l] => {
            r.default_rendering_layout.as_deref() == Some(l.name.as_str())
                && matches!(l.kind.as_str(), "word" | "excel")
        }
        _ => false,
    };
    layouts && !r.processing_only && r.layout_properties.is_empty() && r.extensions.is_empty()
}

/// The calls of the probe, by the default layout's kind (`kind`, empty for no layout):
/// - `Report.SaveAs(Id, '', ReportFormat::Xml, OutStream)` with no layout, a Word or an Excel layout: the same dataset as
///   without the property (XMS_SaveAsXml_SameDatasetAsWithoutProperty);
/// - `Report.SaveAs(Id, '', ReportFormat::Excel, OutStream)` with no layout or an Excel layout: nothing written with no
///   layout, a file that is not empty with an Excel layout (XMS_SaveAsExcel);
/// - `Report.Run(Id, false)` and `Report.RunModal(Id, false)` with no layout or an Excel layout: ReportNoLayout and
///   ReportUnsupportedOperation (XMS_Run_NoRequestPage).
fn measured_call(id: al_ir::BuiltinId, args: &[Value], kind: &str) -> bool {
    match (id, args) {
        (
            ids::REPORT_SAVEAS_2,
            [
                _,
                Value::Text(parameters),
                Value::Option(n, names),
                Value::Library(_, out),
            ],
        ) => {
            let format = names.get(*n as usize).map(|s| s.as_str());
            let to_stream = matches!(&*out.borrow(), LibraryValue::OutStream(..));
            parameters.is_empty()
                && to_stream
                && match format {
                    Some("Xml") => matches!(kind, "" | "word" | "excel"),
                    Some("Excel") => matches!(kind, "" | "excel"),
                    _ => false,
                }
        }
        (ids::REPORT_RUN_3 | ids::REPORT_RUNMODAL_3, [_, Value::Boolean(false)]) => matches!(kind, "" | "excel"),
        _ => false,
    }
}

/// What a `Report.SaveAs` refuses in the report's declaration before it runs: a dataset this runtime cannot write,
/// and a TransactionType (what SaveAs does with it is not pinned).
pub(crate) fn save_as_declaration_refusals(schema: &super::Schema) -> Result<(), Signal> {
    if let Some(why) = schema.report().dataset_unsupported.first() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\": {why}",
            schema.name()
        )));
    }
    if schema.report().transaction_type.is_some() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\", which declares a TransactionType (what SaveAs does with it is not pinned)",
            schema.name()
        )));
    }
    Ok(())
}
