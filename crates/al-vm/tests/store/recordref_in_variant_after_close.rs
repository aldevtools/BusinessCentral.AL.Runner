//! al-runner issue 6924: `V := RecRef; RecRef.Close();` leaves the Variant holding the open RecordRef (Format(V) is not
//! empty, `R := V` gives the table and the row), pinned by recordref-fieldref/recordref/RecordRefInVariantAfterClose.al. This
//! keeps the reporter's repro; no test here may mark an assumption.
//! Pinned and unchanged: the Variant's RecordRef shares the record buffer with the original (types/variant/CxVariants.al:194-198).

use al_builtins::assumptions::{Assumption, take_hits};

const SRC: &str = r#"
table 51760 "VC Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 51762 "VC Message"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Table Number"; Integer) { }
        field(3; "Record SystemId"; Guid) { }
        field(4; "Check Table Number"; Integer) { }
        field(5; "Check SystemId"; Guid) { }
        field(6; Description; Text[100]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }

    procedure CreateMessage(Desc: Text; SourceRecord: Variant; CheckRecord: Variant)
    var
        Message: Record "VC Message";
        RecRef: RecordRef;
        CheckRecRef: RecordRef;
        FldRef: FieldRef;
        CheckFldRef: FieldRef;
    begin
        GetRecordRef(SourceRecord, RecRef);
        FldRef := RecRef.Field(RecRef.SystemIdNo);
        Message.SetRange("Table Number", RecRef.Number);
        Message.SetRange("Record SystemId", FldRef.Value);
        if Format(CheckRecord) <> '' then begin
            GetRecordRef(CheckRecord, CheckRecRef);
            CheckFldRef := CheckRecRef.Field(CheckRecRef.SystemIdNo);
            Message.SetRange("Check Table Number", CheckRecRef.Number);
            Message.SetRange("Check SystemId", CheckFldRef.Value);
            if not Message.IsEmpty() then
                exit;
        end;
        Message.Reset();
        if Message.FindLast() then;
        Message."Entry No." += 1;
        Message."Table Number" := RecRef.Number;
        Message."Record SystemId" := FldRef.Value;
        Message.Description := CopyStr(Desc, 1, 100);
        if Format(CheckRecord) <> '' then begin
            Message."Check Table Number" := CheckRecRef.Number;
            Message."Check SystemId" := CheckFldRef.Value;
        end;
        Message.Insert();
    end;

    local procedure GetRecordRef(RecRelatedVariant: Variant; var ResultRecordRef: RecordRef): Boolean
    begin
        case true of
            RecRelatedVariant.IsRecord:
                ResultRecordRef.GetTable(RecRelatedVariant);
            RecRelatedVariant.IsRecordRef:
                ResultRecordRef := RecRelatedVariant;
            else
                exit(false);
        end;
        exit(true);
    end;
}

codeunit 51761 "VC Checks"
{
    local procedure Mock1(SourceRecord: Variant)
    var
        Message: Record "VC Message";
    begin
        Message.CreateMessage('first', SourceRecord, SourceRecord);
    end;

    local procedure Mock2(SourceRecord: Variant)
    var
        Message: Record "VC Message";
        Row: Record "VC Row";
    begin
        Row := SourceRecord;
        Message.CreateMessage('second ' + Format(Row.RecordId), SourceRecord, SourceRecord);
    end;

    procedure Dedupe(): Text
    var
        Row: Record "VC Row";
        Message: Record "VC Message";
        SourceRecord: Variant;
    begin
        Seed();
        Message.DeleteAll();
        Row.FindFirst();
        SourceRecord := Row;
        Mock1(SourceRecord);
        Mock2(SourceRecord);
        Message.FindLast();
        exit(Format(Message.Count()) + '|' + Message.Description + '|' + Format(Message."Check Table Number"));
    end;

    local procedure Seed()
    var
        Row: Record "VC Row";
    begin
        Row.DeleteAll();
        Row.Id := 7;
        Row.Name := 'seven';
        Row.Insert();
    end;

    local procedure Describe(V: Variant): Text
    var
        R: RecordRef;
    begin
        if Format(V) = '' then
            exit('empty');
        R := V;
        exit(Format(R.Number) + '|' + Format(R.Field(2).Value));
    end;

    // The shape of the report: assigned, the original closed, the Variant passed by value.
    procedure ClosedAfterAssign(): Text
    var
        RecRef: RecordRef;
        V: Variant;
    begin
        Seed();
        RecRef.Open(Database::"VC Row");
        RecRef.FindFirst();
        V := RecRef;
        RecRef.Close();
        exit(Format(V.IsRecordRef()) + '|' + Describe(V));
    end;

    // Pinned (CxVariants.al:194-198): a write through the ref taken out of the Variant reaches the original.
    procedure SharedBuffer(): Text
    var
        RecRef: RecordRef;
        Back: RecordRef;
        V: Variant;
    begin
        Seed();
        RecRef.Open(Database::"VC Row");
        RecRef.FindFirst();
        V := RecRef;
        Back := V;
        Back.Field(2).Value := 'via variant';
        exit(Format(RecRef.Field(2).Value));
    end;

    local procedure F(V: Variant): Text
    begin
        exit('[' + Format(V) + ']');
    end;

    procedure FormatCells(): Text
    var
        Row: Record "VC Row";
        Blank: Record "VC Row";
        Temp: Record "VC Row" temporary;
        Positioned: RecordRef;
        NoRow: RecordRef;
        Closed: RecordRef;
        V: Variant;
        R: Text;
    begin
        Seed();
        Row.FindFirst();
        V := Row;
        R := 'row' + F(V);
        V := Blank;
        R += ' blank' + F(V);
        Temp.Id := 3;
        Temp.Name := 'temp';
        Temp.Insert();
        V := Temp;
        R += ' temp' + F(V);
        Positioned.Open(Database::"VC Row");
        Positioned.FindFirst();
        V := Positioned;
        R += ' positioned' + F(V);
        NoRow.Open(Database::"VC Row");
        V := NoRow;
        R += ' norow' + F(V);
        V := Closed;
        R += ' closed' + F(V);
        exit(R);
    end;

    // No Close: nothing unpinned runs.
    procedure NoCloseAtAll(): Text
    var
        RecRef: RecordRef;
        V: Variant;
    begin
        Seed();
        RecRef.Open(Database::"VC Row");
        RecRef.FindFirst();
        V := RecRef;
        exit(Describe(V));
    end;
}
"#;

fn run(routine: &str) -> (String, Vec<Assumption>) {
    take_hits();
    let r = al_testkit::text(al_testkit::run_in(SRC, "VC Checks", routine));
    (r, take_hits())
}

#[test]
fn a_variant_keeps_its_recordref_when_the_original_is_closed() {
    let (r, hits) = run("ClosedAfterAssign");
    assert_eq!(r, "Yes|51760|seven");
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn the_recordref_in_a_variant_shares_the_buffer_and_is_not_flagged() {
    let (r, hits) = run("SharedBuffer");
    assert_eq!(r, "via variant");
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_recordref_in_a_variant_without_a_close_is_not_flagged() {
    let (r, hits) = run("NoCloseAtAll");
    assert_eq!(r, "51760|seven");
    assert!(hits.is_empty(), "{hits:?}");
}

/// What Format gives today for a Variant holding a Record or a RecordRef: the Format of the record (record/TestFormatRecordContracts.al
/// :111-145 pins it for a Record), and the empty text only for a RecordRef that is not open. Format of the Variant is not pinned; the probe
/// file observes these cells. No cell here is the empty text for a record, so that is not why the reporter's second test fails.
#[test]
fn format_of_a_variant_holding_a_record_or_an_open_recordref_is_not_empty() {
    let (r, hits) = run("FormatCells");
    assert_eq!(
        r,
        "row[7\tseven] blank[0\t] temp[3\ttemp] positioned[7\tseven] norow[0\t] closed[]"
    );
    assert!(hits.is_empty(), "{hits:?}");
}

/// The reporter's second shape (two messages for one record, the second skipped as a duplicate when `Format(CheckRecord) <> ''`),
/// with a Record in the Variant and no Close: the second message is skipped.
#[test]
fn a_record_in_a_variant_formats_to_text_and_the_duplicate_check_runs() {
    let (r, _) = run("Dedupe");
    assert_eq!(r, "1|first|51760");
}
