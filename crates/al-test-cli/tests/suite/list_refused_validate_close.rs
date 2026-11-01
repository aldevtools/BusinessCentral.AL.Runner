//! A list row whose OnValidate refused an entry: Close and Next store nothing of the row
//! (pages/lt-ah-pages/PwCloseRevalidate.al, codeunit 86157).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
codeunit 62900 "RV Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
}
table 62901 "RV Row"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Txt; Text[20]) { trigger OnValidate() var Log: Codeunit "RV Log"; begin Log.Add('VT:' + Txt); end; }
        field(3; Guard; Text[20])
        {
            trigger OnValidate() var Log: Codeunit "RV Log";
            begin Log.Add('VG:' + Guard); if Guard = 'bad' then Error('guard refused'); end;
        }
    }
    keys { key(PK; Id) { } }
    trigger OnInsert() var Log: Codeunit "RV Log"; begin Log.Add('I:' + Id); end;
    trigger OnModify() var Log: Codeunit "RV Log"; begin Log.Add('M:' + Id + '/' + Txt + '/' + Guard); end;
}
page 62902 "RV List"
{
    PageType = List; SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } field(GuardCtl; Rec.Guard) { } } } }
}
"#;

const TESTS: &str = r#"
    var Log: Codeunit "RV Log";
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Seed() var Row: Record "RV Row"; begin
        Row.DeleteAll(); Row.Id := 'R1'; Row.Txt := 'orig1'; Row.Insert(); Row.Id := 'R2'; Row.Txt := 'orig2'; Row.Insert(); Commit(); Log.Take(); end;
    local procedure Stored() R: Text var Row: Record "RV Row"; begin
        if Row.FindSet() then repeat R += Row.Id + '=' + Row.Txt + '/' + Row.Guard + ';'; until Row.Next() = 0; end;
    [TryFunction] local procedure TryClose(var P: TestPage "RV List") begin P.Close(); end;
    [TryFunction] local procedure TryNext(var P: TestPage "RV List") begin P.Next(); end;
    // PwCloseRevalidate.al:247 PW_List_RefusedOnValidate_ThenClose.
    [Test] procedure ThenClose() var P: TestPage "RV List"; begin
        Seed(); P.OpenEdit(); P.GoToKey('R1'); Log.Take();
        P.TxtCtl.SetValue('new1'); asserterror P.GuardCtl.SetValue('bad'); ClearLastError();
        Check('VT:new1;VG:bad;', Log.Take());
        Check('Yes', Format(TryClose(P))); Check('', Log.Take()); Check('R1=orig1/;R2=orig2/;', Stored()); end;
    // :270 PW_List_RefusedOnValidate_ThenNext_ThenClose.
    [Test] procedure ThenNext() var P: TestPage "RV List"; begin
        Seed(); P.OpenEdit(); P.GoToKey('R1'); Log.Take();
        P.TxtCtl.SetValue('new1'); asserterror P.GuardCtl.SetValue('bad'); ClearLastError(); Log.Take();
        Check('Yes', Format(TryNext(P))); Check('', Log.Take()); Check('R1=orig1/;R2=orig2/;', Stored());
        Check('Yes', Format(TryClose(P))); Check('', Log.Take()); Check('R1=orig1/;R2=orig2/;', Stored()); end;
    // :303 PW_List_NewRow_RefusedOnValidate_ThenClose.
    [Test] procedure NewRowThenClose() var P: TestPage "RV List"; begin
        Seed(); P.OpenEdit(); P.New(); Log.Take();
        P.IdCtl.SetValue('N1'); P.TxtCtl.SetValue('newrow'); Check('I:N1;VT:newrow;', Log.Take());
        asserterror P.GuardCtl.SetValue('bad'); ClearLastError(); Log.Take();
        Check('Yes', Format(TryClose(P))); Check('', Log.Take()); Check('R1=orig1/;R2=orig2/;', Stored()); end;
"#;

#[test]
fn a_list_row_with_a_refused_on_validate_is_dropped_by_close_and_next() {
    let src = format!("{OBJECTS}\ncodeunit 62990 T {{ Subtype = Test; {TESTS} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 3);
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}
