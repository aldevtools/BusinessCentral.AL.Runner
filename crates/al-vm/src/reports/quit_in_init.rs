//! `CurrReport.Quit` in OnInitReport of the report reports/runtime/ReportQuitInOnInit.al:34-102 measures, for the ways to
//! start it that a quit stopped only by refusing before (al-runner issue 7964 item 3; ReportQuitInOnInit.al:235-355):
//! RunRequestPage on a variable with no argument and the static one with the id alone or the id and '' answer '' and open
//! no page, an Xml SaveAs to a stream answers false and writes nothing, and `UseRequestPage(false)` as the first use of a variable quits so that its next Run or RunModal runs
//! nothing (a Clear gives the variable a new instance, which does not hold it). A Run, RunModal or Execute that runs
//! OnInitReport itself already stopped there (`REPORT_RUN_1 | REPORT_RUNMODAL_1`, `REPORT_EXECUTE_1`, the static forms)
//! and is not changed here.

use super::*;

/// How a caller of [`Session::report_instance`] takes a quit in the OnInitReport its first use runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InitQuit {
    /// The method cannot stop the report: a quit refuses.
    Refuses,
    /// The method stops: it gets the quit back.
    Stops,
    /// Run and RunModal of a variable: they stop, and also take the quit a `UseRequestPage(false)` deferred.
    StopsTakingDeferred,
    /// `UseRequestPage(false)`: a quit in the measured report is kept for the next Run or RunModal.
    Defers,
}

impl InitQuit {
    /// How `UseRequestPage` on `receiver` with argument `arg` takes a quit: `UseRequestPage(false)` on a variable as its first
    /// use keeps it (ReportQuitInOnInit.al:265); every other form refuses it.
    pub(super) fn of_use_request_page(receiver: &Option<Value>, arg: Option<&Value>) -> Self {
        if receiver.is_some() && matches!(arg, Some(Value::Boolean(false))) {
            InitQuit::Defers
        } else {
            InitQuit::Refuses
        }
    }
}

/// A quit in OnInitReport before a method that cannot stop the report, or of a report the probe did not measure.
const QUIT_BEFORE_NON_RUN: &str =
    "CurrReport.Quit in OnInitReport before a method that does not run the report (not pinned)";

impl Session {
    /// The report instance behind a receiver, initialised: the first use of a report variable
    /// runs `OnInitReport` (reports/TestReportOnInitReportTiming.al:50-60).
    /// Returns whether that trigger quit the report; `how` says what a quit does ([`InitQuit`]).
    pub(super) fn report_instance(&mut self, receiver: Option<Value>, how: InitQuit) -> Result<(Inst, bool), Signal> {
        match receiver {
            Some(Value::Codeunit(rc)) => {
                if let Some(report) = self.engine.program.objects[rc.borrow().object as usize].report.as_ref() {
                    dependency_items::require_pinned_entry(report, false)?;
                }
                if how == InitQuit::StopsTakingDeferred && self.quit_deferred_on(&rc) {
                    return Ok((rc.clone(), self.take_deferred_quit(&rc)));
                }
                self.refuse_after_kept_quit(&rc)?;
                // Used again after a Run, it is a new instance: OnInitReport runs.
                // A second Run on one variable runs OnInitReport again (reports/request-page/RptTriggerOrder.al:353
                // VariableRun_Twice_InitReportEachTime; reports/PnH5aExt.al:396).
                if std::mem::take(&mut rc.borrow_mut().rerun) {
                    rc.borrow_mut().initialized = false;
                }
                let quit = !rc.borrow().initialized && self.report_init(&rc)?;
                match how {
                    InitQuit::Defers if quit => self.defer_quit(&rc).map(|()| (rc, false)),
                    InitQuit::Refuses if quit => unsupported(QUIT_BEFORE_NON_RUN),
                    _ => Ok((rc, quit)),
                }
            }
            other => unsupported(format!("a Report method on {}", other.describe())),
        }
    }

    /// The report the probe measured (ReportQuitInOnInit.al:34-102): ProcessingOnly (:36), a request page (:57-80) without
    /// an OnInit trigger, no UseRequestPage property, no report extension.
    pub(super) fn quit_in_init_measured(&self, schema: &Schema) -> bool {
        let r = schema.report();
        let page = self.engine.program.objects[schema.object].page.as_ref();
        r.processing_only
            && r.has_request_page
            && r.use_request_page.is_none()
            && r.extensions.is_empty()
            && page.is_some_and(|p| p.ext_triggers.is_empty() && p.triggers.iter().all(|(n, _)| n != "oninit"))
    }

    /// Refuses every use of `inst` while it holds the quit `UseRequestPage(false)` kept: the probe runs the variable's Run or
    /// RunModal right after it (:265, :305, :355), and only those take the quit ([`InitQuit::StopsTakingDeferred`]). Called
    /// for every Report method on a variable (`report_builtin`, `report_instance`) and every call of one of its procedures
    /// (`first_use`).
    pub(crate) fn refuse_after_kept_quit(&self, inst: &Inst) -> Result<(), Signal> {
        if self.quit_deferred_on(inst) {
            return unsupported(
                "a use of a report variable after CurrReport.Quit in OnInitReport of UseRequestPage(false) (not pinned)",
            );
        }
        Ok(())
    }

    /// [`Self::refuse_after_kept_quit`] for Report method `id` on a variable, other than the Run and RunModal that take the quit.
    pub(super) fn refuse_method_after_kept_quit(
        &self,
        id: al_ir::BuiltinId,
        receiver: &Option<Value>,
    ) -> Result<(), Signal> {
        match receiver {
            Some(Value::Codeunit(rc)) if !matches!(id, ids::REPORT_RUN_1 | ids::REPORT_RUNMODAL_1) => {
                self.refuse_after_kept_quit(rc)
            }
            _ => Ok(()),
        }
    }

    /// Whether `inst` holds a quit that `UseRequestPage(false)` deferred.
    pub(super) fn quit_deferred_on(&self, inst: &Inst) -> bool {
        self.report
            .report_quit_deferred
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
    }

    /// The quit of the OnInitReport that `UseRequestPage(false)` ran as the first use of `inst` (:265, :305, :355): kept for the
    /// next Run or RunModal. Any other report than the measured one refuses.
    pub(super) fn defer_quit(&mut self, inst: &Inst) -> Result<(), Signal> {
        let schema = self.report_schema(inst)?;
        if !self.quit_in_init_measured(&schema) {
            return unsupported(QUIT_BEFORE_NON_RUN);
        }
        self.report.report_quit_deferred.retain(|w| w.strong_count() > 0);
        self.report.report_quit_deferred.push(Rc::downgrade(inst));
        Ok(())
    }

    /// Takes the deferred quit of `inst` for its Run or RunModal (:265 Run, :305 RunModal, :355 the first of two Runs).
    pub(super) fn take_deferred_quit(&mut self, inst: &Inst) -> bool {
        let had = self.quit_deferred_on(inst);
        if had {
            self.report
                .report_quit_deferred
                .retain(|w| w.upgrade().is_some_and(|i| !Rc::ptr_eq(&i, inst)));
        }
        had
    }

    /// RunRequestPage after a quit in OnInitReport: '' and no page, for the measured report and the argument shapes the probe
    /// measured (`request_page_args_measured`); any other refuses.
    pub(super) fn request_page_after_quit(
        &mut self,
        schema: &Schema,
        static_form: bool,
        args: &[Value],
    ) -> Result<Value, Signal> {
        if !(request_page_args_measured(static_form, args) && self.quit_in_init_measured(schema)) {
            return unsupported("CurrReport.Quit in OnInitReport of a RunRequestPage (not pinned)");
        }
        Ok(Value::Text("".into()))
    }

    /// An Xml SaveAs to a stream with its answer used after a quit in OnInitReport (:325 variable, :335 static): false and
    /// nothing written, for the measured report; any other refuses.
    pub(super) fn save_as_after_quit(
        &mut self,
        schema: &Schema,
        stream: bool,
        result_used: bool,
    ) -> Result<Value, Signal> {
        if !(stream && result_used && self.quit_in_init_measured(schema)) {
            return unsupported("CurrReport.Quit in OnInitReport of a SaveAs of a processing-only report (not pinned)");
        }
        Ok(Value::Boolean(false))
    }
}

/// The RunRequestPage arguments the probe measured after a quit in OnInitReport: the variable form with no argument (:154, test
/// :235), the static form with the report id alone (:156, test :245) and with the report id and the empty Text (:204, test
/// :255). A parameters string that is not empty is not measured.
fn request_page_args_measured(static_form: bool, args: &[Value]) -> bool {
    match (static_form, args) {
        (false, []) | (true, [_]) => true,
        (true, [_, Value::Text(parameters)]) => parameters.is_empty(),
        _ => false,
    }
}
