// Shared setup and checks. Each test is a codeunit of its own, so a refusal or a Commit in one cannot reach another.
codeunit 50409 "XR2 Lib"
{
    procedure AfterInsert(var Item: Record "XR2 Item"; Mode: Integer)
    begin
        Item.DeleteAll();
        Item.Init();
        Item."No." := 'T1';
        Item.Value := 10;
        Item.Mode := Mode;
        Item.Insert(false);
    end;

    procedure AfterGet(var Item: Record "XR2 Item"; Mode: Integer)
    begin
        AfterInsert(Item, Mode);
        Clear(Item);
        Item.Get('T1');
    end;

    // The value a corpus cell pins.
    procedure Expect(Seen: Integer; Want: Integer)
    begin
        if Seen <> Want then
        Error('seen=%1', Seen);
    end;

    // What the test saw, as its failure message.
    procedure Report(Item: Record "XR2 Item")
    begin
        Error('seen=%1 after=%2', Item.Seen, Item.After);
    end;
}

codeunit 50411 "XR2 T01"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_GetBySystemIdInTx()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Read: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 1);
        Read.GetBySystemId(Item.SystemId);
        Read.Validate(Value, 15);
        Lib.Report(Read);
    end;
}

codeunit 50412 "XR2 T02"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_FindLastInTx()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Read: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 1);
        Read.FindLast();
        Read.Validate(Value, 15);
        Lib.Report(Read);
    end;
}

codeunit 50414 "XR2 T04"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_AfterOwnModify()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 1);
        Item.Extra := 1;
        Item.Modify(false);
        Item.Validate(Value, 15);
        Lib.Report(Item);
    end;
}

codeunit 50415 "XR2 T05"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_GetAfterOtherVariableModifyAll()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Other: Record "XR2 Item";
        Read: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 1);
        Other.ModifyAll(Extra, 1);
        Read.Get('T1');
        Read.Validate(Value, 15);
        Lib.Expect(Read.Seen, 10);
    end;
}

codeunit 50416 "XR2 T06"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_TemporaryReadBack()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item" temporary;
    begin
        Lib.AfterInsert(Item, 1);
        Item.Get('T1');
        Item.Validate(Value, 15);
        Lib.Report(Item);
    end;
}

codeunit 50417 "XR2 T07"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Straight_AfterGet()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 2);
        Item.Validate(Value, 15);
        Lib.Expect(Item.Seen, 10);
    end;
}

codeunit 50418 "XR2 T08"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Straight_FindFirstInTx()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Read: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 2);
        Read.FindFirst();
        Read.Validate(Value, 15);
        Lib.Report(Read);
    end;
}

codeunit 50419 "XR2 T09"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Straight_FreshRecord()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Item.DeleteAll();
        Item."No." := 'T1';
        Item.Value := 10;
        Item.Mode := 2;
        Item.Validate(Value, 15);
        Lib.Report(Item);
    end;
}

codeunit 50420 "XR2 T10"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FieldRef_AfterGetFromTest()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Lib.AfterGet(Item, 1);
        RecRef.GetTable(Item);
        FldRef := RecRef.Field(2);
        FldRef.Validate(15);
        RecRef.SetTable(Item);
        Lib.Expect(Item.Seen, 10);
    end;
}

codeunit 50421 "XR2 T11"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FieldRef_OwnInsertBuffer()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Lib.AfterInsert(Item, 1);
        RecRef.GetTable(Item);
        FldRef := RecRef.Field(2);
        FldRef.Validate(15);
        RecRef.SetTable(Item);
        Lib.Report(Item);
    end;
}

codeunit 50422 "XR2 T12"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FieldRef_FromHelperCodeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Helper: Codeunit "XR2 Helper";
    begin
        Lib.AfterGet(Item, 1);
        Helper.FieldRefValidate(Item);
        Lib.Report(Item);
    end;
}

codeunit 50423 "XR2 T13"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FieldRef_ValidateWithoutValue()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Lib.AfterGet(Item, 1);
        RecRef.GetTable(Item);
        FldRef := RecRef.Field(2);
        FldRef.Value := 15;
        FldRef.Validate();
        RecRef.SetTable(Item);
        Lib.Report(Item);
    end;
}

codeunit 50424 "XR2 T14"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_GoToRecord()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Card: TestPage "XR2 Item Card";
    begin
        Lib.AfterInsert(Item, 1);
        Card.OpenEdit();
        Card.GoToRecord(Item);
        Card.Value.SetValue(15);
        Error('seen=%1', Card.Seen.Value());
    end;
}

codeunit 50425 "XR2 T15"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Insert_Codeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Item.DeleteAll();
        Item."No." := 'T1';
        Item.Value := 10;
        Item.Mode := 11;
        Item.Insert(true);
        Lib.Expect(Item.Seen, 10);
    end;
}

codeunit 50426 "XR2 T16"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_Codeunit_AfterGet()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 11);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Expect(Item.Seen, 15);
    end;
}

codeunit 50427 "XR2 T17"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_OneProcedureToCodeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 15);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50428 "XR2 T18"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_Codeunit_AfterOwnInsert()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 11);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50429 "XR2 T19"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_RecordRefModify_Codeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        RecRef: RecordRef;
    begin
        Lib.AfterGet(Item, 11);
        Item.Value := 15;
        RecRef.GetTable(Item);
        RecRef.Modify(true);
        RecRef.SetTable(Item);
        Lib.Report(Item);
    end;
}

codeunit 50430 "XR2 T20"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_CodeunitProcedureAssignsWholeXRec()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 16);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50431 "XR2 T21"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_FromHelperCodeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Helper: Codeunit "XR2 Helper";
    begin
        Lib.AfterGet(Item, 11);
        Item.Value := 15;
        Helper.ModifyWithTrigger(Item);
        Lib.Report(Item);
    end;
}

codeunit 50432 "XR2 T22"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_TemporaryInsert_Codeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item" temporary;
    begin
        Item."No." := 'T1';
        Item.Value := 10;
        Item.Mode := 11;
        Item.Insert(true);
        Lib.Report(Item);
    end;
}

codeunit 50433 "XR2 T23"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Rename_Codeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 11);
        Item.Rename('T2');
        Lib.Report(Item);
    end;
}

codeunit 50434 "XR2 T24"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Write_Modify_SingleInstanceMover()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 18);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50435 "XR2 T25"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Insert_RecProc()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Item.DeleteAll();
        Item."No." := 'T1';
        Item.Value := 10;
        Item.Mode := 12;
        Item.Insert(true);
        Lib.Expect(Item.Seen, 10);
    end;
}

codeunit 50436 "XR2 T26"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProc_AfterGet()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 12);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Expect(Item.Seen, 15);
    end;
}

codeunit 50437 "XR2 T27"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProcStraightFromTrigger()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 17);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50438 "XR2 T28"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProc_AfterOwnInsert()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 12);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50439 "XR2 T29"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProcAssigns_TriggerReadsAfter()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 13);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50440 "XR2 T30"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProcAssignsThenReads()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 14);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50441 "XR2 T31"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Delete_RecProcAssigns_TriggerReadsAfter()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 13);
        Item.Value := 15;
        Item.Delete(true);
        Lib.Report(Item);
    end;
}

codeunit 50442 "XR2 T32"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_Modify_RecProc_FromHelperCodeunit()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Helper: Codeunit "XR2 Helper";
    begin
        Lib.AfterGet(Item, 12);
        Item.Value := 15;
        Helper.ModifyWithTrigger(Item);
        Lib.Report(Item);
    end;
}

codeunit 50443 "XR2 T33"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_ExtOnBeforeModify_RecProc()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 21);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Expect(Item.Seen, 15);
    end;
}

codeunit 50444 "XR2 T34"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Own_ExtOnBeforeModify_ExtProcCallsRecProc()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
    begin
        Lib.AfterGet(Item, 23);
        Item.Value := 15;
        Item.Modify(true);
        Lib.Report(Item);
    end;
}

codeunit 50451 "XR2 P1"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_GoToKey()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Card: TestPage "XR2 Item Card";
    begin
        Lib.AfterInsert(Item, 1);
        Card.OpenEdit();
        Card.GoToKey('T1');
        Card.Value.SetValue(15);
        if Card.Seen.Value() <> '10' then
            Error('seen=%1', Card.Seen.Value());
    end;
}

codeunit 50452 "XR2 P2"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_OpenEditNoMove()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Card: TestPage "XR2 Item Card";
    begin
        Lib.AfterInsert(Item, 1);
        Card.OpenEdit();
        Card.Value.SetValue(15);
        Error('seen=%1', Card.Seen.Value());
    end;
}

codeunit 50453 "XR2 P3"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TestPage_GoToKeyThenReadThenSetValue()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Card: TestPage "XR2 Item Card";
    begin
        Lib.AfterInsert(Item, 1);
        Card.OpenEdit();
        Card.GoToKey('T1');
        if Card.Value.Value() = '' then;
        Card.Value.SetValue(15);
        Error('seen=%1', Card.Seen.Value());
    end;
}

// Last: its Commit marks the tests after it.
codeunit 50499 "XR2 T99"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Validate_FindFirstOfCommittedRow()
    var
        Lib: Codeunit "XR2 Lib";
        Item: Record "XR2 Item";
        Read: Record "XR2 Item";
    begin
        Lib.AfterInsert(Item, 1);
        Commit();
        Read.FindFirst();
        Read.Validate(Value, 15);
        Lib.Report(Read);
    end;
}
