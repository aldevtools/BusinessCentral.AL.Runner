//! `Error('')` out of an OnLookup that `TestPage.<control>.Lookup()` starts: the call ends quietly. The test goes on, the last error
//! has the empty text and code Dialog, the value stays empty, the rows the OnLookup wrote before the error stay, and a later
//! SetValue is saved. This holds for the table field's OnLookup that ran a modal lookup page cancelled by the handler, for a control's
//! OnLookup, and for one that raises it with no page at all
//! (pages/empty-error-text/WcLookupEmptyError.al, WcTableFieldLookup_CancelEmptyError_NoAsserterror,
//! WcControlLookup_CancelEmptyError_NoAsserterror, WcControlLookup_DirectEmptyError_NoAsserterror; BC 27.5).
//! A non-empty text keeps raising, with the code wrapped as TestWrapped:Dialog (WcTableFieldLookup_CancelTextError_Asserterror).

const SRC: &str = r#"
table 69970 "LE Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; TabVal; Code[20]) { trigger OnLookup() begin Mark(1, 'tab before'); Error(''); end; }
        field(4; DirectVal; Code[20]) { }
        field(5; TextVal; Code[20]) { trigger OnLookup() begin Mark(4, 'text before'); Error('LE cancelled'); end; }
    }
    keys { key(PK; Id) { } }
    procedure Mark(Id: Integer; Name: Text)
    var M: Record "LE Mark";
    begin
        M.Id := Id; M.Name := CopyStr(Name, 1, MaxStrLen(M.Name)); M.Insert();
    end;
}
table 69971 "LE Mark"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Id) { } }
}
page 69972 "LE Card"
{
    PageType = Card; SourceTable = "LE Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(IdCtl; Rec.Id) { }
            field(NameCtl; Rec.Name) { }
            field(TabVal; Rec.TabVal) { }
            field(DirectVal; Rec.DirectVal)
            {
                trigger OnLookup(var Text: Text): Boolean
                begin
                    Rec.Mark(3, 'direct before error');
                    Error('');
                end;
            }
            field(TextVal; Rec.TextVal) { }
        }
    }
}
codeunit 69973 C
{
    local procedure Seed()
    var R: Record "LE Row";
    begin
        R.Init(); R.Id := 1; R.Name := 'seed'; R.Insert();
    end;
    local procedure Marks(): Text
    var M: Record "LE Mark"; Result: Text;
    begin
        if M.FindSet() then repeat Result += Format(M.Id) + '=' + M.Name + ';'; until M.Next() = 0;
        exit(Result);
    end;
    procedure TableField(): Text
    var P: TestPage "LE Card"; R: Record "LE Row"; Out: Text;
    begin
        Seed(); P.OpenEdit();
        P.TabVal.Lookup();
        Out := 'text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] value=[' + P.TabVal.Value() + '] marks=' + Marks();
        P.NameCtl.SetValue('after lookup');
        P.Close();
        R.Get(1);
        exit(Out + ' stored=' + R.Name);
    end;
    procedure Direct(): Text
    var P: TestPage "LE Card"; R: Record "LE Row"; Out: Text;
    begin
        Seed(); P.OpenEdit();
        P.DirectVal.Lookup();
        Out := 'text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] value=[' + P.DirectVal.Value() + '] marks=' + Marks();
        P.NameCtl.SetValue('after lookup');
        P.Close();
        R.Get(1);
        exit(Out + ' stored=' + R.Name + ' marks=' + Marks());
    end;
    procedure WithText(): Text
    var P: TestPage "LE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.TextVal.Lookup();
        exit('text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] marks=' + Marks());
    end;
}
"#;

#[test]
fn an_empty_error_out_of_a_table_field_on_lookup_ends_the_lookup_quietly() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "TableField")),
        "text=[] code=[Dialog] value=[] marks=1=tab before; stored=after lookup"
    );
}

#[test]
fn an_empty_error_out_of_a_control_on_lookup_ends_the_lookup_quietly() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Direct")),
        "text=[] code=[Dialog] value=[] marks=3=direct before error; stored=after lookup marks=3=direct before error;"
    );
}

#[test]
fn a_text_error_out_of_on_lookup_keeps_raising_with_the_wrapped_code() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "WithText")),
        "text=[LE cancelled] code=[TestWrapped:Dialog] marks="
    );
}
