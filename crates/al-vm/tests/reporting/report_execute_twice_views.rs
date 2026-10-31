//! `Report.Execute` twice on one variable with parameters, and data item views in the parameters (corpus master, corpus PR 886:
//! report/lt-report-requestpage/ExecuteParamsTwiceAndViews.al:146-235).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 91367 "LT Y7 Exec Row"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } key(ByGrp; Grp, Id) { } }
}
report 91368 "LT Y7 Exec Params"
{
    ProcessingOnly = true;
    UseRequestPage = true;
    dataset
    {
        dataitem(Row; "LT Y7 Exec Row")
        {
            RequestFilterFields = Grp, Id;
            trigger OnPreDataItem()
            begin
                L.Add('pre note=' + NoteVal + ' filters=[' + Row.GetFilters() + '] key=[' + Row.CurrentKey() + '] asc=' + Format(Row.Ascending()));
            end;
            trigger OnAfterGetRecord()
            begin
                L.Add('get ' + Format(Row.Id));
            end;
        }
    }
    requestpage { layout { area(Content) { field(NoteVal; NoteVal) { ApplicationArea = All; } } } }
    var
        L: Codeunit "LT Y7 Exec Log";
        NoteVal: Text[30];
}
codeunit 91369 "LT Y7 Exec Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
codeunit 91370 T
{
    var
        L: Codeunit "LT Y7 Exec Log";

    local procedure Seed()
    var
        Row: Record "LT Y7 Exec Row";
        I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 4 do begin
            Row.Id := I;
            Row.Grp := 1 + ((I - 1) div 2);
            Row.Insert();
        end;
        L.Take();
    end;

    local procedure Params(Note: Text; DataItems: Text): Text
    var
        Txt: Text;
    begin
        Txt := '<?xml version="1.0" standalone="yes"?><ReportParameters name="LT Y7 Exec Params" id="91368"><Options><Field name="NoteVal">' + Note + '</Field></Options>';
        if DataItems <> '' then
            Txt += '<DataItems>' + DataItems + '</DataItems>';
        exit(Txt + '</ReportParameters>');
    end;

    [TryFunction]
    local procedure TryExecute(var R: Report "LT Y7 Exec Params"; P: Text)
    begin
        R.Execute(P);
    end;

    local procedure Exec(var R: Report "LT Y7 Exec Params"; P: Text): Text
    begin
        ClearLastError();
        if TryExecute(R, P) then
            exit('ok|' + L.Take() + '#');
        exit('E[' + GetLastErrorText() + '|' + GetLastErrorCode() + ']|' + L.Take() + '#');
    end;

    procedure Twice(): Text
    var
        R: Report "LT Y7 Exec Params";
        R2: Report "LT Y7 Exec Params";
        Out: Text;
    begin
        Seed();
        Out := Exec(R, Params('one', '')) + Exec(R, Params('two', ''));
        Clear(R);
        Out += Exec(R, Params('three', ''));
        exit(Out + Exec(R2, '') + Exec(R2, Params('two', '')));
    end;

    procedure Views(): Text
    var
        R1: Report "LT Y7 Exec Params";
        R2: Report "LT Y7 Exec Params";
        R3: Report "LT Y7 Exec Params";
        R4: Report "LT Y7 Exec Params";
        Row: Record "LT Y7 Exec Row";
    begin
        Seed();
        Row.SetRange(Id, 1, 3);
        R1.SetTableView(Row);
        exit(Exec(R1, Params('s', '<DataItem name="Row">VERSION(1) SORTING(Field1) WHERE(Field2=1(2))</DataItem>'))
            + Exec(R2, Params('d', '<DataItem name="Row">VERSION(1) SORTING(Field2,Field1) ORDER(Descending) WHERE(Field1=1(2..4))</DataItem>'))
            + Exec(R3, Params('u', '<DataItem name="Nope">VERSION(1) SORTING(Field1) WHERE(Field2=1(2))</DataItem>'))
            + Exec(R4, Params('e', '<DataItem name="Row"></DataItem>')));
    end;
}
"#;

#[test]
fn a_second_execute_of_one_variable_raises_whatever_the_parameters() {
    let ran = "ok|pre note=one filters=[] key=[Id] asc=Yes;get 1;get 2;get 3;get 4;#";
    let refused = "E[The object has been run. Use the function CLEAR(Report).|ObjectHasAlreadyBeenRun]|#";
    assert_eq!(
        text(run_in(SRC, "T", "Twice")),
        format!(
            "{ran}{refused}{}ok|pre note= filters=[] key=[Id] asc=Yes;get 1;get 2;get 3;get 4;#{refused}",
            ran.replace("one", "three")
        )
    );
}

#[test]
fn a_data_item_view_in_the_parameters_is_the_view_of_that_run() {
    assert_eq!(
        text(run_in(SRC, "T", "Views")),
        "ok|pre note=s filters=[Grp: 2] key=[Id] asc=Yes;get 3;get 4;#\
         ok|pre note=d filters=[Id: 2..4] key=[Grp,Id] asc=No;get 4;get 3;get 2;#\
         ok|pre note=u filters=[] key=[Id] asc=Yes;get 1;get 2;get 3;get 4;#\
         ok|pre note=e filters=[] key=[Id] asc=Yes;get 1;get 2;get 3;get 4;#"
    );
}
