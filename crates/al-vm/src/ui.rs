//! Dialogs raised by the code under test (`Message`, `Confirm`, `StrMenu`)
//! and the test handler functions that answer them.
//!
//! BC's test runner binds the handler functions named in a test's
//! `[HandlerFunctions]` for the duration of that test method. A UI call goes to
//! the bound handler of its kind (corpus handlers/TestMessageHandler.al); with
//! none bound it raises "Unhandled UI" (handlers/TestConfirmNoHandler.al); a
//! declared handler that never fired fails the test
//! (handlers/TestPageQueryCloseMessageHandler_Tests.al:51).

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_builtins::value::{Instance, RtType, Value};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiKind {
    Message,
    Confirm,
    StrMenu,
    /// `[PageHandler]`: answers `Page.Run` (ADR-012). Bound by the page of its TestPage parameter.
    Page,
    /// `[ModalPageHandler]`: answers `Page.RunModal`.
    ModalPage,
    /// `[RequestPageHandler]`: answers the request page of a report run. Bound by the report of
    /// its `TestRequestPage` parameter.
    RequestPage,
    /// `[SessionSettingsHandler]`: answers `SessionSettings.RequestSessionUpdate` (corpus
    /// session/TestSessionSettings.al:464-545).
    SessionSettings,
    /// `[SendNotificationHandler]`: answers `Notification.Send` (corpus
    /// handlers/TestNotificationHandler.al:15-26,74-85).
    SendNotification,
    /// `[RecallNotificationHandler]`: runs on `Notification.Recall` of a sent notification (corpus
    /// handlers/TestNotificationSendRecallContracts.al:95-109).
    RecallNotification,
    /// `[ReportHandler]`: answers the run of a report that would show its request page, instead of the page. Bound by the
    /// report of its `var Report` parameter (report/lt-ae-reporthandler/AeReportHandler.al).
    Report,
    /// `[HttpClientHandler]`: answers an `HttpClient` call (ADR-027 section 5; the probe
    /// session/PZHttpHandler.al on the corpus staging branch).
    HttpClient,
}

impl UiKind {
    /// The attribute that marks a handler function of this kind.
    pub fn from_attribute(name: &str) -> Option<UiKind> {
        match name.to_ascii_lowercase().as_str() {
            "messagehandler" => Some(UiKind::Message),
            "confirmhandler" => Some(UiKind::Confirm),
            "strmenuhandler" => Some(UiKind::StrMenu),
            "pagehandler" => Some(UiKind::Page),
            "modalpagehandler" => Some(UiKind::ModalPage),
            "requestpagehandler" => Some(UiKind::RequestPage),
            "sessionsettingshandler" => Some(UiKind::SessionSettings),
            "sendnotificationhandler" => Some(UiKind::SendNotification),
            "recallnotificationhandler" => Some(UiKind::RecallNotification),
            "httpclienthandler" => Some(UiKind::HttpClient),
            "reporthandler" => Some(UiKind::Report),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            UiKind::Message => "Message",
            UiKind::Confirm => "Confirm",
            UiKind::StrMenu => "StrMenu",
            UiKind::Page => "Page",
            UiKind::ModalPage => "ModalPage",
            UiKind::RequestPage => "RequestPage",
            UiKind::SessionSettings => "SessionSettings",
            UiKind::SendNotification => "SendNotification",
            UiKind::RecallNotification => "RecallNotification",
            UiKind::HttpClient => "HttpClient",
            UiKind::Report => "Report",
        }
    }
}

/// `ModalPageHandler(var P: Page "X"; var Response: Action)` (handlers/PSModalPageHandlerTyped.al).
fn typed_modal(kind: UiKind, params: usize, by_var: &[bool], p: Option<&RtType>, response: Option<&RtType>) -> bool {
    kind == UiKind::ModalPage
        && params == 2
        && by_var == [true, true]
        && matches!(p, Some(RtType::Page(_)))
        && matches!(response, Some(RtType::Option(_)) | Some(RtType::Enum(_)))
}

/// A page handler found for a page: (object, routine, instance).
pub(crate) type BoundHandler = (usize, usize, Rc<RefCell<Instance>>);

/// A handler function bound to the running test method.
pub struct UiHandler {
    kind: UiKind,
    /// For page handlers: the page object of the `TestPage` parameter.
    pub page: Option<u32>,
    name: String,
    object: usize,
    routine: usize,
    instance: Rc<RefCell<Instance>>,
    fired: bool,
    /// A `[ModalPageHandler]` whose first parameter is a `Page` and whose second is `var Response: Action`.
    typed: bool,
}

/// The handlers bound while a test method runs.
#[derive(Default)]
pub(crate) struct UiScope {
    handlers: Vec<UiHandler>,
    /// A handler is running: a UI call from inside it is not pinned by the corpus.
    in_handler: bool,
}

impl Session {
    /// Checks that routine `routine` of `object` has the signature BC requires
    /// of a handler of `kind`, and binds it to `instance` (the test codeunit's).
    pub fn ui_handler(
        &self,
        kind: UiKind,
        object: usize,
        routine: usize,
        instance: Rc<RefCell<Instance>>,
    ) -> Result<UiHandler, String> {
        let r = &self.engine.program.objects[object].routines[routine];
        let ty = |i: usize| r.locals.get(i).map(|s| &s.ty);
        let ok = match kind {
            UiKind::Message => r.params == 1 && r.by_var == [false],
            UiKind::Confirm => r.params == 2 && r.by_var == [false, true] && matches!(ty(1), Some(RtType::Boolean)),
            UiKind::StrMenu => {
                r.params == 3 && r.by_var == [false, true, false] && matches!(ty(1), Some(RtType::Integer))
            }
            UiKind::Page | UiKind::ModalPage | UiKind::RequestPage => {
                (r.params == 1 && r.by_var == [true] && matches!(ty(0), Some(RtType::TestPage(_))))
                    || (typed_modal(kind, r.params, &r.by_var, ty(0), ty(1)))
            }
            // ReportHandler(var R: Report "X") (report/lt-ae-reporthandler/AeReportHandler.al:43; must-not-compile 0985 rejects
            // any other shape).
            UiKind::Report => r.params == 1 && r.by_var == [true] && self.is_report_type(ty(0)),
            // SessionSettingsHandler(var SessionSettings: SessionSettings): Boolean
            // (session/TestSessionSettings.al:546-552, error AL0244 names the shape).
            UiKind::SessionSettings => {
                r.params == 1
                    && r.by_var == [true]
                    && matches!(
                        ty(0),
                        Some(RtType::Library(al_builtins::library_kind::LibraryKind::SessionSettings))
                    )
            }
            // SendNotificationHandler(var Notification: Notification): Boolean
            // (handlers/TestNotificationHandler.al:74-85).
            UiKind::SendNotification | UiKind::RecallNotification => {
                r.params == 1
                    && r.by_var == [true]
                    && matches!(
                        ty(0),
                        Some(RtType::Library(al_builtins::library_kind::LibraryKind::Notification))
                    )
            }
            // HttpClientHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
            // (session/PZHttpHandler.al).
            UiKind::HttpClient => {
                r.params == 2
                    && r.by_var == [false, true]
                    && matches!(
                        ty(0),
                        Some(RtType::Library(
                            al_builtins::library_kind::LibraryKind::TestHttpRequestMessage
                        ))
                    )
                    && matches!(
                        ty(1),
                        Some(RtType::Library(
                            al_builtins::library_kind::LibraryKind::TestHttpResponseMessage
                        ))
                    )
            }
        };
        let page = match ty(0) {
            Some(RtType::TestPage(p)) if matches!(kind, UiKind::Page | UiKind::ModalPage | UiKind::RequestPage) => {
                Some(*p)
            }
            Some(RtType::Page(p)) if typed_modal(kind, r.params, &r.by_var, ty(0), ty(1)) => Some(*p),
            Some(RtType::Codeunit(p)) if kind == UiKind::Report => Some(*p),
            _ => None,
        };
        // A ModalPageHandler whose parameter is a `Page` and a `var Response: Action` is pinned by
        // handlers/PSModalPageHandlerTyped.al. Any other page handler of a `Page` is a shape BC accepts that no
        // corpus test has, so what it is handed is not pinned (docs/probes-needed.md, "Page handlers").
        if !ok && matches!(kind, UiKind::Page | UiKind::ModalPage) && matches!(ty(0), Some(RtType::Page(_))) {
            return Err(format!(
                "{} takes a Page instead of a TestPage: what a {} handler of that shape is handed and how its Response: Action is read are not pinned (docs/probes-needed.md)",
                r.name,
                kind.label()
            ));
        }
        if !ok {
            return Err(format!(
                "{} is not shaped like a {} handler function",
                r.name,
                kind.label()
            ));
        }
        // `[SendNotificationHandler(true)]` and `[RecallNotificationHandler(true)]` are optional: one that never runs does not fail
        // the test (al-language/pages/lt-ca-odd-667/CaNotificationTests.al:14-38, handlers at :61 and :70).
        let optional = matches!(kind, UiKind::SendNotification | UiKind::RecallNotification)
            && r.attributes.iter().any(|a| {
                UiKind::from_attribute(&a.name) == Some(kind) && a.args.as_slice() == [al_ir::Const::Boolean(true)]
            });
        Ok(UiHandler {
            kind,
            page,
            name: r.name.clone(),
            object,
            routine,
            instance,
            fired: optional,
            typed: typed_modal(kind, r.params, &r.by_var, ty(0), ty(1)),
        })
    }

    /// Whether `ty` is a `Report` variable's type (an instance of a report object).
    fn is_report_type(&self, ty: Option<&RtType>) -> bool {
        matches!(ty, Some(RtType::Codeunit(o)) if self.engine.program.objects.get(*o as usize).is_some_and(|o| o.report.is_some()))
    }

    /// Starts a test method: from here until `end_ui_scope`, UI calls go to
    /// `handlers`, and any other UI call is "Unhandled UI".
    pub fn begin_ui_scope(&mut self, handlers: Vec<UiHandler>) {
        self.ui_state.scope = Some(UiScope {
            handlers,
            in_handler: false,
        });
    }

    /// Ends the test method; returns the names of the declared handlers that
    /// never fired.
    pub fn end_ui_scope(&mut self) -> Vec<String> {
        self.ui_state
            .scope
            .take()
            .map(|s| s.handlers.into_iter().filter(|h| !h.fired).map(|h| h.name).collect())
            .unwrap_or_default()
    }

    /// Runs the handler of `kind` with `args`; returns the handler's final
    /// parameter values.
    fn dispatch_ui(&mut self, kind: UiKind, shown: &str, args: Vec<Value>) -> Result<Vec<Value>, Signal> {
        self.dispatch_ui_returning(kind, shown, args).map(|(_, finals)| finals)
    }

    /// Like `dispatch_ui`, with the handler's return value too.
    fn dispatch_ui_returning(
        &mut self,
        kind: UiKind,
        shown: &str,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let Some(scope) = self.ui_state.scope.as_mut() else {
            return Err(Signal::Unsupported(format!(
                "{} outside a test method (what the runner does then is not pinned)",
                kind.label()
            )));
        };
        if scope.in_handler {
            return Err(Signal::Unsupported(format!(
                "{} raised from inside a handler function (not pinned)",
                kind.label()
            )));
        }
        let Some(h) = scope.handlers.iter_mut().find(|h| h.kind == kind) else {
            // Text and code: corpus handlers/TestUnhandledUiAndDialogDefaults.al:68-107 (ONE space
            // after the kind; Confirm quotes the question, StrMenu the options only).
            return Err(Signal::Error(Box::new(AlError {
                message: format!("Unhandled UI: {} {shown}", kind.label()),
                code: "MissingUIHandler".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        };
        h.fired = true;
        let (object, routine, instance) = (h.object, h.routine, h.instance.clone());
        scope.in_handler = true;
        // A value too long for the handler's parameter fails before the handler runs, with the error of the conversion and no
        // prefix (handlers/... misc/lt-system-functions/ErrorMessage.al:Message_Long `af235`).
        let bound: Result<Vec<Value>, Signal> = {
            let r = &self.engine.program.objects[object].routines[routine];
            args.into_iter()
                .enumerate()
                .map(|(i, a)| match (&a, r.locals.get(i)) {
                    (Value::Text(_) | Value::Code(_), Some(slot)) if i < r.params => {
                        al_builtins::ops::convert(a.clone(), &slot.ty)
                    }
                    _ => Ok(a),
                })
                .collect()
        };
        let args = match bound {
            Ok(a) => a,
            Err(e) => {
                if let Some(scope) = self.ui_state.scope.as_mut() {
                    scope.in_handler = false;
                }
                return Err(e);
            }
        };
        let result = self.call(object, routine, instance, args);
        if let Some(scope) = self.ui_state.scope.as_mut() {
            scope.in_handler = false;
        }
        // An error raised inside a Message or Confirm handler comes out of the dialog call with the test framework's prefix
        // on its code (handlers/TestUnhandledUiAndDialogDefaults.al:147-156: text as raised, code TestWrapped:Dialog). A StrMenu
        // handler's error keeps its own code (misc/lt-probes-needed-3/PnC09UnhandledUi.al:Handler_Error_StrMenu `Dialog`).
        match result {
            Err(Signal::Error(mut e))
                if matches!(kind, UiKind::Message | UiKind::Confirm) && !e.code.starts_with("TestWrapped") =>
            {
                e.code = format!("TestWrapped:{}", e.code);
                Err(Signal::Error(e))
            }
            other => other,
        }
    }

    /// Whether an `[HttpClientHandler]` is bound to the running test.
    pub(crate) fn has_http_handler(&self) -> bool {
        self.has_ui_handler(UiKind::HttpClient)
    }

    /// Whether a handler of `kind` is bound to the running test.
    pub(crate) fn has_ui_handler(&self, kind: UiKind) -> bool {
        self.ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == kind))
    }

    /// Counts the bound `[HttpClientHandler]`s as run, so the end of the test does not report them unexecuted: before BC 29 a
    /// request under BlockOutboundRequests raises without calling the handler and the test still passes
    /// (al-language-onprem/session/PZHttpBlock.al:46-59, Handler_UnderBlockOutboundRequests_StillAnswers on BC 27 and 28).
    pub(crate) fn http_handlers_count_as_run(&mut self) {
        if let Some(scope) = self.ui_state.scope.as_mut() {
            for h in scope.handlers.iter_mut().filter(|h| h.kind == UiKind::HttpClient) {
                h.fired = true;
            }
        }
    }

    /// Runs the bound `[HttpClientHandler]` with the request and a fresh response; `None` when the test has none.
    /// Returns the handler's answer and the response as it left it.
    pub(crate) fn dispatch_http_handler(
        &mut self,
        request: Value,
        response: Value,
    ) -> Result<Option<(Value, Value)>, Signal> {
        let bound = self
            .ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == UiKind::HttpClient));
        if !bound {
            return Ok(None);
        }
        let (answer, mut finals) = self.dispatch_ui_returning(UiKind::HttpClient, "", vec![request, response])?;
        let response = finals.pop().unwrap_or(Value::Null);
        Ok(Some((answer, response)))
    }

    /// `SessionSettings.RequestSessionUpdate(false)`: the bound `[SessionSettingsHandler]` gets the
    /// settings and must answer true. Without a handler the platform raises "Unhandled UI:
    /// SessionSettings" (the header of session/TestSessionSettings.al); the full text is not
    /// pinned, so that is refused. What a handler's `false` means is not pinned either.
    pub(crate) fn ui_session_settings(&mut self, settings: Value) -> Result<Option<Value>, Signal> {
        let bound = self
            .ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == UiKind::SessionSettings));
        if !bound {
            return Err(Signal::Unsupported(
                "SessionSettings.RequestSessionUpdate without a [SessionSettingsHandler] (the Unhandled UI text is not pinned)"
                    .into(),
            ));
        }
        let (answer, mut finals) = self.dispatch_ui_returning(UiKind::SessionSettings, "", vec![settings])?;
        match answer {
            // A handler that answers false is called once and nothing is raised (session/
            // TestProbeSessionMiscContracts.al:SessionSettings_RequestUpdateFalse_Handler).
            Value::Boolean(_) => Ok(finals.pop()),
            other => Err(Signal::Unsupported(format!(
                "a [SessionSettingsHandler] that answers {other}",
                other = other.describe()
            ))),
        }
    }

    /// `Notification.Send()`: the bound `[SendNotificationHandler]` gets a copy; with none bound
    /// nothing is shown and nothing is raised. `Send` answers true in every case, also when the
    /// handler answers false, and the sender sees what the handler changed in its `var Notification`
    /// (handlers/TestNotificationSendRecallContracts.al:59-93, BC 27.5). Returns the notification
    /// as the handler left it.
    pub(crate) fn ui_send_notification(&mut self, notification: Value, id: [u8; 16]) -> Result<Option<Value>, Signal> {
        if !self.ui_state.sent_notifications.contains(&id) {
            self.ui_state.sent_notifications.push(id);
        }
        let bound = self
            .ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == UiKind::SendNotification));
        if !bound {
            return Ok(None);
        }
        let (_, mut finals) = self.dispatch_ui_returning(UiKind::SendNotification, "", vec![notification])?;
        Ok(finals.pop())
    }

    /// `Notification.Recall()`: true for a notification that was sent, false for one never sent (the recall
    /// handler does not run then). A bound `[RecallNotificationHandler]` runs with the sender's current
    /// notification, at every Recall, and what it answers does not matter; with none bound nothing happens
    /// (handlers/TestNotificationSendRecallContracts.al:95-131, handlers/ProbeNotificationRecallEdges.al,
    /// BC 27.5).
    pub(crate) fn ui_recall_notification(&mut self, notification: Value, id: [u8; 16]) -> Result<bool, Signal> {
        if !self.ui_state.sent_notifications.contains(&id) {
            return Ok(false);
        }
        let bound = self
            .ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == UiKind::RecallNotification));
        if bound {
            self.dispatch_ui_returning(UiKind::RecallNotification, "", vec![notification])?;
        }
        Ok(true)
    }

    /// The declared id of the object at index `object`, as BC prints it (0 when it has none).
    pub(crate) fn object_id_of(&self, object: u32) -> u32 {
        self.engine
            .program
            .objects
            .get(object as usize)
            .and_then(|o| o.id)
            .unwrap_or(0)
    }

    /// The bound handler of `kind` for `page`, marked as used: `Unhandled UI` when there is none
    /// (corpus handlers/TestPageModalHandler_Tests.al:105, TestPageRunHandler_Tests.al:
    /// NonModalPageRunWithoutAHandlerIsRefused). BC looks the handler up before the page opens, so
    /// a refused page never ran its OnOpenPage (handlers/TestPageActionRunObjectNoHandler_Tests.al) and a
    /// handler whose page then fails to open still counts as executed
    /// (autoformat/TestAutoFormatExpressionTiming.al, FailingFormat_RunModal_RaisesBeforeAnyHandler).
    pub(crate) fn bind_page_handler(
        &mut self,
        kind: UiKind,
        page: u32,
    ) -> Result<(usize, usize, Rc<RefCell<Instance>>), Signal> {
        let shown_id = self.object_id_of(page);
        let Some(scope) = self.ui_state.scope.as_mut() else {
            return Err(Signal::Unsupported(format!(
                "{} outside a test method (what the runner does then is not pinned)",
                kind.label()
            )));
        };
        let Some(h) = scope
            .handlers
            .iter_mut()
            .find(|h| h.kind == kind && h.page == Some(page))
        else {
            // handlers/TestUnhandledUiAndDialogDefaults.al:97-104 ("ModalPage 68124", "Page 68124");
            // RequestPage names the report's request page id (handlers/TestReportRuntimeContracts.al:425).
            return Err(Signal::Error(Box::new(AlError {
                message: format!("Unhandled UI: {} {}", kind.label(), shown_id),
                code: "MissingUIHandler".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        };
        h.fired = true;
        Ok((h.object, h.routine, h.instance.clone()))
    }

    /// The bound `Page`-typed `[ModalPageHandler]` of `page`, marked as used, when the test bound one.
    pub(crate) fn take_typed_modal_handler(&mut self, page: u32) -> Option<BoundHandler> {
        let h = self
            .ui_state
            .scope
            .as_mut()?
            .handlers
            .iter_mut()
            .find(|h| h.kind == UiKind::ModalPage && h.typed && h.page == Some(page))?;
        h.fired = true;
        Some((h.object, h.routine, h.instance.clone()))
    }

    /// Whether the test bound a handler of `kind` for `page`.
    /// The refusal for a page of `kind` that the test bound no handler for. Code and text:
    /// handlers/TestUnhandledUiAndDialogDefaults.al:97-104, handlers/TestReportRuntimeContracts.al:425-426.
    pub(crate) fn unhandled_page(&self, kind: UiKind, object: u32) -> Signal {
        Signal::Error(Box::new(AlError {
            message: format!("Unhandled UI: {} {}", kind.label(), self.object_id_of(object)),
            code: "MissingUIHandler".into(),
            call_stack: String::new(),
            ..Default::default()
        }))
    }

    pub(crate) fn has_page_handler(&self, kind: UiKind, page: u32) -> bool {
        self.ui_state
            .scope
            .as_ref()
            .is_some_and(|s| s.handlers.iter().any(|h| h.kind == kind && h.page == Some(page)))
    }

    /// The bound handler of `kind` for `page`, marked as used, or `None` when the test bound
    /// none. A `RunObject` action looks the handler up after the page opened and treats a missing
    /// one differently from `Page.Run` (corpus handlers/TestPageActionRunObjectNoHandler_Tests.al).
    pub(crate) fn try_bind_page_handler(&mut self, kind: UiKind, page: u32) -> Result<Option<BoundHandler>, Signal> {
        let Some(scope) = self.ui_state.scope.as_mut() else {
            return Err(Signal::Unsupported(format!(
                "{} outside a test method (what the runner does then is not pinned)",
                kind.label()
            )));
        };
        Ok(scope
            .handlers
            .iter_mut()
            .find(|h| h.kind == kind && h.page == Some(page))
            .map(|h| {
                h.fired = true;
                (h.object, h.routine, h.instance.clone())
            }))
    }

    /// Keeps the text of every `Message` raised outside a test method, for a scratch run
    /// ([`Session::take_messages`]). Without this such a call is refused.
    pub fn capture_messages(&mut self) {
        self.scratch_messages.get_or_insert_with(Vec::new);
    }

    /// The texts kept since [`Session::capture_messages`] (or the last call), in order.
    pub fn take_messages(&mut self) -> Vec<String> {
        self.scratch_messages.as_mut().map(std::mem::take).unwrap_or_default()
    }

    pub(crate) fn ui_message(&mut self, text: String) -> Result<(), Signal> {
        if self.ui_state.scope.is_none() && self.scratch_messages.is_some() {
            self.cap_message(&text);
            if let Some(log) = self.scratch_messages.as_mut() {
                log.push(text);
            }
            return Ok(());
        }
        self.dispatch_ui(UiKind::Message, &text.clone(), vec![Value::Text(text.into())])
            .map(|_| ())
    }

    pub(crate) fn ui_confirm(&mut self, question: String, default: bool) -> Result<bool, Signal> {
        let finals = self.dispatch_ui(
            UiKind::Confirm,
            &question.clone(),
            vec![Value::Text(question.into()), Value::Boolean(default)],
        )?;
        match finals.get(1) {
            Some(Value::Boolean(b)) => Ok(*b),
            other => Err(Signal::Unsupported(format!(
                "Confirm handler returned {}",
                other.describe()
            ))),
        }
    }

    pub(crate) fn ui_str_menu(&mut self, options: String, default: i32, instruction: String) -> Result<i32, Signal> {
        let finals = self.dispatch_ui(
            UiKind::StrMenu,
            &options.clone(),
            vec![
                Value::Text(options.into()),
                Value::Integer(default),
                Value::Text(instruction.into()),
            ],
        )?;
        match finals.get(1) {
            Some(Value::Integer(n)) => Ok(*n),
            other => Err(Signal::Unsupported(format!(
                "StrMenu handler returned {}",
                other.describe()
            ))),
        }
    }
}
