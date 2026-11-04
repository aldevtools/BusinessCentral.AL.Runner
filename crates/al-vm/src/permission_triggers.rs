//! ADR-062 stage 3: Execute checks around code a platform event starts (table triggers, event subscribers, OnOpenPage).
//! Values: corpus `tests/al-language/permissions/permission-sets/Y84PermExecute.al` (Y84_Exec_*, BC 29.0 Linux CI).

use crate::Session;
use crate::permissions::{Access, Op};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

impl Session {
    /// A record write that runs a table trigger needs Execute on the table first, before the table data permission
    /// (Y84PermExecute.al Y84_Exec_TableDenied `t2_run`, `t2_call`: a set with table data but no Execute on the table
    /// gives `(Table <id> <name> Execute: <app>)`, not the Insert denial). A write that runs no trigger loads no code and is
    /// not checked (Insert(false) and a table without the trigger are not pinned and not checked).
    pub(crate) fn permission_table_trigger_execute(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
        object: usize,
    ) -> Result<(), Signal> {
        let trigger = match method {
            RecordOp::Insert => "OnInsert",
            RecordOp::Modify => "OnModify",
            RecordOp::Delete => "OnDelete",
            _ => return Ok(()),
        };
        if !matches!(args.first(), Some(Value::Boolean(true))) {
            return Ok(());
        }
        let t = self.table(rec)?;
        if self.table_trigger(&t, trigger).is_none() {
            return Ok(());
        }
        self.permission_gate_execute(object)
    }

    /// Whether the call about to start is made by the code of an event subscriber. The code a subscriber calls is not
    /// checked for Execute (Y84PermExecute.al Y84_Exec_Subscriber `sub_tab`, `sub_sub`, `sub_both`: a codeunit method called
    /// from a subscriber runs under a set with no Execute on it, with and without the subscriber's own Execute).
    pub(crate) fn permission_caller_is_subscriber(&self) -> bool {
        if self.permission_caller.is_some() {
            return false;
        }
        let (Some(&object), Some(frame)) = (self.frames.last(), self.call_stack.last()) else {
            return false;
        };
        self.engine.program.objects[object].routines[frame.routine]
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
    }

    /// The page whose OnOpenPage, OnInit or OnAfterGetRecord (Y90PermOpenPage.al:72-88 `on_init`, `after_get_record`) makes the call about to start, when the session grants Execute on codeunits by no wildcard
    /// (an exact set). A codeunit call from OnOpenPage then ends the open with `Page <name> has to close.`, code
    /// `FormAbort:Permission`, whether the codeunit is granted or not (Y84PermExecute.al Y84_Exec_PageOpen `page_page`,
    /// `page_both`). Under a session that grants Execute on codeunits by wildcard the call is checked as any other (Y93WildScope.al:576-599).
    pub(crate) fn permission_open_page_abort(&self, callee: usize) -> Option<Signal> {
        if self.permission_caller.is_some() {
            return None;
        }
        let (&object, frame) = (self.frames.last()?, self.call_stack.last()?);
        let heads = self.engine.program.objects.heads();
        let routine = &self.engine.program.objects[object].routines[frame.routine];
        if heads[object].kind != al_ir::ObjKind::Page
            || !routine.is_trigger
            || !["OnOpenPage", "OnInit", "OnAfterGetRecord"]
                .iter()
                .any(|n| routine.name.eq_ignore_ascii_case(n))
            || heads[callee].kind != al_ir::ObjKind::Codeunit
        {
            return None;
        }
        let eff = self.permissions.as_ref()?;
        let wildcard = Access {
            kind: 5,
            id: u32::MAX,
            op: Op::Execute,
        };
        // The callee is granted by name (a set the test assigned lists `codeunit "X" = X`): Y90PermOpenPage.al:91-118 (Y90_Open_D365BasicPlusAppSet,
        // `Set('D365 BASIC')` + `Assign` of a set that grants the callee by name) opens the page, and so does an exact set with `system * = X`
        // and the callee by name (Y90PermOpenPage.al:59,65, `exact_sys_star`, set LT Y90 P PagesSys). Y84PermExecute.al Y84_Exec_PageOpen
        // `page_both` (the callee by name and no wildcard) aborts.
        let callee_id = heads[callee].id.unwrap_or(0);
        let named = eff.content.named_level(5, callee_id, Op::Execute as usize) == 1;
        let system_wildcard = Access {
            kind: 10,
            id: u32::MAX,
            op: Op::Execute,
        };
        let on_open_page = routine.name.eq_ignore_ascii_case("OnOpenPage");
        if named && (eff.level(wildcard) != 0 || eff.level(system_wildcard) == 1) {
            return None;
        }
        match eff.level(wildcard) {
            0 => {}
            // A wildcard at Yes: the call is checked like any other, and the wildcard grants the callee whatever app declares it
            // (Y93WildScope.al:165-244, `xa cu|W All`). The page opens: Y93WildScope.al:576-599 (q1 `Set('D365 BASIC')`, q2 plus
            // `codeunit * = X`, q4 an exact set of wildcards only, q5 `D365 BASIC` alone, which holds wildcard Execute on every kind),
            // Y90PermOpenPage.al `set_not_exact`, `pages_cu_star`.
            1 => return None,
            // `codeunit * = x` (Indirect): not measured.
            _ if on_open_page => {
                return Some(Signal::Unsupported(
                    "a codeunit call from OnOpenPage under a session whose wildcard Execute on codeunits is indirect only (`codeunit * = x`; Y93WildScope.al pins the wildcard at Yes only)".into(),
                ));
            }
            _ => return None,
        }
        // The text names the page's Caption when it has one (Y90PermOpenPage.al:36-37, `Page LT Y90 Shown Caption has to close.`),
        // else its name. A CaptionML without Caption is not pinned.
        let props = &self.engine.program.objects[object].properties;
        let prop = |name: &str| props.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v);
        let shown = match (prop("Caption"), prop("CaptionML")) {
            (Some(c), _) => crate::pages::unquote(c),
            (None, Some(_)) => {
                return Some(Signal::Unsupported(
                    "a codeunit call from a page trigger ends the open of a page with CaptionML only (the text of `Page <caption> has to close.` is pinned for Caption, Y90PermOpenPage.al:36)".into(),
                ));
            }
            (None, None) => heads[object].name.clone(),
        };
        Some(Signal::Error(Box::new(al_builtins::native::AlError {
            message: format!("Page {shown} has to close."),
            code: "FormAbort:Permission".into(),
            ..Default::default()
        })))
    }
}

/// Whether an error of an action's OnAction comes out of the test page's Invoke with the code `TestWrapped:Permission`: an Execute
/// denial (Y84PermExecute.al Y84_Exec_PageAction) and an Insert denial (permissions/permission-sets/
/// RiskyPagePermissions.al:129).
pub(crate) fn wraps_as_permission(code: &str, message: &str) -> bool {
    matches!(code, "DB:ExecuteObjDenied" | "DB:ClientInsertDenied")
        && message.starts_with("Sorry, the current permissions prevented the action.")
}
