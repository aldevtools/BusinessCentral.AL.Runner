//! `TaskScheduler.CreateTask`, `TaskExists`, `CancelTask` and `SetTaskReady`. A test never sees a task run (it starts in a
//! session of its own after the creating transaction commits: platform/W6TaskScheduler.al:71-86
//! CreateTask_DoesTheTaskRun_AfterAPause), so a task is only an id the session knows until the transaction ends.
//! Pinned by platform/W6TaskScheduler.al:88-176 and platform/lt-platform-members/PlTTaskScheduler.al:40-215:
//! - a created task exists, also when created not ready, with a failure codeunit, a company, a start time or a record id;
//! - CancelTask of it answers true and the task no longer exists; a second CancelTask answers false;
//! - TaskExists and CancelTask of an unknown or the null id answer false; SetTaskReady of an unknown id returns, of a
//!   created task leaves it existing;
//! - a rollback removes the tasks of the transaction (PlTTaskScheduler.al:198-215 CreateTask_RolledBackWithTransaction);
//! - an unknown codeunit or failure codeunit raises the missing-object error (PlTTaskScheduler.al:150-160, 172-182);
//!   codeunit 0 and an unknown company do not (PlTTaskScheduler.al:162-170, 184-193).
//!
//! A task whose creation was committed is still there and has not run, ready or not, with a Not Before in the future or
//! none: Get answers Yes with the same columns, a count of Is Ready and User ID counts it, an exist() FlowField sees it,
//! TaskExists answers Yes and CancelTask removes it (platform/lt-y43-task-after-commit/Y43TaskAfterCommit.al, corpus master,
//! al-runner issue 6592). SetTaskReady of such a task changes its Not Before to a value no test pins: refused.
//!
//! A task of the open transaction has its row in the Scheduled Task table: Get answers Yes, a filter on ID counts it, and
//! CancelTask removes it (platform/lt-wd/WdScheduledTaskGet.al, corpus branch al-runtime-rs-staging-1030c, al-runner issue
//! 6315). Its User ID, User Name, Company, Run Codeunit and Is Ready (No for a task created not ready) are pinned by
//! session/lt-cn-session-leads/CnSessionLeads.al:23-46; Is Ready Yes for a task created ready, and Not Before blank, or
//! the time given (Y43TaskAfterCommit.al:133-142, :282-298); every other column is refused when read (`platform::unknown`).

use crate::Session;
use al_builtins::builtin_id::ids;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::Value;

/// The tasks this session created.
#[derive(Default, Clone)]
pub(crate) struct Tasks {
    /// Created in the running transaction: a rollback drops them.
    open: std::collections::BTreeSet<[u8; 16]>,
    /// Created in a transaction that was committed: they are still there (Y43TaskAfterCommit.al).
    committed: std::collections::BTreeSet<[u8; 16]>,
}

impl Tasks {
    pub(crate) fn commit(&mut self) {
        let open = std::mem::take(&mut self.open);
        self.committed.extend(open);
    }

    pub(crate) fn rollback(&mut self) {
        self.open.clear();
    }
}

/// Table "Scheduled Task" (al-builtins platform.rs).
const SCHEDULED_TASK: u32 = 2_000_000_175;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// A TaskScheduler builtin, or `None` for any other id.
    pub(crate) fn task_scheduler_op(&mut self, id: al_ir::BuiltinId, args: &[Value]) -> Option<Result<Value, Signal>> {
        let answer = match id {
            ids::TASKSCHEDULER_CREATETASK_1 | ids::TASKSCHEDULER_CREATETASK_2 => self.create_task(args),
            ids::TASKSCHEDULER_TASKEXISTS_1 => self.task_id(args, "TaskExists", true).map(|t| {
                Value::Boolean(t.is_some_and(|t| self.tasks.open.contains(&t) || self.tasks.committed.contains(&t)))
            }),
            ids::TASKSCHEDULER_CANCELTASK_1 => self.task_id(args, "CancelTask", true).map(|t| {
                let removed = t.is_some_and(|t| self.tasks.open.remove(&t) || self.tasks.committed.remove(&t));
                if let (true, Some(t)) = (removed, t) {
                    self.remove_task_row(t);
                }
                Value::Boolean(removed)
            }),
            ids::TASKSCHEDULER_SETTASKREADY_1 => self.task_id(args, "SetTaskReady", false).map(|_| Value::Null),
            _ => return None,
        };
        Some(answer)
    }

    /// The task id argument; refused for a task whose creation was committed unless `committed_ok`.
    fn task_id(&self, args: &[Value], what: &str, committed_ok: bool) -> Result<Option<[u8; 16]>, Signal> {
        let Some(Value::Guid(g)) = args.first() else {
            return unsupported(format!("TaskScheduler.{what} with id {}", args.first().describe()));
        };
        if !committed_ok && self.tasks.committed.contains(g) {
            return unsupported(format!(
                "TaskScheduler.{what} of a task whose creation was committed (the Not Before it leaves is not pinned)"
            ));
        }
        Ok(Some(*g))
    }

    /// The row of a task created in the open transaction (the module comment lists the pinned columns). The store's
    /// rollback takes it back with the transaction.
    fn write_task_row(&mut self, id: [u8; 16], args: &[Value]) {
        let Some(t) = self.table_def(SCHEDULED_TASK) else {
            // Nothing in the program reads the table.
            return;
        };
        let whose = "a task created by TaskScheduler.CreateTask";
        let mut row: al_data::Row = t
            .fields
            .iter()
            .map(|f| crate::platform::unknown(&t, f.id, whose))
            .collect();
        let mut set = |name: &str, v: Value| {
            if let Some(p) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) {
                // A text goes in as the field's own kind of text (Code or Text).
                row[p] = match (Value::default_for(&t.fields[p].ty), v) {
                    (Value::Code(_), Value::Text(x)) => Value::Code(x.as_str().to_uppercase().into()),
                    (_, v) => v,
                };
            }
        };
        set("ID", Value::Guid(id));
        set("User ID", Value::Guid(self.host.identity.user_security_id()));
        set("User Name", Value::Text(self.host.identity.user_id().into()));
        if let Some(cu) = args.first() {
            set("Run Codeunit", cu.clone());
        }
        // Is Ready for a task created ready or not (Y43TaskAfterCommit.al:142, :170-180), Company for a task created without a
        // company argument, Not Before blank or the time given (:142, :293-298).
        match args.get(2) {
            None | Some(Value::Boolean(true)) => set("Is Ready", Value::Boolean(true)),
            Some(Value::Boolean(false)) => set("Is Ready", Value::Boolean(false)),
            Some(_) => {}
        }
        match args.get(4) {
            None => set("Not Before", Value::DateTime(Default::default())),
            Some(v @ Value::DateTime(_)) => set("Not Before", v.clone()),
            Some(_) => {}
        }
        if args.len() < 4 {
            set("Company", Value::Text(self.cx.company.clone().into()));
        }
        let cx = self.cx.clone();
        let _ = al_data::Store::insert(&mut self.store, &cx, &t, row);
    }

    fn remove_task_row(&mut self, id: [u8; 16]) {
        let Some(t) = self.table_def(SCHEDULED_TASK) else {
            return;
        };
        let cx = self.cx.clone();
        let _ = al_data::Store::delete(&mut self.store, &cx, &t, &[Value::Guid(id)]);
    }

    fn create_task(&mut self, args: &[Value]) -> Result<Value, Signal> {
        for arg in args.iter().take(2) {
            let Value::Integer(n) = arg else {
                return unsupported(format!("TaskScheduler.CreateTask with codeunit {}", arg.describe()));
            };
            if *n != 0 && self.engine.codeunit_object(u32::try_from(*n).ok()).is_none() {
                return Err(self.missing_object("CodeUnit", i64::from(*n)));
            }
        }
        let id = self.fresh_guid();
        self.tasks.open.insert(id);
        self.write_task_row(id, args);
        Ok(Value::Guid(id))
    }
}
