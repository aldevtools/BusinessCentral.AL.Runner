//! The code of the pages, the table and the procedures the `CurrPage.SaveRecord()` and `CurrPage.Update(true)` cells of
//! `code_inserted_new_line.rs` run, line for line (`pinned_body.rs`): the trigger names and the page facts that
//! `code_inserted_page` checks do not say what the triggers do (review of al-runner PR 8305, round 3). A page whose OnModifyRecord
//! returns false, a trigger that assigns a field, a table trigger that writes, or other code in the control's OnValidate is not
//! measured: such a cell is refused (`code_inserted_cell`).

use super::pinned_body::{Callees, Names, callees_are, page_is, table_is};
use super::*;

/// Table 97470 "LT NS Line" (ListNewLineSaveRecordUpdate.al:18-61).
const PROBE_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer AutoIncrement",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "trigger \"Name\".OnValidate: var Log: Codeunit \"LT NS Log\"; begin Log.Add((('T.V(' + Rec.\"Name\") + ')')); if \
     (Rec.\"Entry No.\" = 0) then begin Rec.Insert(true); end; end",
    "trigger OnInsert: var Log: Codeunit \"LT NS Log\"; begin Log.Add((('T.I(' + Rec.\"Name\") + ')')); end",
    "trigger OnModify: var Log: Codeunit \"LT NS Log\"; begin Log.Add((((('T.M(' + Rec.\"Name\") + '/') + Rec.\"Value\") + \
     ')')); end",
];

/// The first lines every probe page shares (:107-111 and the repeater and Name control after them; :174-178; :212-216).
const PROBE_HEAD: [&str; 7] = [
    "PageType = List",
    "SourceTable = \"LT NS Line\"",
    "AutoSplitKey = true",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
];

/// Page 97472 "LT NS Lines", the five row triggers (:105-169).
const PROBE_LINES: &[&str] = &[
    PROBE_HEAD[0],
    PROBE_HEAD[1],
    PROBE_HEAD[2],
    PROBE_HEAD[3],
    PROBE_HEAD[4],
    PROBE_HEAD[5],
    PROBE_HEAD[6],
    "  trigger onvalidate: begin Log.Add((('P.V(' + Log.Bool((Rec.\"Entry No.\" <> 0))) + ')')); if \
     Log.IsSaveThenUpdateFalse() then begin CurrPage.SaveRecord(); Log.Add('P.SR'); CurrPage.Update(false); end else begin \
     CurrPage.Update(true); end; Log.Add('P.V.end'); end",
    "field Value in Lines = Value {applicationarea=All}",
    "trigger onnewrecord: begin Log.Add('P.NEW'); end",
    "trigger oninsertrecord: begin Log.Add((('P.INS(' + Rec.\"Name\") + ')')); exit(true); end",
    "trigger onmodifyrecord: begin Log.Add((((('P.MOD(' + Rec.\"Name\") + '/') + Rec.\"Value\") + ')')); exit(true); end",
    "trigger onaftergetrecord: begin Log.Add((('P.AGR(' + Rec.\"Name\") + ')')); end",
    "trigger onaftergetcurrrecord: begin Log.Add((('P.AGCR(' + Rec.\"Name\") + ')')); end",
];

/// Page 97473 "LT NS Lines Quiet", no page trigger (:172-206).
const PROBE_QUIET: &[&str] = &[
    PROBE_HEAD[0],
    PROBE_HEAD[1],
    PROBE_HEAD[2],
    PROBE_HEAD[3],
    PROBE_HEAD[4],
    PROBE_HEAD[5],
    PROBE_HEAD[6],
    "  trigger onvalidate: begin if Log.IsSaveThenUpdateFalse() then begin CurrPage.SaveRecord(); CurrPage.Update(false); end \
     else begin CurrPage.Update(true); end; end",
    "field Value in Lines = Value {applicationarea=All}",
];

/// Page 97474 "LT NS Lines Part", OnAfterGetRecord and one CardPart FactBox (:210-254).
const PROBE_HOST: &[&str] = &[
    PROBE_HEAD[0],
    PROBE_HEAD[1],
    PROBE_HEAD[2],
    PROBE_HEAD[3],
    PROBE_HEAD[4],
    PROBE_HEAD[5],
    PROBE_HEAD[6],
    "  trigger onvalidate: begin CurrPage.SaveRecord(); CurrPage.Update(false); end",
    "field Value in Lines = Value {applicationarea=All}",
    "factbox part Peek in - page LT NS Peek link Group Code = field(Group Code) link Entry No. = field(Entry No.) \
     {applicationarea=All; subpagelink=\"Group Code\" = field(\"Group Code\"), \"Entry No.\" = field(\"Entry No.\")}",
    "trigger onaftergetrecord: begin Log.Add((((('P.AGR(' + Rec.\"Name\") + '/') + Rec.\"Value\") + ')')); end",
];

/// Page 97475 "LT NS Peek", the FactBox (:256-270).
const PROBE_PEEK: &[&str] = &[
    "PageType = CardPart",
    "SourceTable = \"LT NS Line\"",
    "ApplicationArea = All",
    "field Name in - = Name {applicationarea=All}",
    "field Value in - = Value {applicationarea=All}",
];

/// The procedures of codeunit 97471 "LT NS Log" the probe's code calls (:63-102).
const PROBE_LOG: &[&str] = &[
    "\"LT NS Log\".Add: begin Buffer := (Buffer + (Entry + ';')); end",
    "\"LT NS Log\".Bool: begin if Value then begin exit('T'); end; exit('F'); end",
    "\"LT NS Log\".IsSaveThenUpdateFalse: begin exit(SaveThenUpdateFalse); end",
];

/// The reporter's table of al-runner issue 8285 (`Repro Line`; corpus pull request 1481, open:
/// tests/al-language/pages/currpage/ListNewLineSaveRecordRepro.al keeps the repro's objects under other names).
const REPRO_TABLE: &[&str] = &[
    "DataClassification = CustomerContent",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer AutoIncrement",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "trigger \"Name\".OnValidate: begin if (Rec.\"Entry No.\" = 0) then begin Rec.Insert(true); end; end",
];

/// The first lines of the reporter's pages.
const REPRO_HEAD: [&str; 7] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "AutoSplitKey = true",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {}",
];

/// The reporter's `Repro Lines Save Update`: SaveRecord, then Update(false).
const REPRO_SAVE: &[&str] = &[
    REPRO_HEAD[0],
    REPRO_HEAD[1],
    REPRO_HEAD[2],
    REPRO_HEAD[3],
    REPRO_HEAD[4],
    REPRO_HEAD[5],
    REPRO_HEAD[6],
    "  trigger onvalidate: begin CurrPage.SaveRecord(); CurrPage.Update(false); end",
    "field Value in Lines = Value {}",
];

/// The reporter's `Repro Lines Update True`.
const REPRO_TRUE: &[&str] = &[
    REPRO_HEAD[0],
    REPRO_HEAD[1],
    REPRO_HEAD[2],
    REPRO_HEAD[3],
    REPRO_HEAD[4],
    REPRO_HEAD[5],
    REPRO_HEAD[6],
    "  trigger onvalidate: begin CurrPage.Update(true); end",
    "field Value in Lines = Value {}",
];

/// The reporter's `Repro Lines With FactBox`.
const REPRO_HOST: &[&str] = &[
    REPRO_HEAD[0],
    REPRO_HEAD[1],
    REPRO_HEAD[2],
    REPRO_HEAD[3],
    REPRO_HEAD[4],
    REPRO_HEAD[5],
    REPRO_HEAD[6],
    "  trigger onvalidate: begin CurrPage.SaveRecord(); CurrPage.Update(false); end",
    "field Value in Lines = Value {}",
    "factbox part LineFactBox in - page «P» link Group Code = field(Group Code) link Entry No. = field(Entry No.) \
     {subpagelink=\"Group Code\" = field(\"Group Code\"), \"Entry No.\" = field(\"Entry No.\")}",
    "trigger onaftergetrecord: begin RowsRead := (RowsRead + 1); end",
];

/// The reporter's `Repro Line FactBox`.
const REPRO_FACTBOX: &[&str] = &[
    "PageType = CardPart",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "field Name in - = Name {}",
];

/// Whose code a cell runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PinnedCode {
    /// The probe's objects: pinned.
    Probe,
    /// The reporter's repro of al-runner issue 8285: assumption `list-new-line-save-record-repro`.
    Repro,
}

/// One measured page with its table and FactBox part.
struct PinnedSet {
    code: PinnedCode,
    page: &'static [&'static str],
    table: &'static [&'static str],
    part: Option<&'static [&'static str]>,
}

/// Every measured page.
const SETS: [PinnedSet; 6] = [
    PinnedSet {
        code: PinnedCode::Probe,
        page: PROBE_LINES,
        table: PROBE_TABLE,
        part: None,
    },
    PinnedSet {
        code: PinnedCode::Probe,
        page: PROBE_QUIET,
        table: PROBE_TABLE,
        part: None,
    },
    PinnedSet {
        code: PinnedCode::Probe,
        page: PROBE_HOST,
        table: PROBE_TABLE,
        part: Some(PROBE_PEEK),
    },
    PinnedSet {
        code: PinnedCode::Repro,
        page: REPRO_SAVE,
        table: REPRO_TABLE,
        part: None,
    },
    PinnedSet {
        code: PinnedCode::Repro,
        page: REPRO_TRUE,
        table: REPRO_TABLE,
        part: None,
    },
    PinnedSet {
        code: PinnedCode::Repro,
        page: REPRO_HOST,
        table: REPRO_TABLE,
        part: Some(REPRO_FACTBOX),
    },
];

impl Session {
    /// Whose code page `st`, its table, its FactBox part and the procedures they call are, line for line: the probe's
    /// (`PROBE_*`) or the reporter's repro (`REPRO_*`). `None` for any other code. Nothing may subscribe to the events of the
    /// pages, the table or the log codeunit (`page_is`, `table_is`, `callees_are`).
    pub(super) fn code_inserted_code(&self, st: &St) -> Result<Option<PinnedCode>, Signal> {
        let table = self.page_rec(st).borrow().table as usize;
        let part = st.borrow().parts.first().map(|(_, p)| p.borrow().object as usize);
        let page = st.borrow().object as usize;
        let engine = self.engine();
        for &PinnedSet {
            code,
            page: page_lines,
            table: table_lines,
            part: part_lines,
        } in &SETS
        {
            let mut callees = Callees::new();
            let names = &mut Names::default();
            let part_ok = match (part, part_lines) {
                (None, None) => true,
                (Some(p), Some(want)) => page_is(&engine, p, want, &mut callees, names),
                _ => false,
            };
            if part_ok
                && page_is(&engine, page, page_lines, &mut callees, names)
                && table_is(&engine, table, table_lines, &mut callees, names)
                && callees_are(&engine, &callees, PROBE_LOG, names)
            {
                return Ok(Some(code));
            }
        }
        Ok(None)
    }
}
