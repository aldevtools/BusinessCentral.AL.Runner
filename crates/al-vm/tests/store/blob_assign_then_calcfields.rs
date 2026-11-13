//! A Blob assigned from another record's Blob, then `CalcFields` on the target with no Modify in between (al-runner issue
//! 7961, the reporter's repro, measured by the reporter on real BC): the target keeps the assigned value, as it keeps one a
//! stream wrote. Assumption `blob-assign-counts-as-write`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 50300 "PX Blob Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Content; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50301 C
{
    procedure AssignFromTempThenCalcFields(): Text
    var
        Row: Record "PX Blob Row";
        Source: Record "PX Blob Row" temporary;
        OutS: OutStream;
        Before: Boolean;
    begin
        Row."No." := 'A';
        Row.Insert();
        Source."No." := 'S';
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Source.Insert();

        Row.Content := Source.Content;
        Before := Row.Content.HasValue();
        Row.CalcFields(Content);
        exit(StrSubstNo('before=%1 after=%2', Before, Row.Content.HasValue()));
    end;

    procedure WriteThenCalcFields(): Text
    var
        Row: Record "PX Blob Row";
        OutS: OutStream;
        Before: Boolean;
    begin
        Row."No." := 'B';
        Row.Insert();
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Before := Row.Content.HasValue();
        Row.CalcFields(Content);
        exit(StrSubstNo('before=%1 after=%2', Before, Row.Content.HasValue()));
    end;

    // Neighbor: the target is a temporary record. CalcFields of a temporary record always reads the row.
    procedure NeighborTemporaryTarget(): Text
    var
        Row: Record "PX Blob Row" temporary;
        Source: Record "PX Blob Row" temporary;
        OutS: OutStream;
    begin
        Row."No." := 'A';
        Row.Insert();
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Source.Insert();
        Row.Content := Source.Content;
        Row.CalcFields(Content);
        exit(Format(Row.Content.HasValue()));
    end;
}
"#;

fn run(routine: &str) -> (String, bool) {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    (out, take_hits().contains(&Assumption::BlobAssignCountsAsWrite))
}

#[test]
fn calcfields_keeps_a_blob_assigned_from_a_saved_temporary_record() {
    assert_eq!(
        run("AssignFromTempThenCalcFields"),
        ("before=Yes after=Yes".into(), true)
    );
}

#[test]
fn calcfields_keeps_a_blob_a_stream_wrote_unflagged() {
    assert_eq!(run("WriteThenCalcFields"), ("before=Yes after=Yes".into(), false));
}

/// The result as before this rule.
#[test]
fn a_temporary_target_reads_its_row_as_before() {
    assert_eq!(run("NeighborTemporaryTarget").0, "No");
}
