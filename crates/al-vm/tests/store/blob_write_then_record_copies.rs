//! A Blob written through CreateOutStream on an inserted row, then copied by record assignment (a by-value parameter that a
//! codeunit stores with `GlobalRec := Rec`, read back with `Rec := GlobalRec` into a var parameter), keeps the write after
//! CalcFields on the last copy (al-runner issue 7360, the reporter's real-BC run on BC 29).

const SRC: &str = r#"
table 50990 "Bc Row"
{
    fields { field(1; "No."; Integer) { } field(10; "Filter Blob"; Blob) { } }
    keys { key(PK; "No.") { } }
}
codeunit 50991 Holder
{
    var GlobalRec: Record "Bc Row";
    procedure SetRec(Rec: Record "Bc Row") begin GlobalRec := Rec; end;
    procedure GetRec(var Rec: Record "Bc Row") begin Rec := GlobalRec; end;
}
codeunit 50992 C
{
    procedure ViaHolder(WithClear: Boolean): Text
    var MyRec: Record "Bc Row"; Copy1: Record "Bc Row"; Copy2: Record "Bc Row"; H: Codeunit Holder;
        OutS: OutStream; InS: InStream; Txt: Text;
    begin
        MyRec.Init(); MyRec."No." := 1; MyRec.Insert(true);
        Copy1 := MyRec;
        if WithClear then Clear(Copy1."Filter Blob");
        Copy1."Filter Blob".CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('VERSION(1) SORTING(No.) WHERE(No.=FILTER(X))');
        H.SetRec(Copy1);
        H.GetRec(Copy2);
        Copy2.CalcFields("Filter Blob");
        Copy2."Filter Blob".CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(Txt);
        exit(Txt);
    end;

    procedure WithClear(): Text begin exit(ViaHolder(true)); end;
    procedure NoClear(): Text begin exit(ViaHolder(false)); end;

    procedure DirectCopy(): Text
    var MyRec: Record "Bc Row"; Copy1: Record "Bc Row"; OutS: OutStream; InS: InStream; Txt: Text;
    begin
        MyRec.Init(); MyRec."No." := 1; MyRec.Insert(true);
        MyRec."Filter Blob".CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('DIRECT');
        Copy1 := MyRec;
        Copy1.CalcFields("Filter Blob");
        Copy1."Filter Blob".CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(Txt);
        exit(Txt);
    end;
}
"#;

const TEXT: &str = "VERSION(1) SORTING(No.) WHERE(No.=FILTER(X))";

#[test]
fn a_blob_write_survives_record_copies_through_a_holder_after_clear() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "WithClear")), TEXT);
}

#[test]
fn a_blob_write_survives_record_copies_through_a_holder() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NoClear")), TEXT);
}

#[test]
fn a_blob_write_survives_one_record_copy_of_an_inserted_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "DirectCopy")), "DIRECT");
}
