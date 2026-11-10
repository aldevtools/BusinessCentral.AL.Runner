//! `NavApp.LoadPackageData` in install code: the rows of an app's table data file (`reader.rs`) are inserted into the table.
//! Pinned by tests/al-language/install-upgrade/install-triggers/Y85PackageData.al (fixtures tests/al-language-xapp-a/
//! Y85XaPackageData.al, Y8XaInstallOrder.al, corpus writer Y85, al-runner issue 6649):
//! - the rows land in the company of the per-company install part, or in the shared table of the per-database part, with the
//!   SystemId of the file; no table trigger runs (Y85_PerCompany_RowsAndNoTrigger);
//! - the stored fields are listed primary key fields first, then the others in declaration order, without FlowFields,
//!   FlowFilters and disabled fields (Y85_FieldOrder_KeyFieldsFirstThenDeclarationOrder);
//! - an AutoIncrement key keeps the file's values and continues after them (Y85_AutoIncrement_ContinuesAfterLoadedRows);
//! - a table without a data file in the app loads nothing (metadata/Y64InstallLeads.al:81-96);
//! - no table event is raised (Y88PackageData.al, Y88_Events_LoadRaisesNoTableEvent); an Option or Enum ordinal the field
//!   does not define, the last Date and the file's Decimal 1E-28 (as 0) load (Y88_Option_UndefinedOrdinal,
//!   Y88_Enum_UndefinedOrdinal, Y88_Date_LastDay, Y88_Decimal_OutOfRange; `reader.rs`).
//!
//! - the install code of an app that is only a dependency of the run (AL source, `--dep`) loads the same way: the fixture
//!   app A of Y85 is a dependency of the test app (Y85PackageData.al:3-6).
//!
//! Refused: the data files of a compiled package (where a package keeps them is not measured), a table of another app, a table with a table extension of another app or a field with `ObsoleteState = Removed`
//! (the file layout of those is not pinned), a table whose database insert trigger is on (whether the load raises
//! OnDatabaseInsert is not pinned), a per-company table loaded in the per-database part (BC fails the publish, the error
//! text is not pinned: install-upgrade/install-triggers/PkgDataParts.al:5), a row whose key exists, and every file content
//! `reader.rs` refuses. A shared table loads from the per-company part too (PkgDataParts.al:40-42).

mod reader;

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};
use reader::Column;

const SYSTEM_FIELD_BASE: u32 = 2_000_000_000;
const SYSTEM_ID: u32 = 2_000_000_000;

fn refuse<T>(what: impl Into<al_builtins::refusal::Refusal>) -> Result<T, Signal> {
    Err(Signal::unsupported(what))
}

impl Session {
    /// `NavApp.LoadPackageData(table)` in install code.
    pub(crate) fn load_package_data(&mut self, table: i32) -> Result<Value, Signal> {
        let what = format!("NavApp.LoadPackageData of table {table}");
        let Some(object) = u32::try_from(table).ok().and_then(|n| self.compiled_table(n)) else {
            return refuse(format!("{what}: a table outside the workspace (not pinned)"));
        };
        let objects = &self.engine.program.objects;
        let app = objects[object].app as usize;
        if self.frames.last().is_none_or(|&o| objects[o].app as usize != app) {
            return refuse(format!(
                "{what}: a table of another app than the running code (not pinned)"
            ));
        }
        // A dependency app (AL source passed with `--dep`) loads its data files as an app under test does: the install
        // codeunit of the corpus fixture app A, which the test app depends on, loads them
        // (install-upgrade/install-triggers/Y85PackageData.al:3-6, al-runner issue 6649). A compiled package's files are
        // not read (`package_data_files` stays empty for it).
        let info = &self.engine.program.apps[app];
        if info.package_data && info.package_data_files.is_empty() {
            return refuse(format!("{what}: package data of a package is not read (not pinned)"));
        }
        let mut files = info
            .package_data_files
            .iter()
            .filter(|(_, bytes)| reader::header_table(bytes) == Some(table as u32));
        let Some((name, bytes)) = files.next() else {
            return Ok(Value::Null);
        };
        if files.next().is_some() {
            return refuse(format!("{what}: several data files for the table (not pinned)"));
        }
        let (name, bytes) = (name.clone(), bytes.clone());
        let Some(def) = self.engine.tables[object].clone() else {
            return refuse(format!("{what}: a table without a schema"));
        };
        // A shared table loads from the per-company part as well: its rows land in the shared table
        // (install-upgrade/install-triggers/PkgDataParts.al:40-42, LoadPackageData_SharedTableInCompanyPart). A per-company
        // table loaded from the per-database part fails the publish of the whole app (PkgDataParts.al:5,
        // tests/al-language-xapp-a/PkgDataPartsFixture.al:4); the error text is not asserted by the corpus, so it is refused.
        if def.per_company && self.cx.company.is_empty() {
            return refuse(format!(
                "{what}: a per-company table in the per-database part of the install (the platform fails the publish \
                 of the app; the error text is not pinned)"
            ));
        }
        self.refuse_unpinned_table(object, &what)?;
        let positions = self.package_columns(&def);
        let columns = self.columns_of(&def, &positions)?;
        let rows = reader::read(&bytes, table as u32, &columns)
            .map_err(|e| Signal::unsupported(format!("{what} ({name}): {e}")))?;
        // Whether the load raises the database trigger events (Global Triggers OnDatabaseInsert) is not pinned.
        if !rows.is_empty() && self.global_enabled(&def, crate::globaltriggers::GlobalOp::Insert)? {
            return refuse(format!(
                "{what}: a table whose database insert trigger is on (whether the load raises it is not pinned)"
            ));
        }
        if rows
            .iter()
            .flat_map(|r| &r.values)
            .any(|v| matches!(v, Value::DateFormula(f) if f.starts_with("<<")))
        {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PackageDataDateformulaBrackets);
        }
        for row in rows {
            let Value::Record(rec) = self.default_value(&RtType::Record(object as u32, false)) else {
                return refuse(format!("{what}: a record buffer"));
            };
            {
                let mut r = rec.borrow_mut();
                for (&p, v) in positions.iter().zip(row.values) {
                    r.fields[p] = v;
                }
                if let Some(i) = def.fields.iter().position(|f| f.id == SYSTEM_ID) {
                    r.fields[i] = Value::Guid(row.system_id);
                }
            }
            let used = std::mem::replace(&mut self.ret_used, true);
            let loading = std::mem::replace(&mut self.package_load, true);
            let done = self.record_op(
                RecordOp::Insert,
                &rec,
                &mut [Value::Boolean(false), Value::Boolean(true)],
            );
            self.package_load = loading;
            self.ret_used = used;
            match done {
                Ok(Value::Boolean(true)) => {}
                Ok(_) | Err(Signal::Error(_)) => {
                    return refuse(format!(
                        "{what}: a row whose key or SystemId exists (the platform fails the install)"
                    ));
                }
                Err(e) => return Err(e),
            }
        }
        Ok(Value::Null)
    }

    /// The field positions in file order: the primary key fields, then the other stored enabled fields in declaration order.
    fn package_columns(&self, t: &al_data::TableDef) -> Vec<usize> {
        let stored = |i: usize| t.fields[i].id < SYSTEM_FIELD_BASE && t.fields[i].stored && self.field_enabled(t, i);
        let mut out: Vec<usize> = t.keys[0].iter().copied().filter(|&i| stored(i)).collect();
        out.extend((0..t.fields.len()).filter(|&i| stored(i) && !t.keys[0].contains(&i)));
        out
    }

    fn columns_of(&self, t: &al_data::TableDef, positions: &[usize]) -> Result<Vec<Column>, Signal> {
        positions
            .iter()
            .map(|&p| {
                let f = &t.fields[p];
                let uncompressed_blob = self
                    .field_property(t, p, "Compressed")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("false"));
                let key = t.keys[0].contains(&p);
                Ok(Column {
                    name: f.name.clone(),
                    ty: f.ty.clone(),
                    uncompressed_blob,
                    key,
                })
            })
            .collect()
    }

    fn refuse_unpinned_table(&self, object: usize, what: &str) -> Result<(), Signal> {
        let objects = &self.engine.program.objects;
        let app = objects[object].app;
        if objects[object]
            .table
            .as_ref()
            .is_some_and(|t| t.extensions.iter().any(|&e| objects[e].app != app))
        {
            return refuse(format!(
                "{what}: a table with a table extension of another app (not pinned)"
            ));
        }
        let def = self.engine.tables[object].as_ref().expect("checked");
        if (0..def.fields.len()).any(|i| self.field_removed(def, i)) {
            return refuse(format!("{what}: a table with a removed field (not pinned)"));
        }
        Ok(())
    }
}
