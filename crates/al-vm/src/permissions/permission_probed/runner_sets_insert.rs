//! The one Insert the permission sets BC's test runner assigns are measured to deny (al-runner issue 8160).
//!
//! With Tests-TestLibraries installed, Microsoft's test runner runs codeunit 130302 "Set State Test Run" before each test
//! method whose TestPermissions is not Disabled, and it assigns its permission sets through Permissions Mock
//! (`al-testrun` `runner_reset.rs`). Those sets are not enforced (assumption `runner-permission-sets-grant-all`), except
//! in the shape corpus al-language-testlibraries/test-framework/testpermissions pins: a test method calls a procedure of
//! a codeunit of its own app, which inserts a row into an empty table of that app with the statement `Insert();`
//! (BiTpHelper.al:56-63), and BC raises `Sorry, the current permissions prevented the action. (TableData <id> <name>
//! Insert: <app>)` with code DB:ClientInsertDenied, for the codeunit property Restrictive, NonRestrictive or none, and for
//! a method attribute Restrictive or NonRestrictive under codeunit Disabled (BiTpDefault.al:35-45, BiTpRestr.al:36-46,
//! BiTpNonRestr.al:36-46, BiTpDisMRestr.al:38-48, BiTpDisMNonRestr.al:38-48; BC 27, 28 and 29), and, under codeunit
//! Restrictive with no method attribute, also when the inserting procedure has the attribute `[InherentPermissions]` `'R'` or `'I'` with scope
//! Entitlements (BiInherentPerms.al:202-213, :232-243). Every other access keeps main's answer.
//!
//! The one access the test runner's permission sets (Tests-TestLibraries codeunit 130302) deny: a
//! procedure of a codeunit of the test's app inserts with `Insert()` into an empty table of that app,
//! with no `[InherentPermissions]` or one of the two measured to deny (al-language-
//! testlibraries/test-framework/testpermissions/BiTp*.al, BiInherentPerms.al); every other access
//! under those sets keeps assumption `runner-permission-sets-grant-all`. Called from the Record
//! branch of `ctx.rs`. Entry points: `Session::runner_sets_insert`.
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
        let access = |op| Access { kind: 0, id, op };
        // No assigned set names the table: its level is blank for every table data permission (BiTpHelper.al:11,
        // BiInherentPerms.al:6-7).
        let blank = [Op::Read, Op::Insert, Op::Modify, Op::Delete]
            .into_iter()
            .all(|op| eff.level(access(op)) == 0);
        if !(eff.complete && blank) {
            return Ok(());
        }
        let access = access(Op::Insert);
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
        let runner_sets_only = self.runner_permission_sets
            && self.permissions.is_none()
            && self.permission_roles.is_empty()
            && !self.ignore_permission_mock
            && !self.permissions_off
            && !self.engine.license_caps()
            && self
                .shadow_roles
                .iter()
                .map(String::as_str)
                .eq(MEASURED_ROLES.iter().copied());
        // The statement form `Own.Insert();` on a Record variable that is not temporary (BiTpHelper.al:62).
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table as usize, r.temporary)
        };
        let statement = !self.ret_used && !temporary;
        // The test method of the running test codeunit calls a procedure of a codeunit of the same app, which inserts
        // (BiTpDefault.al:41 -> BiTpHelper.al:62).
        let (&[test, by], true) = (&self.frames[..], runner_sets_only && statement) else {
            return Ok(None);
        };
        let engine = self.engine.clone();
        let heads = engine.program.objects.heads();
        let codeunit = |o: usize| heads[o].kind == al_ir::ObjKind::Codeunit;
        let app = heads[table].app;
        // The declared properties: on the test codeunit Subtype = Test and at most TestPermissions (BiTpDefault.al:10,
        // BiTpRestr.al:10-11); none on the inserting codeunit (BiTpHelper.al:31, BiInherentPerms.al:38); on the table at most
        // DataClassification = SystemMetadata (BiTpHelper.al:18, BiInherentPerms.al:18).
        let properties = |o: usize, measured: &dyn Fn(&str, &str) -> bool| {
            heads[o].properties.iter().all(|(k, v)| measured(k, v.trim()))
        };
        // On BC 29 the lowering adds the default ITestHandler codeunits of the app set to each test codeunit, as it does to
        // the measured ones (a `D<id>` list). A handler that assigns sets shows in the roles checked above.
        let test_properties = |k: &str, v: &str| {
            (k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
                || k.eq_ignore_ascii_case("TestPermissions")
                || (k == al_ir::TEST_HANDLER_CODEUNITS && v.split(',').all(|h| h.starts_with('D')))
        };
        let table_properties =
            |k: &str, v: &str| k.eq_ignore_ascii_case("DataClassification") && v.eq_ignore_ascii_case("SystemMetadata");
        let measured = self.test_object == Some(test)
            && codeunit(test)
            && codeunit(by)
            && heads[test].app == app
            && heads[by].app == app
            && heads[test]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Subtype"))
            && properties(test, &test_properties)
            && heads[by].properties.is_empty()
            && properties(table, &table_properties)
            && self.table_runs_no_trigger(table)
            && self.runner_sets_insert_routines(test, by, &heads[table].name);
        if !measured {
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

    /// The two routines on the stack in the measured cells. The test method: a procedure with `[Test]`, whose attributes
    /// are among `[Test]`, `[HandlerFunctions]` and `[TestPermissions]` (BiTpDefault.al:33-34, BiTpDisMRestr.al:35-37), and
    /// whose (codeunit TestPermissions, method TestPermissions) pair is one of the five measured cells. The inserting
    /// routine: a procedure with no attribute (BiTpHelper.al:56), or with exactly one of the two `[InherentPermissions]`
    /// attributes whose denial is measured, under codeunit Restrictive with no method attribute: `'R'`
    /// (BiInherentPerms.al:64-72, :202-213) or `'I'` with scope Entitlements (:84-92, :232-243), both on the inserted
    /// table. The letters are compared as written; keywords and the table name in any case.
    fn runner_sets_insert_routines(&self, test: usize, by: usize, table: &str) -> bool {
        let [method, helper] = &self.call_stack[..] else {
            return false;
        };
        let objects = &self.engine.program.objects;
        let (Some(method), Some(helper)) = (
            objects[test].routines.get(method.routine),
            objects[by].routines.get(helper.routine),
        ) else {
            return false;
        };
        let named = |a: &al_ir::Attribute, n: &str| a.name.eq_ignore_ascii_case(n);
        let test_method = !method.is_trigger
            && method.attributes.iter().filter(|a| named(a, "Test")).count() == 1
            && method
                .attributes
                .iter()
                .all(|a| TEST_METHOD_ATTRIBUTES.iter().any(|n| named(a, n)));
        if !(test_method && !helper.is_trigger) {
            return false;
        }
        let codeunit_value = objects.heads()[test]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("TestPermissions"))
            .map(|(_, v)| v.trim().to_ascii_lowercase());
        let method_values: Vec<String> = method
            .attributes
            .iter()
            .filter(|a| named(a, "TestPermissions"))
            .map(|a| fold_attribute(&a.text))
            .collect();
        let method_value = match &method_values[..] {
            [] => None,
            [one] => Some(
                one.strip_prefix("[testpermissions(testpermissions::")
                    .and_then(|v| v.strip_suffix(")]"))
                    .unwrap_or("")
                    .to_string(),
            ),
            _ => Some(String::new()),
        };
        let cell = (codeunit_value.as_deref(), method_value.as_deref());
        let on_table = format!(
            "[inherentpermissions(permissionobjecttype::tabledata,database::\"{}\",",
            table.to_lowercase()
        );
        let inherent = [
            format!("{on_table}'R')]"),
            format!("{on_table}'I',inherentpermissionsscope::entitlements)]"),
        ];
        match &helper.attributes[..] {
            [] => MEASURED_CELLS.contains(&cell),
            [one] => {
                named(one, "InherentPermissions")
                    && inherent.contains(&fold_attribute(&one.text))
                    && cell == RESTRICTIVE
            }
            _ => false,
        }
    }
}

/// The attributes of the measured test methods (BiTpDefault.al:33-34, BiTpDisMRestr.al:35-37).
const TEST_METHOD_ATTRIBUTES: &[&str] = &["Test", "HandlerFunctions", "TestPermissions"];

/// Codeunit TestPermissions = Restrictive with no method attribute (BiTpRestr.al:11, BiInherentPerms.al:168).
const RESTRICTIVE: (Option<&str>, Option<&str>) = (Some("restrictive"), None);

/// The measured (codeunit TestPermissions, method `[TestPermissions]`) pairs, values in lower case: none and none
/// (BiTpDefault.al:35-45), Restrictive and none (BiTpRestr.al:36-46), NonRestrictive and none (BiTpNonRestr.al:36-46),
/// Disabled and Restrictive (BiTpDisMRestr.al:38-48), Disabled and NonRestrictive (BiTpDisMNonRestr.al:38-48).
const MEASURED_CELLS: &[(Option<&str>, Option<&str>)] = &[
    (None, None),
    RESTRICTIVE,
    (Some("nonrestrictive"), None),
    (Some("disabled"), Some("restrictive")),
    (Some("disabled"), Some("nonrestrictive")),
];

/// An attribute as written, with the keywords and quoted names in lower case and the whitespace outside quotes removed;
/// the text between single quotes (the permission letters) stays as written.
fn fold_attribute(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<char> = None;
    for c in text.chars() {
        match quote {
            Some('\'') => {
                out.push(c);
                if c == '\'' {
                    quote = None;
                }
            }
            Some(_) => {
                out.extend(c.to_lowercase());
                if c == '"' {
                    quote = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                }
                if !c.is_whitespace() {
                    out.extend(c.to_lowercase());
                }
            }
        }
    }
    out
}
