// Neighbors of al-runner issue 8637 on the objects of the corpus probe BgTaskQueryCache.al (appended to it): the read forms next
// to the three that open corpus pull request 1495 measures (:1276 FindLast, :1293 IsEmpty, both after SetRange of the group;
// :1312 FindFirst after SetFilter of the group to one value), repeated by a worker after the test changed the rows.
// - Read as on main, from the rows as they are (no flag): another filter next to the group's, a filter on a field outside the
//   key, the filter in another filter group, a range, a SetFilter of two values or of a wildcard, a descending key, Find('+'),
//   SetLoadFields before FindLast, and the three forms on a table whose primary key has one field (a read of a row there).
// - Answered from the cache (flagged): more rows in the group than measured, SetFilter with a placeholder, the test session's
//   read of the same answer before the worker's, and a commit of the table that leaves the answer as it was.
// - Refused: a FindFirst by SetFilter after a FindFirst by SetRange of the same group, or the other way round, when the earlier
//   read's answer differs from the table's (whether they share an entry is not measured); a worker's first read of a form after
//   the test session read it and the table's answer changed since (whether the test's read fills the cache is not measured);
//   a worker's read after a commit of the table changed the answer, and the test session's read after the commit (measured
//   only for FindFirst and Count of a group, :823, :845).
// Each refusal is in a test codeunit of its own, so it does not stop the tests after it. The tests use the probe's page 97519
// "BQ Plain Rows", as BQ Tests 3 does, and declare no BQ Mode.

table 97530 "BQ One"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker: one read of BQ Row or BQ One, chosen by the Mode parameter.
codeunit 97531 "BQ Filter Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        One: Record "BQ One";
        RowRef: RecordRef;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'lasttwo':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Seen := Last(Row);
                end;
            'lastnote':
                begin
                    Row.SetRange(Note, Group);
                    Seen := Last(Row);
                end;
            'lastgroup2':
                begin
                    Row.FilterGroup(2);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                    Seen := Last(Row);
                end;
            'lastrange':
                begin
                    Row.SetRange("Group", Group, Group + 'Z');
                    Seen := Last(Row);
                end;
            'lastdesc':
                begin
                    Row.Ascending(false);
                    Row.SetRange("Group", Group);
                    Seen := Last(Row);
                end;
            'findplus':
                begin
                    Row.SetRange("Group", Group);
                    if Row.Find('+') then
                        Seen := Row.Description
                    else
                        Seen := '-';
                end;
            'lastload':
                begin
                    Row.SetLoadFields(Description);
                    Row.SetRange("Group", Group);
                    Seen := Last(Row);
                end;
            'emptytwo':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Seen := Format(Row.IsEmpty());
                end;
            'emptynote':
                begin
                    Row.SetRange(Note, Group);
                    Seen := Format(Row.IsEmpty());
                end;
            'emptygroup2':
                begin
                    Row.FilterGroup(2);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                    Seen := Format(Row.IsEmpty());
                end;
            'setfilterpair':
                begin
                    Row.SetFilter("Group", '%1|%2', Group, 'NOGROUP');
                    Seen := First(Row);
                end;
            'setfilterwild':
                begin
                    Row.SetFilter("Group", Group + '*');
                    Seen := First(Row);
                end;
            'setfiltertwo':
                begin
                    Row.SetFilter("Group", Group);
                    Row.SetRange(Note, '');
                    Seen := First(Row);
                end;
            'setfilterholder':
                begin
                    Row.SetFilter("Group", '%1', Group);
                    Seen := First(Row);
                end;
            'rangefirst':
                begin
                    Row.SetRange("Group", Group);
                    Seen := First(Row);
                end;
            'reflast':
                begin
                    RowRef.Open(Database::"BQ Row");
                    RowRef.Field(Row.FieldNo("Group")).SetRange(Group);
                    if RowRef.FindLast() then
                        Seen := Format(RowRef.Field(Row.FieldNo(Description)).Value)
                    else
                        Seen := '-';
                end;
            'onelast':
                begin
                    One.SetRange("No.", Group);
                    if One.FindLast() then
                        Seen := One.Description
                    else
                        Seen := '-';
                end;
            'oneempty':
                begin
                    One.SetRange("No.", Group);
                    Seen := Format(One.IsEmpty());
                end;
            'onesetfilter':
                begin
                    One.SetFilter("No.", Group);
                    if One.FindFirst() then
                        Seen := One.Description
                    else
                        Seen := '-';
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;

    local procedure Last(var Row: Record "BQ Row"): Text
    begin
        if Row.FindLast() then
            exit(Row.Description);
        exit('-');
    end;

    local procedure First(var Row: Record "BQ Row"): Text
    begin
        if Row.FindFirst() then
            exit(Row.Description);
        exit('-');
    end;
}

// What the test codeunits below share.
codeunit 97532 "BQ Filter Steps"
{
    procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    procedure SetRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, LineNo);
        Row.Description := Description;
        Row.Modify();
    end;

    procedure AddOne(No: Code[20]; Description: Text[50])
    var
        One: Record "BQ One";
    begin
        One."No." := No;
        One.Description := Description;
        One.Insert();
    end;

    procedure SetOne(No: Code[20]; Description: Text[50])
    var
        One: Record "BQ One";
    begin
        One.Get(No);
        One.Description := Description;
        One.Modify();
    end;

    procedure Open(var Rows: TestPage "BQ Plain Rows")
    begin
        Rows.OpenView();
    end;

    procedure Run(var Rows: TestPage "BQ Plain Rows"; Worker: Integer; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '0');
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Worker, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The worker reads a, the test changes line 1 to b, the worker reads again.
    procedure ChangedRow(Mode: Text; Group: Code[20]; Note: Text[50]): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Assert: Codeunit Assert;
        Seen: Text;
    begin
        AddRow(Group, 1, 'a', Note);
        Open(Rows);
        Assert.AreEqual('a', Run(Rows, Codeunit::"BQ Filter Reader", Mode, Group), 'the first read');
        SetRow(Group, 1, 'b');
        Seen := Run(Rows, Codeunit::"BQ Filter Reader", Mode, Group);
        Rows.Close();
        exit(Seen);
    end;

    // The worker reads an empty group, the test inserts a line of it, the worker reads again.
    procedure InsertedRow(Mode: Text; Group: Code[20]; Note: Text[50]): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Assert: Codeunit Assert;
        Seen: Text;
    begin
        AddRow(CopyStr(Group + 'H', 1, 20), 1, 'h', '');
        Open(Rows);
        Assert.AreEqual('Yes', Run(Rows, Codeunit::"BQ Filter Reader", Mode, Group), 'the first read');
        AddRow(Group, 1, 'c', Note);
        Seen := Run(Rows, Codeunit::"BQ Filter Reader", Mode, Group);
        Rows.Close();
        exit(Seen);
    end;
}

// Shapes the cells do not measure that read the rows as they are, as on main.
codeunit 97533 "BQ Filter Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    [Test]
    procedure FindLast_SecondFilter_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lasttwo', 'NF1', ''), 'FindLast');
    end;

    [Test]
    procedure FindLast_NonKeyFilter_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lastnote', 'NF2', 'NF2'), 'FindLast');
    end;

    [Test]
    procedure FindLast_OtherFilterGroup_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lastgroup2', 'NF3', ''), 'FindLast');
    end;

    [Test]
    procedure FindLast_Range_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lastrange', 'NF4', ''), 'FindLast');
    end;

    [Test]
    procedure FindLast_Descending_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lastdesc', 'NF5', ''), 'FindLast');
    end;

    [Test]
    procedure FindPlus_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('findplus', 'NF6', ''), 'Find(+)');
    end;

    [Test]
    procedure FindLast_SetLoadFields_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('lastload', 'NF7', ''), 'FindLast');
    end;

    [Test]
    procedure FindLast_RecordRef_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('reflast', 'NF18', ''), 'RecordRef FindLast');
    end;

    [Test]
    procedure IsEmpty_SecondFilter_ReadsTheRows()
    begin
        Assert.AreEqual('No', Steps.InsertedRow('emptytwo', 'NF8', ''), 'IsEmpty');
    end;

    [Test]
    procedure IsEmpty_NonKeyFilter_ReadsTheRows()
    begin
        Assert.AreEqual('No', Steps.InsertedRow('emptynote', 'NF9', 'NF9'), 'IsEmpty');
    end;

    [Test]
    procedure IsEmpty_OtherFilterGroup_ReadsTheRows()
    begin
        Assert.AreEqual('No', Steps.InsertedRow('emptygroup2', 'NF10', ''), 'IsEmpty');
    end;

    [Test]
    procedure SetFilter_TwoValues_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('setfilterpair', 'NF11', ''), 'FindFirst');
    end;

    [Test]
    procedure SetFilter_Wildcard_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('setfilterwild', 'NF12', ''), 'FindFirst');
    end;

    [Test]
    procedure SetFilter_SecondFilter_ReadsTheRows()
    begin
        Assert.AreEqual('b', Steps.ChangedRow('setfiltertwo', 'NF13', ''), 'FindFirst');
    end;

    [Test]
    procedure OneFieldKey_Forms_ReadTheRows()
    var
        Rows: TestPage "BQ Plain Rows";
        Reader: Integer;
    begin
        Reader := Codeunit::"BQ Filter Reader";
        Steps.AddOne('NF14', 'a');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Reader, 'onelast', 'NF14'), 'FindLast');
        Assert.AreEqual('a', Steps.Run(Rows, Reader, 'onesetfilter', 'NF14'), 'SetFilter FindFirst');
        Assert.AreEqual('Yes', Steps.Run(Rows, Reader, 'oneempty', 'NF15'), 'IsEmpty');
        Steps.SetOne('NF14', 'b');
        Steps.AddOne('NF15', 'c');
        Assert.AreEqual('b', Steps.Run(Rows, Reader, 'onelast', 'NF14'), 'FindLast again');
        Assert.AreEqual('b', Steps.Run(Rows, Reader, 'onesetfilter', 'NF14'), 'SetFilter FindFirst again');
        Assert.AreEqual('No', Steps.Run(Rows, Reader, 'oneempty', 'NF15'), 'IsEmpty again');
        Rows.Close();
    end;

    // The worker's FindLast fills the cache; the test changes the line and reads the group by FindLast itself, after it wrote
    // the table: its own read sees the table.
    [Test]
    procedure TestFindLast_AfterWrite_ReadsTheTable()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
    begin
        Steps.AddRow('NF16', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NF16'), 'the worker''s FindLast');
        Steps.SetRow('NF16', 1, 'b');
        Row.SetRange("Group", 'NF16');
        Row.FindLast();
        Assert.AreEqual('b', Row.Description, 'the test''s FindLast');
        Rows.Close();
    end;

    // The test page shows the group through a filter, the test changes the line: the worker's first read by SetFilter.
    [Test]
    procedure PageFilter_ChangedRow_WorkerFirstRead()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NF17', 1, 'a', '');
        Steps.Open(Rows);
        Rows.Filter.SetFilter("Group", 'NF17');
        Rows.First();
        Steps.SetRow('NF17', 1, 'b');
        Assert.AreEqual('b', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'setfilter', 'NF17'), 'FindFirst');
        Rows.Close();
    end;
}

// Shapes of the measured forms in states the cells do not have, answered from the cache.
codeunit 97534 "BQ Filter Cache Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // Two lines in the group; the test changes the last one.
    [Test]
    procedure FindLast_TwoRows_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NC1', 1, 'a', '');
        Steps.AddRow('NC1', 2, 'x', '');
        Steps.Open(Rows);
        Assert.AreEqual('x', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC1'), 'FindLast');
        Steps.SetRow('NC1', 2, 'y');
        Assert.AreEqual('x', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC1'), 'FindLast again');
        Rows.Close();
    end;

    [Test]
    procedure SetFilter_Placeholder_FirstAnswer()
    begin
        Assert.AreEqual('a', Steps.ChangedRow('setfilterholder', 'NC2', ''), 'FindFirst');
    end;

    // The test session's FindLast before the worker's, with the same answer.
    [Test]
    procedure TestFindLastFirst_SameAnswer_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
    begin
        Steps.AddRow('NC3', 1, 'a', '');
        Commit();
        Row.SetRange("Group", 'NC3');
        Row.FindLast();
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC3'), 'FindLast');
        Steps.SetRow('NC3', 1, 'b');
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC3'), 'FindLast again');
        Rows.Close();
    end;

    // A commit of another group of the table between the worker's FindLast reads leaves the answer as it was.
    [Test]
    procedure FindLast_CommitSameAnswer_FirstAnswer()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NC4', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC4'), 'FindLast');
        Steps.AddRow('NC4H', 1, 'h', '');
        Commit();
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC4'), 'FindLast after the commit');
        Steps.SetRow('NC4', 1, 'b');
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NC4'), 'FindLast again');
        Rows.Close();
    end;
}

codeunit 97535 "BQ Filter Refused 1"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // A FindFirst by SetRange of the group, the test changes the line, a FindFirst by SetFilter of the group.
    [Test]
    procedure SetRangeThenSetFilter_ChangedRow()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NR1', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Filter Reader", 'rangefirst', 'NR1'), 'SetRange FindFirst');
        Steps.SetRow('NR1', 1, 'b');
        Assert.AreEqual('b', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'setfilter', 'NR1'), 'SetFilter FindFirst');
        Rows.Close();
    end;
}

codeunit 97536 "BQ Filter Refused 2"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // A FindFirst by SetFilter of the group, the test changes the line, a FindFirst by SetRange of the group.
    [Test]
    procedure SetFilterThenSetRange_ChangedRow()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NR2', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'setfilter', 'NR2'), 'SetFilter FindFirst');
        Steps.SetRow('NR2', 1, 'b');
        Assert.AreEqual('b', Steps.Run(Rows, Codeunit::"BQ Filter Reader", 'rangefirst', 'NR2'), 'SetRange FindFirst');
        Rows.Close();
    end;
}

codeunit 97537 "BQ Filter Refused 3"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // The test session's FindLast of the group before it changed the line: the worker's first FindLast.
    [Test]
    procedure TestFindLastFirst_ChangedRow()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
    begin
        Steps.AddRow('NR3', 1, 'a', '');
        Commit();
        Row.SetRange("Group", 'NR3');
        Row.FindLast();
        Steps.SetRow('NR3', 1, 'b');
        Steps.Open(Rows);
        Assert.AreEqual('b', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NR3'), 'FindLast');
        Rows.Close();
    end;
}

codeunit 97538 "BQ Filter Refused 4"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // The test session's IsEmpty of an empty group, then it inserts a line of it: the worker's first IsEmpty.
    [Test]
    procedure TestIsEmptyFirst_Inserted()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
    begin
        Steps.AddRow('NR4H', 1, 'h', '');
        Commit();
        Row.SetRange("Group", 'NR4');
        Assert.IsTrue(Row.IsEmpty(), 'the test''s IsEmpty');
        Steps.AddRow('NR4', 1, 'c', '');
        Steps.Open(Rows);
        Assert.AreEqual('No', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'empty', 'NR4'), 'IsEmpty');
        Rows.Close();
    end;
}

codeunit 97539 "BQ Filter Refused 5"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // The worker's FindLast; the test changes the line and commits: the worker's FindLast again.
    [Test]
    procedure FindLast_ChangeCommitted()
    var
        Rows: TestPage "BQ Plain Rows";
    begin
        Steps.AddRow('NR5', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NR5'), 'FindLast');
        Steps.SetRow('NR5', 1, 'b');
        Commit();
        Assert.AreEqual('b', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NR5'), 'FindLast after the commit');
        Rows.Close();
    end;
}

codeunit 97540 "BQ Filter Refused 6"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        Steps: Codeunit "BQ Filter Steps";

    // The worker's FindLast; the test changes the line and commits: the test session's own FindLast.
    [Test]
    procedure TestFindLast_AfterChangeCommitted()
    var
        Rows: TestPage "BQ Plain Rows";
        Row: Record "BQ Row";
    begin
        Steps.AddRow('NR6', 1, 'a', '');
        Steps.Open(Rows);
        Assert.AreEqual('a', Steps.Run(Rows, Codeunit::"BQ Reader 3", 'last', 'NR6'), 'FindLast');
        Steps.SetRow('NR6', 1, 'b');
        Commit();
        Row.SetRange("Group", 'NR6');
        Row.FindLast();
        Assert.AreEqual('b', Row.Description, 'the test''s FindLast');
        Rows.Close();
    end;
}
