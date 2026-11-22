//! The code of the pages, the table and the procedures the `CurrPage.SaveRecord()` and `CurrPage.Update(true)` cells of
//! `code_inserted_new_line.rs` run, line for line (`pinned_body.rs`): the trigger names and the page facts that
//! `code_inserted_page` checks do not say what the triggers do (review of al-runner PR 8305, round 3). A page whose OnModifyRecord
//! returns false, a trigger that assigns a field, a table trigger that writes, or other code in the control's OnValidate is not
//! measured: such a cell is refused (`code_inserted_cell`).
//!
//! The page with code of al-runner issue 8434 (`code_inserted_coded.rs`) is the probe's too, line for line: its host page, its
//! table, its three FactBox parts, the tables they read and the codeunits they call (`coded_code`, review of al-runner PR 8463,
//! round 1). Any other page keeps main's way: it is not refused (partner tests open such pages and pass on main).

use super::pinned_body::{
    Callees, Names, callees_are, object_code_reads_as, page_is, page_reads_as, table_is, table_reads_as,
};
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

    /// Whether host `st` is page 97492 or 97493 of ListNewLineFactboxesCode.al (corpus pull request 1454, merged, not pinned
    /// yet), line for line: the page (`CODED_HOST`, without `CODED_VIEW` for 97493) with its variables and procedures, its
    /// table 97480, its three FactBox parts 97489-97491 with theirs, the tables 97481-97484 the code reads, the log codeunit
    /// 97485, the calculation codeunit 97487 and the background task 97488. Objects may carry other names (the name slots of
    /// `pinned_body.rs`); every procedure the code calls is one of these objects', or the System Application's of
    /// `CODED_OUTSIDE`. Nothing may subscribe to the parts, the other tables or the codeunits; the host page and its table may
    /// have manual subscribers, which `code_inserted_coded.rs` checks.
    pub(super) fn coded_code(&self, st: &St) -> bool {
        let table = self.page_rec(st).borrow().table as usize;
        let host = st.borrow().object as usize;
        let engine = self.engine();
        let program = &engine.program;
        let Some(page) = program.objects[host].page.as_ref() else {
            return false;
        };
        let mut callees = Callees::new();
        let names = &mut Names::default();
        let view = program.objects[host]
            .properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("sourcetableview"));
        let host_lines: Vec<&str> = CODED_HOST
            .iter()
            .copied()
            .filter(|l| view || *l != CODED_VIEW)
            .collect();
        if !(page_reads_as(program, host, &host_lines, &mut callees, names)
            && object_code_reads_as(program, host, CODED_HOST_CODE, &mut callees, names)
            && table_reads_as(program, table, CODED_TABLE, &mut callees, names)
            && object_code_reads_as(program, table, &[], &mut callees, names))
        {
            return false;
        }
        let mut compared = vec![host, table];
        let parts: [(&str, &[&str], &[&str]); 3] = [
            ("Rules", CODED_RULES, CODED_RULES_CODE),
            ("Picture", CODED_PICTURE, &[]),
            ("Usage", CODED_USAGE, CODED_USAGE_CODE),
        ];
        for (control, lines, code) in parts {
            let Some(part) = page
                .controls
                .iter()
                .find(|c| c.name == control)
                .and_then(|c| c.part_page)
            else {
                return false;
            };
            if !(page_is(&engine, part, lines, &mut callees, names)
                && object_code_reads_as(program, part, code, &mut callees, names))
            {
                return false;
            }
            compared.push(part);
        }
        for (slot, lines) in [
            ('K', CODED_KIND),
            ('S', CODED_SETUP),
            ('G', CODED_CHANGE),
            ('W', CODED_VALUE),
        ] {
            let Some(o) = named(program, ir::ObjKind::Table, names.bound(slot)) else {
                return false;
            };
            if !(table_is(&engine, o, lines, &mut callees, names)
                && object_code_reads_as(program, o, &[], &mut callees, names))
            {
                return false;
            }
        }
        let task = names.bound('N').and_then(|n| n.parse::<u32>().ok()).and_then(|id| {
            program
                .objects
                .iter()
                .position(|o| o.kind == ir::ObjKind::Codeunit && o.id == Some(id) && !o.uncompiled)
        });
        let codeunits = [
            (named(program, ir::ObjKind::Codeunit, names.bound('L')), CODED_LOG),
            (named(program, ir::ObjKind::Codeunit, names.bound('C')), CODED_CALC),
            (task, CODED_TASK),
        ];
        for (o, lines) in codeunits {
            let Some(o) = o else {
                return false;
            };
            if engine.events.any_subscriber_of(o) || !object_code_reads_as(program, o, lines, &mut callees, names) {
                return false;
            }
            compared.push(o);
        }
        callees.iter().all(|&(o, r)| {
            compared.contains(&o)
                || CODED_OUTSIDE.iter().any(|&(object, routine)| {
                    program.objects[o].name == object
                        && program.objects[o].routines.get(r).is_some_and(|x| x.name == routine)
                })
        })
    }
}

/// The one compiled object of `kind` named `name`.
fn named(program: &ir::Program, kind: ir::ObjKind, name: Option<&str>) -> Option<usize> {
    let name = name?;
    let mut found = program
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind == kind && !o.uncompiled && o.name == name)
        .map(|(i, _)| i);
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// The SourceTableView of page 97492 (:503); page 97493 declares none (the primary key).
const CODED_VIEW: &str = "SourceTableView = sorting(\"Sorting\") order(ascending)";

/// The procedure outside the probe's objects its code calls, by object and procedure: the System Application's, whose body is
/// not the probe's to pin (OnOpenPage, :774-781).
const CODED_OUTSIDE: [(&str, &str); 1] = [("Environment Information", "IsSaaSInfrastructure")];

/// Table 97480 "LT NF Line" (:36-151).
const CODED_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer AutoIncrement",
    "field 3 \"Name\": Text[30] {NotBlank=true; TableRelation=\"«K»\".Name where(Blocked = const(false)); \
        ValidateTableRelation=false}",
    "field 4 \"Value\": Text[30]",
    "field 5 \"Kind ID\": Integer",
    "field 6 \"Sorting\": Integer",
    "field 7 \"Variant\": Code[10]",
    "field 8 \"Image\": MediaSet",
    "field 9 \"Style\": Text[30]",
    "field 10 \"Quantity\": Decimal",
    "field 11 \"Has Quantity\": Boolean",
    "field 12 \"Kind Type\": Option Text,Option",
    "field 13 \"Description\": Text[100]",
    "field 14 \"Show Blank\": Boolean",
    "field 15 \"Kind Group\": Text[30]",
    "field 16 \"Billable\": Boolean",
    "field 17 \"Active\": Boolean",
    "field 18 \"Old Flag\": Boolean",
    "field 19 \"Calc State\": Option  ,Pending,Processed",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "key \"Group Code\", \"Sorting\" {}",
    "trigger \"Name\".OnValidate: var Log: Codeunit \"«L»\"; begin Log.Add((('T.VN(' + Rec.\"Name\") + ')')); if \
        (Rec.\"Entry No.\" = 0) then begin Rec.Insert(true); end; CopyKind(); end",
    "trigger \"Value\".OnValidate: var Change: Record \"«G»\"; Log: Codeunit \"«L»\"; begin Log.Add((((('T.VV(' + \
        xRec.\"Value\") + '>') + Rec.\"Value\") + ')')); if (Rec.\"Value\" <> xRec.\"Value\") then begin \
        Rec.\"Variant\" := ''; Change.Init(); Change.\"Group Code\" := Rec.\"Group Code\"; Change.\"Line Entry No.\" \
        := Rec.\"Entry No.\"; Change.\"Old Value\" := xRec.\"Value\"; Change.\"New Value\" := Rec.\"Value\"; \
        Change.Insert(); end; Rec.\"Calc State\" := option#Pending; end",
    "trigger OnInsert: var Last: Record \"«T»\"; Log: Codeunit \"«L»\"; begin Last.SetCurrentKey(field#5); \
        Last.SetRange(field#0, Rec.\"Group Code\"); Last.SetLoadFields(field#5); if Last.FindLast() then begin \
        Rec.\"Sorting\" := (Last.\"Sorting\" + 10); end; Log.Add((((('T.I(' + Rec.\"Name\") + '/') + \
        Format(Rec.\"Sorting\")) + ')')); end",
    "trigger OnModify: var Log: Codeunit \"«L»\"; begin Log.Add((((('T.M(' + Rec.\"Name\") + '/') + Rec.\"Value\") + \
        ')')); end",
    "trigger OnDelete: var Change: Record \"«G»\"; begin Change.SetRange(field#1, Rec.\"Group Code\"); \
        Change.SetRange(field#2, Rec.\"Entry No.\"); Change.DeleteAll(); end",
    "procedure CopyKind: var Kind: Record \"«K»\"; begin Kind.SetRange(field#1, Rec.\"Name\"); if Kind.FindFirst() \
        then begin Rec.\"Kind ID\" := Kind.\"ID\"; Rec.\"Kind Type\" := option#Option; end; end",
];

/// Table 97481 "LT NF Kind" (:153-168).
const CODED_KIND: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"ID\": Integer",
    "field 2 \"Name\": Text[30]",
    "field 3 \"Blocked\": Boolean",
    "key \"ID\" {Clustered=true}",
];

/// Table 97482 "LT NF Setup" (:170-186).
const CODED_SETUP: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Primary Key\": Code[10]",
    "field 2 \"Lock Values\": Boolean",
    "field 3 \"Custom 1 Caption\": Text[30]",
    "field 4 \"Calc Count\": Integer",
    "key \"Primary Key\" {Clustered=true}",
];

/// Table 97483 "LT NF Change" (:188-208).
const CODED_CHANGE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Entry No.\": Integer AutoIncrement",
    "field 2 \"Group Code\": Code[10]",
    "field 3 \"Line Entry No.\": Integer",
    "field 4 \"Old Value\": Text[30]",
    "field 5 \"New Value\": Text[30]",
    "key \"Entry No.\" {Clustered=true}",
];

/// Table 97484 "LT NF Value" (:210-225).
const CODED_VALUE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Kind ID\": Integer",
    "field 2 \"Value\": Text[30]",
    "field 3 \"Blocked\": Boolean",
    "key \"Kind ID\", \"Value\" {Clustered=true}",
];

/// Codeunit 97485 "LT NF Log" (:227-252).
const CODED_LOG: &[&str] = &[
    "SingleInstance = true",
    "var Buffer: Text",
    "procedure Reset(): begin Buffer := ''; end",
    "procedure Add(Entry: Text): begin Buffer := (Buffer + (Entry + ';')); end",
    "procedure Take(): Text: var Result: Text; begin Result := Buffer; Buffer := ''; exit(Result); end",
];

/// Codeunit 97487 "LT NF Calc" (:276-295).
const CODED_CALC: &[&str] = &[
    "procedure CalculateAfterChange(var Line: Record \"«T»\"): var Setup: Record \"«S»\"; Log: Codeunit \"«L»\"; \
        begin if Line.GetBySystemId(Line.\"SystemId\") then begin end; Log.Add((((((('S.CALC(' + Line.\"Name\") + \
        '/') + Line.\"Value\") + '/') + Format(Line.\"Calc State\", 0, 2)) + ')')); Setup.Get(); Setup.\"Calc Count\" \
        := (Setup.\"Calc Count\" + 1); Setup.Modify(); if (Line.\"Calc State\" = option#Pending) then begin \
        Line.\"Calc State\" := option#Processed; Line.Modify(false); end; if \
        Line.GetBySystemId(Line.\"SystemId\") then begin end; end",
];

/// Codeunit 97488 "LT NF Usage Task", the background task (:298-308).
const CODED_TASK: &[&str] = &[
    "trigger OnRun(): var Kind: Record \"«K»\"; Results: Dictionary of [Text, Text]; begin Results.Add('Count', ('K' \
        + Format(Kind.Count()))); SetBackgroundTaskResult(Results); end",
];

/// Page 97489 "LT NF Rules" (:310-346).
const CODED_RULES: &[&str] = &[
    "Caption = ' '",
    "Editable = false",
    "PageType = CardPart",
    "SourceTable = \"«T»\"",
    "UsageCategory = None",
    "ApplicationArea = All",
    "Extensible = false",
    "field Lines in - = var RulesText: Text {applicationarea=All; multiline=true; showcaption=false}",
    "trigger onaftergetrecord: var Change: Record \"«G»\"; Log: Codeunit \"«L»\"; begin Change.SetRange(field#1, \
        Rec.\"Group Code\"); Change.SetRange(field#2, Rec.\"Entry No.\"); RulesText := Format(Change.Count()); \
        Log.Add((((('R.AGR(' + Rec.\"Name\") + '/') + RulesText) + ')')); end",
];

/// The variables of page 97489.
const CODED_RULES_CODE: &[&str] = &["var RulesText: Text"];

/// Page 97490 "LT NF Picture" (:348-397); it has no variable or procedure.
const CODED_PICTURE: &[&str] = &[
    "Caption = 'Picture'",
    "Editable = false",
    "PageType = CardPart",
    "SourceTable = \"«T»\"",
    "UsageCategory = None",
    "ApplicationArea = All",
    "Extensible = false",
    "field Image in - = Image {showcaption=false}",
    "action ImportPicture in - {caption='Import'; image=Import; tooltip='Imports a picture.'}",
    "  trigger onaction: begin Clear(Rec.\"Image\"); end",
    "action ClearPicture in - {caption='Clear'; image=Delete; tooltip='Clears the picture.'}",
    "  trigger onaction: begin Clear(Rec.\"Image\"); end",
    "area Processing: ImportPicture, ClearPicture",
];

/// Page 97491 "LT NF Usage" (:399-494), the part that enqueues the background task.
const CODED_USAGE: &[&str] = &[
    "Caption = 'Usage'",
    "Editable = false",
    "Extensible = false",
    "PageType = CardPart",
    "SourceTable = \"«T»\"",
    "UsageCategory = None",
    "ApplicationArea = All",
    "field Uses in - = var UsageText: Text {applicationarea=All; caption='Uses'}",
    "  trigger ondrilldown: begin Stale := true; end",
    "cuegroup Kinds in - {caption='Kinds'}",
    "field KindName in Kinds = expr begin exit(TempKind.\"Name\"); end {caption='Kind'; drilldown=true; \
        style=Unfavorable; styleexpr=Stale}",
    "  trigger ondrilldown: begin Stale := true; end",
    "  trigger __sourceset: begin TempKind.\"Name\" := ((((NewValue.IsInteger() or NewValue.IsBigInteger()) or \
        NewValue.IsDecimal()) or (NewValue.IsOption() or NewValue.IsDate())) ? fail('The expression \
        Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a \
        Microsoft.Dynamics.Nav.Runtime.NavText value.', 'TypeConversion') : NewValue); end",
    "trigger onaftergetcurrrecord: var Log: Codeunit \"«L»\"; Parameters: Dictionary of [Text, Text]; begin if Stale \
        then begin Log.Add((('W.AGCR(' + Rec.\"Name\") + '/s)')); end else begin Log.Add((('W.AGCR(' + Rec.\"Name\") \
        + ')')); end; if (((GroupCode = Rec.\"Group Code\") and (EntryNo = Rec.\"Entry No.\")) and (not Stale)) then \
        begin exit; end; GroupCode := Rec.\"Group Code\"; EntryNo := Rec.\"Entry No.\"; Stale := false; if (TaskId <> \
        0) then begin CurrPage.CancelBackgroundTask(TaskId); end; Parameters.Add('GroupCode', GroupCode); \
        CurrPage.EnqueueBackgroundTask(TaskId, «N», Parameters); end",
    "trigger onpagebackgroundtaskcompleted: var Log: Codeunit \"«L»\"; begin if (CompletedTaskId <> TaskId) then \
        begin exit; end; if Results.ContainsKey('Count') then begin UsageText := Results.Get('Count'); end; \
        Log.Add((('W.DONE(' + UsageText) + ')')); end",
    "trigger onpagebackgroundtaskerror: var Log: Codeunit \"«L»\"; begin Log.Add((('W.ERR(' + ErrorCode) + ')')); \
        IsHandled := true; end",
];

/// The variables and procedures of page 97491.
const CODED_USAGE_CODE: &[&str] = &[
    "var GroupCode: Code[10]",
    "var EntryNo: Integer",
    "var TaskId: Integer",
    "var Stale: Boolean",
    "var UsageText: Text",
    "var TempKind: Record \"«K»\" temporary",
    "procedure Invalidate(): begin Stale := true; end",
];

/// Page 97492 "LT NF Lines" (:497-816); page 97493 "LT NF Lines Pk" (:818-1136) is the same without its SourceTableView
/// (`CODED_VIEW`). Each label's text names its app (`«A»`) and its page (`Page «H»`).
const CODED_HOST: &[&str] = &[
    "AutoSplitKey = true",
    "Caption = 'Lines'",
    "Editable = true",
    "PageType = List",
    "SourceTable = \"«T»\"",
    CODED_VIEW,
    "UsageCategory = None",
    "ApplicationArea = All",
    "Extensible = false",
    "repeater Group in - {indentationcolumn=0; caption='Group'}",
    "field Name in Group = Name {applicationarea=Basic, Suite; styleexpr=Rec.Style; tablerelation=\"«K»\".Name \
        where(Blocked = const(false)); tooltip='Specifies the kind.'}",
    "  trigger onvalidate: var Kind: Record \"«K»\"; begin if (xRec.\"Name\" <> '') then begin if (Rec.\"Name\" <> \
        xRec.\"Name\") then begin ErrorConstant(TranslatedText('The kind cannot be changed.', '«A»', 'the Label ''The \
        kind cannot be changed.''', 'Page «H» - NamedType 2004540278')); end; end; if (Rec.\"Name\" <> '') then begin \
        Kind.SetRange(field#1, Rec.\"Name\"); if Kind.FindFirst() then begin Rec.\"Kind ID\" := Kind.\"ID\"; end else \
        begin ErrorConstant(TranslatedText('Unknown kind.', '«A»', 'the Label ''Unknown kind.''', 'Page «H» - NamedType \
        1668614503')); end; end; Log.Add((('P.VN(' + Rec.\"Name\") + ')')); CurrPage.SaveRecord(); Log.Add('P.SR'); \
        CalculateAndRefresh(); Log.Add('P.VN.end'); end",
    "field Value in Group = Value {applicationarea=Basic, Suite; editable=expr begin exit(ValueEditable); end; \
        tablerelation=\"«W»\".Value where(\"Kind ID\" = field(\"Kind ID\"), Blocked = const(false))}",
    "  trigger onvalidate: begin Log.Add((('P.VV(' + Rec.\"Value\") + ')')); CurrPage.SaveRecord(); Log.Add('P.SR'); \
        Rec.GetBySystemId(Rec.\"SystemId\"); CalculateAndRefresh(); Log.Add('P.VV.end'); end",
    "field Variant in Group = Variant {applicationarea=Basic, Suite; caption='Variant'; editable=expr begin \
        exit(VariantEditable); end}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Quantity in Group = Quantity {applicationarea=Basic, Suite; caption='Quantity'; blankzero=true; \
        editable=expr begin exit(Rec.\"Has Quantity\"); end}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Active in Group = Active {}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Kind Group in Group = Kind Group {applicationarea=Basic, Suite; caption='Group'; editable=true; \
        style=None; drilldown=false; lookup=false}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Description in Group = Description {applicationarea=Basic, Suite; caption='Description'; editable=false}",
    "field Sorting in Group = Sorting {applicationarea=Basic, Suite; caption='Sorting'}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Billable in Group = Billable {applicationarea=Basic, Suite; caption='Billable'; visible=expr begin \
        exit(IsGroupKind); end}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Custom1 in Group = expr begin exit(CustomValue[1]); end {applicationarea=Basic, Suite; captionclass=expr \
        begin exit(CustomHeader[1]); end; editable=false; tooltip='Specifies the first custom value.'}",
    "  trigger __sourceset: begin CustomValue[1] := Into(NewValue, 4102); end",
    "field Show Blank in Group = Show Blank {applicationarea=Basic, Suite; visible=false; editable=true}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "field Old Flag in Group = Old Flag {applicationarea=Basic, Suite; visible=false; editable=true; \
        obsoletestate=Pending; obsoletetag='28.0'; obsoletereason='Not shown any more.'}",
    "field Kind Type in Group = Kind Type {applicationarea=Basic, Suite; visible=expr begin exit((Rec.\"Kind Type\" \
        = option#Option)); end; editable=true}",
    "  trigger onvalidate: begin SaveChangeAndCalculate(); end",
    "factbox part Rules in - page «R» link Group Code = field(Group Code) link Entry No. = field(Entry No.) \
        {applicationarea=Basic, Suite; subpagelink=\"Group Code\" = field(\"Group Code\"), \"Entry No.\" = \
        field(\"Entry No.\"); updatepropagation=SubPart}",
    "factbox part Picture in - page «P» link Group Code = field(Group Code) link Entry No. = field(Entry No.) \
        {applicationarea=Basic, Suite; subpagelink=\"Group Code\" = field(\"Group Code\"), \"Entry No.\" = \
        field(\"Entry No.\")}",
    "factbox part Usage in - page «U» link Group Code = field(Group Code) link Entry No. = field(Entry No.) \
        {subpagelink=\"Group Code\" = field(\"Group Code\"), \"Entry No.\" = field(\"Entry No.\")}",
    "actiongroup Process in - {caption='Process'}",
    "action Recalculate in Process {applicationarea=Basic, Suite; caption='Recalculate'; enabled=expr begin \
        exit(IsOption); end; image=Calculate; tooltip='Recalculates the lines.'; visible=expr begin exit((Rec.\"Kind \
        ID\" <> 0)); end}",
    "  trigger onaction: begin CalculateAndRefresh(); end",
    "action Suggest in - {applicationarea=Basic, Suite; caption='Suggest'; image=Sparkle; tooltip='Suggests lines.'; \
        visible=expr begin exit(IsSaaS); end}",
    "  trigger onaction: begin CalculateAndRefresh(); end",
    "actionref Recalculate_Promoted in - -> Recalculate {}",
    "area Processing: Process",
    "area Prompting: Suggest",
    "area Promoted: Recalculate_Promoted",
    "trigger ondeleterecord: var Calc: Codeunit \"«C»\"; begin Rec.Delete(true); Calc.CalculateAfterChange(Rec); \
        RefreshUsage(); exit(false); end",
    "trigger onopenpage: var EnvironmentInformation: Codeunit \"Environment Information\"; begin Log.Add('P.OPEN'); \
        Setup.Get(); IsSaaS := EnvironmentInformation.IsSaaSInfrastructure(); end",
    "trigger onaftergetrecord: var Kind: Record \"«K»\"; begin Log.Add((((('P.AGR(' + Rec.\"Name\") + '/') + \
        Rec.\"Value\") + ')')); Setup.Get(); IsOption := (Rec.\"Kind Type\" = option#Option); IsGroupKind := \
        false; if Kind.Get(Rec.\"Kind ID\") then begin IsGroupKind := Kind.\"Blocked\"; end; ValueEditable := (not \
        Setup.\"Lock Values\"); VariantEditable := (Rec.\"Kind ID\" <> 0); if (Setup.\"Custom 1 Caption\" <> '') then \
        begin CustomHeader[1] := Setup.\"Custom 1 Caption\"; CustomValue[1] := Format(Rec.\"Kind ID\"); end; end",
];

/// The variables and procedures of pages 97492 and 97493.
const CODED_HOST_CODE: &[&str] = &[
    "var Setup: Record \"«S»\"",
    "var Log: Codeunit \"«L»\"",
    "var IsOption: Boolean",
    "var IsGroupKind: Boolean",
    "var IsSaaS: Boolean",
    "var ValueEditable: Boolean",
    "var VariantEditable: Boolean",
    "var CannotChangeErr: Text",
    "var UnknownErr: Text",
    "var CustomHeader: array[30] of Text[30]",
    "var CustomValue: array[30] of Text[30]",
    "local procedure RefreshUsage(): begin CurrPage.Usage.Invalidate(); Log.Add('P.UPD'); CurrPage.Update(false); \
        Log.Add('P.UPD.end'); end",
    "local procedure CalculateAndRefresh(): var Calc: Codeunit \"«C»\"; begin Calc.CalculateAfterChange(Rec); \
        RefreshUsage(); end",
    "local procedure SaveChangeAndCalculate(): begin if (Rec.\"Entry No.\" = 0) then begin exit; end; \
        CurrPage.SaveRecord(); CalculateAndRefresh(); end",
];
