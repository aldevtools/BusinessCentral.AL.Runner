//! Where a buffer's row version came from, for the Modify after a failed guarded Insert (al-runner issue 8721;
//! `RecordState::version_entry`). A version enters a buffer at a load, at the variable's own Insert, Modify or Rename,
//! and by `:=`, Record.Copy, a by-value parameter or RecordRef.GetTable. Each stamps what the open transaction had
//! written or locked of the table then (`OpenTx::table_writes`, kept as writes are noted, so a load scans nothing).
//!
//! BC judges the version when it reaches the buffer (corpus PR 1523, aldevtools/BusinessCentral.AL.Language.Tests#1523,
//! tests/al-language/record/crud/TestRecordFailedInsertMovedKey.al; "the probe file" below). A version read before any
//! write or lock of its table raises DB:RecordChanged: P1 :169, P2 :190, P3 :215, P4 :240, P5 :261, P7 :315, P8 :338,
//! and Q3 :454 (only another table written). A version that reached the buffer after one is written. The cells are
//! listed at [`Session::measured_after_table_write`]. A version TransferFields copied with the key is neither read nor
//! written by the buffer, so it has no entry and raises (R8 :844, `recmeta.rs` `transfer_fields`).

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::records::same_key;
use crate::state::TxWriteKind;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, TableWritten, Value, VersionEntry, VersionHow, VersionSource, VersionVia};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// Whether a noted write of kind `kind` after a version reached a buffer is one no cell measures there. Measured
/// through a Record variable: an Insert, a Modify and a Delete (S6 :1146, S8 :1195, R6 :784; issue 8721's cells 6 and
/// 7, measured by its reporter on BC 29).
pub(crate) fn unmeasured_after(kind: TxWriteKind, through_recordref: bool) -> bool {
    through_recordref || !matches!(kind, TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete)
}

/// What a noted write of kind `kind` does to its table's verdict. A cell measures these as a write before the read
/// through a Record variable: an Insert (Q1a :393, Q2 :432), a Modify (Q5 :503), a Delete (R4 :712), a DeleteAll (Q1a's
/// seed, R5a :736) and a ModifyAll (R5b :760). A Rename, a Truncate and any write through a RecordRef are not measured.
pub(crate) fn write_verdict(kind: TxWriteKind, through_recordref: bool) -> TableWritten {
    let measured = matches!(
        kind,
        TxWriteKind::Insert
            | TxWriteKind::Modify
            | TxWriteKind::Delete
            | TxWriteKind::DeleteAll
            | TxWriteKind::ModifyAll
    );
    if measured && !through_recordref {
        TableWritten::Written
    } else {
        TableWritten::Unmeasured
    }
}

/// The variable's own Delete of `rec`: its version is of a row it deleted, which no cell measures after a failed
/// Insert.
pub(crate) fn note_own_delete(rec: &Rec) {
    if let Some(e) = rec.borrow_mut().version_entry.0.as_mut() {
        e.how = VersionHow::OtherWrite;
    }
}

impl Session {
    /// A LockTable of `table` through `rec` counts as a write of the table (R1 :621, R1b :643; through a Record
    /// variable).
    pub(crate) fn note_table_locked(&mut self, rec: &Rec, table: u32) {
        let verdict = if self.through_recordref {
            TableWritten::Unmeasured
        } else {
            TableWritten::Written
        };
        let company = rec.borrow().company.clone();
        self.tx.open.table_writes.note(table, &company, verdict);
    }

    /// A stamp of the open transaction for a version `version` of the row `key` of `table`, reached by `how`.
    fn stamp(
        &self,
        table: u32,
        company: &Option<String>,
        (version, key): (i64, Vec<Value>),
        how: VersionHow,
        table_written: TableWritten,
    ) -> VersionSource {
        VersionSource {
            version,
            key,
            how,
            via: VersionVia::Own,
            table_written,
            company: company.clone(),
            tests: self.pages.test_methods_started,
            commits: self.commits,
            rollbacks: self.rollbacks,
            writes: self.tx.write_counts.now(table, company),
            failed_inserts: 0,
        }
    }

    /// The entry of the version a load into `rec` just read (`how` is [Get, GetBySystemId, a Find]; `pk` the row's
    /// key).
    pub(crate) fn load_version_entry(
        &self,
        t: &TableDef,
        rec: &Rec,
        version: Option<&Value>,
        how: [bool; 3],
        pk: &[Value],
    ) -> VersionEntry {
        let r = rec.borrow();
        let Some(Value::BigInteger(v)) = version else {
            return VersionEntry::default();
        };
        if r.temporary {
            return VersionEntry::default();
        }
        let how = match how {
            _ if self.through_recordref => VersionHow::OtherRead,
            [true, false, false] => VersionHow::Get,
            [false, false, true] => VersionHow::Find,
            _ => VersionHow::OtherRead,
        };
        let written = self.tx.open.table_writes.of(t.object, &r.company);
        VersionEntry(Some(self.stamp(t.object, &r.company, (*v, pk.to_vec()), how, written)))
    }

    /// Stamps the version the variable's own write `how` just gave `rec`. `before`: the table's verdict before the
    /// write, when the write itself was noted as something else (a Rename is noted as an unknown write first).
    pub(crate) fn stamp_own_write(&self, t: &TableDef, rec: &Rec, how: VersionHow, before: Option<TableWritten>) {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) else {
            return;
        };
        let mut r = rec.borrow_mut();
        let Value::BigInteger(v) = r.fields[i] else {
            return;
        };
        if r.temporary {
            r.version_entry = VersionEntry::default();
            return;
        }
        // Through a RecordRef only the own Modify keeps its rule (al-runner issue 7798 reads no RecordRef fact).
        let how = match how {
            VersionHow::OwnInsert | VersionHow::OwnRename if self.through_recordref => VersionHow::OtherWrite,
            h => h,
        };
        let company = r.company.clone();
        let written = match before {
            Some(b) => b.max(TableWritten::Written),
            None => self.tx.open.table_writes.of(t.object, &company),
        };
        let key = t.primary_key(&r.fields);
        r.version_entry = VersionEntry(Some(self.stamp(t.object, &company, (v, key), how, written)));
    }

    /// The variable's own Modify of `rec` was written: the row it wrote (`last_modify`) and its version's entry.
    pub(crate) fn note_own_modify(&self, t: &TableDef, rec: &Rec) {
        let image = rec.borrow().fields.to_vec();
        rec.borrow_mut().last_modify = Some(image);
        self.stamp_own_write(t, rec, VersionHow::OwnModify, None);
    }

    /// Before a Rename of `rec`: the noted writes, the store's change counter and the table's verdict.
    pub(crate) fn rename_note(&self, rec: &Rec, t: &TableDef) -> (usize, u64, TableWritten) {
        let verdict = self.tx.open.table_writes.of(t.object, &rec.borrow().company);
        (self.tx.open.writes.len(), self.store.change_counter(), verdict)
    }

    /// The variable's own Rename of `rec` was written (`note` from `rename_note`): a plain one, noted as one Rename
    /// write, is measured (R7 :813); one that rewrote related rows is not.
    pub(crate) fn note_own_rename(&self, t: &TableDef, rec: &Rec, note: (usize, u64, TableWritten)) {
        let writes = &self.tx.open.writes;
        let plain = writes.len() == note.0 + 1 && writes.last().is_some_and(|w| w.kind == TxWriteKind::Rename);
        let how = if plain {
            VersionHow::OwnRename
        } else {
            VersionHow::OtherWrite
        };
        self.stamp_own_write(t, rec, how, Some(note.2));
    }

    /// Whether the Modify after a failed Insert of `rec`, whose version `mine` is older than the stored row's, is
    /// written. `Ok(false)` raises DB:RecordChanged.
    pub(crate) fn version_after_table_write(&mut self, t: &TableDef, rec: &Rec, mine: i64) -> Result<bool, Signal> {
        let (e, moved, company) = {
            let r = rec.borrow();
            let Some(e) = r.version_entry.0.clone().filter(|e| e.version == mine) else {
                return Ok(false);
            };
            let moved = !same_key(&e.key, &t.primary_key(&r.fields));
            (e, moved, r.company.clone())
        };
        // A version from the buffer's own Modify of another row: written
        // (record/crud/CodeunitRunTableNoWrites.al:828-904, al-runner issue 7798; also Q7 :561, R9 :875).
        if e.how == VersionHow::OwnModify && e.via == VersionVia::Own && moved {
            return Ok(true);
        }
        // Read before any write or lock of its table: raised (see the module comment). An own write always writes it.
        if e.table_written == TableWritten::No {
            return Ok(false);
        }
        if self.measured_after_table_write(t, &e, moved, &company)? {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::FailedInsertAfterTableWrite);
            return Ok(true);
        }
        al_builtins::native::unsupported(format!(
            "Modify after a failed Insert of a {} buffer whose row version came after the open transaction wrote or \
             locked the table, in a shape no test measures",
            t.name
        ))
    }

    /// The written cells of the probe file, by what they vary; a shape outside them refuses.
    /// - The version: a Get (Q1a :393, Q1b :414, Q2 :432, Q4 :481, Q5 :503, R1 :621, R1b :643, R4 :712, R5a :736, R5b
    ///   :760); a FindFirst (R2 :665) or FindSet and Next (R3 :688), assigned; the buffer's own Insert (P6 :283, Q6
    ///   :528) or plain Rename (R7 :813) of another key.
    /// - The copy that holds it: `:=` (the Q and R cells), Record.Copy (S3 :1078), a by-value parameter (S4 :1100),
    ///   RecordRef.GetTable with the Modify through the ref (R10 :907), or none, the key moved on the read buffer (S1a
    ///   :984, S1b :1011) or kept (R6 :784, the row changed by another variable after the read).
    /// - Before the version: an Insert, Modify, Delete, DeleteAll, ModifyAll or LockTable of the table
    ///   (`write_verdict`, `note_table_locked`), TARGET among them (S2b :1058). After it: Insert, Modify or Delete of
    ///   the table's rows (S6 :1146, S8 :1195), a LockTable (S5 :1122), a rollback of a read version (S7 :1171), at
    ///   most one Commit (Q4, Q6).
    /// - Insert or Insert(true), Modify or Modify(true) (S1a, S1b), with TransferFields(Src, false) or not (S1a, R6,
    ///   R10).
    ///
    /// Measured for none of them, so refused: another company, a version carried into another test, an own write rolled
    /// back or followed by an own Delete, two Commits, a write of another table or of a kind `unmeasured_after` names
    /// after the version, a second failed Insert, a Modify whose result is used, and a table with triggers,
    /// subscribers or global triggers.
    fn measured_after_table_write(
        &mut self,
        t: &TableDef,
        e: &VersionSource,
        moved: bool,
        company: &Option<String>,
    ) -> Result<bool, Signal> {
        use VersionHow::{Find, Get, OwnInsert, OwnRename};
        use VersionVia::{Assigned, ByValue, Copied, GetTable, Own};
        let source = matches!(
            (e.how, e.via, moved),
            (Get, Own | Assigned | Copied | ByValue | GetTable, true)
                | (Get, Own, false)
                | (Find, Assigned, true)
                | (OwnInsert | OwnRename, Own, true)
        );
        let read = matches!(e.how, Get | Find);
        let [all, unmeasured, of_table] = self.tx.write_counts.now(t.object, company);
        let shape = source
            && e.table_written == TableWritten::Written
            && e.company.is_none()
            && company.is_none()
            && e.tests == self.pages.test_methods_started
            && (read || e.rollbacks == self.rollbacks)
            && self.commits <= e.commits + 1
            && unmeasured == e.writes[1]
            && all - e.writes[0] == of_table - e.writes[2]
            && e.failed_inserts == 1
            && (e.via == GetTable) == self.through_recordref
            && !self.ret_used
            && self.stale_writes_noted();
        Ok(shape && !self.write_hooks(t)?)
    }

    /// Whether an Insert or Modify of `t` runs code of its own: a trigger, a table extension's trigger, an event
    /// subscriber or a global trigger. The measured table has none, and with none no code runs before the check, so
    /// `ret_used` is still the Modify's.
    fn write_hooks(&mut self, t: &TableDef) -> Result<bool, Signal> {
        if ["OnInsert", "OnModify"]
            .iter()
            .any(|n| self.table_trigger(t, n).is_some())
        {
            return Ok(true);
        }
        let ext = [
            "OnBeforeInsert",
            "OnInsert",
            "OnAfterInsert",
            "OnBeforeModify",
            "OnModify",
            "OnAfterModify",
        ];
        if ext.iter().any(|n| !self.ext_triggers(t, n).is_empty()) {
            return Ok(true);
        }
        for event in [
            "onbeforeinsertevent",
            "onafterinsertevent",
            "onbeforemodifyevent",
            "onaftermodifyevent",
        ] {
            if self.listened(t, event, None)? {
                return Ok(true);
            }
        }
        Ok(self.global_enabled(t, GlobalOp::Insert)? || self.global_enabled(t, GlobalOp::Modify)?)
    }
}
