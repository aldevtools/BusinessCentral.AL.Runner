//! FieldRef filters with an empty value (recordref-fieldref/fieldref/FieldRefFilterEmptyText.al): SetRange of an empty
//! DateFormula or RecordId reads back as two quotes, and `<>` plus that text filters the rows with a value (al-runner issue
//! 7437); two quotes and `<>''` on a Blob field (al-runner issue 7426).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69990 "FE Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Due; DateFormula) { }
        field(3; Rid; RecordId) { }
        field(4; Data; Blob) { }
        field(5; Due2; DateFormula) { }
    }
    keys { key(PK; "Code") { } }
}
codeunit 69991 T
{
    local procedure Seed()
    var R: Record "FE Row"; OutS: OutStream;
    begin
        R.Code := 'NONE1'; R.Insert();
        R.Code := 'NONE2'; R.Insert();
        R.Code := 'DAY';
        Evaluate(R.Due, '<1D>'); Evaluate(R.Due2, '<1D>');
        R.Rid := R.RecordId();
        R.Data.CreateOutStream(OutS); OutS.WriteText('abc');
        R.Insert();
        R.Code := 'MONTH';
        Evaluate(R.Due, '<1M>'); Clear(R.Due2);
        R.Rid := R.RecordId();
        Clear(R.Data);
        R.Insert();
    end;

    local procedure Rows(var RecRef: RecordRef): Text
    var Result: Text;
    begin
        if RecRef.FindSet() then
            repeat
                Result += Format(RecRef.Field(1).Value()) + ',';
            until RecRef.Next() = 0;
        exit(Result);
    end;

    [TryFunction]
    local procedure TryFilter(var FldRef: FieldRef; Expr: Text) begin FldRef.SetFilter(Expr); end;

    local procedure Cell(var RecRef: RecordRef; FieldNo: Integer; Expr: Text): Text
    var FldRef: FieldRef;
    begin
        RecRef.Reset();
        FldRef := RecRef.Field(FieldNo);
        if not TryFilter(FldRef, Expr) then
            exit('E: ' + GetLastErrorText() + ' [' + GetLastErrorCode() + '] ');
        exit(FldRef.GetFilter() + ' | ' + RecRef.GetFilters() + ' | ' + Rows(RecRef) + ' ');
    end;

    procedure EmptyDateFormula(): Text
    var RecRef: RecordRef; FldRef: FieldRef; Empty: DateFormula; T: Text; R: Text;
    begin
        Seed();
        RecRef.Open(Database::"FE Row");
        FldRef := RecRef.Field(2);
        FldRef.SetRange(Empty);
        T := FldRef.GetFilter();
        R := T + ' | ' + RecRef.GetFilters() + ' | ' + Rows(RecRef) + ' ';
        exit(R + Cell(RecRef, 2, '<>' + T) + Cell(RecRef, 5, '<>' + T) + Cell(RecRef, 5, T));
    end;

    procedure EmptyRecordId(): Text
    var RecRef: RecordRef; FldRef: FieldRef; Empty: RecordId; T: Text; R: Text;
    begin
        Seed();
        RecRef.Open(Database::"FE Row");
        FldRef := RecRef.Field(3);
        FldRef.SetRange(Empty);
        T := FldRef.GetFilter();
        R := T + ' | ' + RecRef.GetFilters() + ' | ' + Rows(RecRef) + ' ';
        exit(R + Cell(RecRef, 3, '<>' + T) + Cell(RecRef, 3, T));
    end;

    procedure Blob(Expr: Text): Text
    var RecRef: RecordRef;
    begin
        Seed();
        RecRef.Open(Database::"FE Row");
        exit(Cell(RecRef, 4, Expr));
    end;

    procedure BlobEmpty(): Text begin exit(Blob('')); end;
    procedure BlobQuotes(): Text begin exit(Blob('''''')); end;
    procedure BlobNeQuotes(): Text begin exit(Blob('<>''''')); end;
    procedure BlobNeBare(): Text begin exit(Blob('<>')); end;
    procedure BlobText(): Text begin exit(Blob('abc')); end;
    procedure BlobStar(): Text begin exit(Blob('*')); end;
    procedure BlobNeStar(): Text begin exit(Blob('<>*')); end;
    procedure BlobGtText(): Text begin exit(Blob('>abc')); end;
    procedure BlobRange(): Text begin exit(Blob('abc..abd')); end;
    procedure BlobQuoted(): Text begin exit(Blob('''abc''')); end;
    procedure BlobUnterminated(): Text begin exit(Blob('''')); end;
}
"#;

// FieldRefFilterEmptyText.al:120-151.
#[test]
fn setrange_of_an_empty_dateformula_reads_back_as_two_quotes() {
    assert_eq!(
        text(run_in(SRC, "T", "EmptyDateFormula")),
        "'' | Due: '' | NONE1,NONE2, <>'' | Due: <>'' | DAY,MONTH, <>'' | Due2: <>'' | DAY, '' | Due2: '' | MONTH,NONE1,NONE2, "
    );
}

// FieldRefFilterEmptyText.al:153-190.
#[test]
fn setrange_of_an_empty_recordid_reads_back_as_two_quotes() {
    assert_eq!(
        text(run_in(SRC, "T", "EmptyRecordId")),
        "'' | Rid: '' | NONE1,NONE2, <>'' | Rid: <>'' | DAY,MONTH, '' | Rid: '' | NONE1,NONE2, "
    );
}

// FieldRefFilterEmptyText.al:192-240 and FieldRefFilterMatrix.al:3063-3250 (Filter_Blob_*).
#[test]
fn text_filters_on_a_blob_field() {
    let cell = |routine: &str| text(run_in(SRC, "T", routine));
    assert_eq!(cell("BlobEmpty"), " |  | DAY,MONTH,NONE1,NONE2, ");
    assert_eq!(cell("BlobQuotes"), "' ' | Data: ' ' | MONTH,NONE1,NONE2, ");
    assert_eq!(cell("BlobNeQuotes"), "<>' ' | Data: <>' ' | DAY, ");
    assert_eq!(
        cell("BlobNeBare"),
        "E: The filter \"<>\" is not valid for the Data field on the FE Row table. A value is expected after the '<>'. [Filter:CSide] "
    );
    assert_eq!(
        cell("BlobText"),
        "E: The filter \"abc\" is not valid for the Data field on the FE Row table. The value \"abc\" can't be evaluated into type BLOB. [Filter:CSide] "
    );
    assert_eq!(cell("BlobStar"), "* | Data: * | DAY, ");
    assert_eq!(cell("BlobNeStar"), "<>* | Data: <>* | MONTH,NONE1,NONE2, ");
    assert_eq!(
        cell("BlobQuoted"),
        "E: The filter \"'abc'\" is not valid for the Data field on the FE Row table. The value \"abc\" can't be evaluated into type BLOB. [Filter:CSide] "
    );
}
