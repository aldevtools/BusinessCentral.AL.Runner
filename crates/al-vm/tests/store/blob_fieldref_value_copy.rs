//! `FieldRef.Value := Source.Blob` into a temporary record gives the field a Blob of its own: a later write to the source, and a
//! Clear of the record's field after the row was inserted, do not reach the stored row (al-runner issue 7935, the reporter's
//! repro; Microsoft's Temp Blob List Test passes on BC). It runs through assumption `fieldref-value-blob-copy`; a stored target
//! keeps main's result and is not flagged.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 50430 "RM Row"
{
    TableType = Temporary;
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { Clustered = true; } }
}

table 50431 "RM Stored"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { Clustered = true; } }
}

codeunit 50435 "RM Hold"
{
    var
        HB: Record "RM Row" temporary;

    procedure Fill(Txt: Text)
    var
        OS: OutStream;
    begin
        HB.Data.CreateOutStream(OS);
        OS.WriteText(Txt);
    end;

    procedure ToRec(var RR: RecordRef; FNo: Integer)
    var
        FR: FieldRef;
    begin
        FR := RR.Field(FNo);
        FR.Value := HB.Data;
    end;
}

codeunit 50436 "RM Lst"
{
    var
        Rec: Record "RM Row" temporary;

    procedure Add(H: Codeunit "RM Hold")
    var
        RR: RecordRef;
    begin
        Rec.Key1 := Rec.Count() + 1;
        Clear(Rec.Data);
        RR.GetTable(Rec);
        H.ToRec(RR, Rec.FieldNo(Data));
        RR.SetTable(Rec);
        Rec.Insert();
    end;

    procedure Text(K: Integer) Txt: Text
    var
        IS: InStream;
    begin
        Rec.Get(K);
        Rec.CalcFields(Data);
        Rec.Data.CreateInStream(IS);
        IS.ReadText(Txt);
    end;
}

codeunit 50437 C
{
    procedure SameHolderTwoAdds(): Text
    var
        L: Codeunit "RM Lst";
        H: Codeunit "RM Hold";
    begin
        H.Fill('alpha');
        L.Add(H);
        H.Fill('beta');
        L.Add(H);
        exit(L.Text(1) + '|' + L.Text(2));
    end;

    procedure StoredTargetRewriteSource(): Text
    var
        Src: Record "RM Row" temporary;
        RR: RecordRef;
        FR: FieldRef;
        Target: Record "RM Stored";
        OS: OutStream;
        IS: InStream;
        T: Text;
    begin
        Src.Data.CreateOutStream(OS);
        OS.WriteText('alpha');
        RR.Open(Database::"RM Stored");
        FR := RR.Field(2);
        FR.Value := Src.Data;
        Src.Data.CreateOutStream(OS);
        OS.WriteText('beta');
        RR.SetTable(Target);
        Target.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(T);
    end;

    procedure UnloadedSourceIntoTemporary(): Text
    var
        Src: Record "RM Row" temporary;
        Target: Record "RM Row" temporary;
        RR: RecordRef;
        OS: OutStream;
        IS: InStream;
        T: Text;
    begin
        Src.Key1 := 1;
        Src.Data.CreateOutStream(OS);
        OS.WriteText('alpha');
        Src.Insert();
        Src.Get(1);
        RR.GetTable(Target);
        RR.Field(2).Value := Src.Data;
        RR.SetTable(Target);
        Target.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(T);
    end;
}
"#;

#[test]
fn rows_added_before_a_refill_of_the_source_keep_their_content() {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", "SameHolderTwoAdds"));
    // Row 2 holds what the second Fill left: 'beta' written over 'alpha' without truncation
    // (streams/TestStreamsRuntimeContracts.al:BlobField_CreateOutStream_DoesNotEmptyExistingData).
    assert_eq!(out, "alpha|betaa");
    assert!(take_hits().contains(&Assumption::FieldrefValueBlobCopy));
}

#[test]
fn a_stored_target_keeps_mains_result_and_is_not_flagged() {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", "StoredTargetRewriteSource"));
    // Main's result: the field shares the source's buffer. Not pinned; docs/probes-needed.md row fieldref-value-blob-copy.
    assert_eq!(out, "betaa");
    assert!(!take_hits().contains(&Assumption::FieldrefValueBlobCopy));
}

#[test]
fn an_unloaded_source_keeps_mains_result_and_is_not_flagged() {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", "UnloadedSourceIntoTemporary"));
    // Main's result for a temporary row read back without CalcFields: not pinned through FieldRef.Value.
    assert_eq!(out, "");
    assert!(!take_hits().contains(&Assumption::FieldrefValueBlobCopy));
}
