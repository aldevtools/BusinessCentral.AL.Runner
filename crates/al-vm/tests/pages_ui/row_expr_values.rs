//! A list computes each control whose source is a function or an expression once per row, when it reads its rows: after
//! that row's OnAfterGetRecord. A read shows the value of the row the cursor stands on, also after FindFirstField and Next,
//! which run no trigger and no control function (misc/lt-y14-triage/Y14PageTriage.al:241 Trapped_FindFirstField_FunctionControl,
//! :274 OpenView_FindFirstField_FunctionControl, corpus master).

const SRC: &str = r#"
table 69960 "RX T" { fields { field(1; Code; Code[10]) { } field(2; Store; Text[20]) { } } keys { key(PK; Code) { } } }
table 69961 "RX U" { fields { field(1; Code; Code[10]) { } field(2; Info; Text[30]) { } } keys { key(PK; Code) { } } }
codeunit 69962 "RX Log"
{
    SingleInstance = true;
    var Text: Text;
    procedure Add(Entry: Text) begin Text += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Text; Text := ''; exit(R); end;
}
page 69963 "RX List"
{
    PageType = List; SourceTable = "RX T"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Code; Rec.Code) { }
                field(Store; Rec.Store) { }
                field(Info; Describe()) { }
            }
        }
    }
    var Link: Record "RX U"; Log: Codeunit "RX Log";
    local procedure Describe(): Text
    var Other: Record "RX U";
    begin
        Log.Add('describe(' + Rec.Code + ')');
        if Other.Get(Link.RecordId) then
            exit(Other.Info);
        exit('none');
    end;
    trigger OnAfterGetRecord()
    begin
        Log.Add('agr(' + Rec.Code + ')');
        Clear(Link);
        Link.Code := Rec.Code;
    end;
}
codeunit 69964 Opener { procedure Open(var T: Record "RX T") begin Page.Run(Page::"RX List", T); end; }
codeunit 69965 C
{
    var Log: Codeunit "RX Log";
    local procedure Seed()
    var T: Record "RX T"; U: Record "RX U";
    begin
        T.Code := 'A'; T.Store := 'X1'; T.Insert();
        T.Code := 'B'; T.Store := 'X2'; T.Insert();
        U.Code := 'A'; U.Info := 'info-A'; U.Insert();
        U.Code := 'B'; U.Info := 'info-B'; U.Insert();
        Log.Take();
    end;
    local procedure Shown(var P: TestPage "RX List"): Text
    begin
        exit(P.Code.Value() + '/' + P.Store.Value() + '/' + P.Info.Value() + ' ');
    end;
    local procedure Walk(var P: TestPage "RX List"): Text
    var Out: Text;
    begin
        Out := Log.Take() + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.Store, 'X2')) + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.Info, 'info-A')) + ' ' + Shown(P);
        Out += Format(P.Next()) + ' ' + Shown(P);
        exit(Out + '[' + Log.Take() + ']');
    end;
    procedure Viewed(): Text
    var P: TestPage "RX List";
    begin
        Seed();
        P.OpenView();
        exit(Walk(P));
    end;
    procedure Trapped(): Text
    var T: Record "RX T"; P: TestPage "RX List"; O: Codeunit Opener;
    begin
        Seed();
        T.SetFilter(Code, 'A|B');
        P.Trap();
        O.Open(T);
        exit(Walk(P));
    end;
}
"#;

const WANT: &str = "agr(A);describe(A);agr(B);describe(B);agr(A); A/X1/info-A \
                    Yes B/X2/info-B Yes A/X1/info-A Yes B/X2/info-B []";

#[test]
fn an_opened_list_shows_the_function_value_each_row_had_when_it_was_read() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Viewed")), WANT);
}

#[test]
fn a_trapped_list_shows_the_function_value_each_row_had_when_it_was_read() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Trapped")), WANT);
}
