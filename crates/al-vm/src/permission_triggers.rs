//! ADR-062 stage 3: Execute checks around code a platform event starts (table triggers, event subscribers, OnOpenPage).
//! Values: corpus `tests/al-language/permissions/lt-y84-perm-shapes/Y84PermExecute.al` (Y84_Exec_*, BC 29.0 Linux CI).

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
    /// `page_both`). A session that grants Execute by wildcard (`Permissions Mock.Set` adds All Objects) is not pinned and refuses.
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
            if on_open_page {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageCalleeAndPartShapes);
            }
            return None;
        }
        if eff.level(wildcard) != 0 {
            if !on_open_page {
                // OnInit and OnAfterGetRecord under a wildcard: the call is checked like any other (the abort is pinned for exact sets only).
                return None;
            }
            // A wildcard alone grants the callee: Y90PermOpenPage.al:60,67 (`set_not_exact`) and :32,43 (`pages_cu_star`) pin own-app
            // callees, but whether a wildcard reaches another app's objects is still being probed (corpus Y93), so this refuses.
            return Some(Signal::Unsupported(
                "a codeunit call from OnOpenPage under a session with wildcard Execute on codeunits, the callee granted by the wildcard only (an explicit grant of the callee opens, Y90PermOpenPage.al:91; wildcard scope for other apps' objects is being probed)".into(),
            ));
        }
        Some(Signal::Error(Box::new(al_builtins::native::AlError {
            message: format!("Page {} has to close.", heads[object].name),
            code: "FormAbort:Permission".into(),
            ..Default::default()
        })))
    }
}

/// Whether an error of an action's OnAction comes out of the test page's Invoke with the code `TestWrapped:Permission`: an Execute
/// denial (Y84PermExecute.al Y84_Exec_PageAction) and an Insert denial (permissions/lt-permissions-mock-indirect/
/// RiskyPagePermissions.al:129).
pub(crate) fn wraps_as_permission(code: &str, message: &str) -> bool {
    matches!(code, "DB:ExecuteObjDenied" | "DB:ClientInsertDenied")
        && message.starts_with("Sorry, the current permissions prevented the action.")
}
