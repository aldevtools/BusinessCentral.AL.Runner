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
        field(5; Packed; Blob) { Compressed = true; }
    }
    keys { key(PK; Key1) { Clustered = true; } }

    procedure Take(R: Record "BS Row")
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;
}

table 50446 "BS Ev"
{
    fields
    {
        field(1; Key1; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Key1) { Clustered = true; } }
}

page 50445 "BS Page"
{
    SourceTable = "BS Row";

    procedure Take(R: Record "BS Row")
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;
}

tableextension 50441 "BS Row Ext" extends "BS Row"
{
    fields
    {
        field(50441; Added; Blob) { }
        field(50442; AddedPlain; Blob) { Compressed = false; }
    }
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

    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnIsolated', '', false, false)]
    local procedure OnIsolated(R: Record "BS Row")
    var
        OS: OutStream;
    begin
        R.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;

    [EventSubscriber(ObjectType::Table, Database::"BS Ev", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsert(Rec: Record "BS Ev"; RunTrigger: Boolean)
    var
        OS: OutStream;
    begin
        Rec.Data.CreateOutStream(OS);
        OS.WriteText('zz');
    end;
}

codeunit 50444 C
{
    var
        GVar: Variant;
        GVar2: Variant;
        GRow: Record "BS Row" temporary;

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

    [IntegrationEvent(false, true)]
    local procedure OnIsolated(R: Record "BS Row")
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

    // Record.Copy of an empty Blob from a record that is not temporary into a temporary one: corpus PR 1375 measures this
    // pair only written in memory, written and inserted, and read with Get only (:2101-2145).
    procedure NeighborCopyStoredToTemporaryEmpty(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
    begin
        Src.Init();
        Dst.Copy(Src);
        W(Dst, 'zz');
        exit(Rd(Src));
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
        Clear(Src);
        Src.Get(53);
        Src.CalcFields(Data);
        GVar := Src.Data;
        W(Src, 'beta');
        Dst.Data := GVar;
        A := Rd(Dst);
        E2.TransferFields(E, true);
        W(E2, 'x');
        exit(A + '|' + Rd(E));
    end;

    // ---- Compressed = false and table extension fields in the moves of the first probe ----

    // FieldRef.Value and FieldRef.Validate into a table extension field.
    procedure ValueAndValidateIntoExtensionField(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
        RR: RecordRef;
        RR2: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Dst);
        RR.Field(50441).Value := Src.Data;
        RR2.GetTable(Dst2);
        RR2.Field(50441).Validate(Src.Data);
        W(Src, 'beta');
        RR.SetTable(Dst);
        RR2.SetTable(Dst2);
        exit(RdAdded(Dst) + '|' + RdAdded(Dst2));
    end;

    // FieldRef.Value into a Compressed = false field.
    procedure ValueIntoUncompressedField(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Dst);
        RR.Field(4).Value := Src.Data;
        W(Src, 'beta');
        RR.SetTable(Dst);
        exit(RdPlain(Dst));
    end;

    // GetTable of a Compressed = false Blob read with CalcFields.
    procedure GetTableUncompressedCalcFields(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        Src.Key1 := 54;
        WPlain(Src, 'alpha');
        Src.Insert();
        Src.Get(54);
        Src.CalcFields(Plain);
        RR.GetTable(Src);
        WPlain(Src, 'new');
        RR.SetTable(Dst);
        exit(RdPlain(Dst));
    end;

    // FieldRef.Value into a table extension field from a row read without CalcFields.
    procedure ExtensionFieldValueFromUnreadSource(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        StoreUnloaded(Src, 55);
        RR.GetTable(Dst);
        RR.Field(50441).Value := Src.Data;
        RR.SetTable(Dst);
        exit(RdAdded(Dst));
    end;

    // ---- A stored or saved Blob written again, and a Blob that got its write by a copy (review of al-runner PR 7951) ----

    // Insert, then a new write, then by value (A1).
    procedure NeighborInsertWriteAgainByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 60;
        W(Src, 'alpha');
        Src.Insert();
        W(Src, 'beta');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // The same with a temporary record (A2).
    procedure NeighborTempInsertWriteAgainByValue(): Text
    var
        Src: Record "BS Row" temporary;
    begin
        Src.Key1 := 61;
        W(Src, 'alpha');
        Src.Insert();
        W(Src, 'beta');
        ByValueTemp(Src);
        exit(Rd(Src));
    end;

    // Insert, a new write, Record.Copy, a write of the copy, Modify of the source (A3).
    procedure NeighborInsertWriteAgainCopyModify(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row";
    begin
        Src.Key1 := 62;
        W(Src, 'alpha');
        Src.Insert();
        W(Src, 'beta');
        Dst.Copy(Src);
        W(Dst, 'zz');
        Src.Modify();
        exit(Rd(Src) + '|' + Stored(62));
    end;

    // Modify, then a new write, then by value (A4).
    procedure NeighborModifyWriteAgainByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 63;
        Src.Insert();
        W(Src, 'alpha');
        Src.Modify();
        W(Src, 'beta');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // Insert, a new write, a global Variant and TransferFields(Rec, true) (A5).
    procedure NeighborInsertWriteAgainGlobalVariantTransferFlags(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
    begin
        Src.Key1 := 64;
        W(Src, 'alpha');
        Src.Insert();
        W(Src, 'beta');
        GVar := Src.Data;
        Dst2.TransferFields(Src, true);
        W(Src, 'zz');
        Dst.Data := GVar;
        exit(Rd(Dst) + '|' + Rd(Dst2));
    end;

    // A Blob that got its content by a field assignment, by value (B1).
    procedure NeighborAssignedBlobByValue(): Text
    var
        Src: Record "BS Row";
        Mid: Record "BS Row";
    begin
        W(Src, 'alpha');
        Mid.Data := Src.Data;
        ByValue(Mid);
        exit(Rd(Mid));
    end;

    // A Blob that got its content by FieldRef.Value and SetTable, by value (B3).
    procedure NeighborFieldRefValueBlobByValue(): Text
    var
        Src: Record "BS Row";
        Mid: Record "BS Row";
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.GetTable(Mid);
        RR.Field(2).Value := Src.Data;
        RR.SetTable(Mid);
        ByValue(Mid);
        exit(Rd(Mid));
    end;

    // ---- A Blob of a field that is not plain moved into a Variant that is not local (review of al-runner PR 7951) ----

    local procedure TakeVarWrite(V: Variant; var Src: Record "BS Row"; var Dst: Record "BS Row"; Which: Integer)
    var
        OS: OutStream;
    begin
        case Which of
            1:
                Src.Added.CreateOutStream(OS);
            2:
                Src.Plain.CreateOutStream(OS);
            3:
                Src.Packed.CreateOutStream(OS);
        end;
        OS.WriteText('beta');
        Dst.Data := V;
    end;

    // A table extension field, a Compressed = false field and a Compressed = true field to a by-value Variant parameter
    // (C1, C2, C3).
    procedure NeighborVariationFieldsToVariantParameter(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        OS: OutStream;
        A: Text;
    begin
        Src.Added.CreateOutStream(OS);
        OS.WriteText('alpha');
        TakeVarWrite(Src.Added, Src, Dst, 1);
        A := Rd(Dst);
        Src.Plain.CreateOutStream(OS);
        OS.WriteText('alpha');
        TakeVarWrite(Src.Plain, Src, Dst, 2);
        A += '|' + Rd(Dst);
        Src.Packed.CreateOutStream(OS);
        OS.WriteText('alpha');
        TakeVarWrite(Src.Packed, Src, Dst, 3);
        exit(A + '|' + Rd(Dst));
    end;

    // A table extension field to a global Variant and an array element (C4).
    procedure NeighborExtensionFieldToGlobalAndArray(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
        A: array[2] of Variant;
        OS: OutStream;
    begin
        Src.Added.CreateOutStream(OS);
        OS.WriteText('alpha');
        GVar := Src.Added;
        A[1] := Src.Added;
        Src.Added.CreateOutStream(OS);
        OS.WriteText('beta');
        Dst.Data := GVar;
        Dst2.Data := A[1];
        exit(Rd(Dst) + '|' + Rd(Dst2));
    end;

    // A Compressed = true field to a global Variant and a local one (C5).
    procedure NeighborCompressedFieldToGlobalAndLocal(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
        V: Variant;
        OS: OutStream;
    begin
        Src.Packed.CreateOutStream(OS);
        OS.WriteText('alpha');
        GVar := Src.Packed;
        V := Src.Packed;
        Src.Packed.CreateOutStream(OS);
        OS.WriteText('beta');
        Dst.Data := GVar;
        Dst2.Data := V;
        exit(Rd(Dst) + '|' + Rd(Dst2));
    end;

    // FieldRef.Validate into a table extension field from a row read without CalcFields (corpus PR 1375,
    // streams/BlobCopyStates.al, procedure Into, state get).
    procedure ExtensionFieldValidateFromUnreadSource(): Text
    var
        Src: Record "BS Row";
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        StoreUnloaded(Src, 56);
        RR.GetTable(Dst);
        RR.Field(50441).Validate(Src.Data);
        RR.SetTable(Dst);
        exit(RdAdded(Dst));
    end;

    // ---- Record states and Variant holders of the round-2 review (S cells) ----

    local procedure KeepVar(V: Variant)
    begin
        GVar2 := V;
    end;

    // Insert of an empty Blob, a write, then the record by value (S1; temporary S2).
    procedure NeighborInsertEmptyWriteByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 71;
        Src.Insert();
        W(Src, 'alpha');
        ByValue(Src);
        exit(Rd(Src));
    end;

    procedure NeighborTempInsertEmptyWriteByValue(): Text
    var
        Src: Record "BS Row" temporary;
    begin
        Src.Key1 := 72;
        Src.Insert();
        W(Src, 'alpha');
        ByValueTemp(Src);
        exit(Rd(Src));
    end;

    // A write, Insert, Clear of the field, a new write, then by value (S4).
    procedure NeighborInsertClearFieldWriteByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 73;
        W(Src, 'alpha');
        Src.Insert();
        Clear(Src.Data);
        W(Src, 'beta');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // Insert of an empty Blob, Modify, a write, then by value (S5).
    procedure NeighborInsertEmptyModifyWriteByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 74;
        Src.Insert();
        Src.Modify();
        W(Src, 'alpha');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A write, Insert, Init, a new write, then by value (S14).
    procedure NeighborInsertInitWriteByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 75;
        W(Src, 'alpha');
        Src.Insert();
        Src.Init();
        W(Src, 'beta');
        ByValue(Src);
        exit(Rd(Src));
    end;

    // Temporary: Insert of an empty Blob, a write, Record.Copy (S12).
    procedure NeighborTempInsertEmptyWriteCopy(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
    begin
        Src.Key1 := 76;
        Src.Insert();
        W(Src, 'alpha');
        Dst.Copy(Src);
        W(Src, 'beta');
        exit(Rd(Dst));
    end;

    // Temporary: Insert of an empty Blob, a write, a global Variant and TransferFields(Src, true) (S13).
    procedure NeighborTempInsertEmptyWriteGlobalVariantTransferFlags(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Dst2: Record "BS Row" temporary;
    begin
        Src.Key1 := 77;
        Src.Insert();
        W(Src, 'alpha');
        GVar := Src.Data;
        Dst2.TransferFields(Src, true);
        W(Src, 'beta');
        Dst.Data := GVar;
        exit(Rd(Dst) + '|' + Rd(Dst2));
    end;

    // A Variant that took the empty field moves on into a global Variant after the field was written (S8).
    procedure NeighborVariantHeldBlobToGlobalVariant(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
    begin
        GVar := Src.Data;
        W(Src, 'alpha');
        GVar2 := GVar;
        W(Src, 'beta');
        Dst.Data := GVar2;
        exit(Rd(Dst));
    end;

    // The same into a by-value Variant parameter that keeps it in a global (S9).
    procedure NeighborVariantHeldBlobToVariantParameter(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
    begin
        GVar := Src.Data;
        W(Src, 'alpha');
        KeepVar(GVar);
        W(Src, 'beta');
        Dst.Data := GVar2;
        exit(Rd(Dst));
    end;

    // The field of a global record, of a record in an array, of a `var` parameter record and of a record that is not
    // temporary into a global Variant, and a field into an element of a two-dimensional array (S11, S17, S18, S19, S16).
    procedure NeighborOtherSourceRecordsToGlobalVariant(): Text
    var
        Arr: array[2] of Record "BS Row" temporary;
        Src: Record "BS Row" temporary;
        Stored: Record "BS Row";
        Dst: Record "BS Row" temporary;
        A2: array[2, 2] of Variant;
        Out: Text;
    begin
        Clear(GRow);
        W(GRow, 'alpha');
        GVar := GRow.Data;
        W(GRow, 'beta');
        Dst.Data := GVar;
        Out := Rd(Dst);
        W(Arr[1], 'alpha');
        GVar := Arr[1].Data;
        W(Arr[1], 'beta');
        Dst.Data := GVar;
        Out += '|' + Rd(Dst);
        W(Src, 'alpha');
        Out += '|' + FromVarParameter(Src);
        W(Stored, 'alpha');
        GVar := Stored.Data;
        W(Stored, 'beta');
        Dst.Data := GVar;
        Out += '|' + Rd(Dst);
        Clear(Src);
        W(Src, 'alpha');
        A2[1, 1] := Src.Data;
        W(Src, 'beta');
        Dst.Data := A2[1, 1];
        exit(Out + '|' + Rd(Dst));
    end;

    local procedure FromVarParameter(var Src: Record "BS Row"): Text
    var
        Dst: Record "BS Row" temporary;
    begin
        GVar := Src.Data;
        W(Src, 'beta');
        Dst.Data := GVar;
        exit(Rd(Dst));
    end;

    // ---- Refused ----

    // FieldRef.Value into a field with Compressed = true.
    procedure RefusedValueIntoCompressedField(): Text
    var
        Src: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        W(Src, 'alpha');
        RR.Open(Database::"BS Row", true);
        RR.Field(5).Value := Src.Data;
        exit('ran');
    end;

    // GetTable of a record whose table extension field with the Compressed property holds content.
    procedure RefusedGetTableExtensionFieldWithCompressed(): Text
    var
        Src: Record "BS Row" temporary;
        RR: RecordRef;
        OS: OutStream;
    begin
        Src.AddedPlain.CreateOutStream(OS);
        OS.WriteText('alpha');
        RR.GetTable(Src);
        exit('ran');
    end;

    // ---- Neighbors of the cells of streams/BlobCopyNeighbors.al (corpus PR 1387) that keep main's shared buffer ----

    // A Commit between the Insert and the by-value call.
    procedure NeighborCommitBeforeByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 601;
        W(Src, 'alpha');
        Src.Insert();
        Commit();
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A record method none of the probes ran (SetRange) between the Insert and the by-value call.
    procedure NeighborSetRangeBeforeByValue(): Text
    var
        Src: Record "BS Row";
    begin
        Src.Key1 := 602;
        W(Src, 'alpha');
        Src.Insert();
        Src.SetRange(Key1, 602);
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A temporary record handed by value to a table procedure.
    procedure NeighborTemporaryToTableProcedure(): Text
    var
        Src: Record "BS Row" temporary;
        T: Record "BS Row";
    begin
        W(Src, 'alpha');
        T.Take(Src);
        exit(Rd(Src));
    end;

    // A record handed by value to a procedure of a page.
    procedure NeighborPageProcedure(): Text
    var
        Src: Record "BS Row";
        P: Page "BS Page";
    begin
        W(Src, 'alpha');
        P.Take(Src);
        exit(Rd(Src));
    end;

    // A by-value subscriber of an isolated IntegrationEvent.
    procedure NeighborIsolatedPublisher(): Text
    var
        Src: Record "BS Row";
    begin
        W(Src, 'alpha');
        OnIsolated(Src);
        exit(Rd(Src));
    end;

    // A by-value OnBeforeInsertEvent subscriber of a temporary record.
    procedure NeighborTemporaryInsertEvent(): Text
    var
        Src: Record "BS Ev" temporary;
        OS: OutStream;
        IS: InStream;
        T: Text;
    begin
        Src.Key1 := 606;
        Src.Data.CreateOutStream(OS);
        OS.WriteText('alpha');
        Src.Insert();
        Src.Data.CreateInStream(IS);
        IS.ReadText(T);
        exit(HV(Src.Data.HasValue()) + T);
    end;

    // FieldRef.Validate on the GetTable copy before SetTable(Rec, true).
    procedure NeighborValidateBeforeSetTableShared(): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        RR: RecordRef;
    begin
        Src.Key1 := 607;
        W(Src, 'alpha');
        Src.Insert();
        RR.GetTable(Src);
        RR.Field(1).Validate(607);
        RR.SetTable(Dst, true);
        W(Dst, 'zz');
        exit(RdRR(RR));
    end;

    // A Commit while the local is fresh, then a write, an Insert and the by-value call.
    procedure NeighborCommitWhileFreshThenInsert(): Text
    var
        Src: Record "BS Row";
    begin
        Commit();
        Src.Key1 := 608;
        W(Src, 'alpha');
        Src.Insert();
        ByValue(Src);
        exit(Rd(Src));
    end;

    // A Commit while the local is fresh, then a record assignment from another fresh local and the by-value call.
    procedure NeighborCommitWhileFreshThenAssign(): Text
    var
        Src: Record "BS Row";
        Other: Record "BS Row";
    begin
        Commit();
        W(Other, 'alpha');
        Src := Other;
        ByValue(Src);
        exit(Rd(Src));
    end;

    // One RecordRef call `Which` on the GetTable copy of a temporary record, then SetTable(Rec, true): what the RecordRef
    // reads after a write to the target.
    local procedure RrCallThenShare(Which: Integer): Text
    var
        Src: Record "BS Row" temporary;
        Dst: Record "BS Row" temporary;
        Mid: Record "BS Row" temporary;
        RR: RecordRef;
        RR2: RecordRef;
        FR: FieldRef;
        T: Text;
        I: Integer;
        B: Boolean;
    begin
        Src.Key1 := 610;
        W(Src, 'alpha');
        Src.Insert();
        RR.GetTable(Src);
        case Which of
            1:
                B := RR.IsTemporary();
            2:
                RR.SetView('WHERE(Key1=CONST(610))');
            3:
                RR.SetRecFilter();
            4:
                RR.Ascending(false);
            5:
                T := RR.Name();
            6:
                begin
                    FR := RR.Field(1);
                    I := FR.Value;
                end;
            7:
                RR.Field(1).SetFilter('610');
            8:
                RR.Field(1).SetRange(610);
            9:
                begin
                    RR2.Open(Database::"BS Row", true);
                    RR2.Copy(RR);
                end;
            10:
                RR.SetTable(Mid, false);
            11:
                RR.SetTable(Mid, true);
        end;
        RR.SetTable(Dst, true);
        W(Dst, 'zz');
        exit(RdRR(RR));
    end;

    procedure NeighborRrIsTemporary(): Text
    begin
        exit(RrCallThenShare(1));
    end;

    procedure NeighborRrSetView(): Text
    begin
        exit(RrCallThenShare(2));
    end;

    procedure NeighborRrSetRecFilter(): Text
    begin
        exit(RrCallThenShare(3));
    end;

    procedure NeighborRrAscending(): Text
    begin
        exit(RrCallThenShare(4));
    end;

    procedure NeighborRrName(): Text
    begin
        exit(RrCallThenShare(5));
    end;

    procedure NeighborRrFieldRead(): Text
    begin
        exit(RrCallThenShare(6));
    end;

    procedure NeighborRrFieldSetFilter(): Text
    begin
        exit(RrCallThenShare(7));
    end;

    procedure NeighborRrCopiedFrom(): Text
    begin
        exit(RrCallThenShare(9));
    end;

    procedure NeighborRrSetTableFalseFirst(): Text
    begin
        exit(RrCallThenShare(10));
    end;

    procedure NeighborRrSetTableTrueFirst(): Text
    begin
        exit(RrCallThenShare(11));
    end;

    procedure RrFieldSetRange(): Text
    begin
        exit(RrCallThenShare(8));
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
    assert_eq!(
        flagged("ExtensionFieldStoredGetTableModify"),
        "row=hv:alpha src=hv:newha"
    );
}

#[test]
fn an_open_outstream_keeps_writing_to_the_blob_it_was_created_on() {
    assert_eq!(flagged("OpenOutStreamGetTableSetTableSameRecord"), "hv:al|hv:al");
}

// P3 is corpus PR 1375, streams/BlobCopyStates.al; P4 is corpus PR 1387, streams/BlobCopyNeighbors.al. Each value below is the one
// the cited probe line measured for the same move, source kind and state.

#[test]
fn a_record_by_value_copies_in_every_measured_state_and_taker() {
    // P4 :587 states ins, calc, empty; :925 temporary into a Record parameter, state mem.
    assert_eq!(flagged("NeighborByValueAfterInsert"), "hv:alpha");
    assert_eq!(flagged("NeighborByValueAfterCalcFields"), "hv:alpha");
    assert_eq!(flagged("NeighborByValueEmpty"), "nohv:");
    assert_eq!(flagged("NeighborByValueTemporaryToRecordParameter"), "hv:alpha");
    // P4 :613 into a `temporary` parameter; :639 into a Variant parameter (state mem), which GetTable then reads.
    assert_eq!(flagged("NeighborByValueStoredToOtherParameters"), "hv:alpha|hv:alpha");
}

#[test]
fn a_record_by_value_to_a_tryfunction_copies() {
    // P3 :2200, a codeunit's [TryFunction], state mem.
    assert_eq!(flagged("NeighborByValueTryFunction"), "hv:alpha");
}

#[test]
fn record_copy_copies_in_every_pair_but_not_an_extension_field() {
    // P3 :3108 C6 and :3274 E2 (Compressed = false through Record.Copy); a table extension field through Record.Copy is not
    // measured and shares, as on main.
    assert_eq!(
        flagged("NeighborVariationFieldsByValueAndCopy"),
        "hv:alpha|hv:alpha|hv:yypha|hv:alpha"
    );
    // P4 :1256 temporary into not temporary, state mem.
    assert_eq!(flagged("NeighborCopyTemporaryToStored"), "hv:alpha");
    // P4 :1204 stored to stored, state ins; :1282 temporary to temporary, state empty.
    assert_eq!(flagged("NeighborCopyAfterInsertAndEmpty"), "hv:alpha|nohv:");
    // P4 :1230 stored into temporary, state empty.
    assert_eq!(flagged("NeighborCopyStoredToTemporaryEmpty"), "nohv:");
}

#[test]
fn other_arguments_share_and_the_measured_states_copy() {
    // GetTable copies on the way (flagged); TransferFields(Rec, true, false) and SetTable(Rec, false) share, as on main.
    assert_eq!(run("NeighborOtherArguments").0, "hv:betaa|hv:zzpha");
    // P4 :1434 a stored record's field read into a global Variant, state calc; :1107 TransferFields(Rec, true) between
    // temporary records, state empty.
    assert_eq!(flagged("NeighborOtherStates"), "hv:alpha|nohv:");
}

#[test]
fn fieldref_value_and_validate_into_an_extension_or_uncompressed_field_copy() {
    assert_eq!(flagged("ValueAndValidateIntoExtensionField"), "hv:alpha|hv:alpha");
    assert_eq!(flagged("ValueIntoUncompressedField"), "hv:alpha");
    assert_eq!(flagged("GetTableUncompressedCalcFields"), "hv:alpha");
    assert_eq!(flagged("ExtensionFieldValueFromUnreadSource"), "nohv:");
    assert_eq!(flagged("ExtensionFieldValidateFromUnreadSource"), "nohv:");
}

#[test]
fn content_in_a_field_kind_that_is_not_measured_stays_refused() {
    refused("RefusedValueIntoCompressedField");
    refused("RefusedGetTableExtensionFieldWithCompressed");
}

/// A Blob an Insert or a Modify saved and a stream wrote again (A1 to A5 of the review of al-runner PR 7951): the probe measured
/// the new moves only on a buffer that held nothing when its unsaved write began. They share, as before.

#[test]
fn a_saved_blob_written_again_copies() {
    // P4 :587 state inswrite; :925 the same from a temporary record; :1204 Record.Copy, inswrite (the Modify after it stores the
    // source's own text); :587 insmodwrite.
    assert_eq!(flagged("NeighborInsertWriteAgainByValue"), "hv:betaa");
    assert_eq!(flagged("NeighborTempInsertWriteAgainByValue"), "hv:betaa");
    assert_eq!(flagged("NeighborInsertWriteAgainCopyModify"), "hv:betaa|hv:betaa");
    assert_eq!(flagged("NeighborModifyWriteAgainByValue"), "hv:betaa");
    // P4 :1512 a temporary record's field read into a global Variant and :1107 TransferFields(Rec, true), state inswrite.
    assert_eq!(
        flagged("NeighborInsertWriteAgainGlobalVariantTransferFlags"),
        "hv:betaa|hv:betaa"
    );
}

#[test]
fn a_blob_written_by_a_copy_copies_by_value() {
    // P3 :2880 B1 (a field assignment), :2923 B3 (FieldRef.Value and SetTable).
    assert_eq!(flagged("NeighborAssignedBlobByValue"), "hv:alpha");
    assert_eq!(run("NeighborFieldRefValueBlobByValue").0, "hv:alpha");
}

#[test]
fn other_field_kinds_into_a_variant_that_is_not_local_copy() {
    // P3 :3017 C1-C3, :3087 C4-C5.
    assert_eq!(
        flagged("NeighborVariationFieldsToVariantParameter"),
        "hv:alpha|hv:alpha|hv:alpha"
    );
    assert_eq!(flagged("NeighborExtensionFieldToGlobalAndArray"), "hv:alpha|hv:alpha");
    assert_eq!(flagged("NeighborCompressedFieldToGlobalAndLocal"), "hv:alpha|hv:alpha");
}

#[test]
fn a_record_stored_before_the_write_copies() {
    // P4 :587 insemptywrite, :925 the same from a temporary record, :587 insclearfieldwrite; Insert of an empty Blob, Modify,
    // then a write: :587 insemptywrite and insmodwrite run these calls in other orders; :587 insinitwrite; :1282 Record.Copy
    // between temporary records, insemptywrite; :1512 and :1107, insemptywrite.
    assert_eq!(flagged("NeighborInsertEmptyWriteByValue"), "hv:alpha");
    assert_eq!(flagged("NeighborTempInsertEmptyWriteByValue"), "hv:alpha");
    assert_eq!(flagged("NeighborInsertClearFieldWriteByValue"), "hv:beta");
    assert_eq!(flagged("NeighborInsertEmptyModifyWriteByValue"), "hv:alpha");
    assert_eq!(flagged("NeighborInsertInitWriteByValue"), "hv:beta");
    assert_eq!(flagged("NeighborTempInsertEmptyWriteCopy"), "hv:alpha");
    assert_eq!(
        flagged("NeighborTempInsertEmptyWriteGlobalVariantTransferFlags"),
        "hv:alpha|hv:alpha"
    );
}

/// A field read of an empty temporary record into a global Variant copies (P4 :1512, state empty); the Variant that holds the
/// copy moves on into another Variant, by assignment or as a by-value parameter, as before.
#[test]
fn a_blob_read_into_a_global_variant_is_copied_and_moves_on_with_the_variant() {
    assert_eq!(flagged("NeighborVariantHeldBlobToGlobalVariant"), "nohv:");
    assert_eq!(flagged("NeighborVariantHeldBlobToVariantParameter"), "nohv:");
}

/// The probe read the field from a local record variable. A global record, a record in an array, a `var` parameter record and
/// a two-dimensional array share, as on main; a local record that is not temporary copies (P4 :1434, state mem).
#[test]
fn a_field_of_another_source_record_into_a_global_variant_shares_as_before() {
    assert_eq!(
        flagged("NeighborOtherSourceRecordsToGlobalVariant"),
        "hv:betaa|hv:betaa|hv:betaa|hv:alpha|hv:betaa"
    );
}

/// Shapes next to the cells of P4 that no probe ran: main's shared buffer, and the assumption does not run.
#[test]
fn neighbors_of_the_fourth_probe_keep_mains_shared_buffer() {
    // A Commit between the Insert and the by-value call (P4 ran none).
    assert_eq!(unflagged("NeighborCommitBeforeByValue"), "hv:zzpha");
    // A record method the probes did not run (SetRange) before the by-value call.
    assert_eq!(unflagged("NeighborSetRangeBeforeByValue"), "hv:zzpha");
    // A temporary record into a table procedure, and into a codeunit subscriber (P4 measured a stored record there).
    assert_eq!(unflagged("NeighborTemporaryToTableProcedure"), "hv:zzpha");
    assert_eq!(unflagged("NeighborByValueEvent"), "hv:zzpha");
    // A procedure of a page.
    assert_eq!(unflagged("NeighborPageProcedure"), "hv:zzpha");
    // A subscriber of an isolated IntegrationEvent.
    assert_eq!(unflagged("NeighborIsolatedPublisher"), "hv:zzpha");
    // A by-value OnBeforeInsertEvent subscriber of a temporary record.
    assert_eq!(unflagged("NeighborTemporaryInsertEvent"), "hv:zzpha");
}

/// FieldRef.Validate on the GetTable copy ends its fact: SetTable(Rec, true) after it shares, as on main. GetTable copies on the
/// way (flagged).
#[test]
fn settable_sharetable_after_a_fieldref_validate_shares_as_before() {
    assert_eq!(run("NeighborValidateBeforeSetTableShared").0, "hv:zzpha");
}

/// A Commit while a local is fresh, before the call that makes it a measured source: no probe ran it (P3 :3208 D5 commits
/// before a global record only Clear and a write touch), so the by-value call shares as on main.
#[test]
fn a_commit_before_the_first_record_method_keeps_mains_shared_buffer() {
    assert_eq!(unflagged("NeighborCommitWhileFreshThenInsert"), "hv:zzpha");
    assert_eq!(unflagged("NeighborCommitWhileFreshThenAssign"), "hv:zzpha");
}

/// A RecordRef call the probes did not run on the GetTable copy before SetTable(Rec, true): the copy's fact ends and the table
/// is shared, as on main (GetTable copies on the way, flagged). FieldRef.SetRange with one value is a call P4 ran (:2446
/// `ShareSetRange`): the RecordRef keeps its own Blob.
#[test]
fn a_recordref_call_not_measured_before_settable_sharetable_shares_as_before() {
    for routine in [
        "NeighborRrIsTemporary",
        "NeighborRrSetView",
        "NeighborRrSetRecFilter",
        "NeighborRrAscending",
        "NeighborRrName",
        "NeighborRrFieldRead",
        "NeighborRrFieldSetFilter",
        "NeighborRrCopiedFrom",
        // An earlier SetTable(Rec, false) or SetTable(Rec, true) on the RecordRef (probe r3probe RR3_SetTableFalseFirst,
        // RR3_SetTableTrueFirst: the runner before shared too).
        "NeighborRrSetTableFalseFirst",
        "NeighborRrSetTableTrueFirst",
    ] {
        assert_eq!(run(routine).0, "hv:zzpha", "{routine}");
    }
    assert_eq!(flagged("RrFieldSetRange"), "hv:alpha");
}
