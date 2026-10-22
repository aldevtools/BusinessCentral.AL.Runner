//! CalcFields of a Blob on a record that was read without it, modified, and handed through Codeunit.Run and on by
//! value: the stream must read the stored content.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50110 "Blob Run T"
{
    fields { field(1; K; Integer) { } field(2; B; Blob) { } field(3; N; Integer) { } field(4; Other; Blob) { } }
    keys { key(PK; K) { } }
}
codeunit 50111 "Blob Run Inner"
{
    TableNo = "Blob Run T";
    trigger OnRun()
    begin
        Load(Rec);
    end;
    local procedure Load(R: Record "Blob Run T")
    var
        Probe: Codeunit "Blob Run State";
        InS: InStream;
        S: Text;
    begin
        R.CalcFields(B);
        R.B.CreateInStream(InS);
        InS.ReadText(S);
        Probe.Put(Format(R.B.HasValue()) + ':' + S);
    end;
}
codeunit 50112 "Blob Run State"
{
    SingleInstance = true;
    var V: Text;
    procedure Put(S: Text) begin V := S; end;
    procedure Get(): Text begin exit(V); end;
}
codeunit 50113 "Blob Run Outer"
{
    TableNo = "Blob Run T";
    trigger OnRun()
    var
        E: Record "Blob Run T";
    begin
        E.Get(Rec.K);
        Clear(E.Other);
        E.N += 1;
        E.Modify(true);
        Codeunit.Run(Codeunit::"Blob Run Inner", E);
        Rec := E;
    end;
}
codeunit 50114 C
{
    procedure Main(): Text
    var
        T: Record "Blob Run T";
        Probe: Codeunit "Blob Run State";
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('<a/>');
        T.Modify();
        Codeunit.Run(Codeunit::"Blob Run Outer", T);
        exit(Probe.Get());
    end;
}
"#;

#[test]
fn calcfields_after_get_modify_and_codeunit_run_reads_the_stored_blob() {
    assert_eq!(text(run_in(SRC, "C", "Main")), "Yes:<a/>");
}
