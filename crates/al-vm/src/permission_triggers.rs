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

    /// The page whose OnOpenPage makes the call about to start, when the session grants Execute on codeunits by no wildcard
    /// (an exact set). A codeunit call from OnOpenPage then ends the open with `Page <name> has to close.`, code
    /// `FormAbort:Permission`, whether the codeunit is granted or not (Y84PermExecute.al Y84_Exec_PageOpen `page_page`,
    /// `page_both`). A session that grants Execute by wildcard (`Permissions Mock.Set` adds All Objects) is not pinned to
    /// abort, and Microsoft's tests call codeunits from OnOpenPage under it, so it is checked as any call.
    pub(crate) fn permission_open_page_abort(&self, callee: usize) -> Option<Signal> {
        if self.permission_caller.is_some() {
            return None;
        }
        let (&object, frame) = (self.frames.last()?, self.call_stack.last()?);
        let heads = self.engine.program.objects.heads();
        let routine = &self.engine.program.objects[object].routines[frame.routine];
        if heads[object].kind != al_ir::ObjKind::Page
            || !routine.is_trigger
            || !routine.name.eq_ignore_ascii_case("OnOpenPage")
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
        if eff.level(wildcard) != 0 {
            return None;
        }
        Some(Signal::Error(Box::new(al_builtins::native::AlError {
            message: format!("Page {} has to close.", heads[object].name),
            code: "FormAbort:Permission".into(),
            ..Default::default()
        })))
    }
}
