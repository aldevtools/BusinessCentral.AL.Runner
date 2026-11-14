//! ADR-062 stage 4, the shadow run: with the process switch on (`al_builtins::permission_shadow::enable`, `al-test-cli
//! --permission-shadow FILE`), a test that does not disable TestPermissions runs with the permission sets the test runner
//! assigns composed as a restricted session, every check is evaluated, and every denial or refusal is noted and turned into a grant.
//! The test therefore runs as it does without the switch; the notes say what stage 4 would change.
//!
//! The wrappers here carry the names of the gates in `permissions.rs`, whose own bodies end in `_decided`.

use crate::Session;
use crate::permissions::Levels;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::rc::Rc;

impl Session {
    /// The test runner's step assigned `role` through the mock: remembered for the shadow run (the sets are not enforced).
    pub(crate) fn shadow_add_role(&mut self, role: &str) {
        if al_builtins::permission_shadow::enabled() {
            let role: String = role.to_uppercase().chars().take(20).collect();
            if !self.shadow_roles.contains(&role) {
                self.shadow_roles.push(role);
            }
        }
    }

    /// After the runner's steps: the session holds the runner's sets for the shadow run, when the switch is on and they restrict.
    pub(crate) fn shadow_begin(&mut self) {
        if self.shadow_roles.is_empty()
            || !self.runner_permission_sets
            || self.permissions_off
            || self.ignore_permission_mock
        {
            return;
        }
        let mut eff = crate::permissions::SetModel::of(&self.engine).compose(&self.shadow_roles);
        // A set that is not composed in full (a name two tables share, a grant not read) leaves a lower bound of the sets. The shadow
        // run evaluates it as it is, so that denials show at all, and marks every line (`lower bound`): a "would deny" may be a grant
        // the missing part gives.
        self.shadow_lower_bound = eff.as_ref().is_some_and(|e| !e.complete);
        if let Some(e) = &eff {
            if !e.complete {
                eff = Some(std::sync::Arc::new(crate::permissions::EffectivePermissions {
                    complete: true,
                    ..(**e).clone()
                }));
            }
        }
        self.shadow_active = eff.is_some();
        al_builtins::value::set_born_restricted(eff.is_some());
        self.permissions = eff;
    }

    /// The roles of the runner's step that are not enforced: forgotten on every `Clear`.
    pub(crate) fn shadow_clear(&mut self) {
        self.shadow_roles.clear();
        self.shadow_active = false;
        self.shadow_lower_bound = false;
    }

    /// Notes one line for the test that is running: `kind` is `would deny` or `unknown`.
    pub(crate) fn shadow_note(&self, kind: &str, site: &str, detail: &str) {
        let bound = if self.shadow_lower_bound { "\tlower bound" } else { "" };
        al_builtins::permission_shadow::record(format!(
            "{kind}\t{site}\t{detail}\tsets={}{bound}",
            self.shadow_roles.join("+")
        ));
    }

    fn shadow_signal(&self, site: &str, e: &Signal) {
        match e {
            Signal::Error(e) => self.shadow_note("would deny", site, &format!("{} [{}]", e.message, e.code)),
            Signal::Unsupported(why) => self.shadow_note("unknown", site, why),
            other => self.shadow_note("unknown", site, &format!("{other:?}")),
        }
    }

    fn shadow_unit(&self, site: &str, r: Result<(), Signal>) -> Result<(), Signal> {
        match r {
            Err(e) if self.shadow_active => {
                self.shadow_signal(site, &e);
                Ok(())
            }
            r => r,
        }
    }

    pub(crate) fn permission_gate_record(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
        from_al: bool,
    ) -> Result<(), Signal> {
        let r = self.permission_gate_record_decided(method, rec, args, from_al);
        self.shadow_unit("record", r)
    }

    pub(crate) fn permission_gate_flowfield(
        &mut self,
        rec: &crate::records::Rec,
        table: usize,
        stack: &[crate::flowfields::CalcFrame],
    ) -> Result<(), Signal> {
        let r = self.permission_gate_flowfield_decided(rec, table, stack);
        self.shadow_unit("flowfield", r)
    }

    pub(crate) fn permission_gate_object_read(&mut self, object: usize, tables: &[usize]) -> Result<(), Signal> {
        let r = self.permission_gate_object_read_decided(object, tables);
        self.shadow_unit("object read", r)
    }

    pub(crate) fn permission_gate_execute(&mut self, object: usize) -> Result<(), Signal> {
        let r = self.permission_gate_execute_decided(object);
        self.shadow_unit("execute", r)
    }

    pub(crate) fn permission_gate_call(
        &mut self,
        object: usize,
        routine: usize,
        instance: &Rc<RefCell<Instance>>,
    ) -> Result<(), Signal> {
        let r = self.permission_gate_call_decided(object, routine, instance);
        self.shadow_unit("call", r)
    }

    pub(crate) fn permission_page_open_denied(&mut self, object: usize) -> Result<bool, Signal> {
        match self.permission_page_open_denied_decided(object) {
            Ok(true) if self.shadow_active => {
                let name = self.engine.program.objects.heads()[object].name.clone();
                self.shadow_note(
                    "would deny",
                    "page open",
                    &format!("Page {name} has to close. [FormAbort:Permission]"),
                );
                Ok(false)
            }
            Err(e) if self.shadow_active => {
                self.shadow_signal("page open", &e);
                Ok(false)
            }
            r => r,
        }
    }

    /// `ReadPermission` and `WritePermission`: the answer a restricted session would give, false noted and true returned.
    pub(crate) fn permission_answer(&mut self, rec: &crate::records::Rec, method: RecordOp) -> Result<Value, Signal> {
        let r = self.permission_answer_decided(rec, method);
        if !self.shadow_active {
            return r;
        }
        let table = {
            let t = rec.borrow().table as usize;
            self.engine.program.objects.heads()[t].name.clone()
        };
        match r {
            Ok(Value::Boolean(false)) => {
                self.shadow_note(
                    "would deny",
                    "permission query",
                    &format!("{method:?} of {table} would answer false"),
                );
                Ok(Value::Boolean(true))
            }
            Err(e) => {
                self.shadow_signal("permission query", &e);
                Ok(Value::Boolean(true))
            }
            r => r,
        }
    }

    /// AccessByPermission: a control the runner's sets would leave off the page is noted and stays.
    pub(crate) fn access_by_permission_hidden(&self, guard: &str, opened_under: &[String]) -> Result<bool, Signal> {
        match self.access_by_permission_hidden_decided(guard, opened_under) {
            Ok(true) if self.shadow_active => {
                self.shadow_note(
                    "would deny",
                    "accessbypermission",
                    &format!("`{}` would leave the control off the page", guard.trim()),
                );
                Ok(false)
            }
            Err(e) if self.shadow_active => {
                self.shadow_signal("accessbypermission", &e);
                Ok(false)
            }
            r => r,
        }
    }

    pub(crate) fn effective_permission_levels(&self, kind: i32, id: u32) -> Result<Option<Levels>, Signal> {
        match self.effective_permission_levels_decided(kind, id) {
            Ok(Some(l)) if self.shadow_active => {
                self.shadow_note(
                    "unknown",
                    "effective permission",
                    &format!("kind {kind} id {id} would answer {l:?}"),
                );
                Ok(None)
            }
            Err(e) if self.shadow_active => {
                self.shadow_signal("effective permission", &e);
                Ok(None)
            }
            r => r,
        }
    }
}
