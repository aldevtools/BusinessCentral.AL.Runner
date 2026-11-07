//! A record copy (`Dest2 := Dest`) that gets a Blob through TransferFields and is modified writes the Blob to the
//! database row (al-runner issue 6924, Microsoft's Copy Document test CopyWorkDescriptionSalesArchiveShipment).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50120 "Blob Src"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Note; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50121 "Blob Dst"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Note; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50120 "Blob Copy Tests"
{
    Subtype = Test;

    local procedure ReadNote(var D: Record "Blob Dst"): Text
    var
        I: InStream;
        T: Text;
    begin
        D.CalcFields(Note);
        D.Note.CreateInStream(I);
        I.ReadText(T);
        exit(T);
    end;

    local procedure MakeSource(var S: Record "Blob Src"; Key: Code[10])
    var
        O: OutStream;
    begin
        S.Init();
        S."No." := Key;
        S.Note.CreateOutStream(O);
        O.WriteText('hello');
        S.Insert();
        S.CalcFields(Note);
    end;

    local procedure Fill(var Target: Record "Blob Dst"; Source: Record "Blob Src")
    begin
        Target.TransferFields(Source, false);
        Target.Modify();
    end;

    [Test]
    procedure TransferThroughTheSameRecord()
    var
        S: Record "Blob Src";
        D: Record "Blob Dst";
    begin
        MakeSource(S, 'A');
        D."No." := 'A';
        D.Insert();
        Fill(D, S);
        if ReadNote(D) <> 'hello' then Error('got %1', ReadNote(D));
    end;

    [Test]
    procedure TransferThroughACopyOfTheRecord()
    var
        S: Record "Blob Src";
        D: Record "Blob Dst";
        D2: Record "Blob Dst";
    begin
        MakeSource(S, 'B');
        D."No." := 'B';
        D.Insert();
        D2 := D;
        Fill(D2, S);
        if ReadNote(D) <> 'hello' then Error('got %1', ReadNote(D));
    end;
}
"#;

#[test]
fn blob_transferred_into_a_record_copy_reaches_the_database() {
    for (n, r) in outcomes_in(SRC, "Blob Copy Tests") {
        assert!(matches!(r, al_testrun::Outcome::Pass), "{n}: {r:?}");
    }
}
