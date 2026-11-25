//! A ListPart that serves its rows from OnFindRecord reads its rows again after its own
//! `CurrPage.Update(true)` in a control's OnValidate (al-runner issue 7820): the client asks OnFindRecord `=>`, not the `=><`
//! of an open read.
//!
//! A ListPart with OnFindRecord reads again after its own `CurrPage.Update(true)` in
//! a field's OnValidate (al-runner issue 7820): the read asks `=>`, not the open read's `=><`
//! (`PageState::part_refind_forward`, taken by `page_first_row` on the first-row cells). Only for the exact original
//! and standalone reporter/probe cells: their pages, tables and trigger bodies line for line (`pinned_body.rs`), their
//! calls, values and rows; names use the measured slots. They run under assumption
//! `part-update-first-row-find-forward`; after the read the test only reads and closes (`PinnedEnd`). Entry:
//! `part_refresh_find_forward` (by `part_update_host_unchanged` in `parts.rs`).

use super::pinned_body::{Callees, Names, callees_are, page_is, table_is};
use super::pinned_shape::{NextEnd, PartStep, PinnedEnd, PinnedPart, Reads, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The structure `part_pages_are` checks next to the lines below: the link on the leading key field, no filter but the
/// link's, no extension or subscriber of the pages or the part's table.
const REFRESH_PAGES: PinnedPart = PinnedPart {
    host_type: ir::PageType::Document,
    host_triggers: &[],
    part_triggers: &["onfindrecord", "onaftergetrecord", "onaftergetcurrrecord"],
    part_properties: &["autosplitkey"],
    part_variable_controls: false,
    part_variable_validate: false,
    part_field_validate_update: true,
    part_expression_controls: false,
    unlinked: false,
    sibling_parts: false,
    user_filter: false,
};

/// One exact host-and-part shape measured by the original reporter or corpus PR 1574.
struct Cell {
    host: &'static [&'static str],
    part: &'static [&'static str],
}

enum RefreshRead {
    First,
    Second { rows: Vec<Vec<Value>> },
}

/// The original reporter's host, line for line (`pinned_body.rs`), with its page-level properties.
const ORIGINAL_HOST: &[&str] = &[
    "PageType = Document",
    "SourceTable = \"«H»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "field No. in - = No. {applicationarea=All}",
    "part Lines in - page «P» link Order No. = field(No.) {applicationarea=All; subpagelink=\"Order No.\" = \
     field(\"No.\")}",
];

/// The original reporter's part, line for line (`pinned_body.rs`), with its page-level ApplicationArea.
const ORIGINAL_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "ApplicationArea = All",
    "AutoSplitKey = true",
    "repeater Rows in - {}",
    "field Line No. in Rows = Line No. {applicationarea=All; editable=false}",
    "field Qty in Rows = Qty {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(true); end",
    "trigger onfindrecord: var «V»: Codeunit \"«C»\"; begin «V».Add(('F' + Which)); exit(Rec.Find(Which)); end",
    "trigger onaftergetrecord: var «V»: Codeunit \"«C»\"; begin \
     «V».Add(('R' + Format((Rec.\"Line No.\" div 10000)))); end",
    "trigger onaftergetcurrrecord: var «V»: Codeunit \"«C»\"; begin \
     «V».Add(('C' + Format((Rec.\"Line No.\" div 10000)))); end",
];

/// The standalone host in corpus PR 1574: the original host without page-level ApplicationArea and UsageCategory
/// (the `:143` cell of `tests/al-language/pages/parts/TestPartUpdateFindRecord.al`).
const STANDALONE_HOST: &[&str] = &[
    "PageType = Document",
    "SourceTable = \"«H»\"",
    "field No. in - = No. {applicationarea=All}",
    "part Lines in - page «P» link Order No. = field(No.) {applicationarea=All; subpagelink=\"Order No.\" = \
     field(\"No.\")}",
];

/// The standalone part in corpus PR 1574: the original part without page-level ApplicationArea
/// (the `:143` and `:154` cells of `tests/al-language/pages/parts/TestPartUpdateFindRecord.al`).
const STANDALONE_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "AutoSplitKey = true",
    "repeater Rows in - {}",
    "field Line No. in Rows = Line No. {applicationarea=All; editable=false}",
    "field Qty in Rows = Qty {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(true); end",
    "trigger onfindrecord: var «V»: Codeunit \"«C»\"; begin «V».Add(('F' + Which)); exit(Rec.Find(Which)); end",
    "trigger onaftergetrecord: var «V»: Codeunit \"«C»\"; begin \
     «V».Add(('R' + Format((Rec.\"Line No.\" div 10000)))); end",
    "trigger onaftergetcurrrecord: var «V»: Codeunit \"«C»\"; begin \
     «V».Add(('C' + Format((Rec.\"Line No.\" div 10000)))); end",
];

const ORIGINAL_CELL: Cell = Cell {
    host: ORIGINAL_HOST,
    part: ORIGINAL_PART,
};
const STANDALONE_CELL: Cell = Cell {
    host: STANDALONE_HOST,
    part: STANDALONE_PART,
};

/// The reporter's and probe's tables.
const HEADER_TABLE: &[&str] = &["field 1 \"No.\": Code[20]", "key \"No.\" {Clustered=true}"];
const LINE_TABLE: &[&str] = &[
    "field 1 \"Order No.\": Code[20]",
    "field 2 \"Line No.\": Integer",
    "field 3 \"Qty\": Integer",
    "key \"Order No.\", \"Line No.\" {Clustered=true}",
];

/// The one procedure the part's triggers call: the measured Add body.
const LOG_ADD: &[&str] = &["\"«C»\".Add: begin Entries := (Entries + (Entry + ';')); end"];

/// The control of the part the test types into (`Qty`), its position in the part's controls, and the field it shows.
const TYPED_CONTROL: usize = 2;
const TYPED_FIELD: usize = 2;

impl Session {
    /// A part whose code ran `CurrPage.Update` in a control's OnValidate, with the host's stored row unchanged and without
    /// UpdatePropagation Both (`part_update_host_unchanged`), reads one of the exact issue 7820 cells.
    ///
    /// A part that serves its rows from OnFindRecord asks it `=>` there, reads on to the end of the view, puts the first row
    /// back, asks `=` and reads it with OnAfterGetCurrRecord on the first-row cells. The original-property cell is exact for
    /// one, two and five rows (`R7820PartUpdateFindForward.al:169`, :181 and :193). The standalone no-page-properties cell is
    /// exact for one and five rows (`TestPartUpdateFindRecord.al:143` and :154, corpus PR 1574). The original cell's one
    /// `Next` before the write is exact for three rows on the second row (`R7820PartUpdateFindForward.al:205`), whose read walk
    /// is `F=>;R2;R3;R2;R1;R2;F=;R2;C2;`. Each runs under `part-update-first-row-find-forward`; after it only reads and Close
    /// are let through (`PinnedEnd`). Other shapes keep main's path or its refusal.
    pub(super) fn part_refresh_find_forward(&mut self, host: &St, c: usize, st: &St) -> Result<bool, Signal> {
        let Some(read) = self.refresh_cell(host, c, st)? else {
            return Ok(false);
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PartUpdateFirstRowFindForward);
        host.borrow_mut().pinned_end = Some(PinnedEnd {
            reason: "a call of the test after a part with OnFindRecord read its rows again after its own CurrPage.Update(true), \
                     other than a read and Close (not pinned)",
            reads: Reads::Any,
            caption: false,
            part_reads: false,
            one_action: false,
            first_ends: false,
            part_visible: false,
            part_moves: false,
            no_writes: false,
            next: NextEnd::Refused,
            close: true,
            ok: false,
            walk: false,
        });
        match read {
            RefreshRead::First => {
                st.borrow_mut().part_refind_forward = true;
                let r = self.sync_part(host, c, st);
                st.borrow_mut().part_refind_forward = false;
                r?;
            }
            RefreshRead::Second { rows } => self.read_part_rows_around(st, &rows, 1)?,
        }
        Ok(true)
    }

    /// The entry condition. Names are free through the measured slots; every body, property, call, value and row fact is
    /// exact:
    /// - the host and part are one exact `ORIGINAL_CELL` or `STANDALONE_CELL` pair, and their tables and the procedure the
    ///   part's triggers call read as `HEADER_TABLE`, `LINE_TABLE` and `LOG_ADD`, with the structure of `REFRESH_PAGES`
    ///   (`part_pages_are`);
    /// - the host: opened by the test with OpenEdit, not modal, not a lookup;
    /// - one exact first-row call sequence (`GoToRecord`, `First`, the write to `Qty`), or the original cell's sequence with
    ///   exactly one `Next`;
    /// - the header is O1 and all rows have the exact seeded O1, line and quantity values; the selected row changes only
    ///   `Qty` from its seeded value to 100 (`PageState::row_before_write`);
    /// - the original first-row cells have exactly 1, 2 or 5 rows, the standalone first-row cells exactly 1 or 5, and the
    ///   original later-cursor cell exactly 3 rows with the cursor on row 2.
    fn refresh_cell(&mut self, host: &St, c: usize, st: &St) -> Result<Option<RefreshRead>, Signal> {
        use PartStep::{Host, Own};
        let steps = Self::part_steps(st);
        let first = matches!(
            steps.as_deref(),
            Some([
                Host(TestCall::Page(PageMethod::GoToRecord)),
                Own(TestCall::Page(PageMethod::First)),
                Own(TestCall::Write(TYPED_CONTROL))
            ])
        );
        let second = matches!(
            steps.as_deref(),
            Some([
                Host(TestCall::Page(PageMethod::GoToRecord)),
                Own(TestCall::Page(PageMethod::First)),
                Own(TestCall::Page(PageMethod::Next)),
                Own(TestCall::Write(TYPED_CONTROL))
            ])
        );
        let host_opened = Self::part_host_opened(st) == Some((true, PageMode::Edit, false, false));
        let pages = self.part_pages_are(st, &REFRESH_PAGES)?;
        match (host_opened, pages) {
            (true, true) => {}
            _ => return Ok(None),
        }
        let rec = self.page_rec(st);
        let Some(read_row) = st.borrow().row_before_write.clone() else {
            return Ok(None);
        };
        // The platform's own fields (SystemModifiedAt, the timestamp) change with the save the part's Update(true) made.
        let own_fields = self
            .table(&rec)?
            .fields
            .iter()
            .take_while(|f| f.id < 2_000_000_000)
            .count();
        let changed: Vec<usize> = {
            let r = rec.borrow();
            (0..own_fields)
                .filter(|&i| match (r.fields.get(i), read_row.get(i)) {
                    (Some(a), Some(b)) => match Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)) {
                        true => false,
                        false => true,
                    },
                    _ => true,
                })
                .collect()
        };
        match changed == [TYPED_FIELD] {
            true => {}
            false => return Ok(None),
        }
        let host_table = self.page_rec(host).borrow().table as usize;
        let part_table = rec.borrow().table as usize;
        let engine = self.engine();
        let original = Self::refresh_code_is(&engine, host, c, st, host_table, part_table, &ORIGINAL_CELL);
        let standalone = Self::refresh_code_is(&engine, host, c, st, host_table, part_table, &STANDALONE_CELL);
        drop(engine);
        if let (false, false) = (original, standalone) {
            return Ok(None);
        }
        let rows = self.view_rows(&rec, 6)?;
        let key = self.row_key(&rec)?;
        let table = self.table(&rec)?;
        let at = rows
            .iter()
            .position(|row| Self::same_key(&table.primary_key(row), &key));
        let Some(at) = at else {
            return Ok(None);
        };
        match self.refresh_values_are(host, &rec, &read_row, &rows, at) {
            true => {}
            false => return Ok(None),
        }
        let read = match (original, standalone, first, second, rows.len(), at) {
            (_, true, true, false, 1 | 5, 0) => RefreshRead::First,
            (true, _, true, false, 1 | 2 | 5, 0) => RefreshRead::First,
            (true, _, false, true, 3, 1) => RefreshRead::Second { rows },
            _ => return Ok(None),
        };
        Ok(Some(read))
    }

    /// One exact body/property cell, with fresh body-call and name bindings.
    fn refresh_code_is(
        engine: &Engine,
        host: &St,
        c: usize,
        st: &St,
        host_table: usize,
        part_table: usize,
        cell: &Cell,
    ) -> bool {
        let host_object = host.borrow().object as usize;
        let part_object = st.borrow().object as usize;
        let mut callees = Callees::new();
        let mut names = Names::default();
        let no_update_propagation_both = page_of(engine, host).controls[c].prop("updatepropagation").is_none();
        page_is(engine, host_object, cell.host, &mut callees, &mut names)
            && page_is(engine, part_object, cell.part, &mut callees, &mut names)
            && names.bound('P') == Some(engine.program.objects[part_object].name.as_str())
            && table_is(engine, host_table, HEADER_TABLE, &mut callees, &mut names)
            && table_is(engine, part_table, LINE_TABLE, &mut callees, &mut names)
            && callees_are(engine, &callees, LOG_ADD, &mut names)
            && no_update_propagation_both
    }

    fn refresh_values_are(&self, host: &St, rec: &Rec, before: &[Value], rows: &[Vec<Value>], at: usize) -> bool {
        let header = self.page_rec(host);
        match header.borrow().fields.first() {
            Some(Value::Code(v)) if v.as_str() == "O1" => {}
            _ => return false,
        }
        let row_is = |row: &[Value], i: usize, qty: i32| {
            matches!(row.first(), Some(Value::Code(v)) if v.as_str() == "O1")
                && matches!(row.get(1), Some(Value::Integer(v)) if *v == (i as i32 + 1) * 10_000)
                && matches!(row.get(2), Some(Value::Integer(v)) if *v == qty)
        };
        row_is(before, at, at as i32 + 1)
            && row_is(&rec.borrow().fields, at, 100)
            && rows
                .iter()
                .enumerate()
                .all(|(i, row)| row_is(row, i, if i == at { 100 } else { i as i32 + 1 }))
    }
}
