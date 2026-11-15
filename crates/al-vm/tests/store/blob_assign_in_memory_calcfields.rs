//! `Src.Data := Other.Data` where `Other` holds content a stream wrote in memory, then `CalcFields` on `Src` (a database
//! record): BC reads the row back. With no row (never inserted) the Blob is empty; over a stored row it shows the row's
//! content (streams/BlobCopyNeighbors.al:3102-3150 and :3280-3330, CalcFields_AfterAssign_ByValueAndCopy). A stream write
//! into `Src` itself keeps its content (main's answer).

const SRC: &str = r#"
table 50987 "Bx Row"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { } }
}
codeunit 50988 C
{
    local procedure W(var R: Record "Bx Row"; T: Text)
    var OutS: OutStream;
    begin
        R.Data.CreateOutStream(OutS);
        OutS.WriteText(T);
    end;

    local procedure Rd(var R: Record "Bx Row"): Text
    var InS: InStream; T: Text;
    begin
        if not R.Data.HasValue() then exit('nohv:');
        R.Data.CreateInStream(InS);
        InS.ReadText(T);
        exit('hv:' + T);
    end;

    procedure NeverInserted(): Text
    var Src: Record "Bx Row"; Other: Record "Bx Row"; Before: Text;
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        Before := Rd(Src);
        Src.CalcFields(Data);
        exit(Before + '|' + Rd(Src));
    end;

    procedure OverStored(): Text
    var Src: Record "Bx Row"; Other: Record "Bx Row"; Before: Text;
    begin
        Src.Key1 := 7;
        W(Src, 'alpha');
        Src.Insert();
        W(Other, 'beta');
        Src.Data := Other.Data;
        Before := Rd(Src);
        Src.CalcFields(Data);
        exit(Before + '|' + Rd(Src));
    end;

    procedure OwnWrite(): Text
    var Src: Record "Bx Row";
    begin
        Src.Key1 := 11;
        W(Src, 'gamma');
        Src.CalcFields(Data);
        exit(Rd(Src));
    end;
}
"#;

#[test]
fn an_assigned_blob_of_a_record_never_inserted_reads_empty_after_calcfields() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "NeverInserted")),
        "hv:beta|nohv:"
    );
}

#[test]
fn an_assigned_blob_over_a_stored_row_reads_the_row_after_calcfields() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "OverStored")),
        "hv:beta|hv:alpha"
    );
}

#[test]
fn an_own_stream_write_keeps_its_content_after_calcfields() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "OwnWrite")), "hv:gamma");
}
