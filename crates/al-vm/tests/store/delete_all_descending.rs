//! `DeleteAll(true)` with OnDelete over a record sorted descending runs the trigger in the scan order (assumption
//! delete-all-row-code-scan-order, corpus PR 1659, al-runner issue 9401). Row code that reads any table (a record
//! operation or a FlowField), writes a row of the table (inside the filter or outside it, the row being deleted too),
//! commits or raises an error is not measured: the run
//! refuses with main's text, and the refused run carries no assumption id (records/delete_all_order.rs).

use al_builtins::assumptions::Assumption;
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "DAD Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } }
    trigger OnDelete()
    var Ctl: Codeunit Ctl; R: Record "DAD Row"; S: Record "DAD Sum";
    begin
        Ctl.Add(Format(Id));
        if Id <> 3 then exit;
        case Ctl.Mode() of
            'delete': begin R.Get(1); R.Delete(false); end;
            'modify': begin R.Get(1); R.Name := 'x'; R.Modify(false); end;
            'insert': begin R.Id := 0; R.Insert(false); end;
            'error': Error('stop');
            'outside': begin R.Get(9); R.Name := 'x'; R.Modify(false); end;
            'insertoutside': begin R.Id := 10; R.Insert(false); end;
            'self': begin Rec.Name := 'x'; Rec.Modify(false); end;
            'commit': Commit();
            'get': if R.Get(1) then;
            'count': Ctl.Add(Format(R.Count()));
            'flow': begin S.CalcFields(Rows); Ctl.Add(Format(S.Rows)); end;
            'other': if S.Get(1) then;
        end;
    end;
}
table 69994 "DAD Sum" { fields { field(1; Id; Integer) { } field(2; Rows; Integer) { FieldClass = FlowField; CalcFormula = count("DAD Row"); } } keys { key(PK; Id) { } } }
table 69993 "DAD Temp Row" { TableType = Temporary; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } }
    trigger OnDelete() var Ctl: Codeunit Ctl; begin Ctl.Add(Format(Id)); end;
}
table 2000000006 Company
{
    DataPerCompany = false;
    fields
    {
        field(1; Name; Text[30]) { }
        field(2; "Evaluation Company"; Boolean) { }
        field(3; "Display Name"; Text[250]) { }
        field(8000; Id; Guid) { }
        field(8005; "Business Profile Id"; Text[250]) { }
    }
    keys { key(PK; Name) { } }
}
codeunit 69991 Ctl
{
    SingleInstance = true;
    var Seen: Text; M: Text;
    procedure Add(T: Text) begin Seen += T; end;
    procedure Mode(): Text begin exit(M); end;
    procedure SetMode(T: Text) begin M := T; Seen := ''; end;
    procedure Seen(): Text begin exit(Seen); end;
}
codeunit 69992 C
{
    procedure Run(Mode: Text): Text
    var R: Record "DAD Row"; Ctl: Codeunit Ctl; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(false); end;
        Ctl.SetMode(Mode);
        R.Ascending(false);
        R.DeleteAll(true);
        exit(Ctl.Seen() + '/' + Format(R.Count()));
    end;
    procedure RunFiltered(Mode: Text; Temp: Boolean): Text
    var R: Record "DAD Row"; T: Record "DAD Row" temporary; Ctl: Codeunit Ctl; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(false); T.Id := i; T.Insert(false); end;
        R.Id := 9; R.Insert(false); T.Id := 9; T.Insert(false);
        Ctl.SetMode(Mode);
        if Temp then begin
            T.SetRange(Id, 1, 3);
            T.Ascending(false);
            T.DeleteAll(true);
            T.Reset();
            exit(Ctl.Seen() + '/' + Format(T.Count()));
        end;
        R.SetRange(Id, 1, 3);
        R.Ascending(false);
        R.DeleteAll(true);
        R.Reset();
        exit(Ctl.Seen() + '/' + Format(R.Count()));
    end;
    procedure TableTypeTemporary(): Text
    var R: Record "DAD Temp Row"; Ctl: Codeunit Ctl; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(false); end;
        Ctl.SetMode('');
        R.Ascending(false);
        R.DeleteAll(true);
        exit(Ctl.Seen() + '/' + Format(R.Count()));
    end;
    procedure OtherCompany(): Text
    var R: Record "DAD Row"; Co: Record Company; Ctl: Codeunit Ctl; i: Integer;
    begin
        Co.Name := 'OTHER';
        Co.Insert();
        R.ChangeCompany('OTHER');
        for i := 1 to 3 do begin R.Id := i; R.Insert(false); end;
        Ctl.SetMode('');
        R.Ascending(false);
        R.DeleteAll(true);
        exit(Ctl.Seen() + '/' + Format(R.Count()));
    end;
    procedure Plain(): Text begin exit(Run('')); end;
    procedure FilteredPlain(): Text begin exit(RunFiltered('', false)); end;
    procedure TempPlain(): Text begin exit(RunFiltered('', true)); end;
    procedure ModifiesOutside(): Text begin exit(RunFiltered('outside', false)); end;
    procedure InsertsOutside(): Text begin exit(RunFiltered('insertoutside', false)); end;
    procedure ModifiesItself(): Text begin exit(RunFiltered('self', false)); end;
    procedure TempModifiesItself(): Text begin exit(RunFiltered('self', true)); end;
    procedure Commits(): Text begin exit(RunFiltered('commit', false)); end;
    procedure Deletes(): Text begin exit(Run('delete')); end;
    procedure Modifies(): Text begin exit(Run('modify')); end;
    procedure Inserts(): Text begin exit(Run('insert')); end;
    procedure Errors(): Text begin exit(Run('error')); end;
    procedure Reads(): Text begin exit(RunFiltered('get', false)); end;
    procedure Counts(): Text begin exit(RunFiltered('count', false)); end;
    procedure TempReads(): Text begin exit(RunFiltered('get', true)); end;
    procedure ReadsThroughFlowField(): Text begin exit(RunFiltered('flow', false)); end;
    procedure ReadsOtherTable(): Text begin exit(RunFiltered('other', false)); end;
}
"#;

#[test]
fn descending_delete_all_runs_on_delete_in_scan_order() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Plain")), "321/0");
}

#[test]
fn descending_delete_all_with_a_filter_or_temporary_runs_on_delete_in_scan_order() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "FilteredPlain")), "321/1");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "TempPlain")), "321/1");
}

/// Main's refusal of every descending DeleteAll with row code, kept for the shapes corpus PR 1659 does not measure.
const MAIN_REFUSAL: &str = "DeleteAll with delete events, database triggers or OnDelete over two or more rows of a record \
                            sorted descending: the order of the row code is not pinned";

#[test]
fn descending_delete_all_refuses_row_code_that_reads_or_writes_the_table_commits_or_errors() {
    let mut wrong = Vec::new();
    for routine in [
        "Deletes",
        "Modifies",
        "Inserts",
        "Errors",
        "ModifiesOutside",
        "InsertsOutside",
        "ModifiesItself",
        "TempModifiesItself",
        "Commits",
        "TableTypeTemporary",
        "OtherCompany",
        "Reads",
        "Counts",
        "TempReads",
        "ReadsThroughFlowField",
        "ReadsOtherTable",
    ] {
        al_builtins::assumptions::take_hits();
        let r = al_testkit::run_in(SRC, "C", routine);
        let flagged = al_builtins::assumptions::take_hits().contains(&Assumption::DeleteAllRowCodeScanOrder);
        if !matches!(&r, Err(Signal::Unsupported(m)) if m == MAIN_REFUSAL) || flagged {
            wrong.push(format!("{routine}: {r:?}, flagged {flagged}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
