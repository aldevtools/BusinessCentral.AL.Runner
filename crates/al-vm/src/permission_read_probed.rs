//! Denied reads that corpus PR 1403 measured: a filter that matches no row of a table that holds rows (al-runner issue 7854),
//! and the text of a read denied to code whose Permissions property grants it (al-runner issue 8066).
//!
//! FindFirst is pinned for a dependency table (AEDependencyTableReads.al `Customer_filter_no_match`, in `permissions.rs`).
//! Corpus PR 1403 (open; permissions/testpermissions/Y19PermExtAndCompose.al, test PC_C_NoMatchReadsAndDelete, ops 42-46)
//! measured FindSet, FindLast, Find('-') and Count denied with `Read`, and IsEmpty allowed, on BC 27.5, 28.5 and 29.0
//! (assumption `read-no-match-denied`). The measured shape is an allowlist of its facts here; every other no-match read
//! is refused.
use crate::Session;
use crate::permission_bulk_no_match::{RecordCaller, Target};
use crate::permissions::Access;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};

/// What BC does with a denied read that finds no row, in the measured shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum NoMatchRead {
    /// The read raises BC's `Read` denial.
    Denied,
    /// The read raises nothing and answers as usual.
    Allowed,
}

impl Session {
    /// BC's error for a denied read `method` that finds a row, by the code of object `by`; `indirect` when its Permissions
    /// property grants the read. Without that grant the text names `Read` (Y24PermReads.al Y24_P14_ReadShapesWithRows). With
    /// it, corpus PR 1403 measured `IndirectRead` for Get and FindFirst in the shapes of
    /// [`Session::indirect_read_measured`]; every other read with the grant is refused.
    pub(crate) fn read_denial(&self, method: RecordOp, access: Access, indirect: bool, by: usize) -> Signal {
        if !indirect {
            return self.denial_error(access, "Read", "Read", by);
        }
        let table = self
            .engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(access.id));
        let measured = matches!(method, RecordOp::Get | RecordOp::FindFirst)
            && table.is_some_and(|t| self.indirect_read_measured(access, t, by));
        if measured {
            return self.denial_error(access, "IndirectRead", "Read", by);
        }
        Signal::unsupported(format!(
            "a denied read ({}) by code whose Permissions property grants it: corpus PR 1403 measures `IndirectRead` for Get and FindFirst in one shape only",
            access.describe()
        ))
    }

    /// What BC does with read `method` of `rec`, whose filter matches no row while the table holds `rows` rows, in the
    /// shape corpus PR 1403 measured; `None` (the caller refuses) in every other shape. `by` is the object whose code
    /// makes the access; `indirect` whether its Permissions property grants it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read_no_match_probed(
        &mut self,
        method: RecordOp,
        rec: &crate::records::Rec,
        args: &[Value],
        access: Access,
        indirect: bool,
        by: usize,
        caller: RecordCaller,
        rows: usize,
    ) -> Result<Option<NoMatchRead>, Signal> {
        // Y19PermExtAndCompose.al:631-644 (ops 42-46): `T.FindSet()`, `T.FindLast()`, `T.Find('-')`, `T.Count()` denied
        // (:407-414), `T.IsEmpty()` allowed and true (:415).
        let answer = match (method, args) {
            (RecordOp::FindSet | RecordOp::FindLast | RecordOp::Count, []) => NoMatchRead::Denied,
            (RecordOp::Find, [Value::Text(s)]) if s.as_str() == "-" => NoMatchRead::Denied,
            (RecordOp::IsEmpty, []) => NoMatchRead::Allowed,
            _ => return Ok(None),
        };
        let table = rec.borrow().table as usize;
        let target = self.no_match_target(rec, rows)?;
        // A Record variable of the test codeunit's procedure Op (:561), whose object grants nothing on the table; the
        // sets give nothing on it (`LT PC Only B`, :230, assigned by `Mock.Set`, :522-523).
        let measured = caller == RecordCaller::Record
            && !indirect
            && probed_read_target(&target)
            && self.levels_are(access, &[[0, 0, 0, 0]])
            && self.probed_sets(table)
            && self.test_codeunit_code(by);
        if !measured {
            return Ok(None);
        }
        assume(Assumption::ReadNoMatchDenied);
        Ok(Some(answer))
    }
}

/// The table and filter of the measured reads: "LT PC T" (Y19PermExtAndCompose.al:22-35), per company, one key, its
/// primary key one Integer field Id, its first, of FieldClass Normal; two rows (Seed, :546-549); one filter, a range of one
/// value in filter group 0, the primary key to 100 (`T.SetRange(Id, 100)`, :631).
fn probed_read_target(t: &Target) -> bool {
    let Some((field, value)) = &t.filter else {
        return false;
    };
    t.keys.len() == 1
        && t.key() == [0]
        && t.types.first() == Some(&RtType::Integer)
        && t.classes.first() == Some(&al_ir::FieldClass::Normal)
        && *field == 0
        && matches!(value, Value::Integer(100))
        && t.per_company
        && t.rows == 2
}
