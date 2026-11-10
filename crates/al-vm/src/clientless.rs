//! `--no-client` (al-runner issue 7564, `Session::set_clientless_session`): what the session refuses while it runs as a
//! session without a client. GuiAllowed, Message and Confirm are answered as a page background task worker answers them
//! (pages/background-tasks/BjNoGuiDialogs.al:8-10; `ui.rs`, `hostvalues.rs`); the assumption `no-client-session` marks them.
//! No corpus test pins any other dialog in such a session, so each is refused here (hard rules 3 and 4).
use crate::Session;
use crate::ui::UiKind;
use al_builtins::builtin_id::ids;
use al_builtins::native::Signal;
use al_ir::BuiltinId;

/// The builtins that ask a client for something, other than Message and Confirm, and CurrentClientType: the name for the
/// refusal text.
fn dialog_name(id: BuiltinId) -> Option<&'static str> {
    Some(match id {
        ids::DIALOG_STRMENU_1 => "StrMenu",
        ids::DIALOG_OPEN_1 | ids::DIALOG_UPDATE_1 | ids::DIALOG_CLOSE_1 => "Dialog.Open, Update and Close",
        ids::SYSTEM_HYPERLINK_1 => "Hyperlink",
        ids::NOTIFICATION_SEND_1 | ids::NOTIFICATION_RECALL_1 => "Notification.Send and Recall",
        ids::FILTERPAGEBUILDER_RUNMODAL_1 => "FilterPageBuilder.RunModal",
        ids::SESSIONSETTINGS_REQUESTSESSIONUPDATE_1 => "SessionSettings.RequestSessionUpdate",
        // Not a dialog: which client type a session without a client answers depends on the kind of session (OData, SOAP,
        // Api, Background) and the option names none, so the test session's own answer (Web) would be a guess.
        ids::SESSION_CURRENTCLIENTTYPE_1 => "CurrentClientType",
        ids::FILE_DOWNLOAD_1
        | ids::FILE_DOWNLOADFROMSTREAM_1
        | ids::FILE_UPLOAD_1
        | ids::FILE_UPLOADINTOSTREAM_1
        | ids::FILE_UPLOADINTOSTREAM_2
        | ids::FILE_VIEW_1
        | ids::FILE_VIEWFROMSTREAM_1 => "a file upload, download or view",
        ids::PAGE_RUN_1 | ids::PAGE_RUN_2 | ids::PAGE_RUN_3 | ids::PAGE_RUN_4 | ids::PAGE_RUN_5 => "Page.Run",
        ids::PAGE_RUNMODAL_1
        | ids::PAGE_RUNMODAL_2
        | ids::PAGE_RUNMODAL_3
        | ids::PAGE_RUNMODAL_4
        | ids::PAGE_RUNMODAL_5
        | ids::PAGE_RUNMODAL_6
        | ids::PAGE_RUNMODAL_7 => "Page.RunModal",
        ids::REPORT_RUNREQUESTPAGE_1 | ids::REPORT_RUNREQUESTPAGE_2 => "Report.RunRequestPage",
        _ => return None,
    })
}

pub(crate) fn refusal(what: &str) -> Signal {
    Signal::Unsupported(format!(
        "{what} in a session without a client (--no-client): what BC does there is not pinned (probe wanted, docs/probes-needed.md `no-client-session`)"
    ))
}

impl Session {
    /// Refuses builtin `id` when the session has no client and the builtin asks one for something.
    #[inline]
    pub(crate) fn clientless_guard(&self, id: BuiltinId) -> Result<(), Signal> {
        match dialog_name(id) {
            Some(what) => Err(refusal(what)),
            None => Ok(()),
        }
    }

    /// A report run (`Run`, `RunModal`) whose request page would open, in a session without a client. With no
    /// `[RequestPageHandler]` or `[ReportHandler]` of the test bound for the report there is no client to show the page:
    /// refused. With one bound the run goes on and the handler answers as in a session with a client; that the handler is
    /// reached in a session without a client is not pinned (a ConfirmHandler is not, BjNoGuiDialogs.al:9-10), so the run
    /// marks `no-client-session`. A run that shows no page does not come here.
    pub(crate) fn clientless_request_page_guard(&self, object: u32, name: &str) -> Result<(), Signal> {
        if !self.clientless_session {
            return Ok(());
        }
        if self.has_page_handler(UiKind::RequestPage, object) || self.has_page_handler(UiKind::Report, object) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NoClientSession);
            return Ok(());
        }
        Err(refusal(&format!("the request page of report \"{name}\"")))
    }

    /// Refuses the request window of an xmlport run in a session without a client (no handler can take that page,
    /// xmlports/shapes-properties/Y40XmlPortRequestPage.al:383-423).
    pub(crate) fn clientless_xmlport_window_guard(&self, name: &str) -> Result<(), Signal> {
        if self.clientless_session {
            return Err(refusal(&format!("the request page of xmlport \"{name}\"")));
        }
        Ok(())
    }

    /// Refuses a `Page.Run` or `Page.RunModal` on a Page variable when the session has no client.
    pub(crate) fn clientless_page_guard(&self, method: al_builtins::page_op::PageOp) -> Result<(), Signal> {
        use al_builtins::page_op::PageOp;
        match method {
            PageOp::PageRun | PageOp::PageRunStatic => Err(refusal("Page.Run")),
            PageOp::PageRunModal | PageOp::PageRunModalStatic => Err(refusal("Page.RunModal")),
            _ => Ok(()),
        }
    }
}
