// The shapes whose answer stays main's: in a codeunit before every test that refuses, so no refusal leaves them a session state.
codeunit 50363 "NX Kept Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "NX Mode";
        Steps: Codeunit "NX Steps";

    [Test]
    procedure Reporter_TableOnInsert_AfterValidate()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('', '');
        Steps.InsertAfterValidate(Line);
        if Line."xRec Serial In Table OnInsert" <> 42 then
            Error('xRec."Serial No." in table OnInsert was %1, expected 42', Line."xRec Serial In Table OnInsert");
    end;

    [Test]
    procedure ExtOnAfterInsert_NoXRecRead()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'none');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_InsertFalse()
    var
        Line: Record "NX Line";
    begin
        Steps.Clean();
        Mode.Set('AfterInsert', 'direct');
        Line."Entry No." := 1;
        Line.Validate("Serial No.", 42);
        Line.Insert(false);
        Steps.Expect('');
    end;

    [Test]
    procedure ExtOnModify_AfterGetAndAssign()
    var
        Line: Record "NX Line";
    begin
        Steps.Seed();
        Mode.Set('Modify', 'direct');
        Line.Get(1);
        Line."Serial No." := 43;
        Line.Modify(true);
        Steps.Expect('43');
    end;

    [Test]
    procedure ExtOnAfterModify_AfterGetAndAssign()
    var
        Line: Record "NX Line";
    begin
        Steps.Seed();
        Mode.Set('AfterModify', 'direct');
        Line.Get(1);
        Line."Serial No." := 43;
        Line.Modify(true);
        Steps.Expect('43');
    end;

    // The extension's OnBeforeInsert leaves Rec as it was: the table's OnInsert reads the pinned xRec.
    [Test]
    procedure TableOnInsert_AfterExtOnBeforeInsertWritesNothing()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('BeforeInsert', 'none');
        Steps.InsertAfterAssign(Line);
        Steps.Expect('42');
        if Line."xRec Serial In Table OnInsert" <> 42 then
            Error('xRec."Serial No." in table OnInsert was %1, expected 42', Line."xRec Serial In Table OnInsert");
    end;

    // The extension's OnBeforeDelete leaves Rec as it was: the table's OnDelete reads the pinned xRec.
    [Test]
    procedure TableOnDelete_AfterExtOnBeforeDeleteWritesNothing()
    var
        Line: Record "NX Line";
        BaseLog: Codeunit "NX Base Log";
    begin
        Steps.Seed();
        Mode.Set('BeforeDelete', 'none');
        Line.Get(1);
        Line.Delete(true);
        Steps.Expect('42');
        if BaseLog.Get() <> 42 then
            Error('xRec."Serial No." in table OnDelete was %1, expected 42', BaseLog.Get());
    end;

    // TableCaption and FieldNo read neither xRec's buffer nor its view.
    [Test]
    procedure ExtOnAfterInsert_XRecTableCaption()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'caption');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('NX Line2');
    end;
}

// The two shapes of the taken-out change (a table trigger after an extension trigger wrote Rec): own codeunit, before every
// test that refuses, so no refusal leaves them a session state. Main answers both.
codeunit 50364 "NX Taken Out Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "NX Mode";
        Steps: Codeunit "NX Steps";

    // The extension's OnBeforeInsert writes Rec, then the table's OnInsert reads xRec.
    [Test]
    procedure TableOnInsert_AfterExtOnBeforeInsertWritesRec()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('BeforeInsert', 'recwrite');
        Steps.InsertAfterAssign(Line);
        Steps.Expect('r');
        if Line."xRec Serial In Table OnInsert" <> 42 then
            Error('xRec."Serial No." in table OnInsert was %1, expected 42', Line."xRec Serial In Table OnInsert");
    end;

    // The extension's OnBeforeDelete changes Rec without reading xRec, then the table's own OnDelete reads xRec.
    [Test]
    procedure TableOnDelete_AfterExtOnBeforeDeleteWritesRec()
    var
        Line: Record "NX Line";
        BaseLog: Codeunit "NX Base Log";
    begin
        Steps.Seed();
        Mode.Set('BeforeDelete', 'recwrite');
        Line.Get(1);
        Line.Delete(true);
        Steps.Expect('r');
        if BaseLog.Get() <> 42 then
            Error('xRec."Serial No." in table OnDelete was %1, expected 42', BaseLog.Get());
    end;
}

codeunit 50360 "NX Refused Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "NX Mode";
        Holder: Codeunit "NX Holder";
        Steps: Codeunit "NX Steps";

    // The reporter's two tests, unchanged in substance.
    [Test]
    procedure Reporter_ExtOnAfterInsert_AfterValidate()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'direct');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_PlainAssign()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'direct');
        Steps.InsertAfterAssign(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_LocalProcedure()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'local');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_ExtProcedureOnRec()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'extproc');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_BaseProcedureOnRec()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'baseproc');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_CodeunitVarRecCallsBaseProcedure()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'varcaller');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecByValueToProcedure()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'byvalue');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecKeptInSingleInstance()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'holder');
        Steps.InsertAfterValidate(Line);
        if Holder.KeptSerial() <> 42 then
            Error('kept %1', Holder.KeptSerial());
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecKeptInVariant()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'variant');
        Steps.InsertAfterValidate(Line);
        if Holder.KeptVariantSerial() <> 42 then
            Error('kept %1', Holder.KeptVariantSerial());
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecCopy()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'copy');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecTransferFields()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'transfer');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecRecordRef()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'recref');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecFormat()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'format');
        Steps.InsertAfterValidate(Line);
        if Mode.Take() = '' then
            Error('nothing logged');
    end;

    // Get and the methods that read only xRec's view (filters, key, table) refuse too: the view is not measured either.
    [Test]
    procedure ExtOnAfterInsert_XRecGetThenRead()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'get');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecCount()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'count');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('1');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecGetFilters()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'getfilters');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('[]');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecFieldWriteOnly()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'write');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('w');
    end;

    [Test]
    procedure ExtOnAfterInsert_XRecAssignedFromRecThenRead()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'assign');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_RecordRefInsertTrue()
    var
        Line: Record "NX Line";
        RecRef: RecordRef;
    begin
        Steps.Clean();
        Mode.Set('AfterInsert', 'direct');
        Line."Entry No." := 1;
        Line.Validate("Serial No.", 42);
        RecRef.GetTable(Line);
        RecRef.Insert(true);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterInsert_TemporaryRecord()
    var
        Line: Record "NX Line" temporary;
    begin
        Mode.Set('AfterInsert', 'direct');
        Line."Entry No." := 1;
        Line.Validate("Serial No.", 42);
        Line.Insert(true);
        Steps.Expect('42');
    end;

    // The other insert triggers of the extension.
    [Test]
    procedure ExtOnBeforeInsert_AfterValidate()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('BeforeInsert', 'direct');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnInsert_AfterValidate()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('Insert', 'direct');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnInsert_LocalProcedure()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('Insert', 'local');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;

    // The delete triggers of the extension, after Get.
    [Test]
    procedure ExtOnBeforeDelete_AfterGet()
    var
        Line: Record "NX Line";
    begin
        Steps.Seed();
        Mode.Set('BeforeDelete', 'direct');
        Line.Get(1);
        Line.Delete(true);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnDelete_AfterGet()
    var
        Line: Record "NX Line";
    begin
        Steps.Seed();
        Mode.Set('Delete', 'direct');
        Line.Get(1);
        Line.Delete(true);
        Steps.Expect('42');
    end;

    [Test]
    procedure ExtOnAfterDelete_AfterGet()
    var
        Line: Record "NX Line";
    begin
        Steps.Seed();
        Mode.Set('AfterDelete', 'direct');
        Line.Get(1);
        Line.Delete(true);
        Steps.Expect('42');
    end;

    // The modify triggers of the extension (TestTableExtensionTableTriggers_Tests.al:183 pins OnAfterModify).

    // A table procedure called on Rec runs Validate; the field's OnValidate reads the procedure's xRec.
    [Test]
    procedure ExtOnAfterInsert_BaseProcValidatesThenOnValidateReadsXRec()
    var
        Line: Record "NX Line";
    begin
        Mode.Set('AfterInsert', 'basevalidate');
        Steps.InsertAfterValidate(Line);
        Steps.Expect('42');
    end;
}

// A page insert and a duplicate key, each in a codeunit of its own: a refusal in the middle of either leaves no state to the
// tests above.
codeunit 50361 "NX Page Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "NX Mode";

    [Test]
    procedure ExtOnAfterInsert_TestPageNew()
    var
        Card: TestPage "NX Line Card";
        Line: Record "NX Line";
    begin
        Line.DeleteAll();
        Mode.Set('AfterInsert', 'direct');
        Card.OpenNew();
        Card.EntryNo.SetValue(5);
        Card.SerialNo.SetValue(42);
        Card.Close();
        if Mode.Take() = '' then
            Error('nothing logged');
    end;
}

codeunit 50362 "NX Duplicate Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "NX Mode";

    [Test]
    procedure ExtOnAfterInsert_DuplicateKeyStatement()
    var
        Line: Record "NX Line";
    begin
        Line.DeleteAll();
        Line."Entry No." := 1;
        Line."Serial No." := 42;
        Line.Insert(false);
        Clear(Line);
        Mode.Set('AfterInsert', 'direct');
        Line."Entry No." := 1;
        Line.Validate("Serial No.", 43);
        asserterror begin
            Line.Insert(true);
            Line.FindFirst();
        end;
        if Mode.Take() = '' then
            Error('nothing logged');
    end;
}
