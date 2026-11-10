//! ModifyAll(.., true) under a filter: the Rec and xRec of the OnBeforeModifyEvent subscriber carry the caller's filters and
//! current key, as a Modify does (corpus record/events/Y7408XRecInModifyEvent.al:310 Y7408_Matrix_ModifyFalse_ModifyAll_Rename,
//! `ModifyAll(true) under a filter`: rec filters=[Company: 0001] hasfilter=Yes; xrec the same; copy find=Yes; al-runner issue 7408).

use al_testkit::text;

const SRC: &str = r#"
table 50881 "MF Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Company; Code[20]) { } field(3; Member; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50882 "MF Sub"
{
    SingleInstance = true;
    var Log: Text;
    [EventSubscriber(ObjectType::Table, Database::"MF Row", OnBeforeModifyEvent, '', false, false)]
    local procedure OnBeforeModify(var Rec: Record "MF Row"; var xRec: Record "MF Row")
    var Copied: Record "MF Row";
    begin
        Copied.Copy(xRec);
        Log += '[' + Rec.GetFilters() + '|' + xRec.GetFilters() + '|' + Format(Copied.Find()) + ']';
    end;
    procedure Take(): Text
    var R: Text;
    begin
        R := Log; Log := ''; exit(R);
    end;
}
codeunit 50883 C
{
    procedure F(): Text
    var Row: Record "MF Row"; Sub: Codeunit "MF Sub";
    begin
        Row.DeleteAll();
        Row."No." := 'CT1'; Row.Company := '0001'; Row.Insert();
        Row."No." := 'CT2'; Row.Company := '0002'; Row.Insert();
        Row."No." := 'CT3'; Row.Company := '0001'; Row.Insert();
        Sub.Take();
        Row.SetRange(Company, '0001');
        Row.ModifyAll(Member, 'M-ALL', true);
        exit(Sub.Take());
    end;
}
"#;

#[test]
fn the_modify_event_of_a_modify_all_sees_the_callers_filters() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "[Company: 0001|Company: 0001|Yes][Company: 0001|Company: 0001|Yes]"
    );
}
