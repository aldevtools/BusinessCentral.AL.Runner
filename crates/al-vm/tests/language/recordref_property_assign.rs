//! The assignment forms of RecordRef and FieldRef members act as the call forms: `RR.Mark := b`, `RR.MarkedOnly := b`,
//! `RR.Ascending := b`, `RR.ChangeCompany := c`, `FR.Validate := v` (record/ProbePropertyAssignmentForms.al:709-878,
//! Y36_RecordRef_*). The Mark form inside a FindSet loop over a `var` RecordRef is the shape of System Application's
//! Retention Policy filtering (RetenPolFilteringImpl SetMarks).

use al_testkit::text;

const SRC: &str = r#"
table 69760 "RRP Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69761 C
{
    local procedure Seed()
    var R: Record "RRP Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Txt := 'd'; R.Insert();
        R.Id := 2; R.Txt := 'c'; R.Insert();
        R.Id := 3; R.Txt := 'b'; R.Insert();
        R.Id := 4; R.Txt := 'a'; R.Insert();
    end;

    local procedure SetMarks(var RR: RecordRef; Value: Boolean)
    begin
        if RR.FindSet() then
            repeat
                RR.Mark := Value;
            until RR.Next() = 0;
    end;

    procedure Marks(): Text
    var RR: RecordRef; Result: Text;
    begin
        Seed();
        RR.Open(Database::"RRP Row");
        RR.Field(1).SetRange(2, 3);
        SetMarks(RR, true);
        RR.Field(1).SetRange();
        RR.MarkedOnly(true);
        Result := Format(RR.Count());
        RR.MarkedOnly := false;
        Result += ' ' + Format(RR.Count()) + ' ' + Format(RR.MarkedOnly());
        RR.Ascending := false;
        RR.FindFirst();
        Result += ' ' + Format(RR.Field(1).Value()) + ' ' + Format(RR.Ascending());
        exit(Result);
    end;

    procedure Others(): Text
    var RR: RecordRef; FR: FieldRef;
    begin
        Seed();
        RR.Open(Database::"RRP Row");
        RR.ChangeCompany := CompanyName();
        RR.FindFirst();
        FR := RR.Field(2);
        FR.Validate := 'v';
        exit(Format(FR.Value()) + ' ' + Format(RR.Count()));
    end;
}
"#;

#[test]
fn mark_marked_only_and_ascending_assign_on_a_recordref() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Marks")), "2 4 No 4 No");
}

#[test]
fn change_company_and_fieldref_validate_assign() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Others")), "v 4");
}
