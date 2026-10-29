//! Issue 6133: a fresh record is a copy of the table's initial field values, worked out once per session. A Blob field is a
//! shared buffer, so each fresh record still gets its own; a record parameter that is passed in takes the caller's values.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50130 "Template T"
{
    fields { field(1; K; Integer) { } field(2; B; Blob) { } field(3; D; Text[20]) { InitValue = 'init'; } }
    keys { key(PK; K) { } }
}
codeunit 50131 C
{
    local procedure Fill(): Boolean
    var
        T: Record "Template T";
        OutS: OutStream;
    begin
        // Each call's local record is fresh: the previous call's Blob content is not in it.
        if T.B.HasValue() then
            exit(false);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('x');
        exit(T.B.HasValue());
    end;

    local procedure ByValue(T: Record "Template T"): Text
    begin
        exit(T.D + Format(T.K));
    end;

    local procedure ByVar(var T: Record "Template T")
    begin
        T.D := 'set';
    end;

    procedure Run(): Text
    var
        A: Record "Template T";
        B: Record "Template T";
        OutS: OutStream;
        R: Text;
    begin
        if Fill() and Fill() then
            R := 'fresh;';
        A.B.CreateOutStream(OutS);
        OutS.WriteText('a');
        if not B.B.HasValue() then
            R += 'own;';
        R += B.D + ';';
        A.K := 7;
        A.D := 'v';
        R += ByValue(A) + ';';
        ByVar(A);
        R += A.D;
        exit(R);
    end;
}
"#;

#[test]
fn fresh_records_get_their_own_blob_and_parameters_the_callers_values() {
    assert_eq!(text(run_in(SRC, "C", "Run")), "fresh;own;init;v7;set");
}
