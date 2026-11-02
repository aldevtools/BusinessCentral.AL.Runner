//! `Record.LoadFields` (statement and function form) with a FlowField or a FlowFilter field, on a record never fetched and on a
//! fetched one. Every literal is what real BC 28.5 answered: record/lt-r6795-loadfields-flow/R6795LoadFieldsFlow.al
//! (codeunit 94542, writer R6795, runner issue 6795).

use al_testkit::text;

const SRC: &str = r#"
table 50210 LF
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Decimal) { }
        field(4; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(LN where("Row Id" = field(Id))); }
        field(6; Flt; Integer) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50211 LN
{
    fields
    {
        field(1; "Row Id"; Integer) { }
        field(2; "Line No."; Integer) { }
    }
    keys { key(PK; "Row Id", "Line No.") { Clustered = true; } }
}

codeunit 50210 C
{
    local procedure Seed()
    var
        Row: Record LF;
        Line: Record LN;
    begin
        Row.Id := 1;
        Row.Name := 'one';
        Row.Qty := 5;
        Row.Insert();
        Line."Row Id" := 1;
        Line."Line No." := 1;
        Line.Insert();
        Line."Line No." := 2;
        Line.Insert();
    end;

    local procedure Flags(var Row: Record LF): Text
    begin
        exit(Format(Row.AreFieldsLoaded(Row.Id)) + Format(Row.AreFieldsLoaded(Row.Name)) + Format(Row.AreFieldsLoaded(Row.Qty)));
    end;

    [TryFunction]
    local procedure Stmt(var Row: Record LF; Spec: Integer)
    begin
        case Spec of
            1:
                Row.LoadFields(Row.Cnt);
            2:
                Row.LoadFields(Row.Flt, Row.Name);
            3:
                Row.LoadFields(Row.Name);
        end;
    end;

    // The reporter's shape: a record never fetched, the statement form with one FlowField, then Get and CalcFields.
    procedure NeverFetchedStatement(): Text
    var
        Row: Record LF;
        Txt: Text;
    begin
        Seed();
        Row.LoadFields(Row.Cnt);
        Txt := Flags(Row) + ' ';
        Row.Get(1);
        Txt += Format(Row.Cnt) + ' ';
        Row.CalcFields(Row.Cnt);
        exit(Txt + Format(Row.Cnt));
    end;

    procedure NeverFetchedFunction(): Text
    var
        Row: Record LF;
        Txt: Text;
    begin
        Seed();
        Txt := Format(Row.LoadFields(Row.Cnt)) + Format(Row.LoadFields(Row.Flt, Row.Name)) + Format(Row.LoadFields(Row.Name)) + ' ';
        Row.Id := 1;
        Txt += Format(Row.LoadFields(Row.Cnt)) + ' ' + Flags(Row);
        exit(Txt);
    end;

    procedure KeyAssignedStatement(): Text
    var
        Row: Record LF;
    begin
        Seed();
        Row.Id := 1;
        Row.LoadFields(Row.Cnt, Row.Name);
        exit(Flags(Row));
    end;

    procedure FetchedStatement(): Text
    var
        Row: Record LF;
        Txt: Text;
    begin
        Seed();
        Row.Get(1);
        if not Stmt(Row, 1) then
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        if not Stmt(Row, 2) then
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        Txt += Format(Stmt(Row, 3)) + ' ';
        Row.SetLoadFields(Row.Name);
        Row.FindFirst();
        if not Stmt(Row, 1) then
            Txt += GetLastErrorCode() + ' ';
        exit(Txt + Flags(Row) + ' ' + Format(Row.Cnt));
    end;

    procedure FetchedFunction(): Text
    var
        Row: Record LF;
        Txt: Text;
    begin
        Seed();
        Row.SetLoadFields(Row.Name);
        Row.Get(1);
        Txt := Format(Row.LoadFields(Row.Cnt)) + Format(Row.LoadFields(Row.Flt, Row.Name)) + Format(Row.LoadFields(Row.Name)) + ' ';
        exit(Txt + Flags(Row) + ' ' + Format(Row.Cnt));
    end;

    procedure RecRefFlow(): Text
    var
        Row: Record LF;
        RecRef: RecordRef;
    begin
        Seed();
        RecRef.Open(Database::LF);
        RecRef.LoadFields(4);
        exit('ran');
    end;
}
"#;

/// Statement_NeverFetched_Cnt: `ok`, flags loaded=NoNoNoNoNoNo, `Cnt=0` after the Get, `Cnt=2` after CalcFields (the FlowField is not calculated).
#[test]
fn statement_on_a_record_never_fetched_takes_a_flow_field_and_calculates_nothing() {
    assert_eq!(text(al_testkit::run(SRC, "NeverFetchedStatement")), "NoNoNo 0 2");
}

/// Function_NeverFetched_Cnt/Flt/NameCnt/FltName/Name all `No`; Function_KeyAssignedNeverFetched_Cnt `No`.
/// Statement_KeyAssignedNeverFetched_NameCnt: flags loaded=Yes in every field.
#[test]
fn function_on_a_record_never_fetched_answers_no_and_key_assigned_reads_loaded() {
    assert_eq!(text(al_testkit::run(SRC, "NeverFetchedFunction")), "NoNoNo No YesYesYes");
    assert_eq!(text(al_testkit::run(SRC, "KeyAssignedStatement")), "YesYesYes");
}

/// Statement_AfterGet_Cnt/FltName, Statement_AfterGet_Name `ok`, Statement_AfterGetUnderSetLoadFieldsName_Cnt, Statement_AfterFindFirst_Cnt:
/// DB:Ok for the first FlowField or FlowFilter in the list; nothing changes, Cnt reads 0 until CalcFields.
#[test]
fn statement_on_a_fetched_record_raises_for_a_flow_field() {
    let m = |f: &str| {
        format!(
            "DB:Ok:The field '{f}' in the 'LF' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields. "
        )
    };
    assert_eq!(
        text(al_testkit::run(SRC, "FetchedStatement")),
        format!("{}{}Yes DB:Ok YesYesNo 0", m("Cnt"), m("Flt"))
    );
}

/// Function_AfterGet_Cnt/NameCnt `No`, Function_AfterGet_Name `Yes`; Function_AfterGetUnderSetLoadFieldsName_*: same, Qty stays unloaded.
#[test]
fn function_on_a_fetched_record_answers_no_for_a_flow_field_and_changes_nothing() {
    assert_eq!(text(al_testkit::run(SRC, "FetchedFunction")), "NoNoYes YesYesNo 0");
}

/// RecordRef.LoadFields of a FlowField is not pinned (a probe is needed): refused, not run as the Record form.
#[test]
fn record_ref_load_fields_of_a_flow_field_is_refused() {
    let r = al_testkit::run(SRC, "RecRefFlow");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
