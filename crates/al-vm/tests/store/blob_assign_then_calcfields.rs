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

    // Neighbor: the source holds an unsaved write of its own (never inserted). Outside the assumption: the copy keeps the
    // write as before this rule.
    procedure NeighborUnsavedSource(): Text
    var
        Row: Record "PX Blob Row";
        Source: Record "PX Blob Row" temporary;
        OutS: OutStream;
    begin
        Row."No." := 'C';
        Row.Insert();
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Row.Content := Source.Content;
        Row.CalcFields(Content);
        exit(Format(Row.Content.HasValue()));
    end;
    // Neighbor: a stored source read with Get and CalcFields. Outside the assumption: main's result.
    procedure NeighborStoredSource(): Text
    var
        Row: Record "PX Blob Row";
        Source: Record "PX Blob Row";
        OutS: OutStream;
    begin
        Row."No." := 'D';
        Row.Insert();
        Source."No." := 'S';
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Source.Insert();
        Clear(Source);
        Source.Get('S');
        Source.CalcFields(Content);
        Row.Content := Source.Content;
        Row.CalcFields(Content);
        exit(Format(Row.Content.HasValue()));
    end;

    // Neighbor: a stored target whose Blob already holds content. Outside the assumption: main's result.
    procedure NeighborTargetWithContent(): Text
    var
        Row: Record "PX Blob Row";
        Source: Record "PX Blob Row" temporary;
        OutS: OutStream;
        InS: InStream;
        Txt: Text;
    begin
        Row."No." := 'E';
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText('old');
        Row.Insert();
        Source."No." := 'S';
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Source.Insert();
        Row.Content := Source.Content;
        Row.CalcFields(Content);
        Row.Content.CreateInStream(InS);
        InS.ReadText(Txt);
        exit(Txt);
    end;

    // Neighbor: a stored target read with Get and no CalcFields, the stored Blob with content. Outside the assumption.
    procedure NeighborUnloadedTarget(): Text
    var
        Row: Record "PX Blob Row";
        Source: Record "PX Blob Row" temporary;
        OutS: OutStream;
        InS: InStream;
        Txt: Text;
    begin
        Row."No." := 'F';
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText('old');
        Row.Insert();
        Clear(Row);
        Row.Get('F');
        Source."No." := 'S';
        Source.Content.CreateOutStream(OutS);
        OutS.WriteText('hello');
        Source.Insert();
        Row.Content := Source.Content;
        Row.CalcFields(Content);
        Row.Content.CreateInStream(InS);
        InS.ReadText(Txt);
        exit(Txt);
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

/// The result as before this rule, unflagged.
#[test]
fn a_source_with_an_unsaved_write_is_copied_as_before() {
    assert_eq!(run("NeighborUnsavedSource"), ("Yes".into(), false));
}

/// Main's result, unflagged: the gate needs the source a temporary record's Insert stored.
#[test]
fn a_stored_source_read_with_calcfields_is_copied_as_before() {
    assert_eq!(run("NeighborStoredSource"), ("No".into(), false));
}

/// Main's result, unflagged: the gate needs a target Blob without content.
#[test]
fn a_target_with_content_is_copied_as_before() {
    assert_eq!(run("NeighborTargetWithContent"), ("old".into(), false));
}

/// Main's result, unflagged: the gate needs a loaded target Blob.
#[test]
fn an_unloaded_target_is_copied_as_before() {
    assert_eq!(run("NeighborUnloadedTarget"), ("old".into(), false));
}
