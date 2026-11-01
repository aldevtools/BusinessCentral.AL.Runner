//! Page.Run and Page.RunModal with a field argument and a TestPage handler: the field changes nothing, the call answers as the
//! one without it (corpus pages/lt-page-run-overloads-testpart/P106RunStatic.al:141-866; al-runner issue 5753). A Variant that
//! holds a RecordRef runs the page on the RecordRef's record (pages/lt-ce-page-run/CePageRun.al:382, :548-676; al-runner issue
//! 5759); a Variant that holds an Integer or a Char raises a conversion error (CePageRun.al:654).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 69940 "FA Row" { LookupPageId = "FA List"; fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "Code") { } } }
page 69941 "FA List"
{
    PageType = List; SourceTable = "FA Row"; Editable = false; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
}
page 69942 "FA Card"
{
    PageType = Card; SourceTable = "FA Row"; UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } }
}
codeunit 69943 T
{
    Subtype = Test;
    var Seen: Text; Mode: Integer; GoTo: Code[20];
    local procedure Seed()
    var Row: Record "FA Row";
    begin
        Row.DeleteAll();
        Row."Code" := 'A'; Row.Name := 'Alpha'; Row.Insert();
        Row."Code" := 'B'; Row.Name := 'Bravo'; Row.Insert();
        Row."Code" := 'C'; Row.Name := 'Charlie'; Row.Insert();
        Seen := ''; GoTo := ''; Mode := 0;
    end;
    [Test] [HandlerFunctions('ListH')]
    procedure FieldArgPick()
    var Row: Record "FA Row"; Act: Action;
    begin
        Seed();
        Row.SetRange("Code", 'B', 'C');
        Row.FindFirst();
        GoTo := 'C';
        Act := Page.RunModal(Page::"FA List", Row, Row."Code");
        if Format(Act) + '|' + Seen + '|' + Row."Code" + '|' + Row.GetFilters() <> 'LookupOK|cur=B;rows=BC|C|Code: B..C' then
            Error('%1', Format(Act) + '|' + Seen + '|' + Row."Code" + '|' + Row.GetFilters());
    end;
    [Test] [HandlerFunctions('ListH')]
    procedure FieldArgHandlerDoesNothing()
    var Row: Record "FA Row"; Act: Action;
    begin
        Seed();
        Row.Get('A');
        GoTo := 'B';
        Mode := 3;
        Act := Page.RunModal(Page::"FA List", Row, Row.Name);
        if Format(Act) + '|' + Row."Code" <> 'LookupCancel|B' then
            Error('%1', Format(Act) + '|' + Row."Code");
    end;
    [Test] [HandlerFunctions('CardH')]
    procedure VariantRecRefCard()
    var Row: Record "FA Row"; RecRef: RecordRef; Back: RecordRef; V: Variant; Act: Action; S: Text;
    begin
        Seed();
        Row.Get('B');
        RecRef.GetTable(Row);
        V := RecRef;
        Mode := 0;
        Act := Page.RunModal(Page::"FA Card", V, 0);
        S := Format(Act) + '|' + Seen;
        Seen := '';
        Mode := 2;
        Act := Page.RunModal(Page::"FA Card", V, Row.FieldNo(Name));
        Back := V;
        S += '|' + Format(Act) + '|' + Seen + '|' + Format(RecRef.Field(2).Value()) + '|' + Format(Back.Field(2).Value()) + '|' + Row.Name;
        Row.Get('B');
        S += '|' + Row.Name;
        if S <> 'LookupCancel|card=B|LookupOK|card=B|Edited|Edited|Bravo|Edited' then
            Error('%1', S);
    end;
    [Test]
    procedure VariantNotARecord()
    var V: Variant; Act: Action; S: Text;
    begin
        V := 7;
        asserterror Act := Page.RunModal(Page::"FA Card", V, 0);
        S := GetLastErrorText() + '|' + GetLastErrorCode();
        V := 'B';
        asserterror Act := Page.RunModal(Page::"FA Card", V, 0);
        S += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
        if S <> 'Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavInteger to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle.|Conversion|' +
                 'Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavChar to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle.|Conversion' then
            Error('%1', S);
    end;
    [ModalPageHandler]
    procedure ListH(var P: TestPage "FA List")
    var Codes: Text;
    begin
        Seen := 'cur=' + P."Code".Value();
        if P.First() then
            repeat Codes += P."Code".Value(); until not P.Next();
        Seen += ';rows=' + Codes;
        if GoTo <> '' then P.GoToKey(GoTo);
        if Mode <> 3 then P.OK().Invoke();
    end;
    [ModalPageHandler]
    procedure CardH(var P: TestPage "FA Card")
    begin
        Seen := 'card=' + P."Code".Value();
        if Mode = 2 then begin
            P.Name.SetValue('Edited');
            P.OK().Invoke();
        end;
    end;
}
"#;

#[test]
fn a_field_argument_or_a_variant_holding_a_record_ref_runs_the_page_as_bc_does() {
    let o = outcomes_in(SRC, "T");
    assert_eq!(o.len(), 4);
    for (name, outcome) in o {
        assert_eq!(format!("{outcome:?}"), "Pass", "{name}");
    }
}
