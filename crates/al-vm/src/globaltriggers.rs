//! The events of the platform codeunit `Global Triggers` (2000000002, in the platform
//! `System.app`). Its procedures are body-less business events; BC's server raises them around
//! record writes, so this runtime does too.
//!
//! What the corpus pins (record/TestTriggerDispatchOrder.al, with the fixture
//! `_fixtures/events/ALTGlobalTriggerSubscriber.al`):
//!
//! - The database events come between the table's own trigger (with its extension's) and the After
//!   event: `OnInsert, TableExtOnInsert, OnDatabaseInsert, TableExtOnAfterInsert, After event`
//!   (:44-100). The same for Modify and Delete.
//! - Rename raises `OnGlobalRename` and then `OnDatabaseRename`, between the extension's `OnRename`
//!   and its `OnAfterRename` (:102-123).
//! - A database event is raised only when `GetDatabaseTableTriggerSetup` enabled it for the table
//!   ("when database triggers are enabled", :125-165).
//! - `OnGlobalInsert`, `OnGlobalModify` and `OnGlobalDelete` are not published in BC 27.5 and 28.1,
//!   even when `GetGlobalTableTriggerMask` answers 15; `OnGlobalRename` is (:125-165).
//!
//! What it does not pin, and so is an assumption (docs/probes-needed.md): that a mask bit of 8
//! is the one for Rename (the corpus answers 15, every bit); that the setup is asked once per table
//! and session (AL Runner reads that in BC's `GetTriggersOnTable`); that `Insert(false)` and the
//! other `RunTrigger = false` writes raise the database events too.

use crate::Session;
use crate::dispatch::EventCall;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_data::TableDef;

/// What the subscribers of `Global Triggers` answered for one table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GlobalSetup {
    /// `GetGlobalTableTriggerMask` has the Rename bit.
    pub global_rename: bool,
    pub insert: bool,
    pub modify: bool,
    pub delete: bool,
    pub rename: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlobalOp {
    Insert,
    Modify,
    Delete,
    Rename,
}

/// The Rename bit of the table trigger mask. The corpus sets all four bits (15) and nothing else.
const MASK_RENAME: i32 = 8;
const MASK_ALL: i32 = 15;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|n| n.to_string()).collect()
}

impl Session {
    /// Raises a `Global Triggers` event. While the SQL counters log record methods, its AL code is not logged: it is part
    /// of the record method that raised it (sqlcount.rs, `SqlCounters::mute_begin`).
    fn raise_global(&mut self, call: &EventCall, args: &mut [Value]) -> Result<(), Signal> {
        let m = self.sql.mute_begin(self.calls_made);
        let r = self.raise(call, args);
        self.sql.mute_end(m, self.calls_made);
        r
    }

    /// The setup of table `t`, asking the subscribers the first time (BC caches it per session).
    pub(crate) fn global_setup(&mut self, t: &TableDef) -> Result<GlobalSetup, Signal> {
        let table = t.object as usize;
        if let Some(s) = self.events.global_setups.get(&table) {
            return Ok(*s);
        }
        let key = self.engine.events.global_key;
        let id = self.engine.program.objects[table].id.unwrap_or(0) as i32;
        // A subscriber that writes the table it is asked about sees the table as not enabled.
        self.events.global_setups.insert(table, GlobalSetup::default());
        let mut setup = GlobalSetup::default();
        if self.engine.events.is_subscribed(key, "getglobaltabletriggermask") {
            let mut args = [Value::Integer(id), Value::Integer(0)];
            let n = names(&["TableID", "TableTriggerMask"]);
            let call = EventCall {
                object: key,
                event: "getglobaltabletriggermask",
                element: None,
                isolated: false,
                names: &n,
                sender: None,
            };
            if let Err(e) = self.raise_global(&call, &mut args) {
                self.events.global_setups.remove(&table);
                return Err(e);
            }
            let mask = match &args[1] {
                Value::Integer(m) => *m,
                other => {
                    self.events.global_setups.remove(&table);
                    return unsupported(format!("Global Triggers: mask {}", other.describe()));
                }
            };
            if mask != 0 && mask != MASK_ALL {
                self.events.global_setups.remove(&table);
                return unsupported(format!(
                    "Global Triggers: table trigger mask {mask} for {} (only 0 and 15 are pinned; which bit is Rename is not)",
                    t.name
                ));
            }
            setup.global_rename = mask & MASK_RENAME != 0;
        }
        // With no company open (the per-database part of an install), BC does not ask for the database trigger setup: a
        // write there raises no database event (corpus branch al-runtime-rs-staging-ug eff7eb2a,
        // install/lt-ug-install-trigger-setup/UgInstallTests.al TriggerSetup_AskedPerCompanyInInstall_NeverInPerDatabasePart;
        // assumption install-trigger-setup-per-company-only until the bump).
        let no_company = self.cx.company.is_empty();
        if no_company && self.engine.events.is_subscribed(key, "getdatabasetabletriggersetup") {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::InstallTriggerSetupPerCompanyOnly);
        }
        if !no_company && self.engine.events.is_subscribed(key, "getdatabasetabletriggersetup") {
            let mut args = [
                Value::Integer(id),
                Value::Boolean(false),
                Value::Boolean(false),
                Value::Boolean(false),
                Value::Boolean(false),
            ];
            let n = names(&[
                "TableId",
                "OnDatabaseInsert",
                "OnDatabaseModify",
                "OnDatabaseDelete",
                "OnDatabaseRename",
            ]);
            let call = EventCall {
                object: key,
                event: "getdatabasetabletriggersetup",
                element: None,
                isolated: false,
                names: &n,
                sender: None,
            };
            self.events.setup_no_ops = self.setup_subscribers_changing_nothing(id);
            let raised = self.raise_global(&call, &mut args);
            self.events.setup_no_ops.clear();
            if let Err(e) = raised {
                self.events.global_setups.remove(&table);
                return Err(e);
            }
            let on = |v: &Value| matches!(v, Value::Boolean(true));
            setup.insert = on(&args[1]);
            setup.modify = on(&args[2]);
            setup.delete = on(&args[3]);
            setup.rename = on(&args[4]);
        }
        self.events.global_setups.insert(table, setup);
        Ok(setup)
    }

    /// The first write to `t` asks the subscribers for its setup even when no event follows.
    pub(crate) fn global_setup_asked(&mut self, t: &TableDef) -> Result<(), Signal> {
        if self.engine.events.has_global_triggers {
            self.global_setup(t)?;
        }
        Ok(())
    }

    /// Whether database events are on for `op` on table `t`.
    pub(crate) fn global_enabled(&mut self, t: &TableDef, op: GlobalOp) -> Result<bool, Signal> {
        if !self.engine.events.has_global_triggers {
            return Ok(false);
        }
        let s = self.global_setup(t)?;
        Ok(match op {
            GlobalOp::Insert => s.insert,
            GlobalOp::Modify => s.modify,
            GlobalOp::Delete => s.delete,
            GlobalOp::Rename => s.rename || s.global_rename,
        })
    }

    /// Raises the `Global Triggers` events of `op` on the record `rec` (`xrec`: the old row of a
    /// rename or modify). Called after the table's own trigger and before the write.
    pub(crate) fn global_trigger(
        &mut self,
        t: &TableDef,
        op: GlobalOp,
        rec: &Rec,
        xrec: Option<&Rec>,
    ) -> Result<(), Signal> {
        if !self.engine.events.has_global_triggers {
            return Ok(());
        }
        // A temporary record asks for no setup and raises nothing
        // (session/TestEnvironmentUserTenantContracts.al:GlobalTriggers_TemporaryRecord_RaisesNoDatabaseEventAndAsksNoSetup).
        if rec.borrow().temporary {
            return Ok(());
        }
        let setup = self.global_setup(t)?;
        let any = match op {
            GlobalOp::Insert => setup.insert,
            GlobalOp::Modify => setup.modify,
            GlobalOp::Delete => setup.delete,
            GlobalOp::Rename => setup.rename || setup.global_rename,
        };
        if !any {
            return Ok(());
        }
        let key = self.engine.events.global_key;
        let handle = |r: &Rec| crate::refs::record_ref_value(r);
        // (event, parameter names, arguments), in the order BC raises them.
        let mut calls: Vec<(&str, Vec<String>, Vec<Value>)> = Vec::new();
        match op {
            GlobalOp::Insert => calls.push((
                "ondatabaseinsert",
                names(&["RecRef"]),
                vec![crate::refs::record_ref_shared(rec)],
            )),
            GlobalOp::Modify => calls.push((
                "ondatabasemodify",
                names(&["RecRef"]),
                vec![crate::refs::record_ref_shared(rec)],
            )),
            GlobalOp::Delete => calls.push(("ondatabasedelete", names(&["RecRef"]), vec![handle(rec)])),
            GlobalOp::Rename => {
                let Some(x) = xrec else {
                    return unsupported("Global Triggers: a rename without its old row");
                };
                if setup.global_rename {
                    calls.push((
                        "onglobalrename",
                        names(&["RecRef", "xRecRef"]),
                        vec![handle(rec), handle(x)],
                    ));
                }
                if setup.rename {
                    calls.push((
                        "ondatabaserename",
                        names(&["RecRef", "xRecRef"]),
                        vec![handle(rec), handle(x)],
                    ));
                }
            }
        }
        for (event, n, mut args) in calls {
            let call = EventCall {
                object: key,
                event,
                element: None,
                isolated: false,
                names: &n,
                sender: None,
            };
            self.raise_global(&call, &mut args)?;
        }
        Ok(())
    }
}
