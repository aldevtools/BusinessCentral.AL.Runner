//! `Record.Get` with fewer values than the primary key, where an omitted key field has an InitValue. The corpus pins one shape
//! (record/crud/GetFewerKeyValues.al Get_OneValue_OmittedFieldHasInitValue: plain values, a record that is not temporary and
//! holds the InitValue in the omitted field): the field is searched with its InitValue. Every other shape refuses. A Get whose
//! result is used answers false on a miss without naming the key, so it runs.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69790 "GOI Row"
{
    fields { field(1; A; Code[10]) { } field(2; N; Integer) { InitValue = 5; } field(3; Info; Text[30]) { } }
    keys { key(PK; A, N) { Clustered = true; } }
}
codeunit 69791 C
{
    local procedure Seed()
    var R: Record "GOI Row";
    begin
        R.DeleteAll();
        R.A := 'Q'; R.N := 0; R.Info := 'q-0'; R.Insert();
        R.N := 5; R.Info := 'q-5'; R.Insert();
        R.N := 7; R.Info := 'q-7'; R.Insert();
    end;

    procedure Pinned(): Text
    var R: Record "GOI Row";
    begin
        Seed();
        Clear(R);
        exit(Format(R.Get('Q')) + ' ' + R.Info);
    end;

    procedure UsedMiss(): Text
    var R: Record "GOI Row";
    begin
        Seed();
        if not R.Get('Z') then
            exit('No');
        exit('Yes');
    end;

    procedure HeldOther(): Text
    var R: Record "GOI Row";
    begin
        Seed();
        R.N := 7;
        exit(Format(R.Get('Q')) + ' ' + R.Info);
    end;

    procedure Temporary(): Text
    var R: Record "GOI Row" temporary;
    begin
        R.A := 'Q'; R.Info := 'q-5'; R.Insert();
        Clear(R);
        exit(Format(R.Get('Q')) + ' ' + R.Info);
    end;

    procedure NoValues(): Text
    var R: Record "GOI Row";
    begin
        Seed();
        exit(Format(R.Get()));
    end;

    procedure ByRecordId(): Text
    var R: Record "GOI Row"; RecId: RecordId;
    begin
        Seed();
        Evaluate(RecId, 'GOI Row: Q');
        exit(Format(R.Get(RecId)) + ' ' + R.Info);
    end;

    procedure RefByRecordId(): Text
    var RecRef: RecordRef; RecId: RecordId;
    begin
        Seed();
        Evaluate(RecId, 'GOI Row: Q');
        RecRef.Open(69790);
        exit(Format(RecRef.Get(RecId)));
    end;
}
"#;

fn outcome(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => format!("refused: {why}"),
        r => text(r),
    }
}

#[test]
fn the_pinned_shape_searches_the_init_value() {
    assert_eq!(outcome("Pinned"), "Yes q-5");
}

#[test]
fn a_used_get_that_finds_no_row_answers_false() {
    assert_eq!(outcome("UsedMiss"), "No");
}

#[test]
fn the_unpinned_shapes_refuse() {
    for routine in ["HeldOther", "Temporary", "NoValues", "ByRecordId", "RefByRecordId"] {
        let got = outcome(routine);
        assert!(
            got.starts_with("refused: ") && got.contains("InitValue"),
            "{routine}: {got}"
        );
    }
}
