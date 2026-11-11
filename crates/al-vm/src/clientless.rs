//! What a session without a client does with a dialog: a page background task worker, and the test session itself under
//! `--no-client` (al-runner issue 7564, `Session::set_clientless_session`). Every answer here is the one a page background
//! task worker gives, the session without a client a test can reach: GuiAllowed, Message and Confirm from
//! pages/background-tasks/BjNoGuiDialogs.al:8-10 (`ui.rs`, `hostvalues.rs`), the other dialogs from
//! pages/background-tasks/BjNoClientDialogs.al:168-199 RunPageBackgroundTask_Dialogs and :214-260
//! RunPageBackgroundTask_Dialogs_WithHandlers. In the worker the answers are pinned. That a web-service session answers as
//! that worker does is the assumption `no-client-session`: under `--no-client` each answer marks it
//! ([`Session::note_no_client_answer`]). A dialog or a call shape the probe does not cover is refused in both (hard rules 3
//! and 4).
use crate::Session;
use crate::reports::Schema;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_builtins::page_op::PageOp;
use al_builtins::value::Value;
use al_ir::BuiltinId;

/// The builtins that ask a client for something and that no probe covers in a session without a client, and
/// CurrentClientType: the name for the refusal text.
fn refused_name(id: BuiltinId) -> Option<&'static str> {
    Some(match id {
        ids::NOTIFICATION_RECALL_1 => "Notification.Recall",
        // Not a dialog: which client type a session without a client answers depends on the kind of session (OData, SOAP,
        // Api, Background) and the option names none, so the test session's own answer (Web) would be a guess.
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

    /// A dialog answered as a session without a client answers it: pinned in a page background task worker
    /// (BjNoClientDialogs.al), assumed for the test session under `--no-client` (marks `no-client-session`).
    pub(crate) fn note_no_client_answer(&self) {
        if self.pages.bg_worker.is_none() {
            assume(Assumption::NoClientSession);
        }
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
        self.note_no_client_answer();
        Ok(answer)
    }

    /// `Page.Run` and `Page.RunModal` in a session without a client: `Page.RunModal(Id)`, `Page.RunModal()` on a variable
    /// and `Page.Run(Id)` raise CallbackNotAllowed (BjNoClientDialogs.al `03 Page.RunModal by id`, `13 Page.RunModal on a
    /// variable`, `04 Page.Run by id`). The other shapes are refused: `Run` on a variable, a call with a record, and a page
    /// with triggers, extensions or a Caption (whether its triggers run first and which name the text shows is not pinned).
    /// A TestPage that opens goes on as in a session with a client: whether such a session can open one is not pinned, so
    /// the open marks `no-client-session`.
    pub(crate) fn clientless_page_guard(
        &self,
        method: PageOp,
        recv: Option<&Value>,
        args: &[Value],
    ) -> Result<(), Signal> {
        let (object, modal) = match (method, recv, args) {
            (PageOp::TestPageOpenEdit | PageOp::TestPageOpenNew | PageOp::TestPageOpenView, ..) => {
                self.note_no_client_answer();
                return Ok(());
            }
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
        let Some(id) = o.id.filter(|_| plain) else {
            return Err(self.no_client_refusal(&format!(
                "Page.Run or RunModal of page \"{}\", which has triggers, extensions or a Caption,",
                o.name
            )));
        };
        self.note_no_client_answer();
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
    /// a client: CallbackNotAllowed, and no code of the report ran (BjNoClientDialogs.al `08 Report.RunModal`,
    /// `09 Report.Run`: a report with a requestpage section and no triggers). Refused: a report with an OnInitReport
    /// trigger or a report extension (whether they run first is not pinned), one without a requestpage section, and a run
    /// for which the test bound a RequestPageHandler or ReportHandler (whether the handler is reached is not probed). A run
    /// that shows no page goes on: `explicit` as it is.
    pub(crate) fn clientless_report(&self, schema: &Schema, explicit: Option<bool>) -> Result<Option<bool>, Signal> {
        let (r, object) = (schema.report(), schema.object as u32);
        if !self.without_client() || !explicit.unwrap_or(r.use_request_page != Some(false)) {
            return Ok(explicit);
        }
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
        self.note_no_client_answer();
        Err(self.callback_not_allowed(&format!("run page {id} RequestOptionsPage as modal"), false))
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
