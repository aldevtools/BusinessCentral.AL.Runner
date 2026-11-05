//! `Clear(Rec.BlobField)` inside OnModify, where `xRec` holds the same Blob buffer as `Rec`. No corpus test pins what `xRec`
//! shows after it (searched corpus master 6895fa4b for xRec Blob reads). The in-place Clear of al-runner issue 6791 must not
//! change these shapes: `xRec` keeps the content it had before the Clear, as before that change.

const SRC: &str = r#"
table 50983 "Bx Row"
{
    fields { field(1; "Entry No."; Integer) { } field(10; Content; Blob) { } }
    keys { key(PK; "Entry No.") { } }
    trigger OnModify()
    var OutS: OutStream; InS: InStream; Txt: Text;
    begin
        Clear(Rec.Content);
        Rec.Content.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('NEW');
        xRec.Content.CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(Txt);
        Seen := Txt;
    end;
    procedure GetSeen(): Text begin exit(Seen); end;
    var Seen: Text;
}
codeunit 50984 C
{
    local procedure Run(var R: Record "Bx Row"): Text
    var OutS: OutStream;
    begin
        R.Init(); R."Entry No." := 1;
        R.Content.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('OLD');
        R.Insert();
        R.Get(1);
        R.CalcFields(Content);
        R.Modify(true);
        exit(R.GetSeen());
    end;

    procedure Database(): Text
    var R: Record "Bx Row";
    begin
        exit(Run(R));
    end;

    procedure TempVar(): Text
    var R: Record "Bx Row" temporary;
    begin
        exit(Run(R));
    end;
}
"#;

#[test]
fn a_clear_in_on_modify_leaves_the_xrec_blob_of_a_database_record() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Database")), "OLD");
}

#[test]
fn a_clear_in_on_modify_leaves_the_xrec_blob_of_a_temporary_record() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "TempVar")), "OLD");
}
