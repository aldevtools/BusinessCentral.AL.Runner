//! `Target.Field := Source.Blob` copies the value: a later Clear of the source or a new write to it does not reach the target
//! (al-runner issue 7128, the reporter's repro; it passes on BC 29).

const SRC: &str = r#"
table 50990 "PX Buffer"
{
    TableType = Temporary;
    fields { field(1; "Key"; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; "Key") { Clustered = true; } }
}
table 50991 "PX Line"
{
    fields { field(1; "Line No."; Integer) { } field(2; Body; Blob) { } field(3; Other; Blob) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }
}
codeunit 50992 C
{
    local procedure Fill(var Buffer: Record "PX Buffer"; Value: Text)
    var OutStr: OutStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText(Value);
    end;

    local procedure ReadBody(var Line: Record "PX Line" temporary): Text
    var InStr: InStream; Result: Text;
    begin
        Line.Body.CreateInStream(InStr);
        InStr.ReadText(Result);
        exit(Result);
    end;

    procedure ClearSourceAfterAssign(): Text
    var Buffer: Record "PX Buffer"; Line: Record "PX Line" temporary;
    begin
        Fill(Buffer, 'body');
        Line.Body := Buffer.Data;
        Clear(Buffer.Data);
        exit(ReadBody(Line));
    end;

    procedure RewriteSourceAfterAssign(): Text
    var Buffer: Record "PX Buffer"; Line: Record "PX Line" temporary;
    begin
        Fill(Buffer, 'body');
        Line.Body := Buffer.Data;
        Fill(Buffer, 'other');
        exit(ReadBody(Line));
    end;

    procedure ClearSourceAfterAssignThenInsert(): Text
    var Buffer: Record "PX Buffer"; Line: Record "PX Line" temporary;
    begin
        Line."Line No." := 1;
        Fill(Buffer, 'body');
        Line.Body := Buffer.Data;
        Clear(Buffer.Data);
        Line.Insert();
        Line.Reset();
        Line.FindFirst();
        Line.CalcFields(Body);
        exit(ReadBody(Line));
    end;
}
"#;

#[test]
fn a_clear_of_the_source_does_not_empty_the_assigned_field() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ClearSourceAfterAssign")),
        "body"
    );
}

#[test]
fn a_rewrite_of_the_source_does_not_reach_the_assigned_field() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "RewriteSourceAfterAssign")),
        "body"
    );
}

#[test]
fn a_clear_of_the_source_does_not_empty_the_inserted_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ClearSourceAfterAssignThenInsert")),
        "body"
    );
}
