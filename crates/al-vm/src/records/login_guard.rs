//! The guard of a test session that runs without the login (al-runner issues 7343, 7574, 7605 and 7717; hard rule 4).
//!
//! BC raises the login before the first test of every test session, so the rows an app under test's login subscriber
//! adds are there (events/platform-triggers/LgnLoginChecklistTests.al:137-138). The runner raises it only for a test
//! codeunit whose static reach says it can see those rows (`al-testrun` `login.rs`), because Microsoft's login code
//! cannot run on every database. That rule is a guess about what the test will read. Where it says no, the front end
//! hands over the tables the login subscribers add rows to, in two sets:
//! - The tables the test's own calls cannot insert into (`guard_login_tables`): every read of one of them by the test
//!   session is refused, so a wrong "no" costs a test that is not run there.
//! - The tables the test's own calls can insert into (`guard_login_rows`), with the rows the app under test's login
//!   subscribers add to them when they run alone, in a copy of the session that is thrown away
//!   (`login_rows_alone`). An operation on one of these tables, read or write, is refused when one of those rows is
//!   among the rows it addresses: its primary key (Get, Insert, Modify, Delete, Rename), its SystemId
//!   (GetBySystemId), or its filters (every other read, ModifyAll, DeleteAll; no filter addresses every row). Any other
//!   operation runs: its answer is the same with the login's rows. When the subscribers did not run to their end alone,
//!   every operation on these tables is refused. Unproven: that the subscribers add the same rows alone as inside the
//!   login (the copy has fresh SingleInstance codeunits and runs them in the order the front end gives).
//!
//! A read is checked where rows are read: a record method (`Session::record_op`), a scan (`scan_plain`, the reads of
//! `each_row_of`: queries, FlowField sums, XMLport export, `FieldRef.CalcSum`), and the reads that count or get rows
//! from the store directly (a FlowField, the TableRelation check, the list caption, `GetBySystemId`).
//!
//! Known limits: a write of a table of the first set is let through (Microsoft's event code may insert at run time), and
//! the app's own Company Triggers subscribers run before the guard is set.

use super::*;
use crate::refusal::keys;
use al_builtins::refuse;
use std::collections::{BTreeMap, BTreeSet};

/// The rows of each table (object index) the login subscribers add, when they ran alone.
pub type LoginRows = BTreeMap<u32, Vec<Vec<Value>>>;

/// What decides whether a read of a table is refused before the session's own code wrote it.
#[derive(Default)]
pub(crate) struct ReadGuards {
    /// The `Engine::demo_tables` the running test's own code (not install code) wrote to (`record_op_inner`).
    pub(crate) demo_written: std::collections::HashSet<u32>,
    /// The tables a read of which is refused because the session runs without the login.
    pub(crate) login: BTreeSet<u32>,
    /// The tables the login adds rows to and the test's own calls can insert into: an operation that addresses one of
    /// the login's rows (`login_rows`) is refused.
    pub(crate) login_assumed: BTreeSet<u32>,
    /// The rows the login subscribers added to the tables of `login_assumed` when they ran alone.
    pub(crate) login_rows: LoginRows,
    /// Why the login subscribers did not run to their end alone: every operation on `login_assumed` is refused.
    pub(crate) login_rows_unknown: Option<String>,
}

/// The rows an operation addresses ([`Session::login_guard_table`]).
pub(crate) enum Addressed<'a> {
    /// The rows that pass these filters (stored fields only; no filter: every row).
    Filters(&'a [FieldFilter]),
    /// The rows whose primary key starts with these values; `None` when the key is not known (every row).
    Key(Option<Vec<Value>>),
    /// The row with this SystemId.
    SystemId(&'a Value),
}

impl Session {
    /// The session runs without the login: a read of one of `tables` (table object indexes) is refused from now on.
    /// An empty set (the login was raised, or no app under test subscribes to it) guards nothing.
    pub fn guard_login_tables(&mut self, tables: BTreeSet<u32>) {
        self.guards.login = tables;
    }

    /// The session runs without the login: an operation on one of `tables` (table object indexes), which the test's own
    /// calls can insert into too, is refused from now on when it addresses one of `rows` (from
    /// [`Session::login_rows_alone`]); with `Err(why)`, every operation on them is refused.
    pub fn guard_login_rows(&mut self, tables: BTreeSet<u32>, rows: Result<LoginRows, String>) {
        self.guards.login_assumed = tables;
        match rows {
            Ok(rows) => self.guards.login_rows = rows,
            Err(why) => self.guards.login_rows_unknown = Some(why),
        }
    }

    /// Runs the parameterless login subscribers `subscribers` (object, routine), in this order, alone in a copy of this
    /// session that is thrown away, and answers the rows they added to or changed in each of `tables`. This session is
    /// not changed, apart from a commit. `Err` names the subscriber that did not run to its end.
    pub fn login_rows_alone(
        &mut self,
        subscribers: &[(usize, usize)],
        tables: &BTreeSet<u32>,
    ) -> Result<LoginRows, String> {
        let base = self.capture_baseline();
        let mut copy = Session::with_host(self.engine.clone(), al_host::Host::default());
        copy.restore_baseline(&base);
        for &(o, r) in subscribers {
            if let Err(e) = copy.call_parameterless_subscriber(o, r) {
                let obj = &self.engine.program.objects[o];
                let why = match e {
                    Signal::Unsupported(why) => why.to_string(),
                    Signal::Error(e) => e.message,
                    _ => "it stopped".into(),
                };
                return Err(format!(
                    "the login subscriber {}.{} did not run to its end alone: {why}",
                    obj.name, obj.routines[r].name
                ));
            }
        }
        copy.commit_transaction();
        let mut out = LoginRows::new();
        for &t in tables {
            let Some(def) = self.engine.tables.get(t as usize).cloned().flatten() else {
                continue;
            };
            let before = self.store.stored_rows(&self.cx, &def);
            let added: Vec<Vec<Value>> = copy
                .store
                .stored_rows(&copy.cx, &def)
                .into_iter()
                .filter(|row| !before.iter().any(|b| same_row(&def, b, row)))
                .collect();
            if !added.is_empty() {
                out.insert(t, added);
            }
        }
        Ok(out)
    }

    fn login_table_name(&self, t: u32) -> &str {
        self.engine
            .program
            .objects
            .get(t as usize)
            .map_or("?", |o| o.name.as_str())
    }

    /// A record method on a table the login would add rows to ([`Session::login_guard_table`]). Writes of a guarded table
    /// are let through: what a write needs it reads through the store, and Microsoft's event code may insert at run time.
    /// A temporary record reads no row of the table. GetBySystemId is checked where it reads (`get_by_system_id`).
    pub(crate) fn login_guard_read(&self, method: RecordOp, rec: &Rec, args: &[Value]) -> Result<(), Signal> {
        if self.guards.login.is_empty() && self.guards.login_assumed.is_empty() {
            return Ok(());
        }
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        if temporary {
            return Ok(());
        }
        let assumed = self.guards.login_assumed.contains(&table);
        if !assumed && !self.guards.login.contains(&table) {
            return Ok(());
        }
        let by_key = |s: &Session| {
            let def = s.engine.tables.get(table as usize).cloned().flatten()?;
            Some(def.primary_key(&rec.borrow().fields))
        };
        let addressed = match method {
            RecordOp::Get => Addressed::Key(self.get_key(table, args)),
            RecordOp::Find
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::Next
            | RecordOp::IsEmpty
            | RecordOp::Count
            | RecordOp::CountApprox
            | RecordOp::CalcSums => {
                let (filters, _) = self.split_filters(rec);
                return self.login_guard_table(table, method.name(), Addressed::Filters(&filters));
            }
            RecordOp::ModifyAll | RecordOp::DeleteAll if assumed => {
                let (filters, _) = self.split_filters(rec);
                return self.login_guard_table(table, method.name(), Addressed::Filters(&filters));
            }
            RecordOp::Insert | RecordOp::Modify | RecordOp::Delete if assumed => Addressed::Key(by_key(self)),
            RecordOp::Rename if assumed => {
                self.login_guard_table(table, method.name(), Addressed::Key(by_key(self)))?;
                Addressed::Key(self.get_key(table, args))
            }
            _ => return Ok(()),
        };
        self.login_guard_table(table, method.name(), addressed)
    }

    /// The primary key values `args` of a Get or a Rename name on table `table`, converted to the key's types; `None`
    /// when they cannot be converted.
    fn get_key(&self, table: u32, args: &[Value]) -> Option<Vec<Value>> {
        if let [Value::Library(_, l)] = args {
            if let al_builtins::value::LibraryValue::RecordId { key, .. } = &*l.borrow() {
                return Some(key.clone());
            }
        }
        let def = self.engine.tables.get(table as usize).cloned().flatten()?;
        let pk = def.keys.first()?;
        if args.len() > pk.len() {
            return None;
        }
        // The conversion may mark an assumption; the Get itself marks it again where it converts.
        let marks = al_builtins::assumptions::take_hit_bits();
        let key = args
            .iter()
            .zip(pk)
            .map(|(a, &f)| ops::convert(a.clone(), &def.fields[f].ty).ok())
            .collect();
        al_builtins::assumptions::add_hit_bits(marks);
        key
    }

    /// A read of the stored rows of table `table` (an object index) through a scan with `filters` (`scan_plain`,
    /// `each_row_of`); a temporary record reads no row of the table.
    pub(crate) fn login_guard_scan(&self, table: u32, temporary: bool, filters: &[FieldFilter]) -> Result<(), Signal> {
        if temporary || (self.guards.login.is_empty() && self.guards.login_assumed.is_empty()) {
            return Ok(());
        }
        self.login_guard_table(table, "reading rows", Addressed::Filters(filters))
    }

    /// An operation by `what` on the stored rows `addressed` of table `table` (an object index): refused when the session
    /// runs without the login and a login subscriber adds rows to the table that the test's own calls cannot insert, or
    /// when the test's own calls can insert into it and one of the rows the login adds is among those addressed. Install
    /// code runs before the guard is set. The caller passes no temporary record.
    pub(crate) fn login_guard_table(&self, table: u32, what: &str, addressed: Addressed) -> Result<(), Signal> {
        if self.install_scope {
            return Ok(());
        }
        if self.guards.login.contains(&table) {
            return unsupported(refuse!(
                keys::RUNNER_LOGIN_GUARDED_TABLE_READ,
                "{} on {}: the login was not raised for this test codeunit, and a login subscriber of an app under test adds rows to this table that are not there (the test's reach did not show that it reads them)",
                what,
                self.login_table_name(table)
            ));
        }
        if !self.guards.login_assumed.contains(&table) {
            return Ok(());
        }
        if let Some(why) = &self.guards.login_rows_unknown {
            return unsupported(refuse!(
                keys::RUNNER_LOGIN_ROWS_UNKNOWN,
                "{} on {}: the login was not raised for this test codeunit, a login subscriber of an app under test adds rows to this table, and which rows is not known ({})",
                what,
                self.login_table_name(table),
                why.as_str()
            ));
        }
        let Some(rows) = self.guards.login_rows.get(&table) else {
            return Ok(());
        };
        let hit = match &addressed {
            // A filter on a field that is not stored, and one whose matching is not pinned, count as filters that pass
            // the row.
            Addressed::Filters(filters) => {
                let fields = self.engine.program.objects[table as usize]
                    .table
                    .as_ref()
                    .map(|t| &t.fields);
                let stored: Vec<FieldFilter> = filters
                    .iter()
                    .filter(|f| fields.is_none_or(|fs| fs.get(f.field).is_some_and(|x| x.stored)))
                    .cloned()
                    .collect();
                rows.iter()
                    .any(|row| al_builtins::filter::row_passes(row, &stored).unwrap_or(true))
            }
            Addressed::Key(None) => true,
            Addressed::Key(Some(key)) => {
                let Some(def) = self.engine.tables.get(table as usize).cloned().flatten() else {
                    return Ok(());
                };
                rows.iter().any(|row| {
                    let pk = def.primary_key(row);
                    key.len() <= pk.len() && ops::compare_keys(&pk[..key.len()], key)
                })
            }
            Addressed::SystemId(id) => {
                let Some(def) = self.engine.tables.get(table as usize).cloned().flatten() else {
                    return Ok(());
                };
                let at = def.fields.iter().position(|f| f.id == 2_000_000_000);
                rows.iter()
                    .any(|row| at.is_none_or(|i| ops::compare_keys(&row[i..=i], std::slice::from_ref(*id))))
            }
        };
        if !hit {
            return Ok(());
        }
        unsupported(refuse!(
            keys::RUNNER_LOGIN_ROW_ADDRESSED,
            "{} on {}: the login was not raised for this test codeunit, and the operation addresses a row a login subscriber of an app under test adds to this table, which is not there",
            what,
            self.login_table_name(table)
        ))
    }
}

/// Whether rows `a` and `b` of table `t` hold the same values: the same primary key and no field that compares as
/// different (a field whose values cannot be compared, a BLOB, counts as the same).
fn same_row(t: &TableDef, a: &[Value], b: &[Value]) -> bool {
    ops::compare_keys(&t.primary_key(a), &t.primary_key(b))
        && a.iter()
            .zip(b)
            .all(|(x, y)| !matches!(ops::compare(x, y), Ok(o) if o != std::cmp::Ordering::Equal))
}
