//! A `TableType = CRM` table through the platform's CRM test connection (`@@test@@`, the only connection string the
//! runtime registers, ADR-027 section 3 row 15). BC serves the table through the session's default CRM table connection,
//! never through SQL; each registered connection name has rows of its own (corpus
//! tests/al-language-onprem/session/Y16CrmTableConnectionOps.al:143-318, BC 27.5/28.5/29.0 OnPrem):
//!
//! - FindSet runs in primary key order, SetRange, SetFilter, Reset, Ascending(false), FindFirst, FindLast and IsEmpty act as
//!   on any table (:154-182);
//! - Modify, Delete and Rename change the rows (:184-222); a missing key raises `DB:RecordNotFound` and a duplicate Insert
//!   `DB:RecordExists` with the table's usual texts (:224-251);
//! - a registered connection that is not the default is not used: the "must be registered" error (:253-279);
//! - UnregisterTableConnection drops the connection's rows, and a second connection starts empty (:281-310);
//! - Insert gives a null Guid primary key a fresh Guid and keeps a preset one (session/TestDatabaseTableConnectionCrm.al:128-170).
//!
//! What is not pinned is refused: a method that reads or writes the rows other than those above, a table or table extension
//! trigger, a table event subscriber or a database trigger on a write, a Rename of a key other fields relate to, a primary key
//! that is not one Guid field, and any read of a connection's rows after a rollback that may have had to take back a write.

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::TableDef;
use al_data::memory::MemStore;

/// The ordinal of `TableConnectionType::CRM` (misc/W5TableConnections.al:117-136).
pub(crate) const CRM_KIND: i32 = 0;

/// The rows of one registered CRM table connection.
#[derive(Default)]
pub(crate) struct CrmStore {
    pub(crate) store: MemStore,
    /// A write since the last commit.
    uncommitted: bool,
    /// A write since the connection was registered.
    written: bool,
    /// A rollback came after a write: whether it took the write back is not pinned.
    after_rollback: bool,
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

const MUST_REGISTER: &str = "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.";

impl Session {
    /// The lower-case name of the default CRM table connection, if one is set.
    fn crm_default(&self) -> Option<String> {
        self.default_table_connection.get(&CRM_KIND).map(|n| n.to_lowercase())
    }

    /// `RegisterTableConnection(CRM, name, '@@test@@')`: the connection starts with no rows (Y16CrmTableConnectionOps.al:281-310,
    /// `count on second connection` 0).
    pub(crate) fn crm_registered(&mut self, name: &str) {
        self.crm_stores.insert(name.to_string(), CrmStore::default());
    }

    /// `UnregisterTableConnection(CRM, name)` drops its rows (Y16CrmTableConnectionOps.al:281-310, `count after re-register` 0).
    pub(crate) fn crm_unregistered(&mut self, name: &str) {
        self.crm_stores.remove(name);
    }

    /// The lower-case name of the connection a record of a CRM table uses: the default one. `None` for any other record.
    pub(crate) fn crm_connection_of(&self, rec: &Rec) -> Option<String> {
        let r = rec.borrow();
        if !r.crm || r.temporary {
            return None;
        }
        self.crm_default()
    }

    /// The first use of a record of a CRM table needs a default CRM table connection, whatever the use (Init, TableCaption, a
    /// field, IsEmpty): text and code from session/TestCrmTableNoConnectionContracts.al:15-17,111-118. A connection that is
    /// registered but not the default is not enough (Y16CrmTableConnectionOps.al:253-279, 312-318).
    pub(crate) fn connection_check(&self, rec: &Rec) -> Result<(), Signal> {
        // A temporary record of a CRM table is an in-memory table of its own and needs no connection (not
        // pinned by a corpus line; Base Application's `CRM Synch. Helper.ClearCache` relies on it,
        // docs/probes-needed.md).
        if !rec.borrow().crm || rec.borrow().temporary {
            return Ok(());
        }
        match self.crm_default().and_then(|n| self.crm_stores.get(&n)) {
            Some(s) if s.after_rollback => unsupported(
                "a TableType = CRM table after a rollback that came after a write to its table connection (whether the \
                 rollback takes the write back is not pinned)",
            ),
            Some(_) => Ok(()),
            None => Err(crate::transactions::al_error_code(MUST_REGISTER, "Argument")),
        }
    }

    /// A record method on a record of a CRM table through its connection: the pinned ones run, the rest is refused (module
    /// doc). Insert gives a null Guid primary key a fresh Guid (session/TestDatabaseTableConnectionCrm.al:128-170).
    pub(crate) fn crm_method_check(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        if !rec.borrow().crm || rec.borrow().temporary {
            return Ok(());
        }
        let refuse = |why: &str| -> Result<(), Signal> {
            unsupported(format!(
                "{} on a TableType = CRM table through a table connection: {why}",
                method.name()
            ))
        };
        match method {
            // Pinned on the connection's rows (module doc).
            RecordOp::Get
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::Next
            | RecordOp::Count
            | RecordOp::IsEmpty
            // Pinned on the buffer and the filters (Y16CrmTableConnectionOps.al:154-182, 184-222).
            | RecordOp::Init
            | RecordOp::Clear
            | RecordOp::Reset
            | RecordOp::SetRange
            | RecordOp::SetFilter
            | RecordOp::Ascending => return Ok(()),
            // Methods that touch neither the rows nor the connection: they act on the variable as for any table.
            RecordOp::TableName
            | RecordOp::TableCaption
            | RecordOp::FieldName
            | RecordOp::FieldCaption
            | RecordOp::FieldNo
            | RecordOp::FieldError
            | RecordOp::TestField
            | RecordOp::Validate
            | RecordOp::GetFilter
            | RecordOp::GetFilters
            | RecordOp::HasFilter
            | RecordOp::GetRangeMin
            | RecordOp::GetRangeMax
            | RecordOp::CopyFilter
            | RecordOp::CopyFilters
            | RecordOp::FilterGroup
            | RecordOp::SetCurrentKey
            | RecordOp::CurrentKey
            | RecordOp::GetAscending
            | RecordOp::IsTemporary
            | RecordOp::TransferFields
            | RecordOp::SetRecFilter
            | RecordOp::RecordId => return Ok(()),
            RecordOp::Insert | RecordOp::Modify | RecordOp::Delete | RecordOp::DeleteAll | RecordOp::Rename => {}
            _ => return refuse("not pinned"),
        }
        // The table's own triggers, its extensions' triggers, the table events and the database triggers on a write.
        let run = match (method, args.first()) {
            (RecordOp::Rename, _) => true,
            (_, Some(v)) => al_builtins::ops::truthy(v)?,
            (_, None) => false,
        };
        let (triggers, events, op): (&[&str], [&str; 2], _) = match method {
            RecordOp::Insert => (
                &["OnInsert", "OnBeforeInsert", "OnAfterInsert"],
                ["onbeforeinsertevent", "onafterinsertevent"],
                crate::globaltriggers::GlobalOp::Insert,
            ),
            RecordOp::Modify => (
                &["OnModify", "OnBeforeModify", "OnAfterModify"],
                ["onbeforemodifyevent", "onaftermodifyevent"],
                crate::globaltriggers::GlobalOp::Modify,
            ),
            RecordOp::Rename => (
                &["OnRename", "OnBeforeRename", "OnAfterRename"],
                ["onbeforerenameevent", "onafterrenameevent"],
                crate::globaltriggers::GlobalOp::Rename,
            ),
            _ => (
                &["OnDelete", "OnBeforeDelete", "OnAfterDelete"],
                ["onbeforedeleteevent", "onafterdeleteevent"],
                crate::globaltriggers::GlobalOp::Delete,
            ),
        };
        if run
            && triggers
                .iter()
                .any(|n| self.table_trigger(t, n).is_some() || !self.ext_triggers(t, n).is_empty())
        {
            return refuse("whether the table's triggers run is not pinned");
        }
        for e in events {
            if self.listened(t, e, None)? {
                return refuse("whether the table events are raised is not pinned");
            }
        }
        if self.global_enabled(t, op)? {
            return refuse("whether the database trigger runs is not pinned");
        }
        if method == RecordOp::Rename && !self.rename_references(t)?.is_empty() {
            return refuse("whether fields that relate to the table follow the rename is not pinned");
        }
        if method == RecordOp::Insert {
            let [pk] = t.keys[0].as_slice() else {
                return refuse("a primary key that is not one Guid field is not pinned");
            };
            let null = match &rec.borrow().fields[*pk] {
                Value::Guid(g) => *g == [0; 16],
                _ => return refuse("a primary key that is not one Guid field is not pinned"),
            };
            if null {
                let id = Value::Guid(self.fresh_system_id());
                rec.borrow_mut().fields[*pk] = id;
            }
        }
        if let Some(s) = self.crm_default().and_then(|n| self.crm_stores.get_mut(&n)) {
            s.uncommitted = true;
            s.written = true;
        }
        Ok(())
    }

    /// A commit: the connection's writes so far are no longer taken back by an `asserterror`.
    pub(crate) fn crm_committed(&mut self) {
        for s in self.crm_stores.values_mut() {
            s.uncommitted = false;
        }
    }

    /// A rollback of the transaction (an `asserterror`, a failing `Codeunit.Run`, the end of a test): what it does to a
    /// write to a connection since the last commit is not pinned, so the rows refuse from now on.
    pub(crate) fn crm_rolled_back(&mut self) {
        for s in self.crm_stores.values_mut() {
            if s.uncommitted {
                s.after_rollback = true;
            }
        }
    }

    /// The database goes back to a checkpoint (the end of a test method under test isolation): it also takes back what was
    /// committed since, which for a connection's rows is not pinned.
    pub fn crm_restored(&mut self) {
        for s in self.crm_stores.values_mut() {
            if s.written {
                s.after_rollback = true;
            }
        }
    }
}
