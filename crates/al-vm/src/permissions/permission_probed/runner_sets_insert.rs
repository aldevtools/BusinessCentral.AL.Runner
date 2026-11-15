//! The one Insert the permission sets BC's test runner assigns are measured to deny (al-runner issue 8160).
//!
//! With Tests-TestLibraries installed, Microsoft's test runner runs codeunit 130302 "Set State Test Run" before each test
//! method whose TestPermissions is not Disabled, and it assigns its permission sets through Permissions Mock
//! (`al-testrun` `runner_reset.rs`). Those sets are not enforced (assumption `runner-permission-sets-grant-all`), except
//! in the shape corpus al-language-testlibraries/test-framework/testpermissions pins: a test method calls a procedure of
//! a codeunit of its own app, which inserts a row into an empty table of that app with `Insert()`
//! (BiTpHelper.al:56-63), and BC raises `Sorry, the current permissions prevented the action. (TableData <id> <name>
//! Insert: <app>)` with code DB:ClientInsertDenied, for the codeunit property Restrictive, NonRestrictive or none, and for
//! a method attribute Restrictive or NonRestrictive under codeunit Disabled (BiTpDefault.al:35-45, BiTpRestr.al:36-46,
//! BiTpNonRestr.al:36-46, BiTpDisMRestr.al:38-48, BiTpDisMNonRestr.al:38-48; BC 27, 28 and 29). Every other access keeps
//! main's answer.
use crate::Session;
use crate::permissions::{Access, Op, SetModel, Verdict};
use al_builtins::native::Signal;

/// The roles the runner's step assigned in the measured cells, in the order the step assigns them: Permissions Mock's
/// `Set('D365 Full Access')` (All Objects, then the set) and the `Assign` calls of Library - Lower Permissions (BiTpHelper.al:6-10).
const MEASURED_ROLES: &[&str] = &["ALL OBJECTS", "D365 FULL ACCESS", "TEST TABLES", "D365 BASIC", "LOCAL"];

impl Session {
    /// AL code's `Insert()` on a Record variable while the session holds only the runner's sets: BC's denial in the
    /// measured shape, else nothing (main's answer).
    pub(crate) fn runner_sets_insert(&mut self, rec: &crate::records::Rec) -> Result<(), Signal> {
        let Some((table, id, by)) = self.runner_sets_insert_cell(rec)? else {
            return Ok(());
        };
        let Some(eff) = SetModel::of(&self.engine).compose(&self.shadow_roles) else {
            return Ok(());
        };
        if !eff.complete {
            return Ok(());
        }
        let access = Access {
            kind: 0,
            id,
            op: Op::Insert,
        };
        self.permissions = Some(eff);
        let verdict = self.permission_verdict(access, Some(table), false);
        self.permissions = None;
        match verdict {
            Verdict::Deny { indirect: false, by: b } if b == by => {
                Err(self.denial_error(access, "Insert", "Insert", by))
            }
            _ => Ok(()),
        }
    }

    /// The measured cell, every fact by equality: (table object, table id, inserting codeunit), or `None`.
    fn runner_sets_insert_cell(&mut self, rec: &crate::records::Rec) -> Result<Option<(usize, u32, usize)>, Signal> {
        // Only the runner's sets are assigned: no set of the test's own, no switch that changes the check.
        if !self.runner_permission_sets
            || self.permissions.is_some()
            || !self.permission_roles.is_empty()
            || self.ignore_permission_mock
            || self.permissions_off
            || self.engine.license_caps()
            || self
                .shadow_roles
                .iter()
                .map(String::as_str)
                .ne(MEASURED_ROLES.iter().copied())
        {
            return Ok(None);
        }
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table as usize, r.temporary)
        };
        if temporary {
            return Ok(None);
        }
        // The test method of the running test codeunit calls a procedure of a codeunit of the same app, which inserts
        // (BiTpDefault.al:41 -> BiTpHelper.al:62).
        let [test, by] = self.frames[..] else {
            return Ok(None);
        };
        let engine = self.engine.clone();
        let heads = engine.program.objects.heads();
        let codeunit = |o: usize| heads[o].kind == al_ir::ObjKind::Codeunit;
        let subtype_test = |o: usize| {
            heads[o]
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
        };
        let declares = |o: usize, name: &str| heads[o].properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
        let app = heads[table].app;
        if self.test_object != Some(test)
            || !codeunit(test)
            || !subtype_test(test)
            || !codeunit(by)
            || subtype_test(by)
            || heads[test].app != app
            || heads[by].app != app
            || declares(by, "Permissions")
            || declares(table, "InherentPermissions")
            || declares(table, "InherentEntitlements")
            || !self.table_runs_no_trigger(table)
        {
            return Ok(None);
        }
        // The table holds no row (BiTpHelper.al:56-63: every variant's insert is rolled back by its asserterror).
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let rows = al_data::Store::count(&*self.store_for(rec)?, &cx, &t, &[])
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
        if rows != 0 {
            return Ok(None);
        }
        Ok(heads[table].id.map(|id| (table, id, by)))
    }
}
