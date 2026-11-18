//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836).
//!
//! The corpus probe measured that the property changes nothing in some static calls (assumption
//! excel-layout-multiple-data-sheets, reports/layouts-saveas/ExcelMultipleDataSheetsReport.al, corpus pull request 1368). Such
//! a call admits the report it asks for ([`Session::report_call`]); the instance it makes for that report claims the admission
//! ([`Session::claim_excel_sheets`]), and only that instance runs as the same report without the property. Every other use
//! of such a report refuses where its schema is read ([`Session::excel_sheets_refusal`]).
//!
//! SubstituteReport: the probe ran with subscribers that leave NewReportId as it is (Base Application's ReportManagement; the
//! corpus app's codeunit 79254, reports/layouts-saveas/AMReports.al:229-234). The claim is made where the call makes the
//! instance of the report that runs after SubstituteReport (`Session::report_by_number`), so a subscriber that sets another
//! report leaves the admission unclaimed: that report runs as it would without this module, and refuses here if it declares
//! the property too.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::value::{Instance, LibraryValue, Value};

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

/// The admission of the running report call (`ReportState::excel_sheets`).
#[derive(Default)]
pub(crate) struct Admission {
    /// The report (object index) the running call admits, until its instance claims it.
    pending: Option<usize>,
    /// The instance that claimed it: the one instance that runs as the report without the property.
    claimed: Option<Weak<RefCell<Instance>>>,
}

impl Admission {
    /// Outside every report call.
    pub(crate) fn is_idle(&self) -> bool {
        self.pending.is_none() && self.claimed.is_none()
    }
}

impl Session {
    /// Every report builtin call: one of [`ENTRIES`] runs with the admission of its own call ([`Session::excel_sheets_admits`],
    /// none for any other call) and the outer one restored after it, whatever it returns; the other calls (`CurrReport` inside a
    /// run) keep the running call's.
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
        let admission = Admission {
            pending: self.excel_sheets_admits(id, args),
            claimed: None,
        };
        let outer = std::mem::replace(&mut self.report.excel_sheets, admission);
        let r = self.report_builtin(id, receiver, args, running);
        self.report.excel_sheets = outer;
        r
    }

    /// The report declaring `ExcelLayoutMultipleDataSheets = true` that the call `id` with `args` asks for as the probe
    /// measured it, by equality on each fact of the probe (ExcelMultipleDataSheetsReport.al): the call ([`measured_call`]) and
    /// the report ([`measured_report`]).
    fn excel_sheets_admits(&self, id: al_ir::BuiltinId, args: &[Value]) -> Option<usize> {
        let objects = &self.engine.program.objects;
        let Some(Value::Integer(n)) = args.first() else {
            return None;
        };
        let object = super::report_numbered(objects, *n)?;
        let report = objects[object].report.as_ref()?;
        let measured = report.excel_multiple_data_sheets
            && measured_report(objects, object, report)
            && measured_call(id, args, &report.default_layout_kind);
        measured.then_some(object)
    }

    /// The static call made `inst`, an instance of report `object`, for the report that runs after SubstituteReport: it claims
    /// the running call's admission when the call admitted this report.
    pub(super) fn claim_excel_sheets(&mut self, object: usize, inst: &super::Inst) {
        let a = &mut self.report.excel_sheets;
        if a.pending == Some(object) {
            a.pending = None;
            a.claimed = Some(Rc::downgrade(inst));
        }
    }

    /// Why the report `r` of `inst` cannot be used now: it declares `ExcelLayoutMultipleDataSheets = true` and `inst` is not
    /// the instance a measured call claimed. The claimed instance runs under assumption excel-layout-multiple-data-sheets.
    /// Checked where every run and every use of a report variable reads its schema (`Session::report_schema`).
    pub(super) fn excel_sheets_refusal(&self, inst: &super::Inst, r: &al_ir::Report) -> Option<String> {
        if !r.excel_multiple_data_sheets {
            return None;
        }
        let claimed = (self.report.excel_sheets.claimed.as_ref())
            .is_some_and(|w| w.upgrade().is_some_and(|c| Rc::ptr_eq(&c, inst)));
        if claimed {
            assume(Assumption::ExcelLayoutMultipleDataSheets);
            return None;
        }
        Some(
            "ExcelLayoutMultipleDataSheets = true, outside the measured calls (the static Report.SaveAs(Id, '', Xml or Excel, \
             OutStream) and Report.Run or RunModal(Id, false) of a report declaring UseRequestPage = false, with no layout or one \
             Word or Excel rendering layout, no code and no report extension; not pinned)"
                .to_string(),
        )
    }
}

/// The report facts of the probe's reports 76406 to 76408 (ExcelMultipleDataSheetsReport.al), each by equality:
/// `UseRequestPage = false` and no `requestpage` section, no trigger or procedure, not ProcessingOnly, no `RDLCLayout`,
/// `WordLayout` or `ExcelLayout` property, no reportextension of it in the program, and either no layout at all or one
/// `rendering` layout of type Word or Excel that is the `DefaultRenderingLayout`.
fn measured_report(objects: &al_ir::Objects, object: usize, r: &al_ir::Report) -> bool {
    let layouts = match r.rendering_layouts.as_slice() {
        [] => r.default_rendering_layout.is_none() && r.default_layout_kind.is_empty(),
        [l] => {
            r.default_rendering_layout.as_deref() == Some(l.name.as_str())
                && matches!(l.kind.as_str(), "word" | "excel")
        }
        _ => false,
    };
    let heads = objects.heads();
    let id = heads[object].id;
    let extended = heads.iter().any(|o| {
        o.is_kind(al_ir::ObjKind::ReportExtension) && (o.extends_id == id || o.report_ext_base == Some(object))
    });
    layouts
        && r.use_request_page == Some(false)
        && !r.has_request_page
        && only_column_routines(objects, object, r)
        && !r.processing_only
        && r.layout_properties.is_empty()
        && r.extensions.is_empty()
        && !extended
}

/// Every routine of report `object` evaluates a column of its dataset: the report declares no trigger or procedure, as the
/// probe's reports declare none.
fn only_column_routines(objects: &al_ir::Objects, object: usize, r: &al_ir::Report) -> bool {
    let columns: Vec<usize> = (r.items.iter())
        .flat_map(|i| i.columns.iter().filter_map(|c| c.routine))
        .collect();
    (0..objects[object].routines.len()).all(|i| columns.contains(&i))
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
