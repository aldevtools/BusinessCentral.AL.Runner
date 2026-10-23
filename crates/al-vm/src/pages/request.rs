//! The request page of a report: the same test client as a page, over the report's own instance.
//!
//! A report whose request page opens (`Report.Run`, `RunModal`, `RunRequestPage`) opens it for the
//! test's `[RequestPageHandler]`, which receives a `TestRequestPage` for the report. The page's
//! controls bind to the report's global variables, so what a handler writes is what the report
//! reads (corpus handlers/TestReportRequestPageControl.al); its data items are controls whose
//! filters a handler sets (`RequestPage.Rows.SetFilter`, handlers/TestReportRunRequestPage.al).
//!
//! Every rule cites the corpus test that pins it; the rest is refused.

use super::*;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::page_op::PageOp;

type Inst = Rc<RefCell<al_builtins::value::Instance>>;

/// How the request page closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestClosed {
    /// The handler confirmed with OK.
    Ok,
    /// The handler cancelled: no parameters, and a run stops
    /// (handlers/TestReportRunWithRequestPage.al:Report_Run_CancelledRequestPageRunsNothingAndRaisesNoError).
    Cancel,
    /// The handler called `SaveAsXml`: the report ran there, and the page is closed.
    SavedAsXml,
    /// The handler invoked the Preview action: the report runs as a preview.
    Preview,
    /// The handler invoked the Print action: the report runs as after OK.
    Print,
}

impl Session {
    /// Whether `st` is the request page of a report.
    pub(super) fn is_request_page(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.program.objects[object].report.is_some()
    }

    /// The built-in actions of a request page. Plain OK is the confirmation of a processing-only
    /// report and is not offered otherwise; Cancel is always offered
    /// (handlers/TestReportRunWithRequestPage.al:Report_Run_NonProcessingOnlyRequestPageHasNoOKAction).
    /// The page `RunRequestPage` opens has OK whatever the report renders: it confirms the
    /// parameters (handlers/TestReportRunRequestPage.al:TestReport_CapturedParameters_ReplayThroughSaveAs_
    /// FilterTheDataset, a report with an RDLC layout).
    pub(super) fn request_builtin_offered(&self, st: &St, which: &str) -> bool {
        let object = st.borrow().object as usize;
        let processing_only = self.engine.program.objects[object]
            .report
            .as_ref()
            .is_some_and(|r| r.processing_only);
        match which {
            "OK" => processing_only || st.borrow().parameters_wanted,
            "Cancel" => true,
            "Preview" | "Print" => !processing_only && !st.borrow().parameters_wanted,
            _ => false,
        }
    }

    /// The Preview and Print actions are pinned on the request page of a report that renders, started by `Report.Run`
    /// (report/W3ReportObjectIdPreviewLanguage.al:717-752 QV_Preview_Run_PreviewAction, QV_Preview_Run_PrintAction; :484-511
    /// Preview_RequestPagePreviewAction). On a processing-only report, on the page `RunRequestPage` opens, in a `RunModal` and
    /// on a report with SaveValues they are not pinned and refused. The CloseAction they close with formats as their name
    /// (:731 `rp close Preview`, :750 `rp close Print`), a member the Action type here does not have: a request page with an
    /// OnQueryClosePage is refused.
    pub(super) fn request_preview_print_pinned(&self, st: &St, which: &str) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        let report = o.report.as_ref();
        let why = if report.is_some_and(|r| r.processing_only) {
            "the request page of a processing-only report"
        } else if st.borrow().parameters_wanted {
            "the page RunRequestPage opens"
        } else if self.report.run_modal {
            "the request page of a report started by RunModal"
        } else if o
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("SaveValues") && v.trim().eq_ignore_ascii_case("true"))
        {
            "the request page of a report with SaveValues"
        } else if self.declares(st, "onqueryclosepage")
            || o.page
                .as_ref()
                .is_some_and(|p| p.ext_triggers.iter().any(|(n, _, _)| n == "onqueryclosepage"))
        {
            return unsupported(format!(
                "the {which} action on a request page with OnQueryClosePage (its CloseAction {which} is not built)"
            ));
        } else {
            return Ok(());
        };
        unsupported(format!("the {which} action on {why} (not pinned)"))
    }

    /// Whether the request page's SourceTableView is the one shape the corpus pins: a SORTING that names the primary key and a
    /// WHERE of CONST conditions only (report/lt-y-request-page/YRequestPageTests.al:72-87, the request page's `Rec` then has
    /// neither the filter nor another key). Anything else is refused.
    pub(super) fn request_source_view_pinned(&mut self, st: &St, view: &str) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let refuse = || unsupported(format!("a request page with SourceTableView = {view} (not pinned)"));
        let mut rest = view.trim();
        while !rest.is_empty() {
            let Some(open) = rest.find('(') else {
                return refuse();
            };
            let word = rest[..open].trim().to_ascii_lowercase();
            let mut depth = 0i32;
            let mut close = None;
            for (i, c) in rest[open..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(close) = close else {
                return refuse();
            };
            let inner = &rest[open + 1..close];
            match word.as_str() {
                "sorting" => {
                    let named: Vec<String> = split_conditions(inner)
                        .iter()
                        .map(|f| f.trim().trim_matches('"').to_ascii_lowercase())
                        .collect();
                    let pk: Vec<String> = table
                        .keys
                        .first()
                        .map(|k| k.iter().map(|&i| table.fields[i].name.to_ascii_lowercase()).collect())
                        .unwrap_or_default();
                    if named != pk {
                        return refuse();
                    }
                }
                "where" => {
                    let all_const = split_conditions(inner).iter().all(|c| {
                        c.split_once('=')
                            .is_some_and(|(_, cond)| cond.trim().to_ascii_lowercase().starts_with("const("))
                    });
                    if !all_const {
                        return refuse();
                    }
                }
                _ => return refuse(),
            }
            rest = rest[close + 1..].trim();
        }
        Ok(())
    }

    /// Opens the request page of the report `inst` is an instance of, hands it to the bound
    /// `[RequestPageHandler]` and reports how the handler closed it. The page's `OnInit` and
    /// `OnOpenPage` run before the handler sees it, and write the report's own variables
    /// (handlers/TestRequestPageOnInit.al, handlers/TestReportRunRequestPage.al: `rp-open`).
    pub(crate) fn run_request_page(
        &mut self,
        inst: &Inst,
        object: usize,
        seed: Option<&str>,
        run_ext: Vec<(usize, Inst)>,
        want_parameters: bool,
    ) -> Result<(RequestClosed, Option<String>), Signal> {
        let engine = self.engine();
        let name = engine.program.objects[object].name.clone();
        let Some(page) = engine.program.objects[object].page.as_ref() else {
            return unsupported(format!(
                "report \"{name}\" has no request page the test client can drive"
            ));
        };
        if let Some(why) = page.unsupported.first() {
            return unsupported(format!("the request page of report \"{name}\": {why}"));
        }
        // handlers/TestReportRuntimeContracts.al:425-429: with no [RequestPageHandler] the page
        // opens and the run is refused with "Unhandled UI: RequestPage <report id>".
        if !self.has_page_handler(UiKind::RequestPage, object as u32) {
            return Err(self.unhandled_page(UiKind::RequestPage, object as u32));
        }
        let st: St = Rc::new(RefCell::new(PageState::closed(object as u32)));
        {
            let mut s = st.borrow_mut();
            s.instance = Some(inst.clone());
            s.modal = true;
            s.parameters_wanted = want_parameters;
        }
        // The OnInit ran at the procedure that used the variable first (request-page-oninit-at-first-use).
        if self.request_page_init_done(inst) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RequestPageOnInitAtFirstUse);
            st.borrow_mut().inited = true;
        }
        inst.borrow_mut().globals[page.curr_page_slot] = Value::TestPage(st.clone());
        // Each reportextension that extends the request page runs on an instance of its own: the
        // one the run uses when it is the extension that changes the run, so that what a handler
        // writes into the extension's variables is what its triggers read.
        let mut exts = Vec::new();
        for &e in &page.extensions {
            let ext = match run_ext.iter().find(|(re, _)| *re == e) {
                Some((_, i)) => i.clone(),
                None => self.instance(e),
            };
            exts.push((e, ext));
        }
        // A parameters string is loaded after the page's OnInit and before its OnOpenPage: OnInit sets a variable,
        // OnOpenPage and the handler read the seeded value (report/lt-aq/AqTests.al:362-381,
        // Seeded_OnInitSetsControl_HandlerReads: `OnInit int=5 opt=Alpha;OnOpenPage int=9 opt=Gamma;`).
        st.borrow_mut().request_seed = seed.map(str::to_string);
        st.borrow_mut().ext_instances = exts.clone();
        let Some((hobject, hroutine, hinst)) =
            self.open_page_as(&st, PageMode::Edit, None, Some(UiKind::RequestPage), false)?
        else {
            return unsupported("a request page without a handler");
        };
        self.ui_state.page_handler_depth += 1;
        // The handler of a `RunRequestPage` finds no transaction, while the page's own triggers write as ever
        // (report/TestProbeReportContracts.al:RequestPage_TableWriteInsideHandler; handlers/TestReportInstanceRunReqPage.al
        // writes a log row from OnOpenPage).
        let outer = std::mem::replace(&mut self.report.request_only_page, want_parameters);
        let handled = self.call(hobject, hroutine, hinst, vec![Value::TestPage(st.clone())]);
        self.report.request_only_page = outer;
        self.ui_state.page_handler_depth -= 1;
        if let Err(e) = handled {
            let open = st.borrow().open;
            if let (Signal::Error(err), true) = (&e, open) {
                let closed = self.close_after_handler_error(&st, err);
                self.mark_closed(&st);
                closed?;
            }
            self.mark_closed(&st);
            return Err(e);
        }
        if st.borrow().open {
            self.mark_closed(&st);
            return unsupported("a request page handler that returned without closing the page (not pinned)");
        }
        let result = st.borrow().result.clone();
        match result.as_deref() {
            Some("OK") => {
                let parameters = if want_parameters {
                    Some(self.request_parameters(inst, &exts, object)?)
                } else {
                    None
                };
                Ok((RequestClosed::Ok, parameters))
            }
            Some("Cancel") => Ok((RequestClosed::Cancel, None)),
            Some("Preview") => Ok((RequestClosed::Preview, None)),
            Some("Print") => Ok((RequestClosed::Print, None)),
            Some("SaveAsXml" | "SaveAsPdf" | "SaveAsExcel") => Ok((RequestClosed::SavedAsXml, None)),
            other => unsupported(format!("a request page closed with {} (not pinned)", other.describe())),
        }
    }

    /// An AL error the handler does not catch closes the still open request page with Cancel before it leaves the run:
    /// OnQueryClosePage(Cancel), then OnClosePage, and the handler's error is the one raised
    /// (report/lt-report-requestpage/RptRequestPage.al:304, :413, :453). The corpus pins this for a page without a
    /// source table, parts, close triggers of an extension or refused entries; any other shape, an answer or an error of
    /// those triggers, and the page events are not pinned and are refused. With SaveValues the values are not stored,
    /// as after a Cancel action (handlers/TestPageSaveValues.al); whether BC stores them here is not pinned, and the
    /// earlier behavior is kept for handlers/TestReportMultiLineDataItemView.al:GetDemandToReserve_HandlerError_ReachesTheTest.
    fn close_after_handler_error(&mut self, st: &St, err: &al_builtins::native::AlError) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        let refuse = |what: &str| unsupported(format!("a request page handler error with {what} (close not pinned)"));
        let ext_closes = page_of(&self.engine(), st)
            .ext_triggers
            .iter()
            .any(|(n, _, _)| n == "onqueryclosepage" || n == "onclosepage");
        if ext_closes {
            return refuse("close triggers in a request page extension");
        }
        if self.has_source(st) {
            return refuse("a request page with a source table");
        }
        if !st.borrow().parts.is_empty() {
            return refuse("a request page with parts");
        }
        // One refused entry whose own TestValidation error is the one leaving the handler (the handler did not catch its
        // SetValue) closes the page the same way: OnQueryClosePage(Cancel) sees the variable's old value, then OnClosePage
        // (report/lt-report-requestpage/RptRequestPage.al:1243 RI_HandlerSetValue_OnValidateError_ReachesRun, an OnValidate
        // error; :1264 RI_HandlerSetValue_NotEvaluable_ReachesRun, a value that cannot be evaluated). Two refused entries,
        // an unsaved entry, or another error leaving the handler are not pinned.
        let refused_entry_left = {
            let s = st.borrow();
            match s.errors.as_slice() {
                [] => !s.dirty,
                [(idx, stored)] => {
                    let engine = self.engine();
                    let name = &page_of(&engine, st).controls[*idx].name;
                    !s.dirty
                        && err.code == "TestValidation"
                        && err.message == format!("Validation error for Field: {name},  Message = '{stored}'")
                }
                _ => false,
            }
        };
        if !refused_entry_left {
            return refuse("a refused or unsaved entry on the request page");
        }
        if self.declares(st, "onaftergetcurrrecord") {
            return refuse("OnAfterGetCurrRecord on the request page");
        }
        let engine = self.engine();
        if engine.events.is_subscribed(object, "onqueryclosepageevent")
            || engine.events.is_subscribed(object, "onclosepageevent")
        {
            return refuse("subscribers to the request page's close events");
        }
        let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
            .iter()
            .map(|m| m.to_string())
            .collect();
        let cancel = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == "Cancel")
            .expect("Cancel is an Action member") as i32;
        match self.query_close_answer(st, vec![Value::Option(cancel, members)]) {
            Ok(Some(false)) => return refuse("an OnQueryClosePage that answers false"),
            Ok(_) => {}
            Err(Signal::Error(_)) => return refuse("an error in OnQueryClosePage"),
            Err(other) => return Err(other),
        }
        match self.page_trigger(st, "onclosepage", vec![]) {
            Ok(_) => Ok(()),
            Err(Signal::Error(_)) => refuse("an error in OnClosePage"),
            Err(other) => Err(other),
        }
    }

    /// `TestRequestPage.SaveAsXml(ParametersFileName, ReportDataFileName)` from a request page handler: runs the
    /// report there and then with the page's values, writes the parameters file and the .NET DataSet of the
    /// rows, and closes the page. Pinned by al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al
    /// (a Target = OnPrem app): one run with the request page's filters applied, two files, every later use
    /// of the handler's page raises "The TestPage is not open.". What the parameters file holds between the
    /// parts the corpus asserts is assumed (docs/probes-needed.md).
    pub(super) fn request_save_as_xml(&mut self, st: &St, args: &[Value]) -> Result<Value, Signal> {
        self.require_open(st)?;
        if !self.is_request_page(st) {
            return unsupported("SaveAsXml on a page that is not a request page");
        }
        let (Some(Value::Text(parameters_file)), Some(Value::Text(data_file))) = (args.first(), args.get(1)) else {
            return unsupported("SaveAsXml with file names that are not Text");
        };
        let (parameters_file, data_file) = (parameters_file.clone(), data_file.clone());
        let object = st.borrow().object as usize;
        let inst = st.borrow().instance.clone().expect("open page");
        let schema = self.report_schema_of(&inst)?;
        if let Some(why) = schema.report().dataset_unsupported.first() {
            return unsupported(format!("SaveAsXml of \"{}\": {why}", schema.name()));
        }
        if schema.report().transaction_type.is_some() {
            return unsupported("SaveAsXml of a report that declares a TransactionType (not pinned)");
        }
        let written = self.request_page_report_run(st, object, &inst, &schema)?;
        let data = written.finish_data_set()?;
        let parameters = self.save_as_xml_parameters();
        self.host
            .files
            .write(&parameters_file, &parameters)
            .map_err(Signal::Unsupported)?;
        self.host.files.write(&data_file, &data).map_err(Signal::Unsupported)?;
        st.borrow_mut().result = Some("SaveAsXml".into());
        self.mark_closed(st);
        Ok(Value::Null)
    }

    /// `TestRequestPage.SaveAsPdf(FileName)` and `SaveAsExcel(FileName)` from a request page handler (ADR-040): for a
    /// report on the allow-list the report runs there and then with the page's values, no output is written, the file
    /// name is marked as pending, and the page is closed as after `SaveAsXml`. Any other report stays refused.
    pub(super) fn request_save_as_render(
        &mut self,
        st: &St,
        args: &[Value],
        format: &'static str,
    ) -> Result<Value, Signal> {
        self.require_open(st)?;
        if !self.is_request_page(st) {
            return unsupported(format!("SaveAs{format} on a page that is not a request page"));
        }
        let name = match args.first() {
            Some(Value::Text(t) | Value::Code(t)) => t.to_string(),
            _ => return unsupported(format!("SaveAs{format} with a file name that is not Text")),
        };
        let object = st.borrow().object as usize;
        let inst = st.borrow().instance.clone().expect("open page");
        let schema = self.report_schema_of(&inst)?;
        if !self.output_is_ignored(&schema) {
            return unsupported(format!(
                "SaveAs{format} of \"{}\": only the Xml dataset is produced (a rendered format needs a layout renderer, \
                 and the Linux tier the corpus was measured on does not render them either)",
                schema.name()
            ));
        }
        // What the handler sees of a layout without a file, or of a processing-only report, is not pinned here.
        let report = schema.report();
        if report.layout_file_missing || report.processing_only {
            return unsupported(format!(
                "SaveAs{format} of \"{}\", which has no layout file or is processing-only (what the handler sees is not pinned)",
                schema.name()
            ));
        }
        if let Some(why) = report.dataset_unsupported.first() {
            return unsupported(format!("SaveAs{format} of \"{}\": {why}", schema.name()));
        }
        if report.transaction_type.is_some() {
            return unsupported(format!(
                "SaveAs{format} of a report that declares a TransactionType (not pinned)"
            ));
        }
        let written = self.request_page_report_run(st, object, &inst, &schema)?;
        // The dataset is built to run the triggers and dropped; the file is never written.
        drop(written);
        self.render_pending_files
            .insert(crate::ctx::pending_key(&name), schema.name().into());
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportRenderDiscarded);
        st.borrow_mut().result = Some(format!("SaveAs{format}"));
        self.mark_closed(st);
        Ok(Value::Null)
    }

    /// Runs the report of the open request page there and then, with the page's values: the part of `SaveAsXml`,
    /// `SaveAsPdf` and `SaveAsExcel` they share. Gives the dataset the run wrote.
    fn request_page_report_run(
        &mut self,
        st: &St,
        object: usize,
        inst: &Inst,
        schema: &crate::reports::Schema,
    ) -> Result<crate::dataset::Dataset, Signal> {
        let ext: Vec<(Inst, usize)> = {
            let state = st.borrow();
            schema
                .report()
                .extensions
                .iter()
                .filter_map(|want| {
                    state
                        .ext_instances
                        .iter()
                        .find(|(e, _)| e == want)
                        .map(|(e, i)| (i.clone(), *e))
                })
                .collect()
        };
        let run = self.report_run_for(object, inst, ext, true);
        let outer = self.report.dataset.replace(crate::dataset::Dataset::default());
        let ran = self.report_after_page(&run, schema);
        let written = std::mem::replace(&mut self.report.dataset, outer);
        match ran {
            Ok(true) => {}
            Ok(false) => return unsupported("CurrReport.Quit during a SaveAsXml (not pinned)"),
            Err(Signal::Error(e)) => {
                return unsupported(format!(
                    "an error in a SaveAsXml run (what the handler sees is not pinned): {}",
                    e.message
                ));
            }
            Err(other) => return Err(other),
        }
        Ok(written.expect("a request page run has a dataset"))
    }

    /// The parameters file of `SaveAsXml`: a byte order mark, then `ArrayOfReportParameter` with the user id
    /// and the execution time. LF line ends, none after the last element; each parameter name is followed by a
    /// fixed 32 digit GUID; the execution time has seven fraction digits and `+00:00`
    /// (al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:SaveAsXml_ParametersFile_RawForm).
    fn save_as_xml_parameters(&self) -> Vec<u8> {
        let ms = self.now_millis();
        let day = ms.div_euclid(86_400_000) + i64::from(al_builtins::format::day_from_civil(1970, 1, 1));
        let (y, mo, d) = al_builtins::format::civil_from_day(day as i32);
        let in_day = ms.rem_euclid(86_400_000);
        let time = format!(
            "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{:03}0000+00:00",
            in_day / 3_600_000,
            in_day / 60_000 % 60,
            in_day / 1000 % 60,
            in_day % 1000
        );
        let one = |name: &str, value: &str| {
            format!(
                "  <ReportParameter>\r\n    <Name>{name}</Name>\r\n    <Values>\r\n      <string>{value}</string>\r\n    </Values>\r\n    <Visible>true</Visible>\r\n  </ReportParameter>\r\n"
            )
        };
        let text = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<ArrayOfReportParameter xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">\r\n{}{}</ArrayOfReportParameter>",
            one(
                "NavUserIdAE6AEB42076449B78A3E78A5EE366751",
                &xml_escape(&self.host.identity.user_id())
            ),
            one("ExecutionTime7587CE19257C4965B7E6066BDFF21EE3", &time)
        );
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    /// A request page with `ShowFilter = false` hides the filters of its data items. What a TestRequestPage or
    /// `RunRequestPage` then does with them is not pinned by the corpus (docs/probes-needed.md).
    fn require_filters_shown(&self, object: usize) -> Result<(), Signal> {
        let hidden = self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("ShowFilter") && v.trim().eq_ignore_ascii_case("false"));
        if hidden {
            return unsupported("a data item of a request page with ShowFilter = false (not pinned)");
        }
        Ok(())
    }

    /// The record of data item control `idx` of the open request page.
    fn request_item_record(&self, st: &St, idx: usize) -> Result<Rec, Signal> {
        let engine = self.engine();
        let ir::Binding::Var(slot) = page_of(&engine, st).controls[idx].binding else {
            return unsupported("a data item control without a record");
        };
        let inst = st.borrow().instance.clone().expect("open page");
        let v = inst.borrow().globals[slot].clone();
        match v {
            Value::Record(r) => Ok(r),
            other => unsupported(format!("a data item control holding {}", other.describe())),
        }
    }

    /// The methods of a data item of the request page (`TestFilter`): they read and write the
    /// filters of the item's record, in the user filter group 0, as the page's `Filter` does
    /// (handlers/TestReportRunRequestPage.al: `RequestPage.Rows.SetFilter("Entry No.", '1')` reaches
    /// the parameters). Nothing here moves a row: a request page has none.
    pub(super) fn request_item_op(
        &mut self,
        method: PageOp,
        st: &St,
        idx: usize,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        self.require_filters_shown(st.borrow().object as usize)?;
        let rec = self.request_item_record(st, idx)?;
        let table = self.table(&rec)?;
        let field_arg = |i: usize| match args.get(i) {
            Some(Value::Integer(p)) => Ok(*p as usize),
            other => unsupported(format!("TestFilter field argument {}", other.describe())),
        };
        match method {
            PageOp::TestFilterSetFilter => {
                let pos = field_arg(0)?;
                let text = match args.get(1) {
                    Some(Value::Text(t) | Value::Code(t)) => t.clone(),
                    other => return unsupported(format!("TestFilter.SetFilter with {}", other.describe())),
                };
                let ty = table.fields[pos].ty.clone();
                // An empty text takes the filter off the field (see pages/filter.rs).
                if text.trim().is_empty() {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::TestFilterSetFilterEmptyClears,
                    );
                    rec.borrow_mut().filters.retain(|f| !(f.field == pos && f.group == 0));
                    return Ok(Value::Null);
                }
                let expr = self
                    .parse_filter_text(&text, &ty)
                    .map_err(|e| Signal::Unsupported(format!("TestFilter.SetFilter: {e}")))?;
                let mut r = rec.borrow_mut();
                match r.filters.iter().position(|f| f.field == pos && f.group == 0) {
                    Some(i) => r.filters[i].expr = expr,
                    None => r.filters.push(FieldFilter {
                        field: pos,
                        expr,
                        group: 0,
                    }),
                }
                Ok(Value::Null)
            }
            PageOp::TestFilterGetFilter => {
                let pos = field_arg(0)?;
                let first = rec.borrow().filters.iter().find(|f| f.field == pos).cloned();
                match first {
                    Some(f) => Ok(Value::Text(self.filter_text(&f.expr)?.into())),
                    None => Ok(Value::Text(String::new().into())),
                }
            }
            other => unsupported(format!("{other} on a data item of a request page (not pinned)")),
        }
    }

    /// `Invoke` of an action of the request page. BC's test client does not find it: the error
    /// names the action by an id the compiler derives from the report id and the action name
    /// (handlers/TestReportRequestPageAction.al:RequestPageAction_Invoke_IsNotFoundOnThePage_AndOnActionDoesNotRun
    /// and the two tests after it), and no trigger runs. `Visible` and `Enabled` raise the same error
    /// (report/W3ReportActions.al:186).
    pub(super) fn request_action_not_found(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let o = &engine.program.objects[object];
        let c = &page_of(&engine, st).controls[idx];
        let Some(id) = o.id else {
            return unsupported("an action of a request page of a report without an id");
        };
        let message = format!(
            "The action with ID = {} is not found on the page.",
            action_id(id, &c.name)
        );
        // The code is TestActionNotFound for an action declared `Enabled = false` or `Visible = false`
        // (report/lt-aq/AqTests.al:334-358, Actions_DisabledAndHidden_Invoke); for any other action it is not pinned.
        let off = |name: &str| {
            c.props
                .iter()
                .any(|(k, v)| k == name && matches!(v, ir::PropVal::Lit(l) if l.eq_ignore_ascii_case("false")))
        };
        let code = if off("enabled") || off("visible") {
            "TestActionNotFound".to_string()
        } else {
            al_builtins::native::UNPINNED_ERROR.to_string()
        };
        Err(Signal::Error(Box::new(al_builtins::native::AlError {
            message,
            code,
            ..Default::default()
        })))
    }

    /// The parameters of the request page as BC returns them from `RunRequestPage`: the report's
    /// options (the controls bound to variables) and the view of every data item.
    pub(crate) fn request_parameters(
        &mut self,
        inst: &Inst,
        exts: &[(usize, Inst)],
        object: usize,
    ) -> Result<String, Signal> {
        let engine = self.engine();
        let o = &engine.program.objects[object];
        let page = o.page.as_ref().expect("a request page");
        let mut options = String::new();
        let mut items = String::new();
        for c in &page.controls {
            match (&c.kind, &c.binding) {
                (ir::ControlKind::Field, b) if b.var_slot(c.owner).is_some() => {
                    let (slot, who) = b.var_slot(c.owner).expect("a variable binding");
                    let owner = owner_inst(inst, exts, who);
                    let v = owner.borrow().globals[slot].clone();
                    let text = match &v {
                        Value::Boolean(b) => b.to_string(),
                        // An option travels as its ordinal.
                        Value::Option(n, _) | Value::Enum(_, n) => n.to_string(),
                        other => al_builtins::format::try_format(other, 0, 9, self).map_err(Signal::Unsupported)?,
                    };
                    // The parameter carries the name of the variable, not of the control (report/TestProbeReportContracts.al:
                    // RunRequestPage_ParametersText: control OptName shows variable NameValue).
                    let var_name = engine.program.objects[who.unwrap_or(object)]
                        .globals
                        .get(slot)
                        .map_or_else(|| c.name.clone(), |g| g.name.clone());
                    // An empty value is an empty element, `<Field name="TxtVal" />`, for Text, Code, Date, Time and DateTime
                    // alike (report/W3ReportParametersText.al:255).
                    if text.is_empty() {
                        options.push_str(&format!("<Field name=\"{}\" />", xml_escape(&var_name)));
                    } else {
                        options.push_str(&format!(
                            "<Field name=\"{}\">{}</Field>",
                            xml_escape(&var_name),
                            xml_text(&text)
                        ));
                    }
                }
                (ir::ControlKind::DataItem, ir::Binding::Var(slot)) => {
                    // ShowFilter = false still writes the data item with its view (report/TestProbeReportContracts.al:
                    // RequestPage_ShowFilterFalse); only the item's filter methods stay refused.
                    let rec = match inst.borrow().globals[*slot].clone() {
                        Value::Record(r) => r,
                        other => {
                            return unsupported(format!(
                                "data item {} holds {other}",
                                c.name,
                                other = other.describe()
                            ));
                        }
                    };
                    let view = self.parameter_view(&rec)?;
                    items.push_str(&format!(
                        "<DataItem name=\"{}\">{}</DataItem>",
                        xml_escape(&c.name),
                        xml_text(&view)
                    ));
                }
                _ => {}
            }
        }
        let id = o.id.map_or(String::new(), |i| i.to_string());
        // A page with no option control has no `Options` element
        // (handlers/TestReportRequestPageSourceTableCompiledContracts.al:RunRequestPage_RequestPageRecIsUnfiltered).
        let options = if options.is_empty() {
            String::new()
        } else {
            format!("<Options>{options}</Options>")
        };
        Ok(format!(
            "<?xml version=\"1.0\" standalone=\"yes\"?><ReportParameters name=\"{}\" id=\"{id}\">{options}<DataItems>{items}</DataItems></ReportParameters>",
            xml_escape(&o.name)
        ))
    }

    /// Loads a parameters string into the request page before it opens: the report's options and
    /// the view of every data item. Only the text this runtime hands out is understood (its own
    /// `RunRequestPage` result), because the corpus pins nothing else: a string it cannot read, or
    /// that names another report, is refused
    /// (handlers/TestReportInstanceRunReqPage.al:TestReport_InstanceRunRequestPage_SeededWithParameters_StillRunsTheHandler
    /// pins only that the handler still runs and its own filter comes back).
    pub(crate) fn apply_request_parameters(
        &mut self,
        inst: &Inst,
        exts: &[(usize, Inst)],
        object: usize,
        xml: &str,
    ) -> Result<(), Signal> {
        let engine = self.engine();
        let o = &engine.program.objects[object];
        let page = o.page.as_ref().expect("a request page");
        let head = format!(
            "<?xml version=\"1.0\" standalone=\"yes\"?><ReportParameters name=\"{}\" id=\"{}\">",
            xml_escape(&o.name),
            o.id.map_or(String::new(), |i| i.to_string())
        );
        if !xml.starts_with(&head) || !xml.ends_with("</ReportParameters>") {
            return unsupported(
                "a parameters string this runtime did not produce for this report (its format is not pinned)",
            );
        }
        let options = xml
            .split("<Options>")
            .nth(1)
            .and_then(|r| r.split("</Options>").next())
            .unwrap_or("");
        for (name, text) in named_elements(options, "Field") {
            let Some(c) = page.controls.iter().find(|c| {
                c.kind == ir::ControlKind::Field
                    && c.binding.var_slot(c.owner).is_some_and(|(slot, who)| {
                        engine.program.objects[who.unwrap_or(object)]
                            .globals
                            .get(slot)
                            .is_some_and(|g| g.name == name)
                    })
            }) else {
                return unsupported(format!(
                    "a parameter for the variable {name}, which no control of the request page shows"
                ));
            };
            let Some((slot, who)) = c.binding.var_slot(c.owner) else {
                return unsupported(format!("a parameter for the control {name}, which shows no variable"));
            };
            let owner = owner_inst(inst, exts, who);
            let current = owner.borrow().globals[slot].clone();
            let value = match &current {
                Value::Text(_) => Value::Text(text.into()),
                Value::Code(_) => Value::Code(text.into()),
                Value::Boolean(_) if text == "true" || text == "false" => Value::Boolean(text == "true"),
                Value::Integer(_) | Value::Decimal(_) | Value::Option(..) | Value::Enum(..) => {
                    let ty = match &current {
                        Value::Integer(_) => RtType::Integer,
                        Value::Decimal(_) => RtType::Decimal,
                        Value::Option(_, m) => RtType::Option(m.iter().cloned().collect()),
                        Value::Enum(id, _) => RtType::Enum(*id),
                        _ => unreachable!(),
                    };
                    let number = match &current {
                        Value::Decimal(_) => ops::convert(Value::Text(text.clone().into()), &RtType::Decimal),
                        _ => text
                            .parse::<i32>()
                            .map(Value::Integer)
                            .map_err(|_| Signal::Unsupported(String::new())),
                    };
                    match (number, &current) {
                        (Ok(Value::Integer(n)), Value::Option(_, m)) => Value::Option(n, m.clone()),
                        (Ok(Value::Integer(n)), Value::Enum(id, _)) => Value::Enum(*id, n),
                        (Ok(v), _) => ops::convert(v, &ty)?,
                        (Err(_), _) => return unsupported(format!("the parameter {name} = {text}")),
                    }
                }
                other => {
                    return unsupported(format!(
                        "a parameter for the control {name} holding {}",
                        other.describe()
                    ));
                }
            };
            owner.borrow_mut().globals[slot] = value;
        }
        let items = xml
            .split("<DataItems>")
            .nth(1)
            .and_then(|r| r.split("</DataItems>").next())
            .unwrap_or("");
        for (name, view) in named_elements(items, "DataItem") {
            let Some(c) = page
                .controls
                .iter()
                .find(|c| c.kind == ir::ControlKind::DataItem && c.name == name)
            else {
                return unsupported(format!(
                    "a parameter for the data item {name}, which the request page does not have"
                ));
            };
            self.require_filters_shown(object)?;
            let ir::Binding::Var(slot) = c.binding else { continue };
            let Value::Record(rec) = inst.borrow().globals[slot].clone() else {
                continue;
            };
            self.apply_parameter_view(&rec, &view)?;
        }
        Ok(())
    }

    /// `Report.RunRequestPage(Id, Parameters)` loads the parameters string after the request page's OnInit
    /// (report/lt-aq/AqTests.al:362-381 pins it for a control value only). On a report with an OnInit or OnInitReport
    /// trigger, a data item view in the string that differs from the view the item has at that point is refused: whether
    /// it replaces or adds to a view those triggers set, and what they read of it, is not pinned.
    pub(crate) fn refuse_seeded_view_after_init(
        &mut self,
        inst: &Inst,
        object: usize,
        xml: &str,
    ) -> Result<(), Signal> {
        let engine = self.engine();
        let o = &engine.program.objects[object];
        let page = o.page.as_ref().expect("a request page");
        let on_init = |p: &ir::Page| p.triggers.iter().any(|(n, _)| n == "oninit");
        let has_init = on_init(page)
            || o.report.as_ref().is_some_and(|r| r.on_init.is_some())
            || page
                .extensions
                .iter()
                .any(|&e| engine.program.objects[e].page.as_ref().is_some_and(on_init));
        if !has_init {
            return Ok(());
        }
        let items = xml
            .split("<DataItems>")
            .nth(1)
            .and_then(|r| r.split("</DataItems>").next())
            .unwrap_or("");
        for (name, view) in named_elements(items, "DataItem") {
            let Some(c) = page
                .controls
                .iter()
                .find(|c| c.kind == ir::ControlKind::DataItem && c.name == name)
            else {
                continue;
            };
            let ir::Binding::Var(slot) = c.binding else { continue };
            let Value::Record(rec) = inst.borrow().globals[slot].clone() else {
                continue;
            };
            if self.parameter_view(&rec)?.trim() != view.trim() {
                return unsupported(format!(
                    "a view for the data item {name} in the parameters of RunRequestPage, on a report with OnInit or OnInitReport (whether it replaces the view those triggers set is not pinned)"
                ));
            }
        }
        Ok(())
    }

    /// The inverse of `parameter_view`.
    fn apply_parameter_view(&mut self, rec: &Rec, view: &str) -> Result<(), Signal> {
        let t = self.table(rec)?;
        let by_id = |text: &str| {
            text.trim()
                .strip_prefix("Field")
                .and_then(|n| n.parse::<u32>().ok())
                .and_then(|id| t.fields.iter().position(|f| f.id == id))
        };
        let mut rest = view.trim();
        let mut key = None;
        let mut ascending = true;
        let mut filters = Vec::new();
        while !rest.is_empty() {
            let Some(open) = rest.find('(') else {
                return unsupported(format!("a view this runtime did not write: {view}"));
            };
            let word = rest[..open].trim().to_ascii_uppercase();
            let mut depth = 0usize;
            let mut close = None;
            for (i, c) in rest[open..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(close) = close else {
                return unsupported(format!("a view this runtime did not write: {view}"));
            };
            let inner = &rest[open + 1..close];
            match word.as_str() {
                "VERSION" => {}
                "SORTING" => {
                    let wanted: Option<Vec<usize>> = inner.split(',').map(by_id).collect();
                    key = wanted.and_then(|w| t.keys.iter().position(|k| k.len() >= w.len() && k[..w.len()] == w[..]));
                    if key.is_none() {
                        return unsupported(format!("a view with a sorting no key has: {view}"));
                    }
                }
                "ORDER" => ascending = !inner.trim().eq_ignore_ascii_case("descending"),
                "WHERE" => {
                    for cond in split_conditions(inner) {
                        let Some((field, filter)) = cond.split_once("=1(") else {
                            return unsupported(format!("a view condition this runtime did not write: {cond}"));
                        };
                        let (Some(pos), Some(text)) = (by_id(field), filter.strip_suffix(')')) else {
                            return unsupported(format!("a view condition this runtime did not write: {cond}"));
                        };
                        let ty = t.fields[pos].ty.clone();
                        let expr = self
                            .parse_filter_text(text, &ty)
                            .map_err(|e| Signal::Unsupported(format!("a parameter filter: {e}")))?;
                        filters.push(FieldFilter {
                            field: pos,
                            expr,
                            group: 0,
                        });
                    }
                }
                other => return unsupported(format!("a view part {other} this runtime did not write")),
            }
            rest = rest[close + 1..].trim();
        }
        let mut r = rec.borrow_mut();
        r.filters.retain(|f| f.group != 0);
        r.filters.extend(filters);
        r.key = key.unwrap_or(0);
        r.ascending = ascending;
        Ok(())
    }

    /// A record's view the way the parameters spell it: field numbers, not captions, and each
    /// filter as `<n>(<filter>)` (the corpus pins `WHERE(Field1=1(1))` for `SetFilter("Entry No.",
    /// '1')`: handlers/TestReportRunRequestPage.al:81-86). A descending view is `ORDER(1)` and a Boolean
    /// filter on Yes is `1` (report/W3ReportParametersText.al:256, :306). Only the filters the page holds
    /// in filter group 0 are written: those of the item's DataItemTableView are not
    /// (report/W3ReportSaveAsExecute.al:209). The leading `VERSION(1)` and the key clause are docs/probes-needed.md.
    /// Not pinned, and refused: a filter in another group, a Boolean filter on No, a direction per key field.
    fn parameter_view(&mut self, rec: &Rec) -> Result<String, Signal> {
        let t = self.table(rec)?;
        let (key, ascending, field_descending, mut filters) = {
            let r = rec.borrow();
            (r.key, r.ascending, r.field_descending, r.filters.clone())
        };
        if field_descending != 0 {
            return unsupported(
                "a data item view with a direction per key field in the request page parameters (not pinned)",
            );
        }
        filters.retain(|f| f.group != crate::reports::ITEM_VIEW_GROUP);
        if filters.iter().any(|f| f.group != 0) {
            return unsupported(
                "a data item filter in a filter group other than 0 in the request page parameters (not pinned)",
            );
        }
        filters.sort_by_key(|f| f.field);
        let field = |pos: usize| format!("Field{}", t.fields[pos].id);
        let mut out = format!(
            "VERSION(1) SORTING({})",
            t.keys[key].iter().map(|&p| field(p)).collect::<Vec<_>>().join(",")
        );
        if !ascending {
            out.push_str(" ORDER(1)");
        }
        if !filters.is_empty() {
            let mut parts = Vec::new();
            for f in &filters {
                let expr = boolean_as_number(&f.expr)?;
                parts.push(format!("{}=1({})", field(f.field), self.filter_text(&expr)?));
            }
            out.push_str(&format!(" WHERE({})", parts.join(",")));
        }
        Ok(out)
    }
}

/// `e` with a Boolean Yes written as `1` (report/W3ReportParametersText.al:306 `Field4=1(1)` for `SetFilter(Flag, 'Yes')`).
/// No is not pinned and refused.
fn boolean_as_number(e: &FilterExpr) -> Result<FilterExpr, Signal> {
    let map = |v: &Value| -> Result<Value, Signal> {
        match v {
            Value::Boolean(true) => Ok(Value::Integer(1)),
            Value::Boolean(false) => unsupported("a Boolean filter on No in the request page parameters (not pinned)"),
            other => Ok(other.clone()),
        }
    };
    let opt = |v: &Option<Value>| -> Result<Option<Value>, Signal> { v.as_ref().map(map).transpose() };
    Ok(match e {
        FilterExpr::Range(a, b) => FilterExpr::Range(opt(a)?, opt(b)?),
        FilterExpr::Eq(v) => FilterExpr::Eq(map(v)?),
        FilterExpr::RangeSame(v) => FilterExpr::RangeSame(map(v)?),
        FilterExpr::Ne(v) => FilterExpr::Ne(map(v)?),
        FilterExpr::Lt(v) => FilterExpr::Lt(map(v)?),
        FilterExpr::Le(v) => FilterExpr::Le(map(v)?),
        FilterExpr::Gt(v) => FilterExpr::Gt(map(v)?),
        FilterExpr::Ge(v) => FilterExpr::Ge(map(v)?),
        FilterExpr::Or(xs) => FilterExpr::Or(xs.iter().map(boolean_as_number).collect::<Result<_, _>>()?),
        FilterExpr::And(xs) => FilterExpr::And(xs.iter().map(boolean_as_number).collect::<Result<_, _>>()?),
        other => other.clone(),
    })
}

/// The instance that holds the variables of a control: the report's, or its extension's.
fn owner_inst(report: &Inst, exts: &[(usize, Inst)], owner: Option<usize>) -> Inst {
    match owner {
        None => report.clone(),
        Some(e) => exts
            .iter()
            .find(|(o, _)| *o == e)
            .map(|(_, i)| i.clone())
            .expect("an extension of the request page has an instance"),
    }
}

/// The text between `<tag ...>` and `</tag>` pairs of `xml`, as (attribute `name`, unescaped text).
fn named_elements(xml: &str, tag: &str) -> Vec<(String, String)> {
    let open = format!("<{tag} name=\"");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at + open.len()..];
        let Some(q) = after.find("\">") else { break };
        let name = xml_unescape(&after[..q]);
        let body = &after[q + 2..];
        let Some(end) = body.find(&close) else { break };
        out.push((name, xml_unescape(&body[..end])));
        rest = &body[end + close.len()..];
    }
    out
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// Splits `Field1=1(a),Field2=1(b)` at the commas outside parentheses.
fn split_conditions(s: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0i32, 0usize);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// The id BC's test client names an action of a request page by: FNV-1a over the UTF-16 text of
/// the report id followed by the action name, read as a signed 32-bit number and made positive.
/// Derived from the three ids the corpus pins, for report 60398
/// (handlers/TestReportRequestPageAction.al:31-33: StampText 727081671, AppendText 622821794,
/// RefuseAction 1188892507, which is the negation of the signed hash).
pub(crate) fn action_id(report_id: u32, name: &str) -> i64 {
    let mut h: u32 = 0x811c_9dc5;
    for unit in format!("{report_id}{name}").encode_utf16() {
        for b in unit.to_le_bytes() {
            h ^= u32::from(b);
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    i64::from(h as i32).abs()
}

/// Element text as BC writes it: `&`, `<` and `>` escaped, a double quote kept (report/W3ReportParametersText.al:272
/// `a&lt;b&amp;"c"`, :306 `Field3=1(&gt;10)`).
fn xml_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::action_id;

    // handlers/TestReportRequestPageAction.al:31-33
    #[test]
    fn action_ids_are_the_ones_the_corpus_pins() {
        assert_eq!(action_id(60398, "StampText"), 727_081_671);
        assert_eq!(action_id(60398, "AppendText"), 622_821_794);
        assert_eq!(action_id(60398, "RefuseAction"), 1_188_892_507);
    }

    // testpart/TestTestPart.al:233 and pageapplicationarea/TestPageApplicationAreaPrecompiledControlRemoval.al:57,92,124
    #[test]
    fn control_ids_are_the_ones_the_corpus_pins() {
        assert_eq!(action_id(60344, "Hidden"), 1_318_487_454);
        assert_eq!(action_id(329, "Name"), 301_776_846);
        assert_eq!(action_id(5361, "User Defined"), 104_384_132);
        assert_eq!(action_id(6478, "Ignore Status on Service Docs."), 1_790_009_997);
    }
}
