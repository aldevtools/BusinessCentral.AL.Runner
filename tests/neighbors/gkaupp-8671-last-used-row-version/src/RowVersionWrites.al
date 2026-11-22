// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-lastusedrowversion-method
// Scope: in-scope
// Fixtures used: Probe Observer (69160); own tables 76600 and 76601
// Probe: how Database.LastUsedRowVersion() moves after a write in the still open transaction of a test (the shape of an incremental export that
// stores LastUsedRowVersion() as a watermark right after inserting its test rows): after Insert, Modify, Delete, Rename, ModifyAll, DeleteAll and
// Truncate (each after an Insert in the same transaction and, for Delete, also on a committed row), across Commit, after an asserterror that rolled
// the writes back, for two writes in a row and for a write to a second table. Also its relation to the SystemRowVersion of the row written last.
// Only relations are asserted (same, greater, less), never a row version number: the numbers differ per database. Existing pins of the neighbours:
// PlTRowVersionValue.al (Insert, Modify and MinimumActiveRowVersion in an open transaction, Delete and DeleteAll without a prior Insert) and
// MinimumActiveRowVersionStages.al (MinimumActiveRowVersion under three TestIsolation settings), not repeated here.
// A fact the runs showed: an Insert is flushed to the database later (by a read of the table, another write or Commit), and LastUsedRowVersion() grows
// only at that flush. The tests named ...AfterInsert write right after an unflushed Insert; the ...AfterFlushedInsert and Watermark ones read the row first.
// Measured on BC 27.5, 28.5 and 29.0: LastUsedRowVersion() grows (greater than the read before) at the flush of an Insert, at a Modify, Rename and
// ModifyAll of flushed rows, at a Delete, DeleteAll and Truncate of a row that was inserted unflushed (the delete flushes the insert), and at a Commit
// that flushes an Insert. A row's SystemRowVersion equals LastUsedRowVersion() read right after its flush; a later row is greater than an earlier one;
// a rolled back Insert leaves no row, the next Insert grows it again. The shape of the export in the issue: a watermark read right after an unflushed
// Insert lies below the row; read after a flush the row lies at or below it.
// Not asserted: that LastUsedRowVersion() stays the same across a Delete, DeleteAll, Truncate, read, rollback or second Commit. The value is
// database-wide, so a write of another session between two reads moves it (seen once on 28.5); such a cell cannot be made deterministic. Those tests
// assert IsInWriteTransaction and the row relations only.
// Observation names: <stage> last/prev is LastUsedRowVersion() against its previous read, row/last is the SystemRowVersion of the row written last against
// LastUsedRowVersion() read right after the write; tx is IsInWriteTransaction.
// BC versions: 27+

table 76600 "RVW Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76601 "RVW Other Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// local object: shares one write sequence between the test codeunits of three TestIsolation settings.
codeunit 76602 "RVW Sequence"
{
    procedure Rel(Value: BigInteger; Other: BigInteger): Text
    begin
        if Value = Other then
            exit('same');
        if Value > Other then
            exit('greater');
        exit('less');
    end;

    procedure Step(var Observer: Codeunit "Probe Observer"; Tag: Text; var Prev: BigInteger)
    var
        Last: BigInteger;
    begin
        Last := LastUsedRowVersion();
        Observer.Observe(Tag + ' last/prev', Rel(Last, Prev));
        Observer.Observe(Tag + ' tx', Format(IsInWriteTransaction()));
        Prev := Last;
    end;

    // Only IsInWriteTransaction is observed: whether LastUsedRowVersion() stayed the same cannot be asserted, because
    // the value is database-wide and a write of another session between two reads moves it.
    procedure StepQuiet(var Observer: Codeunit "Probe Observer"; Tag: Text; var Prev: BigInteger)
    begin
        Observer.Observe(Tag + ' tx', Format(IsInWriteTransaction()));
        Prev := LastUsedRowVersion();
    end;

    procedure StepRowQuiet(var Observer: Codeunit "Probe Observer"; Tag: Text; var Prev: BigInteger; RowVersion: BigInteger)
    begin
        Observer.Observe(Tag + ' row/last', Rel(RowVersion, LastUsedRowVersion()));
        StepQuiet(Observer, Tag, Prev);
    end;

    procedure StepRow(var Observer: Codeunit "Probe Observer"; Tag: Text; var Prev: BigInteger; RowVersion: BigInteger)
    begin
        Observer.Observe(Tag + ' row/last', Rel(RowVersion, LastUsedRowVersion()));
        Step(Observer, Tag, Prev);
    end;

    // Insert, read, Modify, Delete of one row, in one open transaction.
    procedure Run(var Observer: Codeunit "Probe Observer")
    var
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        StepRow(Observer, 'insert', Prev, Row.SystemRowVersion);
        StepQuiet(Observer, 'read again', Prev);
        Row.Name := 'x';
        Row.Modify();
        Row.Get(1);
        StepRow(Observer, 'modify', Prev, Row.SystemRowVersion);
        Row.Delete();
        StepQuiet(Observer, 'delete', Prev);
        Commit();
    end;
}

codeunit 76603 "RVW Writes"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure LastUsedRowVersion_InsertModifyDelete_OpenTransaction()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
    begin
        Seq.Run(Observer);
        Observer.Expect('insert row/last', 'same');
        Observer.Expect('insert last/prev', 'greater');
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('read again tx', 'Yes');
        Observer.Expect('modify row/last', 'same');
        Observer.Expect('modify last/prev', 'greater');
        Observer.Expect('modify tx', 'Yes');
        Observer.Expect('delete tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_DeleteAfterInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Delete();
        Seq.Step(Observer, 'delete', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Commit();
        Observer.Expect('delete last/prev', 'greater');
        Observer.Expect('delete tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_DeleteCommittedRow()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Delete();
        Seq.StepQuiet(Observer, 'delete', Prev);
        Commit();
        Seq.StepQuiet(Observer, 'commit', Prev);
        Observer.Expect('delete tx', 'Yes');
        Observer.Expect('commit tx', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_RenameAfterInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Rename(2);
        Seq.Step(Observer, 'rename', Prev);
        Row.Get(2);
        Seq.StepRowQuiet(Observer, 'get renamed', Prev, Row.SystemRowVersion);
        Row.DeleteAll();
        Commit();
        Observer.Expect('rename last/prev', 'greater');
        Observer.Expect('rename tx', 'Yes');
        Observer.Expect('get renamed row/last', 'same');
        Observer.Expect('get renamed tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_ModifyAllAfterInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.ModifyAll(Name, 'y');
        Seq.Step(Observer, 'modifyall', Prev);
        Row.DeleteAll();
        Commit();
        Observer.Expect('modifyall last/prev', 'greater');
        Observer.Expect('modifyall tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_DeleteAllAfterInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.DeleteAll();
        Seq.Step(Observer, 'deleteall', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Commit();
        Observer.Expect('deleteall last/prev', 'greater');
        Observer.Expect('deleteall tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_TruncateAfterInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Truncate();
        Seq.Step(Observer, 'truncate', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Commit();
        Observer.Expect('truncate last/prev', 'greater');
        Observer.Expect('truncate tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_InsertThenCommit()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
        AtInsert: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Insert();
        AtInsert := LastUsedRowVersion();
        Seq.StepQuiet(Observer, 'insert', Prev);
        Commit();
        Observer.Observe('commit last/insert', Seq.Rel(LastUsedRowVersion(), AtInsert));
        Seq.Step(Observer, 'commit', Prev);
        Commit();
        Seq.StepQuiet(Observer, 'second commit', Prev);
        Row.DeleteAll();
        Commit();
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('commit last/insert', 'greater');
        Observer.Expect('commit last/prev', 'greater');
        Observer.Expect('commit tx', 'No');
        Observer.Expect('second commit tx', 'No');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_AfterAsserterrorRollback()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
        Last: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        asserterror begin
            Row.Id := 1;
            Row.Insert();
            Last := LastUsedRowVersion();
            Error('rolled back');
        end;
        Seq.StepQuiet(Observer, 'after rollback', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        Seq.StepRow(Observer, 'insert after rollback', Prev, Row.SystemRowVersion);
        Row.DeleteAll();
        Commit();
        Observer.Expect('after rollback tx', 'No');
        Observer.Expect('count', '0');
        Observer.Expect('insert after rollback row/last', 'same');
        Observer.Expect('insert after rollback last/prev', 'greater');
        Observer.Expect('insert after rollback tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_TwoInserts_RowOrder()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
        First: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        First := Row.SystemRowVersion;
        Seq.StepRow(Observer, 'insert 1', Prev, First);
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        Observer.Observe('row 2/row 1', Seq.Rel(Row.SystemRowVersion, First));
        Seq.StepRow(Observer, 'insert 2', Prev, Row.SystemRowVersion);
        Row.Get(1);
        Observer.Observe('row 1 unchanged', Seq.Rel(Row.SystemRowVersion, First));
        Row.DeleteAll();
        Commit();
        Observer.Expect('insert 1 row/last', 'same');
        Observer.Expect('insert 1 last/prev', 'greater');
        Observer.Expect('insert 1 tx', 'Yes');
        Observer.Expect('row 2/row 1', 'greater');
        Observer.Expect('insert 2 row/last', 'same');
        Observer.Expect('insert 2 last/prev', 'greater');
        Observer.Expect('insert 2 tx', 'Yes');
        Observer.Expect('row 1 unchanged', 'same');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_InsertTwoTables()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Insert();
        Seq.StepQuiet(Observer, 'insert row', Prev);
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Seq.StepRow(Observer, 'insert other', Prev, Other.SystemRowVersion);
        Row.Get(1);
        Observer.Observe('row/other', Seq.Rel(Row.SystemRowVersion, Other.SystemRowVersion));
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Observer.Expect('insert row tx', 'Yes');
        Observer.Expect('insert other row/last', 'same');
        Observer.Expect('insert other last/prev', 'greater');
        Observer.Expect('insert other tx', 'Yes');
        Observer.Expect('row/other', 'greater');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_WatermarkShape_UnflushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Row: Record "RVW Row";
        Mark: BigInteger;
    begin
        // The export shape: a watermark read right after an Insert, then the rows above it and below LastUsedRowVersion() + 1.
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Mark := LastUsedRowVersion();
        Row.Id := 2;
        Row.Insert();
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '>%1&<%2', Mark, LastUsedRowVersion() + 1);
        Observer.Observe('above mark below last+1 found', Format(Row.FindFirst()));
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '>%1', Mark);
        Observer.Observe('above mark count', Format(Row.Count()));
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '<=%1', Mark);
        Observer.Observe('at or below mark count', Format(Row.Count()));
        Row.Reset();
        Row.DeleteAll();
        Commit();
        Observer.Expect('above mark below last+1 found', 'No');
        Observer.Expect('above mark count', '2');
        Observer.Expect('at or below mark count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_WatermarkShape_FlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Row: Record "RVW Row";
        Mark: BigInteger;
    begin
        // The export shape: a watermark read right after an Insert, then the rows above it and below LastUsedRowVersion() + 1.
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        Mark := LastUsedRowVersion();
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '>%1&<%2', Mark, LastUsedRowVersion() + 1);
        Observer.Observe('above mark below last+1 found', Format(Row.FindFirst()));
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '>%1', Mark);
        Observer.Observe('above mark count', Format(Row.Count()));
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '<=%1', Mark);
        Observer.Observe('at or below mark count', Format(Row.Count()));
        Row.Reset();
        Row.DeleteAll();
        Commit();
        Observer.Expect('above mark below last+1 found', 'Yes');
        Observer.Expect('above mark count', '1');
        Observer.Expect('at or below mark count', '1');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_DeleteAfterFlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        Prev := LastUsedRowVersion();
        Row.Delete();
        Seq.StepQuiet(Observer, 'delete', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Row.DeleteAll();
        Commit();
        Observer.Expect('delete tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_RenameAfterFlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        Prev := LastUsedRowVersion();
        Row.Rename(2);
        Seq.Step(Observer, 'rename', Prev);
        Row.Get(2);
        Seq.StepRowQuiet(Observer, 'get renamed', Prev, Row.SystemRowVersion);
        Row.DeleteAll();
        Commit();
        Observer.Expect('rename last/prev', 'greater');
        Observer.Expect('rename tx', 'Yes');
        Observer.Expect('get renamed row/last', 'same');
        Observer.Expect('get renamed tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_ModifyAllAfterFlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        Prev := LastUsedRowVersion();
        Row.ModifyAll(Name, 'y');
        Seq.Step(Observer, 'modifyall', Prev);
        Row.DeleteAll();
        Commit();
        Observer.Expect('modifyall last/prev', 'greater');
        Observer.Expect('modifyall tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_DeleteAllAfterFlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        Prev := LastUsedRowVersion();
        Row.DeleteAll();
        Seq.StepQuiet(Observer, 'deleteall', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Row.DeleteAll();
        Commit();
        Observer.Expect('deleteall tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_TruncateAfterFlushedInsert()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
        Row: Record "RVW Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        Prev := LastUsedRowVersion();
        Row.Truncate();
        Seq.StepQuiet(Observer, 'truncate', Prev);
        Observer.Observe('count', Format(Row.Count()));
        Row.DeleteAll();
        Commit();
        Observer.Expect('truncate tx', 'Yes');
        Observer.Expect('count', '0');
        Observer.Verify();
    end;
}

codeunit 76604 "RVW Writes Codeunit Iso"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure LastUsedRowVersion_InsertModifyDelete_CodeunitIsolation()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
    begin
        Seq.Run(Observer);
        Observer.Expect('insert row/last', 'same');
        Observer.Expect('insert last/prev', 'greater');
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('read again tx', 'Yes');
        Observer.Expect('modify row/last', 'same');
        Observer.Expect('modify last/prev', 'greater');
        Observer.Expect('modify tx', 'Yes');
        Observer.Expect('delete tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_SecondTest_CodeunitIsolation()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
    begin
        Seq.Run(Observer);
        Observer.Expect('insert row/last', 'same');
        Observer.Expect('insert last/prev', 'greater');
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('read again tx', 'Yes');
        Observer.Expect('modify row/last', 'same');
        Observer.Expect('modify last/prev', 'greater');
        Observer.Expect('modify tx', 'Yes');
        Observer.Expect('delete tx', 'Yes');
        Observer.Verify();
    end;
}

codeunit 76605 "RVW Writes Function Iso"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure LastUsedRowVersion_InsertModifyDelete_FunctionIsolation()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
    begin
        Seq.Run(Observer);
        Observer.Expect('insert row/last', 'same');
        Observer.Expect('insert last/prev', 'greater');
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('read again tx', 'Yes');
        Observer.Expect('modify row/last', 'same');
        Observer.Expect('modify last/prev', 'greater');
        Observer.Expect('modify tx', 'Yes');
        Observer.Expect('delete tx', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure LastUsedRowVersion_SecondTest_FunctionIsolation()
    var
        Observer: Codeunit "Probe Observer";
        Seq: Codeunit "RVW Sequence";
    begin
        Seq.Run(Observer);
        Observer.Expect('insert row/last', 'same');
        Observer.Expect('insert last/prev', 'greater');
        Observer.Expect('insert tx', 'Yes');
        Observer.Expect('read again tx', 'Yes');
        Observer.Expect('modify row/last', 'same');
        Observer.Expect('modify last/prev', 'greater');
        Observer.Expect('modify tx', 'Yes');
        Observer.Expect('delete tx', 'Yes');
        Observer.Verify();
    end;
}
