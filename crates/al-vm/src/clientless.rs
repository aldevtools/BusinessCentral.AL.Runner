//! What a session without a client does with a dialog: a page background task worker, and the test session itself under
//! `--no-client` (al-runner issue 7564, `Session::set_clientless_session`).
//!
//! In the worker every answer is pinned: GuiAllowed, Message and Confirm from pages/background-tasks/BjNoGuiDialogs.al:8-10
//! (`ui.rs`, `hostvalues.rs`), the other dialogs from pages/background-tasks/BjNoClientDialogs.al:168-199
//! RunPageBackgroundTask_Dialogs and :214-260 RunPageBackgroundTask_Dialogs_WithHandlers.
//!
//! The test session under `--no-client` stands for a web-service session: Microsoft's test runner called over OData. Its
//! answers are the reporter's measurements on BC 28.4 (al-runner issues 8154, 8158 and 8028), outside the pinned corpus: the
//! assumption `no-client-session` (each answer marks it), with the written probe in the corpus (`docs/probes-needed.md`).
//! GuiAllowed is false and CurrentClientType is ODataV4 (`hostvalues.rs`); the test framework calls the bound handlers of
//! Confirm, StrMenu, Message, Notification.Send and Hyperlink, and an unhandled Message or Hyperlink fails with "Unhandled UI"
//! (`ui.rs`, [`Session::no_client_test_session`]); a processing-only report skips its request page
//! ([`Session::clientless_report`]). BC fails a TestPage open and a read of the Session table there, with errors the corpus
//! does not measure: refused (`clientless_test_page_reason`, `platform` `Virtual::Session`). Every other dialog or call
//! shape is refused in both sessions (hard rules 3 and 4); the worker's answers do not stand in for the web-service session.
use crate::Session;
use crate::reports::Schema;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::page_op::{PageOp, PageRecv};
use al_builtins::value::Value;
use al_ir::BuiltinId;

/// The builtins that ask a client for something and that no probe covers in a session without a client, and
/// CurrentClientType: the name for the refusal text.
fn refused_name(id: BuiltinId) -> Option<&'static str> {
    Some(match id {
        ids::NOTIFICATION_RECALL_1 => "Notification.Recall",
        // Not a dialog: not measured in the worker. The test session under `--no-client` answers it (`clientless_guard`).
        ids::SESSION_CURRENTCLIENTTYPE_1 => "CurrentClientType",
        ids::FILE_DOWNLOAD_1
        | ids::FILE_DOWNLOADFROMSTREAM_1
        | ids::FILE_UPLOAD_1
        | ids::FILE_VIEW_1
        | ids::FILE_VIEWFROMSTREAM_1 => "a file upload, download or view",
        ids::REPORT_RUNREQUESTPAGE_1 | ids::REPORT_RUNREQUESTPAGE_2 => "Report.RunRequestPage",
        _ => return None,
    })
}

impl Session {
    /// Whether the code runs in a session without a client: a page background task worker, or the test session under
    /// `--no-client`. One test on the common path of every builtin call.
    #[inline]
    pub(crate) fn without_client(&self) -> bool {
        self.ui_state.clientless || self.pages.bg_worker.is_some()
    }

    /// Whether the code runs in the test session under `--no-client` (the web-service session of al-runner issue 8154): not
    /// in a page background task worker, and not in install code, which keeps the install session's pinned answers.
    #[inline]
    pub(crate) fn no_client_test_session(&self) -> bool {
        self.ui_state.clientless && self.pages.bg_worker.is_none() && !self.install_without_client
    }

    /// A dialog of `kind` (Message, Hyperlink, Confirm, StrMenu, Notification.Send) in the test session under `--no-client`.
    /// In a web-service session the test framework hands it to the test's bound handler, and an unhandled Message or
    /// Hyperlink fails with "Unhandled UI" (al-runner issue 8154, the reporter's cells on BC 28.4): `Ok`, the dialog goes on
    /// as in a session with a client, and the answer marks `no-client-session`. What a Confirm, StrMenu or Notification.Send
    /// that no handler takes does there is not measured: refused. Elsewhere `Ok` and nothing marked.
    pub(crate) fn no_client_dialog(&self, kind: UiKind) -> Result<(), Signal> {
        if !self.no_client_test_session() {
            return Ok(());
        }
        let call = match kind {
            UiKind::Message | UiKind::Hyperlink => None,
            UiKind::Confirm => Some("Confirm"),
            UiKind::StrMenu => Some("StrMenu"),
            UiKind::SendNotification => Some("Notification.Send"),
            other => return Err(self.no_client_refusal(other.label())),
        };
        if let Some(call) = call.filter(|_| !self.has_ui_handler(kind)) {
            return Err(self.no_client_refusal(&format!("{call} without a {}Handler", kind.label())));
        }
        assume(Assumption::NoClientSession);
        Ok(())
    }

    /// The refusal of `what`, a dialog or call shape no probe covers in a session without a client.
    pub(crate) fn no_client_refusal(&self, what: &str) -> Signal {
        if self.pages.bg_worker.is_some() {
            return Signal::unsupported(format!(
                "{what} in a page background task worker: what BC does there is not pinned"
            ));
        }
        Signal::unsupported(format!(
            "{what} in a session without a client (--no-client): what BC does there is not pinned (probe wanted, docs/probes-needed.md `no-client-session`)"
        ))
    }

    /// Why `what`, a TestPage or TestRequestPage shape, does not run in the test session under `--no-client`. BC fails a
    /// TestPage open in a web-service session (GuiAllowed false, ClientType ODataV4) with NotSupportedException at
    /// NavSession.CreateNavTestService: al-runner issue 8028, the reporter's measurement on BC, outside the corpus. The
    /// corpus does not measure that session, so the error is not copied and the open is refused (`clientless_page_guard`).
    pub(crate) fn clientless_test_page_reason(what: &str) -> String {
        format!(
            "{what} in a session without a client (--no-client): BC fails it there (al-runner issue 8028), and its error is not measured"
        )
    }

    /// Why a test that binds `handler`, a PageHandler or ModalPageHandler that takes a TestPage, does not run under
    /// `--no-client` (`ui_handler`): BC fails a TestPage open there (`clientless_test_page_reason`); whether and how it fails
    /// with such a handler bound is not measured.
    pub(crate) fn clientless_test_page_handler_reason(handler: &str) -> String {
        format!(
            "{handler}, which takes a TestPage, in a session without a client (--no-client): BC fails a TestPage open there (al-runner issue 8028), and what it does with this handler is not measured"
        )
    }

    /// Whether the whole test session is a session without a client (`set_clientless_session`).
    pub fn is_clientless_session(&self) -> bool {
        self.ui_state.clientless
    }

    /// The runner did not run the login code that sets WorkDate, only because the session has no client
    /// (`has_interactive_client`: Base Application's `LogInManagement.CompanyOpen` asks GuiAllowed). That GuiAllowed is
    /// false there is the assumption `no-client-session`, so a WorkDate read after it marks the assumption.
    pub fn note_clientless_login_skipped(&mut self) {
        self.ui_state.clientless_login_skipped = true;
    }

    /// A read of a WorkDate that no code set: marks `no-client-session` when the WorkDate the test session starts with
    /// rests on it (`note_clientless_login_skipped`).
    #[inline]
    pub(crate) fn clientless_work_date_read(&self) {
        if self.ui_state.clientless_login_skipped {
            assume(Assumption::NoClientSession);
        }
    }

    /// Builtin `id` with the operands `args` in a session without a client. `Some`: the worker's answer, the builtin is
    /// not called and no handler of the test sees the call (BjNoClientDialogs.al RunPageBackgroundTask_Dialogs_WithHandlers,
    /// `handlers after the task` empty). `None`: the builtin runs as in a session with a client. An error: the builtin, or
    /// this shape of it, is not probed.
    pub(crate) fn clientless_guard(&self, id: BuiltinId, args: &[Value]) -> Result<Option<Value>, Signal> {
        if self.pages.bg_worker.is_none() {
            return self.no_client_test_session_guard(id);
        }
        let answer = match id {
            // `05 Dialog Open Update Close` and `14 Dialog Open only` return without an error: the calls run as they do with a
            // client, where they show nothing either.
            ids::DIALOG_OPEN_1 | ids::DIALOG_UPDATE_1 | ids::DIALOG_CLOSE_1 => None,
            // `06 Hyperlink`: returns.
            ids::SYSTEM_HYPERLINK_1 => Some(Value::Null),
            // `07 Notification.Send`: returns Yes.
            ids::NOTIFICATION_SEND_1 => Some(Value::Boolean(true)),
            // `10 FilterPageBuilder.RunModal`: returns No.
            ids::FILTERPAGEBUILDER_RUNMODAL_1 => Some(Value::Boolean(false)),
            // `11 UploadIntoStream`, the form with five arguments: returns No.
            ids::FILE_UPLOADINTOSTREAM_1 | ids::FILE_UPLOADINTOSTREAM_2 if args.len() == 5 => {
                Some(Value::Boolean(false))
            }
            ids::FILE_UPLOADINTOSTREAM_1 | ids::FILE_UPLOADINTOSTREAM_2 => {
                return Err(self.no_client_refusal("UploadIntoStream with other than five arguments"));
            }
            // `12 SessionSettings.RequestSessionUpdate`: `RequestSessionUpdate(false)` returns.
            ids::SESSIONSETTINGS_REQUESTSESSIONUPDATE_1 if matches!(args, [Value::Boolean(false)]) => Some(Value::Null),
            ids::SESSIONSETTINGS_REQUESTSESSIONUPDATE_1 => {
                return Err(self.no_client_refusal("SessionSettings.RequestSessionUpdate(true)"));
            }
            _ => match refused_name(id) {
                Some(what) => return Err(self.no_client_refusal(what)),
                // Not a dialog, or one with a path of its own (StrMenu, Message, Confirm in `ui.rs`; pages and reports below).
                None => return Ok(None),
            },
        };
        Ok(answer)
    }

    /// Builtin `id` in the test session under `--no-client` (and in install code there, which keeps the install session's
    /// own answers: `None`). Hyperlink and Notification.Send go on to the test's handlers (`ui.rs`), CurrentClientType to its
    /// answer (`hostvalues.rs`). The other dialogs the worker answers (Dialog.Open, Update and Close, FilterPageBuilder.RunModal,
    /// UploadIntoStream, RequestSessionUpdate) are not measured in a web-service session, nor are the calls `refused_name`
    /// lists: refused.
    fn no_client_test_session_guard(&self, id: BuiltinId) -> Result<Option<Value>, Signal> {
        if self.install_without_client {
            return Ok(None);
        }
        let what = match id {
            ids::SYSTEM_HYPERLINK_1 | ids::NOTIFICATION_SEND_1 | ids::SESSION_CURRENTCLIENTTYPE_1 => return Ok(None),
            ids::DIALOG_OPEN_1 | ids::DIALOG_UPDATE_1 | ids::DIALOG_CLOSE_1 => "Dialog.Open, Update or Close",
            ids::FILTERPAGEBUILDER_RUNMODAL_1 => "FilterPageBuilder.RunModal",
            ids::FILE_UPLOADINTOSTREAM_1 | ids::FILE_UPLOADINTOSTREAM_2 => "UploadIntoStream",
            ids::SESSIONSETTINGS_REQUESTSESSIONUPDATE_1 => "SessionSettings.RequestSessionUpdate",
            _ => match refused_name(id) {
                Some(what) => what,
                None => return Ok(None),
            },
        };
        Err(self.no_client_refusal(what))
    }

    /// The refusal of `method` in the test session under `--no-client` when its receiver is a test client object
    /// (TestPage, TestPart, TestField, TestAction, TestFilter, TestRequestPage); `None` for a `Page` receiver (a Page
    /// variable, CurrPage), which goes on to the checks of `clientless_page_guard`.
    fn clientless_test_client_refusal(&self, method: PageOp) -> Option<Signal> {
        match method.recv() {
            PageRecv::Page => None,
            PageRecv::TestPage
            | PageRecv::TestPart
            | PageRecv::TestField
            | PageRecv::TestAction
            | PageRecv::TestFilter
            | PageRecv::TestRequestPage => Some(match method {
                PageOp::TestPageOpenEdit
                | PageOp::TestPageOpenNew
                | PageOp::TestPageOpenView
                | PageOp::TestPageTrap => Signal::unsupported(Self::clientless_test_page_reason(method.name())),
                _ => self.no_client_refusal(method.name()),
            }),
        }
    }

    /// `Page.Run` and `Page.RunModal` in a session without a client. In a page background task worker `Page.RunModal(Id)`,
    /// `Page.RunModal()` on a variable and `Page.Run(Id)` raise CallbackNotAllowed (BjNoClientDialogs.al `03 Page.RunModal by
    /// id`, `13 Page.RunModal on a variable`, `04 Page.Run by id`); the other shapes are refused there: `Run` on a variable, a
    /// call with a record, and a page with triggers, extensions or a Caption (whether its triggers run first and which name
    /// the text shows is not pinned). In the test session under `--no-client` every page run of an existing page is refused:
    /// none is measured in a web-service session (al-runner issue 8154). Install code there keeps its own answers.
    /// A TestPage open (OpenView, OpenEdit, OpenNew, and Trap, which catches the next page that opens) is refused in the
    /// test session under `--no-client` (`clientless_test_page_reason`). So is every other call on a TestPage, TestPart,
    /// TestField, TestAction, TestFilter or TestRequestPage there: no TestPage can be open in that session, and the answer
    /// on a page that is not open ("The TestPage is not open.") is pinned only with a client
    /// (testpage/lifecycle/TestPageTeardown.al:3-24). In a page background task worker without `--no-client` these calls
    /// go on as before: the measurement is of a web-service session only.
    pub(crate) fn clientless_page_guard(
        &self,
        method: PageOp,
        recv: Option<&Value>,
        args: &[Value],
    ) -> Result<(), Signal> {
        if self.is_clientless_session() {
            if let Some(refusal) = self.clientless_test_client_refusal(method) {
                return Err(refusal);
            }
        }
        // Install code under `--no-client` keeps the install session's pinned answers (`install_page_run`).
        if self.pages.bg_worker.is_none() && self.install_without_client {
            return Ok(());
        }
        let (object, modal) = match (method, recv, args) {
            // A page background task worker without `--no-client`: a TestPage call goes on as before.
            (
                PageOp::TestPageOpenEdit | PageOp::TestPageOpenNew | PageOp::TestPageOpenView | PageOp::TestPageTrap,
                ..,
            ) => return Ok(()),
            (PageOp::PageRun, ..) => return Err(self.no_client_refusal("Page.Run on a Page variable")),
            (PageOp::PageRunModal, Some(Value::TestPage(st)), [])
                if crate::pages::st_of(st).borrow().set_record.is_none() =>
            {
                (crate::pages::st_of(st).borrow().object as usize, true)
            }
            (PageOp::PageRunModal, ..) => {
                return Err(self.no_client_refusal("Page.RunModal on a Page variable with a record"));
            }
            (PageOp::PageRunStatic | PageOp::PageRunModalStatic, None, [Value::Integer(id)]) => {
                let heads = self.engine.program.objects.heads();
                let found = heads.iter().position(|o| {
                    o.is_kind(al_ir::ObjKind::Page) && o.id.is_some_and(|i| i64::from(i) == i64::from(*id))
                });
                match found {
                    Some(object) => (object, method == PageOp::PageRunModalStatic),
                    // No such page: the error of a session with a client, raised before anything opens.
                    None => return Ok(()),
                }
            }
            (PageOp::PageRunStatic, ..) => return Err(self.no_client_refusal("Page.Run with a record or a page name")),
            (PageOp::PageRunModalStatic, ..) => {
                return Err(self.no_client_refusal("Page.RunModal with a record or a page name"));
            }
            _ => return Ok(()),
        };
        let o = &self.engine.program.objects[object];
        let plain = o
            .page
            .as_ref()
            .is_some_and(|p| p.triggers.is_empty() && p.extensions.is_empty())
            && !o.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("Caption"));
        // The test session under `--no-client`: no page run is measured in a web-service session (al-runner issue 8154).
        if self.pages.bg_worker.is_none() {
            return Err(self.no_client_refusal(&format!("Page.Run or RunModal of page \"{}\"", o.name)));
        }
        let Some(id) = o.id.filter(|_| plain) else {
            return Err(self.no_client_refusal(&format!(
                "Page.Run or RunModal of page \"{}\", which has triggers, extensions or a Caption,",
                o.name
            )));
        };
        let how = if modal { " as modal" } else { "" };
        Err(self.callback_not_allowed(&format!("run page {id} {}{how}", o.name), false))
    }

    /// A page that code other than a probed `Page.Run` or `Page.RunModal` call would show (an action's RunObject, a
    /// lookup): refused in a session without a client.
    pub(crate) fn clientless_page_ui_guard(&self) -> Result<(), Signal> {
        match self.without_client() {
            true => Err(self.no_client_refusal("a page shown by an action or a lookup")),
            false => Ok(()),
        }
    }

    /// The static `Report.Run(Id)` and `Report.RunModal(Id)` whose request page would open (the RequestWindow argument
    /// `explicit`, else the report's UseRequestPage, as for a fresh instance in `report_builtin`), in a session without
    /// a client. `plain`: the call names the report and nothing else. A run that shows no page goes on: `explicit` as it is.
    ///
    /// In a page background task worker: CallbackNotAllowed, and no code of the report ran (BjNoClientDialogs.al `08
    /// Report.RunModal`, `09 Report.Run`: a report with a requestpage section and no triggers). Refused there: a report with
    /// an OnInitReport trigger or a report extension (whether they run first is not pinned), one without a requestpage
    /// section, and a run for which the test bound a RequestPageHandler or ReportHandler (whether the handler is reached is
    /// not probed). In the test session under `--no-client`: [`Session::no_client_test_session_report`].
    pub(crate) fn clientless_report(
        &self,
        schema: &Schema,
        explicit: Option<bool>,
        plain: bool,
    ) -> Result<RequestWindow, Signal> {
        let r = schema.report();
        if !self.without_client() || !explicit.unwrap_or(r.use_request_page != Some(false)) {
            return Ok(RequestWindow::As(explicit));
        }
        if self.pages.bg_worker.is_none() {
            // Install code under `--no-client` keeps what it does without the option.
            if self.install_without_client {
                return Ok(RequestWindow::As(explicit));
            }
            return self.no_client_test_session_report(schema, explicit, plain);
        }
        let object = schema.object as u32;
        if self.has_page_handler(UiKind::RequestPage, object) || self.has_page_handler(UiKind::Report, object) {
            return Err(self.no_client_refusal(&format!(
                "the request page of report \"{}\" with a RequestPageHandler or ReportHandler bound",
                schema.name()
            )));
        }
        let id = self.engine.program.objects[schema.object].id;
        let Some(id) = id.filter(|_| r.has_request_page && r.on_init.is_none() && r.extensions.is_empty()) else {
            return Err(self.no_client_refusal(&format!(
                "the request page of report \"{}\", which has an OnInitReport trigger, a report extension or no requestpage section,",
                schema.name()
            )));
        };
        Err(self.callback_not_allowed(&format!("run page {id} RequestOptionsPage as modal"), false))
    }

    /// The static `Report.Run(Id)` or `Report.RunModal(Id)` (`plain`, no RequestWindow) of the measured report, whose
    /// request page would open, in the test session under `--no-client` (al-runner issue 8154, the reporter's cells on BC
    /// 28.4 in a web-service session): the request page is skipped and the report runs, so a bound RequestPageHandler is
    /// never called (the test then fails with "The following UI handlers were not executed"); a bound ReportHandler for the
    /// report is called, as with a client (`report_handler_answers`). Marks `no-client-session`. The measured report
    /// ([`measured_skip_report`]) is processing-only with an empty requestpage section and no trigger anywhere; every other
    /// report or call shape is refused.
    fn no_client_test_session_report(
        &self,
        schema: &Schema,
        explicit: Option<bool>,
        plain: bool,
    ) -> Result<RequestWindow, Signal> {
        let o = &self.engine.program.objects[schema.object];
        if !(plain && explicit.is_none() && measured_skip_report(o)) {
            return Err(self.no_client_refusal(&format!(
                "the request page of report \"{}\" (only a static Report.Run or RunModal of a processing-only report with an empty requestpage section and no trigger is measured)",
                schema.name()
            )));
        }
        assume(Assumption::NoClientSession);
        if self.has_page_handler(UiKind::Report, schema.object as u32) {
            return Ok(RequestWindow::As(explicit));
        }
        Ok(RequestWindow::Skipped)
    }

    /// A report run on a variable (`Run`, `RunModal`) whose request page would open (`shows`), in a session without a
    /// client: not probed, refused. Else `shows` as it is. The static forms do not come here (`clientless_report`).
    pub(crate) fn clientless_request_page_guard(&self, shows: bool, schema: &Schema) -> Result<bool, Signal> {
        if self.without_client() && shows {
            let name = schema.name();
            return Err(self.no_client_refusal(&format!("the request page of report \"{name}\", run on a variable,")));
        }
        Ok(shows)
    }

    /// Refuses the request window of an xmlport run in a session without a client (not probed; no handler can take that
    /// page, xmlports/shapes-properties/Y40XmlPortRequestPage.al:383-423).
    pub(crate) fn clientless_xmlport_window_guard(&self, name: &str) -> Result<(), Signal> {
        if self.without_client() {
            return Err(self.no_client_refusal(&format!("the request page of xmlport \"{name}\"")));
        }
        Ok(())
    }
}

/// What a static report run does with its request page in a session without a client (`clientless_report`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestWindow {
    /// As in a session with a client: the call's RequestWindow argument, `None` without one.
    As(Option<bool>),
    /// The test session under `--no-client` skips the request page; what `CurrReport.UseRequestPage()` answers then is not
    /// measured.
    Skipped,
}

/// The report of the reporter's cells (al-runner issue 8154; its shape as report 51440 in al-runner issue 8028): `ProcessingOnly = true`, one data item
/// without triggers, an empty `requestpage` section. Required by equality: processing-only with no output, a requestpage
/// section with no trigger, no page extension and no control but the data items' own, no report trigger, no data item
/// trigger or RequestFilterFields, no report extension.
fn measured_skip_report(o: &al_ir::Object) -> bool {
    let (Some(r), Some(page)) = (o.report.as_ref(), o.page.as_ref()) else {
        return false;
    };
    r.processing_only
        && r.output.is_empty()
        && r.has_request_page
        && r.on_init.is_none()
        && r.on_pre.is_none()
        && r.on_post.is_none()
        && r.on_pre_rendering.is_none()
        && r.extensions.is_empty()
        && !r.request_page_ok_only
        && r.items.iter().all(|i| {
            i.on_pre.is_none() && i.on_after_get.is_none() && i.on_post.is_none() && i.request_filter_fields.is_none()
        })
        && page.triggers.is_empty()
        && page.extensions.is_empty()
        && page.controls.iter().all(|c| c.kind == al_ir::ControlKind::DataItem)
}
