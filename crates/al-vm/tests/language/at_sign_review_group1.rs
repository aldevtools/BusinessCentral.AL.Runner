//! Review neighbors of the temporary-record filter-group-1 sequence at AtSignAloneMethods.al:507.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 69620 "At Sign Review Group1 Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Kind; Code[20]) { }
        field(3; Note; Text[50]) { }
        field(5; Tag; Integer) { }
        field(4; Kind Lookup; Code[20])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("At Sign Review Group1 Row".Kind where("No." = field("No.")));
        }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69620 "At Sign Review Group1"
{
    procedure FreshTemporaryClear(): Integer
    var
        Row: Record "At Sign Review Group1 Row" temporary;
    begin
        Row.FilterGroup(1);
        Row.SetRange(Kind);
        Row.FilterGroup(0);
        exit(Row.Count());
    end;

    procedure MainIntegerSequence()
    var
        Row: Record "At Sign Review Group1 Row" temporary;
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, 1);
        Row.SetRange(Tag);
    end;

    procedure ExactSequence()
    var
        Row: Record "At Sign Review Group1 Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Row.FilterGroup(1);
        Row.SetFilter(Kind, '<>@');
        Row.FilterGroup(0);
        if Row.Count() <> 0 then
            Error('count');
        if Row.FindSet() then
            Error('findset');
        Row.FilterGroup(1);
        Row.SetRange(Kind);
        Row.SetRange(Note);
    end;

    procedure DatabaseSetter()
    var
        Db: Record "At Sign Review Group1 Row";
        Temp: Record "At Sign Review Group1 Row" temporary;
    begin
        Db."No." := 1;
        Db.Insert();
        Db.FilterGroup(1);
        Db.SetFilter(Kind, '<>@');
        Temp := Db;
        Temp.Insert();
        Temp.FilterGroup(0);
        Temp.FindSet();
    end;

    procedure FieldRefSetter()
    var
        Temp: Record "At Sign Review Group1 Row" temporary;
        R: RecordRef;
        F: FieldRef;
    begin
        Temp."No." := 1;
        Temp.Insert();
        R.GetTable(Temp);
        R.FilterGroup(1);
        F := R.Field(Temp.FieldNo(Kind));
        F.SetFilter('<>@');
    end;

    procedure FlowFieldClear()
    var Temp: Record "At Sign Review Group1 Row" temporary;
    begin
        Temp.FilterGroup(1);
        Temp.SetFilter("Kind Lookup", '<>@');
        Temp.SetRange("Kind Lookup");
    end;

    procedure DifferentFieldClear()
    var Temp: Record "At Sign Review Group1 Row" temporary;
    begin
        Temp.FilterGroup(1);
        Temp.SetFilter(Kind, '<>@');
        Temp.SetRange(Tag);
    end;
}
"#;

#[test]
fn a_clear_without_the_temporary_not_at_entry_is_refused() {
    match al_testkit::run_in(SRC, "At Sign Review Group1", "FreshTemporaryClear") {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains("SetRange in filter group 1 (the global group) on a temporary record"),
            "{why}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_main_temporary_integer_sequence_keeps_running() {
    assert!(al_testkit::run_in(SRC, "At Sign Review Group1", "MainIntegerSequence").is_ok());
}

#[test]
fn the_exact_temporary_sequence_still_runs() {
    assert!(al_testkit::run_in(SRC, "At Sign Review Group1", "ExactSequence").is_ok());
}

#[test]
fn a_temporary_reader_does_not_claim_a_database_setter() {
    let Err(Signal::Unsupported(why)) = al_testkit::run_in(SRC, "At Sign Review Group1", "DatabaseSetter") else {
        panic!("expected a refusal");
    };
    assert!(why.contains("another record set"), "{why}");
}

#[test]
fn a_fieldref_set_does_not_claim_the_record_sequence() {
    let Err(Signal::Unsupported(why)) = al_testkit::run_in(SRC, "At Sign Review Group1", "FieldRefSetter") else {
        panic!("expected a refusal");
    };
    assert!(why.contains("SetFilter in filter group 1"), "{why}");
}

#[test]
fn a_nonstored_field_does_not_claim_the_record_sequence() {
    let Err(Signal::Unsupported(why)) = al_testkit::run_in(SRC, "At Sign Review Group1", "FlowFieldClear") else {
        panic!("expected a refusal");
    };
    assert!(why.contains("SetFilter in filter group 1"), "{why}");
}

#[test]
fn a_clear_of_another_field_does_not_claim_the_record_sequence() {
    let Err(Signal::Unsupported(why)) = al_testkit::run_in(SRC, "At Sign Review Group1", "DifferentFieldClear") else {
        panic!("expected a refusal");
    };
    assert!(why.contains("SetRange in filter group 1"), "{why}");
}
