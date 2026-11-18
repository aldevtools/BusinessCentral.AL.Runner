//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836).
//!
//! The corpus probe measured that the property changes nothing in some static calls of reports declared as its reports are
//! (assumption excel-layout-multiple-data-sheets, reports/layouts-saveas/ExcelMultipleDataSheetsReport.al, corpus pull
//! request 1368). Such a call admits the report it asks for ([`Session::report_call`]); the instance it makes for that report
//! claims the admission ([`Session::claim_excel_sheets`]), and only that instance runs as the same report without the
//! property. Every other run of such a report refuses where its schema is read ([`Session::excel_sheets_refusal`]); a
//! variable of it is refused before the code runs (al-sema `reports::variable_refusal`).
//!
//! SubstituteReport: the probe ran with subscribers that leave NewReportId as it is (Base Application's ReportManagement; the
//! corpus app's codeunit 79254, reports/layouts-saveas/AMReports.al:229-234). Run and RunModal raise it before they make the
//! instance (`Session::report_by_number`), so a subscriber that sets another report leaves the admission unclaimed: that report
//! runs as it would without this module, and refuses here if it declares the property too. SaveAs of these reports raises no
//! SubstituteReport in the runner, as for the same report without the property (docs/probes-needed.md, the row on
//! SubstituteReport for an Xml SaveAs other than the pinned one), so a subscriber that would set another report is not asked.

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
    /// measured it, by equality on each fact of the probe (ExcelMultipleDataSheetsReport.al): the call ([`measured_call`],
    /// with the answer of a SaveAs used), made in the body of a TryFunction whose caller uses its result (TrySave, TryRun
    /// and TryRunModal, :127-146, called at :157, :198, :200, :241), and the report ([`measured_report`]).
    fn excel_sheets_admits(&self, id: al_ir::BuiltinId, args: &[Value]) -> Option<usize> {
        let objects = &self.engine.program.objects;
        let Some(Value::Integer(n)) = args.first() else {
            return None;
        };
        let object = super::report_numbered(objects, *n)?;
        let report = objects[object].report.as_ref()?;
        let measured = report.excel_multiple_data_sheets
            && measured_report(objects, object, report)
            && measured_call(id, args, &report.default_layout_kind, self.ret_used)
            && self.in_own_used_try();
        measured.then_some(object)
    }

    /// The static call made `inst`, an instance of report `object`, for the report that runs after SubstituteReport. The
    /// first instance the call makes takes the admission: it claims it when the call admitted this report, and the run goes on
    /// under assumption excel-layout-multiple-data-sheets.
    pub(super) fn claim_excel_sheets(&mut self, object: usize, inst: &super::Inst) {
        let a = &mut self.report.excel_sheets;
        if a.pending.take() == Some(object) {
            a.claimed = Some(Rc::downgrade(inst));
            assume(Assumption::ExcelLayoutMultipleDataSheets);
        }
    }

    /// Why the report `r` of `inst` cannot be used now: it declares `ExcelLayoutMultipleDataSheets = true` and `inst` is not
    /// the instance a measured call claimed. Checked where every run of a report reads its schema (`Session::report_schema`).
    pub(super) fn excel_sheets_refusal(&self, inst: &super::Inst, r: &al_ir::Report) -> Option<String> {
        if !r.excel_multiple_data_sheets {
            return None;
        }
        let claimed = (self.report.excel_sheets.claimed.as_ref())
            .is_some_and(|w| w.upgrade().is_some_and(|c| Rc::ptr_eq(&c, inst)));
        (!claimed).then(|| {
            "ExcelLayoutMultipleDataSheets = true, outside the measured calls (the static Report.SaveAs(Id, '', Xml or Excel, \
             OutStream) with its answer used and Report.Run or RunModal(Id, false), in a TryFunction whose result is used, of a \
             report declared as the probe's reports are; not pinned)"
                .to_string()
        })
    }
}

/// The object properties of the probe's reports 76406 to 76408 (ExcelMultipleDataSheetsReport.al:42-45, :67-71, :97-101), by
/// name and value: `UsageCategory = None`, `UseRequestPage = false`, `ExcelLayoutMultipleDataSheets = true`, and
/// `DefaultRenderingLayout` naming the one layout when there is one. No other property (`DefaultLayout`, `Caption`,
/// `TransactionType`, `FormatRegion`, `ProcessingOnly`, `RDLCLayout`, ...).
fn measured_properties(o: &al_ir::Object, layout: Option<&str>) -> bool {
    let mut props: Vec<(String, String)> = (o.properties.iter())
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_ascii_lowercase()))
        .collect();
    props.sort();
    let mut measured: Vec<(String, String)> = [
        ("excellayoutmultipledatasheets", "true"),
        ("usagecategory", "none"),
        ("userequestpage", "false"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    if let Some(l) = layout {
        measured.push(("defaultrenderinglayout".to_string(), l.to_ascii_lowercase()));
    }
    measured.sort();
    props == measured
}

/// The report facts of the probe's reports 76406 to 76408 (ExcelMultipleDataSheetsReport.al:40-122), each by equality: the
/// object properties ([`measured_properties`]); no layout, or one `rendering` layout of type Word or Excel that states only
/// `Type` and `LayoutFile`; no request page, no label, no global variable, no trigger or procedure; the dataset
/// ([`measured_dataset`]); no reportextension of it in the program.
fn measured_report(objects: &al_ir::Objects, object: usize, r: &al_ir::Report) -> bool {
    let o = &objects[object];
    let layouts = match (o.layouts.as_slice(), r.rendering_layouts.as_slice()) {
        ([], []) => measured_properties(o, None),
        ([l], [rl]) => {
            let mut names: Vec<String> = l.properties.iter().map(|(k, _)| k.to_ascii_lowercase()).collect();
            names.sort();
            names == ["layoutfile", "type"]
                && matches!(rl.kind.as_str(), "word" | "excel")
                && measured_properties(o, Some(&l.name))
        }
        _ => false,
    };
    let heads = objects.heads();
    let id = heads[object].id;
    let extended = heads.iter().any(|h| {
        h.is_kind(al_ir::ObjKind::ReportExtension) && (h.extends_id == id || h.report_ext_base == Some(object))
    });
    layouts
        && !r.has_request_page
        && r.labels.is_empty()
        && !r.declares_globals
        && only_column_routines(objects, object, r)
        && measured_dataset(objects, object, r)
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

/// The dataset of the probe's reports (ExcelMultipleDataSheetsReport.al:44-54): one top-level data item or more, each
/// stating `DataItemTableView` and no other property (:46, :52), so no link, `UseTemporary`, request filter, `MaxIteration`,
/// `PrintOnlyIfDetail`, `CalcFields`, page or obsolescence property, and no nested item; columns stating no property
/// (:47-48, :53), each a field of the item (`column(Number; Number)`) or an expression of operators over constants and
/// fields of the item's record (`column(Doubled; Number * 2)`), so no call. The tables are not compared.
fn measured_dataset(objects: &al_ir::Objects, object: usize, r: &al_ir::Report) -> bool {
    let routines = &objects[object].routines;
    let column = |c: &al_ir::ReportColumn| {
        let source = c.field.is_some()
            || c.routine.and_then(|i| routines.get(i)).is_some_and(|rt| {
                rt.body
                    .read(|stmts| matches!(stmts, [al_ir::Stmt::Exit(Some(e))] if operators_only(e)))
            });
        c.stated_properties.is_empty() && c.unsupported.is_none() && c.value_unsupported.is_none() && source
    };
    let item_properties = |i: &al_ir::ReportItem| {
        let stated: Vec<&str> = i.stated_properties.iter().map(String::as_str).collect();
        stated == ["dataitemtableview"]
    };
    !r.items.is_empty()
        && (r.items.iter()).all(|i| i.parent.is_none() && item_properties(i) && i.columns.iter().all(column))
}

/// `e` is built of constants, fields of a global record and unary and binary operators only.
fn operators_only(e: &al_ir::Expr) -> bool {
    use al_ir::{Expr, Place};
    match e {
        Expr::Const(_) => true,
        Expr::Load(Place::Field(p, _)) => matches!(**p, Place::Global(_)),
        Expr::Unary(_, a) => operators_only(a),
        Expr::Binary(_, a, b) => operators_only(a) && operators_only(b),
        _ => false,
    }
}

/// The calls of the probe, by the default layout's kind (`kind`, empty for no layout):
/// - `Saved := Report.SaveAs(Id, '', ReportFormat::Xml, OutStream)` with no layout, a Word or an Excel layout: the same dataset
///   as without the property (XMS_SaveAsXml_SameDatasetAsWithoutProperty);
/// - `Saved := Report.SaveAs(Id, '', ReportFormat::Excel, OutStream)` with no layout or an Excel layout: nothing written with
///   no layout, a file that is not empty with an Excel layout (XMS_SaveAsExcel);
/// - `Report.Run(Id, false)` and `Report.RunModal(Id, false)` with no layout or an Excel layout: ReportNoLayout and
///   ReportUnsupportedOperation (XMS_Run_NoRequestPage).
///
/// The format is a value of the ReportFormat type (the probe passes a `ReportFormat` parameter, :129), and the answer of a
/// SaveAs is used (`answer_used`, :134).
fn measured_call(id: al_ir::BuiltinId, args: &[Value], kind: &str, answer_used: bool) -> bool {
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
            let report_format = al_builtins::sysenum::members("reportformat")
                .is_some_and(|m| m.iter().copied().eq(names.iter().map(String::as_str)));
            let format = names.get(*n as usize).map(|s| s.as_str());
            let to_stream = matches!(&*out.borrow(), LibraryValue::OutStream(..));
            answer_used
                && report_format
                && parameters.is_empty()
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
