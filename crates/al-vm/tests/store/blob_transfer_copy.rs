//! A Blob that moves through `FieldRef.Value :=`, `FieldRef.Validate`, `RecordRef.GetTable` / `SetTable`, `TransferFields` or a
//! Variant is a copy taken at that moment (assumption `blob-transfer-copies`: corpus PR 1369,
//! recordref-fieldref/fieldref/FieldRefValueBlobCopy.al; al-runner issue 7935). The shapes below repeat cells of that file; the
//! expected values are the ones it asserts. `hv:` / `nohv:` is `HasValue`, then the text a stream reads.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
table 50430 "RM Tmp"
{
    TableType = Temporary;
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { Clustered = true; } }
}

table 50431 "RM Row"
{
    fields
    {
        field(1; Key1; Integer) { }
        field(2; Data; Blob) { }
        field(3; Other; Blob) { }
        field(4; Plain; Blob) { Compressed = false; }
        field(5; Flow; Blob) { FieldClass = FlowField; CalcFormula = lookup("RM Row".Data where(Key1 = field(Key1))); }
    }
    keys { key(PK; Key1) { Clustered = true; } }
}

tableextension 50432 "RM Row Ext" extends "RM Row"
{
    fields { field(50432; Added; Blob) { } }
}

codeunit 50435 "RM Hold"
{
    var
        HB: Record "RM Tmp" temporary;

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
        Rec: Record "RM Tmp" temporary;

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
    local procedure W(var R: Record "RM Row"; T: Text)
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText(T);
    end;

    local procedure HV(B: Boolean): Text
    begin
        if B then
            exit('hv:');
        exit('nohv:');
    end;

    local procedure Rd(var R: Record "RM Row"): Text
    var
        IS: InStream;
        T: Text;
    begin
        R.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(R.Data.HasValue()) + T);
    end;

    local procedure Stored(K: Integer): Text
    var
        R: Record "RM Row";
        IS: InStream;
        T: Text;
        B: Boolean;
    begin
        if not R.Get(K) then
            exit('<no row>');
        B := R.Data.HasValue();
        R.CalcFields(Data);
        R.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(B) + T);
    end;

    // The reporter's repro (FieldRefValueBlobCopy.al:248).
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

    // A04 (:1426): a stored target.
    procedure StoredTargetRewriteSource(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row";
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.Open(Database::"RM Row");
        RR.Field(2).Value := Src.Data;
        W(Src, 'beta');
        RR.SetTable(Dst);
        exit(Rd(Dst));
    end;

    // A09 (:1518): a stored source read with Get and no CalcFields.
    procedure UnloadedStoredSource(): Text
    var
        Src: Record "RM Row";
        Dst: Record "RM Row" temporary;
        RR: RecordRef;
        Hand: Text;
    begin
        Src.Key1 := 9;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(9);
        RR.GetTable(Dst);
        RR.Field(2).Value := Src.Data;
        RR.SetTable(Dst);
        Hand := Rd(Dst);
        Dst.Key1 := 1;
        Dst.Insert();
        Dst.Get(1);
        Dst.CalcFields(Data);
        exit('hand=' + Hand + ' row=' + Rd(Dst));
    end;

    // A10 (:1540): a temporary source read with Get and no CalcFields, loaded afterwards.
    procedure UnloadedTemporarySource(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        Src.Key1 := 1;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(1);
        RR.GetTable(Dst);
        RR.Field(2).Value := Src.Data;
        Src.CalcFields(Data);
        RR.SetTable(Dst);
        exit(Rd(Dst));
    end;

    // A17 (:1659).
    procedure ValidateRewriteSource(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        RR: RecordRef;
        FR: FieldRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Dst);
        FR := RR.Field(2);
        FR.Validate(Src.Data);
        W(Src, 'beta');
        RR.SetTable(Dst);
        exit(Rd(Dst) + '|' + Rd(Src));
    end;

    // S5, mode 1 (:1807): an empty source replaces the target's value.
    procedure EmptySource(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        RR: RecordRef;
        Hand: Text;
    begin
        W(Dst, 'old');
        RR.GetTable(Dst);
        RR.Field(2).Value := Src.Data;
        RR.SetTable(Dst);
        W(Src, 'new');
        Hand := Rd(Dst);
        Dst.Key1 := 4;
        Dst.Insert();
        Dst.Get(4);
        Dst.CalcFields(Data);
        exit(Hand + '|' + Rd(Dst));
    end;

    // G06 (:2009).
    procedure GetTableClearSourceInsert(): Text
    var
        Src: Record "RM Row";
        RR: RecordRef;
    begin
        Src.Key1 := 26;
        W(Src, 'alpha');
        RR.GetTable(Src);
        Clear(Src.Data);
        RR.Insert();
        exit('row=' + Stored(26) + ' src=' + Rd(Src));
    end;

    // G03 (:1966): a write to the SetTable target does not reach the RecordRef.
    procedure SetTableWriteTarget(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        Dst2: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Src);
        RR.SetTable(Dst);
        W(Dst, 'zz');
        RR.SetTable(Dst2);
        exit(Rd(Dst) + '|' + Rd(Dst2));
    end;

    // F04 (:2122).
    procedure TransferFieldsClearSourceInsert(): Text
    var
        Src: Record "RM Row";
        Dst: Record "RM Row";
    begin
        Src.Key1 := 34;
        W(Src, 'alpha');
        Dst.TransferFields(Src);
        Clear(Src.Data);
        Dst.Insert();
        exit('hand=' + Rd(Dst) + ' row=' + Stored(34));
    end;

    // RunVariant a (:1117): a Variant holds the value of the moment it was assigned.
    procedure VariantRewriteSource(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        V: Variant;
    begin
        W(Src, 'alpha');
        V := Src.Data;
        W(Src, 'beta');
        Dst.Data := V;
        exit(Rd(Dst));
    end;

    // Not measured: a field a table extension declares.
    procedure ExtensionFieldWithContent(): Text
    var
        Src: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.Open(Database::"RM Row", true);
        RR.Field(50432).Value := Src.Data;
        exit('ran');
    end;

    // Not measured: Compressed = false.
    procedure UncompressedFieldWithContent(): Text
    var
        Src: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.Open(Database::"RM Row", true);
        RR.Field(4).Value := Src.Data;
        exit('ran');
    end;

    // Not measured: a FlowField.
    procedure FlowFieldWithContent(): Text
    var
        Src: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.Open(Database::"RM Row", true);
        RR.Field(5).Value := Src.Data;
        exit('ran');
    end;

    // Not measured: GetTable of a record whose extension field holds content.
    procedure GetTableExtensionFieldWithContent(): Text
    var
        Src: Record "RM Row" temporary;
        RR: RecordRef;
        OS: OutStream;
    begin
        Src.Added.CreateOutStream(OS);
        OS.WriteText('alpha');
        RR.GetTable(Src);
        exit('ran');
    end;

    // Not measured: TransferFields of a record whose Compressed = false field holds content.
    procedure TransferFieldsUncompressedWithContent(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        OS: OutStream;
    begin
        Src.Plain.CreateOutStream(OS);
        OS.WriteText('alpha');
        Dst.TransferFields(Src);
        exit('ran');
    end;

    // Not measured: a source read without CalcFields into a field a table extension declares.
    procedure ExtensionFieldFromUnreadSource(): Text
    var
        Src: Record "RM Row";
        RR: RecordRef;
    begin
        Src.Key1 := 50;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(50);
        RR.Open(Database::"RM Row", true);
        RR.Field(50432).Value := Src.Data;
        exit('ran');
    end;

    // Out of scope, main's result (the buffer is shared): TransferFields with InitPrimaryKeyFields written.
    procedure TransferFieldsTwoArguments(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
    begin
        W(Src, 'alpha');
        Dst.TransferFields(Src, true);
        W(Src, 'beta');
        exit(Rd(Dst));
    end;

    // Out of scope, main's result (the buffer is shared): SetTable with ShareTable written.
    procedure SetTableTwoArguments(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        Dst2: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Src);
        RR.SetTable(Dst, false);
        W(Dst, 'zz');
        RR.SetTable(Dst2, false);
        exit(Rd(Dst2));
    end;

    var
        GV: Variant;

    // Out of scope, main's result (the buffer is shared): a global Variant.
    procedure GlobalVariant(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
    begin
        W(Src, 'alpha');
        GV := Src.Data;
        W(Src, 'beta');
        Dst.Data := GV;
        exit(Rd(Dst));
    end;

    // Out of scope, main's result: empty Blobs in the fields that are not measured move as before.
    procedure EmptyUnmeasuredFieldsMove(): Text
    var
        Src: Record "RM Row" temporary;
        Dst: Record "RM Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Src);
        RR.Field(50432).Value := Src.Plain;
        RR.SetTable(Dst);
        Dst.TransferFields(Src);
        exit(Rd(Dst));
    end;

    // Out of scope, main's result: GetTable of a stored row read without CalcFields; CalcFields through the RecordRef loads it.
    procedure GetTableUnloadedRow(): Text
    var
        Src: Record "RM Row";
        Dst: Record "RM Row";
        RR: RecordRef;
    begin
        Src.Key1 := 40;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(40);
        RR.GetTable(Src);
        RR.SetTable(Dst);
        Dst.CalcFields(Data);
        exit(Rd(Dst));
    end;
}
"#;

/// The text `routine` returns, and whether the run passed through the assumption.
fn run(routine: &str) -> (String, bool) {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    (out, take_hits().contains(&Assumption::BlobTransferCopies))
}

fn refused(routine: &str) {
    clear_hits();
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a Blob with content"), "{routine}: {why}"),
        other => panic!("{routine}: {other:?}"),
    }
}

#[test]
fn rows_added_before_a_refill_of_the_source_keep_their_content() {
    // Row 2 holds what the second Fill left: 'beta' written over 'alpha' without truncation
    // (streams/TestStreamsRuntimeContracts.al:BlobField_CreateOutStream_DoesNotEmptyExistingData).
    assert_eq!(run("SameHolderTwoAdds"), ("alpha|betaa".into(), true));
}

#[test]
fn fieldref_value_into_a_stored_record_copies() {
    assert_eq!(run("StoredTargetRewriteSource"), ("hv:alpha".into(), true));
}

#[test]
fn fieldref_value_from_a_source_read_without_calcfields_is_empty() {
    assert_eq!(run("UnloadedStoredSource"), ("hand=nohv: row=nohv:".into(), true));
    assert_eq!(run("UnloadedTemporarySource"), ("nohv:".into(), true));
}

#[test]
fn fieldref_validate_copies() {
    assert_eq!(run("ValidateRewriteSource"), ("hv:alpha|hv:betaa".into(), true));
}

#[test]
fn an_empty_source_empties_the_target() {
    assert_eq!(run("EmptySource"), ("nohv:|nohv:".into(), true));
}

#[test]
fn gettable_copies_and_the_inserted_row_keeps_the_content() {
    assert_eq!(
        run("GetTableClearSourceInsert"),
        ("row=hv:alpha src=nohv:".into(), true)
    );
}

#[test]
fn settable_copies() {
    assert_eq!(run("SetTableWriteTarget"), ("hv:zzpha|hv:alpha".into(), true));
}

#[test]
fn transferfields_copies_and_the_inserted_row_keeps_the_content() {
    assert_eq!(
        run("TransferFieldsClearSourceInsert"),
        ("hand=hv:alpha row=hv:alpha".into(), true)
    );
}

#[test]
fn a_variant_holds_a_copy() {
    assert_eq!(run("VariantRewriteSource"), ("hv:alpha".into(), true));
}

#[test]
fn content_into_or_from_a_field_that_is_not_measured_is_refused() {
    refused("ExtensionFieldWithContent");
    refused("UncompressedFieldWithContent");
    refused("FlowFieldWithContent");
    refused("GetTableExtensionFieldWithContent");
    refused("TransferFieldsUncompressedWithContent");
    refused("ExtensionFieldFromUnreadSource");
}

#[test]
fn call_forms_that_are_not_measured_keep_mains_result() {
    assert_eq!(run("TransferFieldsTwoArguments"), ("hv:betaa".into(), false));
    // GetTable copied (flagged); the two SetTable calls share the RecordRef's buffer as before.
    assert_eq!(run("SetTableTwoArguments").0, "hv:zzpha");
    assert_eq!(run("GlobalVariant"), ("hv:betaa".into(), false));
}

#[test]
fn empty_blobs_of_fields_that_are_not_measured_keep_mains_result() {
    // Main's result; the measured Data field is copied on the way (flagged).
    assert_eq!(run("EmptyUnmeasuredFieldsMove").0, "hv:alpha");
}

#[test]
fn gettable_of_a_row_read_without_calcfields_keeps_mains_result() {
    assert_eq!(run("GetTableUnloadedRow"), ("hv:alpha".into(), false));
}
