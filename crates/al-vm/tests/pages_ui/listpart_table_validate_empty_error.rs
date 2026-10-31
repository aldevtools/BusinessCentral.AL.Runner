//! `Error('')` out of a table field's OnValidate, raised while a test edits the field on a ListPart inside a Card: the SetValue
//! ends quietly. The test goes on, the last error has the empty text and code NSValidateField:Dialog, the control shows the old
//! value, and a later edit of the same control is stored (misc/lt-y13-triage/Y13PageTriage.al:350-374, corpus master).

const SRC: &str = r#"
table 69960 "LV Head" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 69961 "LV Line"
{
    fields
    {
        field(1; HeadId; Integer) { }
        field(2; LineNo; Integer) { }
        field(3; Name; Text[30]) { trigger OnValidate() begin if Name = 'empty' then Error(''); end; }
    }
    keys { key(PK; HeadId, LineNo) { } }
}
page 69962 "LV Lines"
{
    PageType = ListPart; SourceTable = "LV Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec.LineNo) { } field(Name; Rec.Name) { } } } }
}
page 69963 "LV Card"
{
    PageType = Card; SourceTable = "LV Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } part(Lines; "LV Lines") { SubPageLink = HeadId = field(Id); } } }
}
codeunit 69964 C
{
    procedure F(): Text
    var H: Record "LV Head"; L: Record "LV Line"; P: TestPage "LV Card"; Out: Text;
    begin
        H.Id := 1; H.Insert();
        L.HeadId := 1; L.LineNo := 10000; L.Name := 'seed'; L.Insert();
        Commit();
        P.OpenEdit();
        P.GoToKey(1);
        P.Lines.First();
        P.Lines.Name.SetValue('empty');
        Out := 'went on text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] shown=' + P.Lines.Name.Value();
        P.Lines.Name.SetValue('fine');
        Out += ' then=' + P.Lines.Name.Value();
        P.Close();
        L.Get(1, 10000);
        exit(Out + ' stored=' + L.Name);
    end;
}
"#;

#[test]
fn an_empty_error_out_of_a_table_on_validate_on_a_list_part_in_a_card_ends_the_set_value_quietly() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "went on text=[] code=[NSValidateField:Dialog] shown=seed then=fine stored=fine"
    );
}
