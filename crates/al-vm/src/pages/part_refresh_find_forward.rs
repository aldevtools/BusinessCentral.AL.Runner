//! A ListPart that serves its rows from OnFindRecord reads its rows again from its first row after its own
//! `CurrPage.Update(true)` in a control's OnValidate (al-runner issue 7820): the client asks OnFindRecord `=>`, not the `=><`
//! of an open read.
//!
//! A ListPart with OnFindRecord on its first row reads again after its own `CurrPage.Update(true)` in
//! a field's OnValidate (al-runner issue 7820): the read asks `=>`, not the open read's `=><`
//! (`PageState::part_refind_forward`, taken by `page_first_row`). Only for the two exact reporter/probe
//! cells: their pages, tables and trigger bodies line for line (`pinned_body.rs`), their calls, a
//! changed value (`PageState::row_before_write`) and one or five rows, under the assumption
//! `part-update-first-row-find-forward`; after it only reads and Close (`PinnedEnd`). Entry points:
//! `part_reread_from_first_row` (by `part_update_host_unchanged` in `parts.rs`).

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
    "trigger onfindrecord: var ReadLog: Codeunit \"«C»\"; begin ReadLog.Add(('F' + Which)); exit(Rec.Find(Which)); end",
    "trigger onaftergetrecord: var ReadLog: Codeunit \"«C»\"; begin \
     ReadLog.Add(('R' + Format((Rec.\"Line No.\" div 10000)))); end",
    "trigger onaftergetcurrrecord: var ReadLog: Codeunit \"«C»\"; begin \
     ReadLog.Add(('C' + Format((Rec.\"Line No.\" div 10000)))); end",
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
    "trigger onfindrecord: var ReadLog: Codeunit \"«C»\"; begin ReadLog.Add(('F' + Which)); exit(Rec.Find(Which)); end",
    "trigger onaftergetrecord: var ReadLog: Codeunit \"«C»\"; begin \
     ReadLog.Add(('R' + Format((Rec.\"Line No.\" div 10000)))); end",
    "trigger onaftergetcurrrecord: var ReadLog: Codeunit \"«C»\"; begin \
     ReadLog.Add(('C' + Format((Rec.\"Line No.\" div 10000)))); end",
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
    /// A part on its first row whose code ran `CurrPage.Update` in a control's OnValidate, with the host's stored row unchanged
    /// and without UpdatePropagation Both (`part_update_host_unchanged`), reads its rows again from its first row (`sync_part`).
    ///
    /// A part that serves its rows from OnFindRecord asks it `=>` there, reads on to the end of the view, puts the first row
    /// back, asks `=` and reads it with OnAfterGetCurrRecord: `F=>;R1;F=;R1;C1;` with one row, `F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;`
    /// with five (al-runner issue 7820 and corpus PR 1574's standalone cells at
    /// `tests/al-language/pages/parts/TestPartUpdateFindRecord.al:143` and :154; assumption
    /// `part-update-first-row-find-forward` until the corpus probe is pinned). Pinned only for the exact cells
    /// (`refresh_cell`); the reporter/probe then only reads the part's values, so every later call but a read and Close is
    /// refused. Another part with OnFindRecord keeps the open read's `=><` as before (it reads the same rows; only its Which
    /// differs in the measured shapes, and nothing measures the others; Base Application's sales subforms take this path
    /// under Microsoft's tests, so a refusal would stop them).
    pub(super) fn part_reread_from_first_row(&mut self, host: &St, c: usize, st: &St) -> Result<(), Signal> {
        // A previous refresh must not lend its mark to this one when the new shape does not match.
        st.borrow_mut().part_refind_forward = false;
        if self.declares(st, "onfindrecord") && self.refresh_cell(host, c, st)? {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PartUpdateFirstRowFindForward);
            st.borrow_mut().part_refind_forward = true;
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
        }
        let r = self.sync_part(host, c, st);
        st.borrow_mut().part_refind_forward = false;
        r
    }

    /// The entry condition. Every fact is the reporter's (al-runner issue 7820), and all are required:
    /// - the host and part are one exact `ORIGINAL_CELL` or `STANDALONE_CELL` pair, and their tables and the procedure the
    ///   part's triggers call read as `HEADER_TABLE`, `LINE_TABLE` and `LOG_ADD`, with the structure of `REFRESH_PAGES`
    ///   (`part_pages_are`);
    /// - the host: opened by the test with OpenEdit, not modal, not a lookup;
    /// - the test's calls since the open: `GoToRecord` on the host, `First` on the part, the value typed into `Qty`;
    /// - the typed value differs from the row the part held before the SetValue (`row_before_write`), and no other field does;
    /// - one or five rows in the part's view (the counts the repro measured).
    fn refresh_cell(&mut self, host: &St, c: usize, st: &St) -> Result<bool, Signal> {
        use PartStep::{Host, Own};
        let calls_ok = matches!(
            Self::part_steps(st).as_deref(),
            Some([
                Host(TestCall::Page(PageMethod::GoToRecord)),
                Own(TestCall::Page(PageMethod::First)),
                Own(TestCall::Write(TYPED_CONTROL))
            ])
        );
        if !(calls_ok
            && Self::part_host_opened(st) == Some((true, PageMode::Edit, false, false))
            && self.part_pages_are(st, &REFRESH_PAGES)?)
        {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let Some(read_row) = st.borrow().row_before_write.clone() else {
            return Ok(false);
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
                    (Some(a), Some(b)) => !Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)),
                    _ => true,
                })
                .collect()
        };
        if changed != [TYPED_FIELD] {
            return Ok(false);
        }
        let host_table = self.page_rec(host).borrow().table as usize;
        let part_table = rec.borrow().table as usize;
        let engine = self.engine();
        let host_object = host.borrow().object as usize;
        let part_object = st.borrow().object as usize;
        let has_prop = |object: usize, name: &str| {
            engine.program.objects[object]
                .properties
                .iter()
                .any(|(key, _)| key.eq_ignore_ascii_case(name))
        };
        let cell = match (
            has_prop(host_object, "applicationarea"),
            has_prop(host_object, "usagecategory"),
            has_prop(part_object, "applicationarea"),
        ) {
            (true, true, true) => &ORIGINAL_CELL,
            (false, false, false) => &STANDALONE_CELL,
            _ => {
                drop(engine);
                return Ok(false);
            }
        };
        let mut callees = Callees::new();
        let names = &mut Names::default();
        let code_ok = page_is(&engine, host_object, cell.host, &mut callees, names)
            && page_is(&engine, part_object, cell.part, &mut callees, names)
            && names.bound('P') == Some(engine.program.objects[part_object].name.as_str())
            && table_is(&engine, host_table, HEADER_TABLE, &mut callees, names)
            && table_is(&engine, part_table, LINE_TABLE, &mut callees, names)
            && callees_are(&engine, &callees, LOG_ADD, names)
            && page_of(&engine, host).controls[c].prop("updatepropagation").is_none();
        drop(engine);
        if !code_ok {
            return Ok(false);
        }
        let rows = self.view_rows(&rec, 6)?.len();
        Ok(rows == 1 || rows == 5)
    }
}
