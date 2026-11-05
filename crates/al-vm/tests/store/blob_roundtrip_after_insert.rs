//! A Blob written through CreateOutStream after Insert, without Modify, reads back after CalcFields: on a temporary record and on
//! a database row, with and without Clear(Blob) first (al-runner issue 6791).

const SRC: &str = r#"
table 50980 "Br Row"
{
    fields { field(1; "Entry No."; Integer) { } field(10; Content; Blob) { } }
    keys { key(PK; "Entry No.") { } }
}
table 50981 "Br Temp Row"
{
    TableType = Temporary;
    fields { field(1; "Entry No."; Integer) { } field(10; Content; Blob) { } }
    keys { key(PK; "Entry No.") { } }
}
codeunit 50982 C
{
    local procedure Cell(var R: Record "Br Row"; No: Integer; WithClear: Boolean): Text
    var OutS: OutStream; InS: InStream; Txt: Text; Hv: Boolean;
    begin
        R.Init(); R."Entry No." := No; R.Insert(true);
        if WithClear then Clear(R.Content);
        R.Content.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('ROUNDTRIP');
        Hv := R.Content.HasValue();
        R.CalcFields(Content);
        R.Content.CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(Txt);
        exit(Format(Hv) + ':' + Txt);
    end;

    procedure TempVar(): Text
    var R: Record "Br Row" temporary; T: Text;
    begin
        T := Cell(R, 1, true) + '|' + Cell(R, 2, false);
        exit(T);
    end;

    procedure Database(): Text
    var R: Record "Br Row"; T: Text;
    begin
        T := Cell(R, 1, true) + '|' + Cell(R, 2, false);
        exit(T);
    end;

    procedure TempTable(): Text
    var R: Record "Br Temp Row"; OutS: OutStream; InS: InStream; Txt: Text;
    begin
        R.Init(); R."Entry No." := 1; R.Insert(false);
        Clear(R.Content);
        R.Content.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('ROUNDTRIP');
        if not R.Content.HasValue() then exit('no value');
        R.CalcFields(Content);
        R.Content.CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(Txt);
        exit(Txt);
    end;
}
"#;

#[test]
fn a_blob_written_after_insert_reads_back_on_a_temporary_variable() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "TempVar")),
        "Yes:ROUNDTRIP|Yes:ROUNDTRIP"
    );
}

#[test]
fn a_blob_written_after_insert_reads_back_on_a_temporary_table() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "TempTable")), "ROUNDTRIP");
}

#[test]
fn a_blob_written_after_insert_reads_back_on_a_database_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Database")),
        "Yes:ROUNDTRIP|Yes:ROUNDTRIP"
    );
}
