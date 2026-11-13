//! The Blob copies of the second probe (assumption `blob-transfer-copies`: corpus PR 1372, streams/BlobValueCopyShapes.al;
//! al-runner issue 7948): an empty and a not-loaded Blob, the Variant forms, SetTable(Rec, true), TransferFields with more
//! arguments, Record.Copy, a record by value, and Blob fields with Compressed = false or of a table extension. The measured
//! routines repeat cells of that file and expect the values it asserts. The `Neighbor` routines are shapes it does not
//! measure: they expect what the runner gave before (the Blob is shared), unflagged where no measured move runs on the way.
//! `hv:` / `nohv:` is `HasValue`, then the text a stream reads.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
table 50440 "BS Row"
{
    fields
    {
        field(1; Key1; Integer) { }
        field(2; Data; Blob) { }
        field(3; Other; Blob) { }
        field(4; Plain; Blob) { Compressed = false; }
    }
    keys { key(PK; Key1) { Clustered = true; } }
}

tableextension 50441 "BS Row Ext" extends "BS Row"
{
    fields { field(50441; Added; Blob) { } }
}

codeunit 50443 "BS Sub"
{
    EventSubscriberInstance = StaticAutomatic;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnTake', '', false, false)]
    local procedure OnTake(R: Record "BS Row" temporary)
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;
}

codeunit 50444 C
{
    var
        GVar: Variant;

    local procedure W(var R: Record "BS Row"; T: Text)
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText(T);
    end;

    local procedure WAdded(var R: Record "BS Row"; T: Text)
    var
        OS: OutStream;
    begin
        R.Added.CreateOutStream(OS);
        OS.WriteText(T);
    end;

    local procedure WPlain(var R: Record "BS Row"; T: Text)
    var
        OS: OutStream;
    begin
        R.Plain.CreateOutStream(OS);
        OS.WriteText(T);
    end;

    local procedure HV(B: Boolean): Text
    begin
        if B then
            exit('hv:');
        exit('nohv:');
    end;

    local procedure Rd(var R: Record "BS Row"): Text
    var
        IS: InStream;
        T: Text;
    begin
        R.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(R.Data.HasValue()) + T);
    end;

    local procedure RdAdded(var R: Record "BS Row"): Text
    var
        IS: InStream;
        T: Text;
    begin
        R.Added.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(R.Added.HasValue()) + T);
    end;

    local procedure RdPlain(var R: Record "BS Row"): Text
    var
        IS: InStream;
        T: Text;
    begin
        R.Plain.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(R.Plain.HasValue()) + T);
    end;

    local procedure RdRR(var RR: RecordRef): Text
    var
        R: Record "BS Row" temporary;
    begin
        RR.SetTable(R);
        exit(Rd(R));
    end;

    local procedure Stored(K: Integer): Text
    var
        R: Record "BS Row";
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

    local procedure StoreUnloaded(var Src: Record "BS Row"; K: Integer)
    begin
        Src.Init();
        Src.Key1 := K;
        W(Src, 'alpha');
        Src.Insert();
        Clear(Src);
        Src.Get(K);
    end;

    local procedure ByValue(R: Record "BS Row"): Text
    begin
        W(R, 'zz');
        exit(Rd(R));
    end;

    local procedure ByValueTemp(R: Record "BS Row" temporary): Text
    begin
        W(R, 'zz');
        exit(Rd(R));
    end;

    [TryFunction]
    local procedure TryByValue(R: Record "BS Row")
    begin
        W(R, 'zz');
    end;

    local procedure FromVariant(RecVar: Variant; var RR: RecordRef)
    begin
        RR.GetTable(RecVar);
    end;

    local procedure TakeVar(V: Variant; var Src: Record "BS Row"; var Dst: Record "BS Row")
    begin
        W(Src, 'beta');
        Dst.Data := V;
    end;

    [IntegrationEvent(false, false)]
    local procedure OnTake(R: Record "BS Row" temporary)
    begin
    end;

    // ---- Measured cells of streams/BlobValueCopyShapes.al (corpus PR 1372) ----

    // E2 (:188).
    procedure GetTableEmptyWriteSourceInsert(): Text
    var
        Src: Record "BS Row";
        RR: RecordRef;
        Hand: Text;
    begin
        Src.Key1 := 2;
        RR.GetTable(Src);
        W(Src, 'x');
        Hand := RdRR(RR);
        RR.Insert();
        exit('rr=' + Hand + ' row=' + Stored(2) + ' src=' + Rd(Src));
    end;

    // E4b (:252).
    procedure TransferFieldsEmptyWriteTargetInsertSource(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
        Hand: Text;
    begin
        Src.Key1 := 41;
        Dst.TransferFields(Src);
        W(Dst, 'y');
        Hand := Rd(Src);
        Src.Insert();
        exit('src=' + Hand + ' row=' + Stored(41) + ' dst=' + Rd(Dst));
    end;

    // E5 (:278) and E6 (:297).
    procedure ValidateAndVariantEmptyWriteSource(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        V: Variant;
        RR: RecordRef;
        A: Text;
    begin
        RR.GetTable(Dst);
        RR.Field(2).Validate(Src.Data);
        V := Src.Data;
        W(Src, 'x');
        A := RdRR(RR);
        Dst.Data := V;
        exit(A + '|' + Rd(Dst));
    end;

    // N3 (:417).
    procedure SetTableUnloadedCalcFieldsTarget(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
        RR: RecordRef;
    begin
        StoreUnloaded(Src, 13);
        RR.Open(Database::"BS Row");
        RR.FindFirst();
        RR.SetTable(Dst);
        Dst.CalcFields(Data);
        exit('dst=' + Rd(Dst) + ' rr=' + RdRR(RR));
    end;

    // N4 (:441).
    procedure SetTableUnloadedWriteTargetModify(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
        RR: RecordRef;
        Hand: Text;
    begin
        StoreUnloaded(Src, 14);
        RR.Open(Database::"BS Row");
        RR.FindFirst();
        RR.SetTable(Dst);
        W(Dst, 'new');
        Hand := RdRR(RR);
        RR.Modify();
        exit('rr=' + Hand + ' row=' + Stored(14));
    end;

    // N5 (:468) and N6 (:491).
    procedure ValidateAndVariantUnloaded(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
        V: Variant;
        RR: RecordRef;
        A: Text;
    begin
        StoreUnloaded(Src, 15);
        RR.GetTable(Dst);
        RR.Field(2).Validate(Src.Data);
        V := Src.Data;
        Src.CalcFields(Data);
        A := RdRR(RR);
        Dst.Data := V;
        exit(A + '|' + Rd(Dst));
    end;

    // V3 (:590).
    procedure VariantByValueParameter(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
    begin
        W(Src, 'alpha');
        TakeVar(Src.Data, Src, Dst);
        exit(Rd(Dst) + '|' + Rd(Src));
    end;

    // V5 (:652) and the global Variant (:672).
    procedure VariantArrayElementAndGlobal(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        V: array[2] of Variant;
        A: Text;
    begin
        W(Src, 'alpha');
        V[1] := Src.Data;
        GVar := Src.Data;
        W(Src, 'beta');
        Dst.Data := V[1];
        A := Rd(Dst);
        Dst.Data := GVar;
        exit(A + '|' + Rd(Dst));
    end;

    // X6 (:712).
    procedure SetTableShareTable(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
        A: Text;
    begin
        W(Src, 'alpha');
        RR.GetTable(Src);
        RR.SetTable(Dst, true);
        W(Dst, 'zz');
        A := RdRR(RR);
        exit(A + '|' + Rd(Src) + '|' + Rd(Dst));
    end;

    // TF2 (:741), TF2f (:762), TF3 (:781).
    procedure TransferFieldsWithFlags(): Text
    var
        Src: Record "BS Row" temporary;
        A: Record "BS Row" temporary;
        B: Record "BS Row" temporary;
        C: Record "BS Row" temporary;
    begin
        W(Src, 'alpha');
        A.TransferFields(Src, true);
        B.TransferFields(Src, false);
        C.TransferFields(Src, true, true);
        W(Src, 'beta');
        exit(Rd(A) + '|' + Rd(B) + '|' + Rd(C));
    end;

    // P1 (:824).
    procedure GetTableOfRecordInLocalVariant(): Text
    var
        Src: Record "BS Row" temporary;
        V: Variant;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        V := Src;
        W(Src, 'beta');
        RR.GetTable(V);
        W(Src, 'zz');
        exit(RdRR(RR));
    end;

    // P2 (:847).
    procedure GetTableOfRecordAsVariantParameter(): Text
    var
        Src: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        FromVariant(Src, RR);
        W(Src, 'beta');
        exit(RdRR(RR));
    end;

    // X2 (:906), X2s (:929).
    procedure RecordCopyTemporary(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
    begin
        W(Src, 'alpha');
        Dst.Copy(Src);
        Dst2.Copy(Src, true);
        W(Src, 'beta');
        W(Dst, 'zz');
        exit(Rd(Dst) + '|' + Rd(Dst2) + '|' + Rd(Src));
    end;

    // X2r (:952).
    procedure RecordCopyWriteTargetInsertSource(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
    begin
        Src.Key1 := 21;
        W(Src, 'alpha');
        Dst.Copy(Src);
        W(Dst, 'zz');
        Src.Insert();
        exit('row=' + Stored(21) + ' src=' + Rd(Src) + ' dst=' + Rd(Dst));
    end;

    // P5 (:990).
    procedure RecordByValue(): Text
    var
        Src: Record "BS Row";
        Inside: Text;
    begin
        W(Src, 'alpha');
        Inside := ByValue(Src);
        exit('inside=' + Inside + ' caller=' + Rd(Src));
    end;

    // P5t (:975).
    procedure RecordByValueTemporary(): Text
    var
        Src: Record "BS Row" temporary;
        Inside: Text;
    begin
        W(Src, 'alpha');
        Inside := ByValueTemp(Src);
        exit('inside=' + Inside + ' caller=' + Rd(Src));
    end;

    // Compressed = false (:1174) and a table extension field (:1190): GetTable, SetTable and TransferFields(Rec).
    procedure VariationFields(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        WPlain(Src, 'alpha');
        WAdded(Src, 'gamma');
        RR.GetTable(Src);
        Dst2.TransferFields(Src);
        WPlain(Src, 'beta');
        WAdded(Src, 'delta');
        RR.SetTable(Dst);
        WPlain(Dst, 'zz');
        WAdded(Dst, 'zz');
        RR.SetTable(Src);
        exit(RdPlain(Dst) + '|' + RdAdded(Dst) + '|' + RdPlain(Dst2) + '|' + RdAdded(Dst2) + '|' + RdPlain(Src) + '|' + RdAdded(Src));
    end;

    // A stored table extension field read with CalcFields (:1206).
    procedure ExtensionFieldStoredGetTableModify(): Text
    var
        Src: Record "BS Row";
        R: Record "BS Row";
        RR: RecordRef;
    begin
        Src.Key1 := 26;
        WAdded(Src, 'alpha');
        Src.Insert();
        Src.Get(26);
        Src.CalcFields(Added);
        RR.GetTable(Src);
        WAdded(Src, 'new');
        RR.Modify();
        R.Get(26);
        R.CalcFields(Added);
        exit('row=' + RdAdded(R) + ' src=' + RdAdded(Src));
    end;

    // X4 (:1234).
    procedure OpenOutStreamGetTableSetTableSameRecord(): Text
    var
        Src: Record "BS Row" temporary;
        RR: RecordRef;
        OS: OutStream;
    begin
        Src.Data.CreateOutStream(OS);
        OS.WriteText('al');
        RR.GetTable(Src);
        RR.SetTable(Src);
        OS.WriteText('pha');
        exit(Rd(Src) + '|' + RdRR(RR));
    end;

    // ---- Shapes the probes do not measure ----

    // A record whose Blob an Insert stored, by value.
    procedure NeighborByValueAfterInsert(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 50;
        W(Src, 'alpha');
        Src.Insert();
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A record read with Get and CalcFields, by value.
    procedure NeighborByValueAfterCalcFields(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 51;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(51);
        Src.CalcFields(Data);
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A record with an empty Blob, by value.
    procedure NeighborByValueEmpty(): Text
    var
        Src: Record "BS Row" temporary;
    begin
        ByValueTemp(Src);
        exit(Rd(Src));
    end;

    // A temporary record handed to a parameter that is not declared temporary.
    procedure NeighborByValueTemporaryToRecordParameter(): Text
    var
        Src: Record "BS Row" temporary;
    begin
        W(Src, 'alpha');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A record that is not temporary handed to a `temporary` parameter and to a Variant parameter.
    procedure NeighborByValueStoredToOtherParameters(): Text
    var
        Src: Record "BS Row";
        RR: RecordRef;
        A: Text;
    begin
        W(Src, 'alpha');
        ByValueTemp(Src);
        A := Rd(Src);
        FromVariant(Src, RR);
        W(Src, 'beta');
        exit(A + '|' + RdRR(RR));
    end;

    // A procedure with an attribute.
    procedure NeighborByValueTryFunction(): Text
    var
        Src: Record "BS Row";
    begin
        W(Src, 'alpha');
        if TryByValue(Src) then;
        exit(Rd(Src));
    end;

    // An event: the publisher's parameter and the subscriber's.
    procedure NeighborByValueEvent(): Text
    var
        Src: Record "BS Row" temporary;
    begin
        W(Src, 'alpha');
        OnTake(Src);
        exit(Rd(Src));
    end;

    local procedure ByValueWriteOthers(R: Record "BS Row")
    begin
        WAdded(R, 'zz');
        WPlain(R, 'zz');
    end;

    // A table extension field and a Compressed = false field, by value and through Record.Copy.
    procedure NeighborVariationFieldsByValueAndCopy(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
        A: Text;
    begin
        WAdded(Src, 'alpha');
        WPlain(Src, 'alpha');
        ByValueWriteOthers(Src);
        A := RdAdded(Src) + '|' + RdPlain(Src);
        Dst.Copy(Src);
        WAdded(Dst, 'yy');
        WPlain(Dst, 'yy');
        exit(A + '|' + RdAdded(Src) + '|' + RdPlain(Src));
    end;

    // Record.Copy between a temporary record and one that is not.
    procedure NeighborCopyTemporaryToStored(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row";
    begin
        W(Src, 'alpha');
        Dst.Copy(Src);
        W(Dst, 'zz');
        exit(Rd(Src));
    end;

    // Record.Copy of a record whose Blob an Insert stored, and of one with an empty Blob.
    procedure NeighborCopyAfterInsertAndEmpty(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
        E: Record "BS Row" temporary;
        E2: Record "BS Row" temporary;
    begin
        Src.Key1 := 52;
        W(Src, 'alpha');
        Src.Insert();
        Dst.Copy(Src);
        W(Dst, 'zz');
        E2.Copy(E);
        W(E2, 'x');
        exit(Rd(Src) + '|' + Rd(E));
    end;

    // TransferFields(Rec, true, false) and SetTable(Rec, false).
    procedure NeighborOtherArguments(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        Dst.TransferFields(Src, true, false);
        RR.GetTable(Src);
        RR.SetTable(Dst2, false);
        W(Dst2, 'zz');
        W(Src, 'beta');
        exit(Rd(Dst) + '|' + RdRR(RR));
    end;

    // A global Variant and TransferFields(Rec, true) of a Blob read with CalcFields, and of an empty one.
    procedure NeighborOtherStates(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
        E: Record "BS Row" temporary;
        E2: Record "BS Row" temporary;
        A: Text;
    begin
        Src.Key1 := 53;
        W(Src, 'alpha');
        Src.Insert();
        Src.Get(53);
        Src.CalcFields(Data);
        GVar := Src.Data;
        W(Src, 'beta');
        Dst.Data := GVar;
        A := Rd(Dst);
        E2.TransferFields(E, true);
        W(E, 'x');
        exit(A + '|' + Rd(E2));
    end;

    // ---- Refused ----

    // FieldRef.Validate into a table extension field.
    procedure RefusedValidateIntoExtensionField(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Dst);
        RR.Field(50441).Validate(Src.Data);
        exit('ran');
    end;

    // GetTable of a Compressed = false Blob read with CalcFields.
    procedure RefusedGetTableUncompressedCalcFields(): Text
    var
        Src: Record "BS Row";
        RR: RecordRef;
    begin
        Src.Key1 := 54;
        WPlain(Src, 'alpha');
        Src.Insert();
        Src.Get(54);
        Src.CalcFields(Plain);
        RR.GetTable(Src);
        exit('ran');
    end;

    // FieldRef.Value into a table extension field from a row read without CalcFields.
    procedure RefusedExtensionFieldValueFromUnreadSource(): Text
    var
        Src: Record "BS Row";
        RR: RecordRef;
    begin
        StoreUnloaded(Src, 55);
        RR.Open(Database::"BS Row", true);
        RR.Field(50441).Value := Src.Data;
        exit('ran');
    end;
}
"#;

/// The text `routine` returns, and whether the run passed through the assumption.
fn run(routine: &str) -> (String, bool) {
    clear_hits();
    let out = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    (out, take_hits().contains(&Assumption::BlobTransferCopies))
}

fn flagged(routine: &str) -> String {
    let (out, hit) = run(routine);
    assert!(hit, "{routine} did not pass through the assumption");
    out
}

fn unflagged(routine: &str) -> String {
    let (out, hit) = run(routine);
    assert!(!hit, "{routine} passed through the assumption");
    out
}

fn refused(routine: &str) {
    clear_hits();
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a Blob with content moved by"), "{routine}: {why}"),
        other => panic!("{routine}: {other:?}"),
    }
}

#[test]
fn an_empty_blob_stays_empty_in_the_copy_and_in_the_inserted_row() {
    assert_eq!(flagged("GetTableEmptyWriteSourceInsert"), "rr=nohv: row=nohv: src=hv:x");
    assert_eq!(
        flagged("TransferFieldsEmptyWriteTargetInsertSource"),
        "src=nohv: row=nohv: dst=hv:y"
    );
    assert_eq!(flagged("ValidateAndVariantEmptyWriteSource"), "nohv:|nohv:");
}

#[test]
fn settable_of_a_row_read_without_calcfields_gives_a_blob_of_its_own() {
    assert_eq!(flagged("SetTableUnloadedCalcFieldsTarget"), "dst=hv:alpha rr=hv:");
    assert_eq!(flagged("SetTableUnloadedWriteTargetModify"), "rr=hv: row=hv:alpha");
}

#[test]
fn validate_and_a_variant_of_a_row_read_without_calcfields_are_empty() {
    assert_eq!(flagged("ValidateAndVariantUnloaded"), "nohv:|nohv:");
}

#[test]
fn a_variant_parameter_an_array_element_and_a_global_hold_a_copy() {
    assert_eq!(flagged("VariantByValueParameter"), "hv:alpha|hv:betaa");
    assert_eq!(flagged("VariantArrayElementAndGlobal"), "hv:alpha|hv:alpha");
}

#[test]
fn settable_with_sharetable_and_transferfields_with_flags_copy() {
    assert_eq!(flagged("SetTableShareTable"), "hv:alpha|hv:alpha|hv:zzpha");
    assert_eq!(flagged("TransferFieldsWithFlags"), "hv:alpha|hv:alpha|hv:alpha");
}

#[test]
fn a_record_in_a_variant_holds_the_blob_of_the_moment_it_was_taken() {
    assert_eq!(flagged("GetTableOfRecordInLocalVariant"), "hv:alpha");
    assert_eq!(flagged("GetTableOfRecordAsVariantParameter"), "hv:alpha");
}

#[test]
fn record_copy_copies_and_the_inserted_source_row_keeps_its_content() {
    assert_eq!(flagged("RecordCopyTemporary"), "hv:zzpha|hv:alpha|hv:betaa");
    assert_eq!(
        flagged("RecordCopyWriteTargetInsertSource"),
        "row=hv:alpha src=hv:alpha dst=hv:zzpha"
    );
}

#[test]
fn a_record_passed_by_value_leaves_the_callers_blob() {
    assert_eq!(flagged("RecordByValue"), "inside=hv:zzpha caller=hv:alpha");
    assert_eq!(flagged("RecordByValueTemporary"), "inside=hv:zzpha caller=hv:alpha");
}

#[test]
fn uncompressed_and_table_extension_fields_copy_in_the_measured_moves() {
    assert_eq!(
        flagged("VariationFields"),
        "hv:zzpha|hv:zzmma|hv:alpha|hv:gamma|hv:alpha|hv:gamma"
    );
    assert_eq!(flagged("ExtensionFieldStoredGetTableModify"), "row=hv:alpha src=hv:newha");
}

#[test]
fn an_open_outstream_keeps_writing_to_the_blob_it_was_created_on() {
    assert_eq!(flagged("OpenOutStreamGetTableSetTableSameRecord"), "hv:al|hv:al");
}

#[test]
fn a_record_by_value_in_a_state_or_pair_that_is_not_measured_shares_as_before() {
    assert_eq!(unflagged("NeighborByValueAfterInsert"), "hv:zzpha");
    assert_eq!(unflagged("NeighborByValueAfterCalcFields"), "hv:zzpha");
    assert_eq!(unflagged("NeighborByValueEmpty"), "hv:zz");
    assert_eq!(unflagged("NeighborByValueTemporaryToRecordParameter"), "hv:zzpha");
    // The SetTable of the read helper copies (flagged).
    assert_eq!(run("NeighborByValueStoredToOtherParameters").0, "hv:zzpha|hv:betaa");
}

#[test]
fn a_record_by_value_in_a_call_that_is_not_measured_shares_as_before() {
    assert_eq!(unflagged("NeighborByValueTryFunction"), "hv:zzpha");
    assert_eq!(unflagged("NeighborByValueEvent"), "hv:zzpha");
}

#[test]
fn record_copy_outside_the_measured_cells_shares_as_before() {
    assert_eq!(
        unflagged("NeighborVariationFieldsByValueAndCopy"),
        "hv:zzpha|hv:zzpha|hv:yypha|hv:yypha"
    );
    assert_eq!(unflagged("NeighborCopyTemporaryToStored"), "hv:zzpha");
    assert_eq!(unflagged("NeighborCopyAfterInsertAndEmpty"), "hv:zzpha|hv:x");
}

#[test]
fn arguments_and_states_that_are_not_measured_share_as_before() {
    // GetTable copies on the way (flagged); TransferFields(Rec, true, false) and SetTable(Rec, false) share.
    assert_eq!(run("NeighborOtherArguments").0, "hv:betaa|hv:zzpha");
    assert_eq!(unflagged("NeighborOtherStates"), "hv:betaa|hv:x");
}

#[test]
fn content_in_a_field_kind_outside_the_measured_moves_stays_refused() {
    refused("RefusedValidateIntoExtensionField");
    refused("RefusedGetTableUncompressedCalcFields");
    refused("RefusedExtensionFieldValueFromUnreadSource");
}
