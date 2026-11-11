//! The platform page events of a page without a source table (al-runner issue 7430; corpus
//! events/page-events/NoSourcePageEvents.al with its fixtures NoSourcePageFixtures.al). Such a page raises only OnOpenPageEvent,
//! OnAfterGetCurrRecordEvent, OnQueryClosePageEvent and OnClosePageEvent, each after the page's own trigger of that name; the
//! record events never fire (there is no record), and a subscriber cannot take Rec or xRec (AL0441). Pinned for these page
//! types: Card, List, Worksheet, ListPlus, StandardDialog, NavigatePage, CardPart and ListPart; every other page type (a Role
//! Center among them) is refused at the open. Each further rule below names the page types and the shape its pins cover; a
//! neighbor keeps the earlier behavior or is refused.

use super::*;
use al_builtins::page_op::PageMethod;

/// The page types whose open and close NoSourcePageEvents.al pins (`""` is a page without a PageType, a Card). A Role Center
/// is not one: its parts open and close in their own order there (NoSourcePageEvents.al:1628), which the client does not build.
const PINNED_TYPES: [&str; 9] = [
    "",
    "card",
    "list",
    "worksheet",
    "listplus",
    "standarddialog",
    "navigatepage",
    "cardpart",
    "listpart",
];

/// The page types on which an action is pinned to run OnAfterGetCurrRecord once after it, also after CurrPage.Update(true) and
/// Update(false) (NoSourcePageEvents.al:1734 NoSourceCard_Action_UpdateTrue_EventLog, :1768 UpdateFalse, :1838-:1906 List,
/// :1942-:2010 Worksheet, :2046-:2114 ListPlus, :2258-:2326 NavigatePage; the card also testpage/P146PageExprSetValue.al:351).
const ACTION_TYPES: [&str; 6] = ["", "card", "list", "worksheet", "listplus", "navigatepage"];

/// The page types on which New, Next, Previous, First and Last are pinned, each after OpenEdit (NoSourcePageEvents.al:3062-:3363).
const NAVIGATE_TYPES: [&str; 4] = ["", "card", "list", "worksheet"];

/// The page types on which an action after a SetValue on an expression control is pinned: the page refreshes after the action
/// and the expression controls read as they are now (pages/W1Controls.al:424 TempRecordControl_SetValue on a Card,
/// testpage/no-source-controls/NoSourceArrayAndWizard.al:302 Wizard_SetValues_ThenNextAction_EnabledAndVisible on a NavigatePage).
const EXPR_SET_ACTION_TYPES: [&str; 3] = ["", "card", "navigatepage"];

/// The page types on which a ModalPageHandler error closes the page with its close triggers (NoSourcePageEvents.al:1149
/// NavigatePage, :1349 CardPart, :1549 ListPart).
const HANDLER_ERROR_CLOSE_TYPES: [&str; 3] = ["navigatepage", "cardpart", "listpart"];

/// The page types on which Cancel of a modal run in lookup mode runs OnAfterGetCurrRecord before it closes
/// (NoSourcePageEvents.al:163 NoSourceCard_RunModal_Cancel_EventLog, :361 List, :559 Worksheet, :757 ListPlus, :954 StandardDialog).
const LOOKUP_CANCEL_TYPES: [&str; 6] = ["", "card", "list", "worksheet", "listplus", "standarddialog"];

/// What OpenNew does on a page over no table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::pages) enum NoSourceNew {
    /// Fails with DB:ClientInsertDenied after OnOpenPage and OnOpenPageEvent.
    Denied,
    /// Opens as OpenView does.
    AsView,
}

/// The events such a page raises, with the parameters other than Rec their subscribers may take.
const RAISED: [&str; 4] = [
    "onopenpageevent",
    "onaftergetcurrrecordevent",
    "onqueryclosepageevent",
    "onclosepageevent",
];

impl Session {
    /// Whether `st` is a page over no table, not a request page (it has its own rules), of a page type in `types`.
    fn no_source_of(&self, st: &St, types: &[&str]) -> bool {
        let object = st.borrow().object as usize;
        !self.has_source(st)
            && !self.is_request_page(st)
            && types.contains(&self.engine.object_facts[object].page_type_lower())
    }

    /// A page over no table of a page type NoSourcePageEvents.al pins for open and close.
    pub(in crate::pages) fn no_source_type_pinned(&self, st: &St) -> bool {
        self.no_source_of(st, &PINNED_TYPES)
    }

    /// A page over no table on which an action runs OnAfterGetCurrRecord once after it (`ACTION_TYPES`).
    pub(in crate::pages) fn no_source_action_refresh_pinned(&self, st: &St) -> bool {
        self.no_source_of(st, &ACTION_TYPES)
    }

    /// A page over no table on which an action after a SetValue on an expression control is pinned (`EXPR_SET_ACTION_TYPES`).
    pub(in crate::pages) fn no_source_expr_set_action_pinned(&self, st: &St) -> bool {
        self.no_source_of(st, &EXPR_SET_ACTION_TYPES)
    }

    /// A page over no table that a ModalPageHandler error closes with its close triggers (`HANDLER_ERROR_CLOSE_TYPES`).
    pub(in crate::pages) fn no_source_handler_error_close_pinned(&self, st: &St) -> bool {
        self.no_source_of(st, &HANDLER_ERROR_CLOSE_TYPES)
    }

    /// The one entry condition of New, Next, Previous, First and Last on a page over no table. Everything else (OpenView,
    /// OpenNew, a page a Page variable or Page.Run ran that the test trapped, a modal page in its handler, a part, another page
    /// type) is refused as before.
    fn no_source_navigate_pinned(&self, st: &St) -> bool {
        // The page type: a Card, a List or a Worksheet over no table (NoSourcePageEvents.al:3062-:3144 Card, :3164-:3246 List,
        // :3266-:3348 Worksheet).
        self.no_source_of(st, &NAVIGATE_TYPES)
            // The open: the test's own `P.OpenEdit()` on the top-level page (:3069 NoSourceCard_Navigate_New_EventLog, :3091
            // Next, :3111 Previous, :3131 First, :3151 Last; :3171-:3253 List, :3273-:3355 Worksheet).
            && st.borrow().test_open_edit()
    }

    /// TestPage.New() on a page over no table is denied as InsertAllowed = false denies it, with the same text and code
    /// (NoSourcePageEvents.al:3062 NoSourceCard_Navigate_New_EventLog `New method failed because Insert is not allowed. Page = LT
    /// PNS Card, Id = 97610.`, DB:ClientInsertDenied, nothing runs; :3164 List, :3266 Worksheet; all after OpenEdit).
    pub(in crate::pages) fn no_source_new_denied(&self, st: &St) -> bool {
        self.no_source_navigate_pinned(st)
    }

    /// Next, Previous, First and Last on a page over no table (`page_move` sends such a page here, `result_used` the use of the
    /// answer). Next, Previous and First run OnAfterGetCurrRecord and its event, Last runs nothing, and none fails
    /// (NoSourcePageEvents.al:3084 NoSourceCard_Navigate_Next_EventLog, :3104 Previous, :3124 First, :3144 Last `log after Last`
    /// empty; the same for a List, :3186-:3246, and a Worksheet, :3288-:3348; all after OpenEdit). The Boolean they answer is not
    /// observed there: a call that uses it is refused, as is another page type or open mode.
    pub(in crate::pages) fn no_source_move(
        &mut self,
        st: &St,
        how: PageMethod,
        result_used: bool,
    ) -> Result<Value, Signal> {
        if !self.no_source_navigate_pinned(st) || result_used {
            return unsupported(format!("{how}() on a page without a source table (not pinned)"));
        }
        match how {
            PageMethod::Next | PageMethod::Previous | PageMethod::First => {
                self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
            }
            PageMethod::Last => {}
            _ => return unsupported(format!("{how}() on a page without a source table (not pinned)")),
        }
        Ok(Value::Boolean(false))
    }

    /// Cancel of a modal run in lookup mode of a page over no table runs OnAfterGetCurrRecord and its event before the page
    /// closes (`LOOKUP_CANCEL_TYPES`); a non-modal StandardDialog runs nothing there (NoSourcePageEvents.al:1002
    /// NoSourceStdDialog_OpenView_ThenCancelAction_EventLog).
    pub(in crate::pages) fn no_source_lookup_cancel_reads(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.modal && s.lookup && s.host.is_none()
        };
        if shape && self.no_source_of(st, &LOOKUP_CANCEL_TYPES) {
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        }
        Ok(())
    }

    /// What OpenNew does on a page over no table. A Card fails with DB:ClientInsertDenied after OnOpenPage, and no other trigger
    /// runs (testpage/P146PageExprSetValue.al:460 QG_NoSource_OpenNew_StepLog `qg open;`); so do a List, a ListPlus, a CardPart
    /// and a ListPart, after OnOpenPage and OnOpenPageEvent (events/page-events/NoSourcePageEvents.al:90 NoSourceCard_OpenNew_EventLog,
    /// :288 List, :684 ListPlus, :1276 CardPart, :1476 ListPart). A StandardDialog and a NavigatePage open as with OpenView (:882,
    /// :1077). A Worksheet fails before (`open_page_as`); another page type is not pinned.
    pub(in crate::pages) fn no_source_open_new(&self, st: &St) -> Result<NoSourceNew, Signal> {
        let object = st.borrow().object as usize;
        match self.engine.object_facts[object].page_type_lower() {
            "" | "card" | "list" | "listplus" | "cardpart" | "listpart" => Ok(NoSourceNew::Denied),
            "standarddialog" | "navigatepage" => Ok(NoSourceNew::AsView),
            _ => unsupported("OpenNew of a page without a source table of this page type (not pinned)"),
        }
    }

    /// Raises `event` of a page without a source table (`page_event` sends such a page here). OnQueryClosePageEvent hands
    /// `AllowClose`, as on a page with a source table; the other three hand nothing.
    pub(super) fn no_source_page_event(&mut self, st: &St, event: &str, allow: Option<bool>) -> Result<bool, Signal> {
        if !RAISED.contains(&event) {
            return unsupported(format!(
                "page event {event} on a page without a source table (not pinned)"
            ));
        }
        let object = st.borrow().object as usize;
        let (names, mut args): (Vec<String>, Vec<Value>) = if event == "onqueryclosepageevent" {
            (vec!["AllowClose".into()], vec![Value::Boolean(true)])
        } else {
            (vec![], vec![])
        };
        let call = crate::dispatch::EventCall {
            object,
            event,
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        self.raise(&call, &mut args)?;
        Ok(match (allow, args.last()) {
            (Some(_), Some(Value::Boolean(b))) => *b,
            _ => true,
        })
    }

    /// The reason to refuse the open of a page without a source table that a subscriber of a platform page event listens to: a
    /// page type NoSourcePageEvents.al does not pin.
    pub(super) fn no_source_event_refusal(&self, engine: &Engine, st: &St, o: usize, ev: &str) -> Option<String> {
        let object = st.borrow().object as usize;
        let is_platform_event = al_ir::RAISED_PAGE_EVENTS.contains(&ev) || ev == "ondeleterecordevent";
        let action_event = matches!(ev, "onbeforeactionevent" | "onafteractionevent");
        if o != object || !is_platform_event || action_event || engine.program.objects[object].rec_slot.is_some() {
            return None;
        }
        let kind = engine.object_facts[object].page_type_lower();
        if PINNED_TYPES.contains(&kind) {
            return None;
        }
        Some(format!(
            "page event {ev} on a page without a source table (PageType {kind}): when the test client raises it there is not pinned"
        ))
    }

    /// A ModalPageHandler that raised an error left a page over no table open (it has no parts): the platform closes it with its
    /// close triggers and their events, and RunModal raises the handler's error (NoSourcePageEvents.al:1149
    /// NoSourceNavPage_RunModal_Cancel_EventLog `handler NavPage;NavPage.page.OnQueryClosePage;NavPage.OnQueryClosePageEvent;
    /// NavPage.page.OnClosePage;NavPage.OnClosePageEvent;`, TestActionNotFound; :1349 CardPart, :1549 ListPart). A close the page
    /// refuses is not pinned.
    pub(in crate::pages) fn close_no_source_modal_after_handler_error(&mut self, st: &St) -> Result<(), Signal> {
        let a = self.unattended_action(st);
        self.close_with(st, a, false, false)?;
        if st.borrow().open {
            return unsupported(
                "a page without a source table left open by a ModalPageHandler that raised an error, whose close it refused (not pinned)",
            );
        }
        Ok(())
    }

    /// An action in `area(Processing)` of a StandardDialog over no table is not on the page: invoking it raises
    /// `The action with ID = n is not found on the page.` (TestActionNotFound) and runs nothing (NoSourcePageEvents.al:2150
    /// NoSourceStdDialog_Action_UpdateTrue_EventLog, :2186 UpdateFalse, :2222 CloseSelf). Another area is not pinned and runs
    /// as before.
    pub(in crate::pages) fn no_source_dialog_action_absent(&self, st: &St, idx: usize) -> bool {
        let object = st.borrow().object as usize;
        if self.has_source(st)
            || self.is_request_page(st)
            || self.engine.object_facts[object].page_type_lower() != "standarddialog"
        {
            return false;
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        if page.controls[idx].kind != ir::ControlKind::Action {
            return false;
        }
        let mut root = idx;
        while let Some(p) = page.controls[root].parent {
            root = p;
        }
        page.action_areas
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("processing") && a.roots.contains(&root))
    }

    /// `CurrPage.Close()` in OnOpenPage of a Card over no table that the test opened with OpenEdit does nothing: the open goes
    /// on (OnOpenPageEvent, OnAfterGetCurrRecord and its event) and a later TestPage.Close runs the close triggers once
    /// (NoSourcePageEvents.al:2781 NoSourceCard_CurrPage_CloseInOpen_EventLog). Another page type, open mode or run is not pinned
    /// and closes as before.
    pub(in crate::pages) fn no_source_close_in_open_ignored(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        // The open: the test's own OpenEdit (:2790 through TryOpenEditCard, :3607), while OnOpenPage runs.
        let shape = {
            let s = st.borrow();
            s.opening && s.test_open_edit()
        };
        shape
            && self.no_source_of(st, &PINNED_TYPES)
            && matches!(self.engine.object_facts[object].page_type_lower(), "" | "card")
    }

    /// OnAfterGetCurrRecordEvent of the one row a page over no table shows when it opens. An error a subscriber raises there at
    /// OpenEdit of a Card keeps its text and gets `:TargetInvocation` added to its code (NoSourcePageEvents.al:2518
    /// NoSourceCard_SubscriberError_OnAfterGetCurrRecordEvent `Dialog:TargetInvocation`). Another page type or open mode is not
    /// pinned: refused.
    pub(in crate::pages) fn no_source_open_current_record_event(&mut self, st: &St) -> Result<(), Signal> {
        match self.page_event(st, "onaftergetcurrrecordevent", None, None) {
            Ok(_) => Ok(()),
            Err(Signal::Error(mut e)) if self.no_source_of(st, &PINNED_TYPES) => {
                let object = st.borrow().object as usize;
                // The open: the test's own OpenEdit of a Card (:2525).
                let card_edit = st.borrow().test_open_edit()
                    && matches!(self.engine.object_facts[object].page_type_lower(), "" | "card");
                if !card_edit {
                    return unsupported(
                        "an error in an OnAfterGetCurrRecordEvent subscriber while a page without a source table opens, other than OpenEdit of a Card (not pinned)",
                    );
                }
                e.code = format!("{}:TargetInvocation", e.code);
                Err(Signal::Error(e))
            }
            Err(e) => Err(e),
        }
    }

    /// The order the parts of a Card over no table are asked and closed in: the last declared first, in both rounds
    /// (NoSourcePageEvents.al:2964 NoSourceHostCard_OpenView_PartsAndHostLog, a CardPart then a ListPart declared: `ListPart.page.
    /// OnQueryClosePage;...;CardPart.page.OnQueryClosePage;...;Host.OnQueryClosePageEvent;ListPart.page.OnClosePage;...;CardPart.
    /// page.OnClosePage;...`). Pinned for two parts over no table that run close code; more of them is refused, and a host with
    /// a source table or a part with one keeps the declared order.
    pub(in crate::pages) fn no_source_host_part_close_order(&self, st: &St, parts: &mut [St]) -> Result<(), Signal> {
        if !self.no_source_of(st, &PINNED_TYPES) || parts.iter().any(|p| self.has_source(p)) {
            return Ok(());
        }
        let object = st.borrow().object as usize;
        let card = matches!(self.engine.object_facts[object].page_type_lower(), "" | "card");
        let closing_code = parts.iter().filter(|p| self.runs_close_code(p)).count();
        if closing_code < 2 {
            return Ok(());
        }
        if !card || parts.len() != 2 {
            return unsupported(
                "closing a page without a source table whose parts run close code, other than a Card with two parts (order not pinned)",
            );
        }
        parts.reverse();
        Ok(())
    }

    /// `CurrPage.Close()` or a Page variable's `Close()` that does nothing: a Page variable not shown (`page_var_not_shown`) or
    /// the close in OnOpenPage `no_source_close_in_open_ignored` names.
    pub(in crate::pages) fn close_does_nothing(&self, st: &St) -> bool {
        self.page_var_not_shown(st) || self.no_source_close_in_open_ignored(st)
    }
}
