codeunit 71858 Tests
{
    Subtype = Test;

    var
        Log: Codeunit "N9185 Log";

    local procedure Fresh(var R: Record "N9185 Row")
    begin
        R.DeleteAll();
        R.Init();
        R.Id := 'A';
        R.Name := 'N0';
        R.Tag := 'T0';
    end;

    local procedure RunInsert(ExtMode: Integer; SubMode: Integer): Text
    var
        R: Record "N9185 Row";
    begin
        Fresh(R);
        Log.Set(ExtMode, SubMode);
        R.Insert(true);
        exit(Log.Text());
    end;

    local procedure RunDelete(ExtMode: Integer; SubMode: Integer; ChangeBuffer: Boolean): Text
    var
        R: Record "N9185 Row";
    begin
        Fresh(R);
        Log.Set(0, 0);
        R.Insert(false);
        // The 1611 probe reads the row first (XebXRecAfterExtBeforeTrigger.al:268-290).
        R.Get('A');
        if ChangeBuffer then
            R.Name := 'BUF';
        Log.Set(ExtMode, SubMode);
        R.Delete(true);
        exit(Log.Text());
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:292
    [Test]
    procedure Insert_NothingChanges()
    begin
        Check(RunInsert(0, 0), 'N0/N0||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:329
    [Test]
    procedure Insert_ExtSetsName()
    begin
        Check(RunInsert(1, 0), 'N0/EXT||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:479
    [Test]
    procedure Insert_SubscriberSetsName()
    begin
        Check(RunInsert(0, 1), 'N0/SUB||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:553
    [Test]
    procedure Insert_SubscriberAndExtSetName()
    begin
        Check(RunInsert(1, 1), 'N0/EXT||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:590
    [Test]
    procedure Insert_ExtSetRange()
    begin
        Check(RunInsert(3, 0), 'N0/N0|Name: N0|Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:627
    [Test]
    procedure Insert_ExtSetCurrentKey()
    begin
        Check(RunInsert(4, 0), 'N0/N0||Name');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:403
    [Test]
    procedure Insert_AfterGetOfOtherRow_ExtSetsName()
    var
        R: Record "N9185 Row";
    begin
        R.DeleteAll();
        R.Init();
        R.Id := 'OTHER';
        R.Name := 'O';
        R.Insert(false);
        R.Get('OTHER');
        R.Id := 'A';
        R.Name := 'N0';
        R.Tag := 'T0';
        Log.Set(1, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0/EXT||Id');
    end;

    // Neighbor of :403 and :839: a row at the call (Get of another row), then Insert(true) with the extension's SetRange.
    // No 1611 cell measures it: main's answer, refused on head.
    [Test]
    procedure Neighbor_AfterGetOfOtherRow_ExtSetRange_Insert()
    var
        R: Record "N9185 Row";
    begin
        R.DeleteAll();
        R.Init();
        R.Id := 'OTHER';
        R.Name := 'O';
        R.Insert(false);
        R.Get('OTHER');
        R.Id := 'A';
        R.Name := 'N0';
        R.Tag := 'T0';
        Log.Set(3, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0/N0||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:664
    [Test]
    procedure Delete_NothingChanges()
    begin
        Check(RunDelete(0, 0, false), 'N0/N0||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:769
    [Test]
    procedure Delete_BufferChanged_ExtSetsName()
    begin
        Check(RunDelete(1, 0, true), 'BUF/EXT||Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:839
    [Test]
    procedure Delete_ExtSetRange()
    begin
        Check(RunDelete(3, 0, false), 'N0/N0|Name: N0|Id');
    end;

    // Measured: XebXRecAfterExtBeforeTrigger.al:874
    [Test]
    procedure Delete_SubscriberSetsName()
    begin
        Check(RunDelete(0, 1, false), 'N0/SUB||Id');
    end;

    // Not measured: a temporary record.
    [Test]
    procedure Neighbor_TemporaryInsert()
    var
        R: Record "N9185 Row" temporary;
    begin
        R.Id := 'A';
        R.Name := 'N0';
        Log.Set(1, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0/EXT||Id');
    end;

    // Not measured: the caller's record had filters before the call.
    [Test]
    procedure Neighbor_CallerFilters()
    var
        R: Record "N9185 Row";
    begin
        Fresh(R);
        R.SetRange(Tag, 'T0');
        Log.Set(1, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0/EXT|Tag: T0|Id');
    end;

    // Not measured: the caller's record had a current key other than the primary key.
    [Test]
    procedure Neighbor_CallerKey()
    var
        R: Record "N9185 Row";
    begin
        Fresh(R);
        R.SetCurrentKey(Name);
        Log.Set(1, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0/EXT||Name');
    end;

    // Not measured: the extension's OnBeforeInsert reads a row into Rec.
    [Test]
    procedure Neighbor_ExtGetsRow()
    var
        R: Record "N9185 Row";
    begin
        Fresh(R);
        R.Id := 'OTHER';
        R.Insert(false);
        R.Id := 'A';
        Log.Set(5, 0);
        asserterror R.Insert(true);
    end;

    // Not measured: the extension sets a filter in another filter group.
    [Test]
    procedure Neighbor_ExtFilterGroup()
    begin
        Check(RunInsert(6, 0), 'N0/N0||Id');
    end;

    // Not measured: two extensions declare OnBeforeInsert.
    [Test]
    procedure Neighbor_TwoExtensions()
    var
        R: Record "N9185 Two";
    begin
        R.DeleteAll();
        R.Id := 'A';
        R.Name := 'N0';
        Log.Set(0, 0);
        R.Insert(true);
        Check(Log.Text(), 'N0');
    end;

    // Main's behavior: the extension declares no OnBeforeInsert.
    [Test]
    procedure Main_NoExtBefore_SubscriberSetsName()
    var
        R: Record "N9185 NoBefore";
    begin
        R.DeleteAll();
        R.Id := 'A';
        R.Name := 'N0';
        Log.Set(0, 1);
        R.Insert(true);
        Check(Log.Text(), 'SUB');
    end;

    // Main's behavior: the table's code never names xRec.
    [Test]
    procedure Main_TableNeverNamesXRec()
    var
        R: Record "N9185 NoXRec";
    begin
        R.DeleteAll();
        R.Id := 'A';
        R.Name := 'N0';
        Log.Set(0, 0);
        R.Insert(true);
        Check(Log.Text(), 'EXT');
    end;

    // Not measured: the extension reads the same row again (Get, Find('=')), marks Rec, sets MarkedOnly, changes the
    // company, sets a descending order or assigns another variable of the same row to Rec.
    [Test]
    procedure Neighbor_ExtGetsSameRow_Delete()
    begin
        Check(RunDelete(7, 0, true), 'BUF/N0||Id');
    end;

    [Test]
    procedure Neighbor_ExtFindEqual_Delete()
    begin
        Check(RunDelete(8, 0, true), 'BUF/N0||Id');
    end;

    [Test]
    procedure Neighbor_ExtMarks()
    begin
        Check(RunDelete(9, 0, true), 'BUF/BUF||Id');
    end;

    [Test]
    procedure Neighbor_ExtMarkedOnly()
    begin
        Check(RunDelete(10, 0, true), 'BUF/BUF||Id');
    end;

    [Test]
    procedure Neighbor_ExtChangeCompany()
    begin
        Check(RunDelete(11, 0, true), 'BUF/BUF||Id');
    end;

    [Test]
    procedure Neighbor_ExtAscendingFalse()
    begin
        Check(RunDelete(12, 0, true), 'BUF/BUF||Id');
    end;

    [Test]
    procedure Neighbor_ExtRecAssign()
    begin
        Check(RunDelete(13, 0, true), 'BUF/N0||Id');
    end;

    // Not measured: a filter written into Rec by another record's CopyFilter in the extension's trigger (1611 measures only
    // Rec's own SetRange, XebXRecAfterExtBeforeTrigger.al:839).
    [Test]
    procedure Neighbor_ExtCopyFilterInto_Delete()
    begin
        Check(RunDelete(14, 0, true), 'BUF/BUF||Id');
    end;
}
