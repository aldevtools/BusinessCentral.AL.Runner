//! A RecordRef held by a Variant, assigned or passed by value to a Record (streams/ArRecordProbes.al:333, :366 and the
//! SG_VariantRecordRef_* tests of corpus bump PR 738): the Record gets the buffer's values on the ref's row, not its filters, key,
//! order or temporariness; another table and a ref that is not open raise.

use al_testkit::text;

const SRC: &str = r#"
table 50480 "RRR Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50481 "RRR Other" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50480 C
{
    local procedure Seed()
    var R: Record "RRR Row"; O: Record "RRR Other"; i: Integer;
    begin
        R.DeleteAll(); O.DeleteAll();
        for i := 1 to 4 do begin R.Id := i; R.Name := 'r' + Format(i); R.Insert(); end;
        O.Code := 'O1'; O.Insert();
    end;
    local procedure Describe(R: Record "RRR Row"): Text
    begin
        exit(Format(R.Id) + '|' + R.Name + '|' + R.GetFilters() + '|' + Format(R.Count()) + '|' + R.GetView(false) + '|' + Format(R.IsTemporary()));
    end;
    [TryFunction] local procedure TryAssign(V: Variant; var Row: Record "RRR Row") begin Row := V; end;
    [TryFunction] local procedure TryDescribe(V: Variant; var T: Text) begin T := Describe(V); end;
    procedure Param(): Text
    var RR: RecordRef; V: Variant;
    begin
        Seed();
        RR.Open(Database::"RRR Row");
        RR.SetView('SORTING(Id) ORDER(Descending) WHERE(Id=FILTER(2..3))');
        RR.FindFirst();
        RR.Field(2).Value := 'unsaved';
        V := RR;
        exit(Describe(V));
    end;
    procedure Assign(): Text
    var RR: RecordRef; V: Variant; Row: Record "RRR Row";
    begin
        Seed();
        RR.Open(Database::"RRR Row");
        RR.Field(1).SetRange(2, 3);
        RR.FindFirst();
        V := RR;
        Row := V;
        exit(Describe(Row) + '|' + Format(Row.Next()) + '|' + Format(Row.Id));
    end;
    procedure Temporary(): Text
    var RR: RecordRef; V: Variant; Row: Record "RRR Row";
    begin
        Seed();
        RR.Open(Database::"RRR Row", true);
        RR.Field(1).Value := 77;
        RR.Field(2).Value := 'temp';
        RR.Insert();
        V := RR;
        Row := V;
        exit(Describe(Row));
    end;
    procedure OtherTable(): Text
    var RR: RecordRef; V: Variant; Row: Record "RRR Row"; T: Text; Ok: Boolean; Out: Text;
    begin
        Seed();
        RR.Open(Database::"RRR Other");
        RR.FindFirst();
        V := RR;
        Row.Get(2);
        Ok := TryAssign(V, Row);
        Out := Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Row.Id);
        Ok := TryDescribe(V, T);
        exit(Out + '|' + Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + T);
    end;
    procedure NotOpen(): Text
    var RR: RecordRef; V: Variant; Row: Record "RRR Row"; Ok: Boolean;
    begin
        Seed();
        V := RR;
        Row.Get(2);
        Ok := TryAssign(V, Row);
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Row.Id) + '|' + Row.Name);
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

/// ArRecordProbes.al:366 (`3|r3||4`) and SG_VariantRecordRef_KeyViewAndUnsavedValue (`VERSION(1) SORTING(Field1)`).
#[test]
fn a_record_parameter_gets_the_buffer_without_filters_or_key() {
    assert_eq!(run("Param"), "3|unsaved||4|VERSION(1) SORTING(Field1)|No");
}

/// ArRecordProbes.al:333 VariantRecordRef_AssignedToRecord: `filters` empty, `count` 4, Next from row 2 gives 3.
#[test]
fn an_assigned_record_gets_the_row_without_filters() {
    assert_eq!(run("Assign"), "2|r2||4|VERSION(1) SORTING(Field1)|No|1|3");
}

/// SG_VariantRecordRef_Temporary_Assign: `77|temp`, not temporary, `count` 4.
#[test]
fn a_temporary_ref_gives_a_record_that_is_not_temporary() {
    assert_eq!(run("Temporary"), "77|temp||4|VERSION(1) SORTING(Field1)|No");
}

/// SG_VariantRecordRef_OtherTable_AssignAndParameter.
#[test]
fn another_table_raises_on_assignment_and_as_a_parameter() {
    assert_eq!(
        run("OtherTable"),
        "No|The requested operation is not supported.|NotSupportedOperation|2|No|The definition of the Code field has changed; old type: Integer, new type: Code. Start your activity again.|ObjectDefinitionChanged|"
    );
}

/// SG_VariantRecordRef_NotOpen_Assign.
#[test]
fn a_ref_that_is_not_open_raises_a_conversion_error() {
    assert_eq!(
        run("NotOpen"),
        "No|Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavRecordRef to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle.|Conversion|2|r2"
    );
}
