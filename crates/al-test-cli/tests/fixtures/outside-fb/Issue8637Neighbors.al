// Neighbors of al-runner issue 8637 that no cell of the corpus probe BgTaskQueryCache.al measures (appended to it), on its
// table 97500 and page 97519 "BQ Plain Rows", without BQ Mode. Each test asserts BC's answer (a local BC 29.1 container).
// - BQ Gate Tests: the cache answers five filters, filter groups 3 and 5, and the cross-column OR group -1 with the first
//   answers. They are past what the cells measure, and the runner keeps main's answer, so they fail on main and on the branch
//   alike.
// - BQ Gate Test First 1 and 2 (a codeunit each, as the runner refuses them): the test session's reads of two filters, or of a
//   filter outside the key, after a commit and before its change answer the worker's first reads. No cell measures a test
//   session's read first in these shapes, and the runner refuses the worker's read.

// The worker: FindFirst, FindLast, Count and IsEmpty of the filters a Mode names.
codeunit 97530 "BQ Gate Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        SetShape(Row, Parameters.Get('Mode'), Group);
        Results.Add('Seen', Forms(Row));
        Page.SetBackgroundTaskResult(Results);
    end;

    procedure SetShape(var Row: Record "BQ Row"; Mode: Text; Group: Text)
    begin
        case Mode of
            'five':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                    Row.SetFilter(Description, '<>%1', 'zz');
                    Row.SetRange("Line No.", 1, 9);
                    Row.FilterGroup(2);
                    Row.SetRange(Note, '');
                    Row.FilterGroup(0);
                end;
            'group3':
                begin
                    Row.FilterGroup(3);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'group5':
                begin
                    Row.FilterGroup(5);
                    Row.SetRange("Group", Group);
                    Row.FilterGroup(0);
                end;
            'orgroup':
                begin
                    Row.FilterGroup(-1);
                    Row.SetRange("Group", Group);
                    Row.SetRange(Description, 'NODESCRIPTION');
                    Row.FilterGroup(0);
                end;
            'two':
                begin
                    Row.SetRange("Group", Group);
                    Row.SetRange(Note, '');
                end;
            'note':
                Row.SetRange(Note, Group);
        end;
    end;

    procedure Forms(var Row: Record "BQ Row"): Text
    var
        First: Text;
        Last: Text;
    begin
        First := '-';
        Last := '-';
        if Row.FindFirst() then
            First := Row.Description;
        if Row.FindLast() then
            Last := Row.Description;
        exit(First + '|' + Last + '|' + Format(Row.Count()) + '|' + Format(Row.IsEmpty()));
    end;
}

codeunit 97531 "BQ Gate Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    // Inserts a line, then reads the group (as AddRow of BQ Tests).
    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
        Row.SetRange("Group", Group);
        if Row.FindFirst() then;
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The four reads of a Mode on an empty group; the test inserts a line the filters keep; the four reads again.
    local procedure InsertAfterFirstRead(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Assert.AreEqual('-|-|0|Yes', Run(Rows, Mode, G), Mode + ': the first reads');
        AddRow(G, 1, 'a');
        Seen := Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure FiveFilters_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('five'), 'after the insert');
    end;

    [Test]
    procedure FilterGroup3_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group3'), 'after the insert');
    end;

    [Test]
    procedure FilterGroup5_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('group5'), 'after the insert');
    end;

    [Test]
    procedure OrGroup_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes', InsertAfterFirstRead('orgroup'), 'after the insert');
    end;
}

codeunit 97532 "BQ Gate Test First 1"
{
    Subtype = Test;

    // The test session reads the four forms of two filters after a commit; it changes line 1 and inserts line 2: the worker's
    // first four reads.
    [Test]
    procedure TestReadsFirst_TwoFilters()
    var
        TestFirst: Codeunit "BQ Gate Test First";
    begin
        TestFirst.Check('two', false);
    end;
}

codeunit 97533 "BQ Gate Test First 2"
{
    Subtype = Test;

    // The same with one filter, outside the key.
    [Test]
    procedure TestReadsFirst_NonKeyFilter()
    var
        TestFirst: Codeunit "BQ Gate Test First";
    begin
        TestFirst.Check('note', true);
    end;
}

codeunit 97534 "BQ Gate Test First"
{
    procedure Check(Mode: Text; GroupAsNote: Boolean)
    var
        Row: Record "BQ Row";
        Reader: Codeunit "BQ Gate Reader";
        Rows: TestPage "BQ Plain Rows";
        Assert: Codeunit Assert;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        G: Code[20];
        Note: Text[50];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        if GroupAsNote then
            Note := G;
        Insert(G, 1, 'a', Note);
        Commit();
        Reader.SetShape(Row, Mode, G);
        Assert.AreEqual('a|a|1|No', Reader.Forms(Row), 'the test''s reads');
        Row.Reset();
        Row.Get(G, 1);
        Row.Description := 'b';
        Row.Modify();
        Insert(G, 2, 'c', Note);
        Rows.OpenView();
        Parameters.Add('Group', G);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Rows.Close();
        Assert.AreEqual('a|a|1|No', Seen, 'the worker''s reads');
    end;

    local procedure Insert(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
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
}
