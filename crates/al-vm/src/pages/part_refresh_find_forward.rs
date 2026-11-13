//! A ListPart that serves its rows from OnFindRecord reads its rows again from its first row after its own
//! `CurrPage.Update(true)` in a control's OnValidate (al-runner issue 7820): the client asks OnFindRecord `=>`, not the `=><`
//! of an open read.

use super::pinned_body::{Callees, callees_are, page_is, table_is};
use super::pinned_shape::{PartStep, PinnedEnd, PinnedPart, TestCall};
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

/// The reporter's host page `Repro Order Card`, line for line (`pinned_body.rs`).
const HOST_PAGE: &[&str] = &[
    "PageType = Document",
    "SourceTable = \"Repro Order Header\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "field No. in - = No. {applicationarea=All}",
    "part Lines in - page Repro Order Subform link Order No. = field(No.) {applicationarea=All; subpagelink=\"Order No.\" = \
     field(\"No.\")}",
];

/// The reporter's part `Repro Order Subform`.
const PART_PAGE: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"Repro Order Line\"",
    "ApplicationArea = All",
    "AutoSplitKey = true",
    "repeater Rows in - {}",
    "field Line No. in Rows = Line No. {applicationarea=All; editable=false}",
    "field Qty in Rows = Qty {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(true); end",
    "trigger onfindrecord: var ReadLog: Codeunit \"Repro Read Log\"; begin ReadLog.Add(('F' + Which)); exit(Rec.Find(Which)); end",
    "trigger onaftergetrecord: var ReadLog: Codeunit \"Repro Read Log\"; begin \
     ReadLog.Add(('R' + Format((Rec.\"Line No.\" div 10000)))); end",
    "trigger onaftergetcurrrecord: var ReadLog: Codeunit \"Repro Read Log\"; begin \
     ReadLog.Add(('C' + Format((Rec.\"Line No.\" div 10000)))); end",
];

/// The reporter's tables `Repro Order Header` and `Repro Order Line`.
const HEADER_TABLE: &[&str] = &["field 1 \"No.\": Code[20]", "key \"No.\" {Clustered=true}"];
const LINE_TABLE: &[&str] = &[
    "field 1 \"Order No.\": Code[20]",
    "field 2 \"Line No.\": Integer",
    "field 3 \"Qty\": Integer",
    "key \"Order No.\", \"Line No.\" {Clustered=true}",
];

/// The one procedure the part's triggers call: `Repro Read Log`.Add.
const LOG_ADD: &[&str] = &["\"Repro Read Log\".Add: begin Entries := (Entries + (Entry + ';')); end"];

/// The control of the part the test types into (`Qty`), its position in the part's controls, and the field it shows.
const TYPED_CONTROL: usize = 2;
const TYPED_FIELD: usize = 2;

impl Session {
    /// A part on its first row whose code ran `CurrPage.Update` in a control's OnValidate, with the host's stored row unchanged
    /// and without UpdatePropagation Both (`part_update_host_unchanged`), reads its rows again from its first row (`sync_part`).
    ///
    /// A part that serves its rows from OnFindRecord asks it `=>` there, reads on to the end of the view, puts the first row
    /// back, asks `=` and reads it with OnAfterGetCurrRecord: `F=>;R1;F=;R1;C1;` with one row, `F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;`
    /// with five (al-runner issue 7820, the reporter's repro passed on BC 29.1; assumption `part-update-first-row-find-forward`
    /// until the corpus probe lands). Pinned only for the repro itself (`refresh_repro`); the repro then only reads the part's
    /// values, so every later call but a read and Close is refused. Another part with OnFindRecord keeps the open read's `=><`
    /// as before (it reads the same rows; only its Which differs in the measured shape, and nothing measures the others; Base
    /// Application's sales subforms take this path under Microsoft's tests, so a refusal would stop them).
    pub(super) fn part_reread_from_first_row(&mut self, host: &St, c: usize, st: &St) -> Result<(), Signal> {
        if self.declares(st, "onfindrecord") && self.refresh_repro(host, c, st)? {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PartUpdateFirstRowFindForward);
            st.borrow_mut().part_refind_forward = true;
            host.borrow_mut().pinned_end = Some(PinnedEnd {
                reason: "a call of the test after a part with OnFindRecord read its rows again after its own CurrPage.Update(true), \
                         other than a read and Close (not pinned)",
                reads: true,
                one_action: false,
                first_ends: false,
            });
        }
        let r = self.sync_part(host, c, st);
        st.borrow_mut().part_refind_forward = false;
        r
    }

    /// The entry condition. Every fact is the reporter's (al-runner issue 7820), and all are required:
    /// - the host, the part, their tables and the procedure the part's triggers call read as `HOST_PAGE`, `PART_PAGE`,
    ///   `HEADER_TABLE`, `LINE_TABLE` and `LOG_ADD`, with the structure of `REFRESH_PAGES` (`part_pages_are`);
    /// - the host: opened by the test with OpenEdit, not modal, not a lookup;
    /// - the test's calls since the open: `GoToRecord` on the host, `First` on the part, the value typed into `Qty`;
    /// - the typed value differs from the row the part held before the SetValue (`row_before_write`), and no other field does;
    /// - one or five rows in the part's view (the counts the repro measured).
    fn refresh_repro(&mut self, host: &St, c: usize, st: &St) -> Result<bool, Signal> {
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
        let mut callees = Callees::new();
        let code_ok = page_is(&engine, host.borrow().object as usize, HOST_PAGE, &mut callees)
            && page_is(&engine, st.borrow().object as usize, PART_PAGE, &mut callees)
            && table_is(&engine, host_table, HEADER_TABLE, &mut callees)
            && table_is(&engine, part_table, LINE_TABLE, &mut callees)
            && callees_are(&engine, &callees, LOG_ADD)
            && page_of(&engine, host).controls[c].prop("updatepropagation").is_none();
        drop(engine);
        if !code_ok {
            return Ok(false);
        }
        let rows = self.view_rows(&rec, 6)?.len();
        Ok(rows == 1 || rows == 5)
    }
}
