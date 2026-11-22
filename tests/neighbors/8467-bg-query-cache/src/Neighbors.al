// Neighbors of al-runner issue 8467 the probe does not measure, on the probe's tables and page. Refused: a group read of a
// worker after the test session read that group and then wrote the table; a field load into a copy of a record the cache
// answered; CalcFields of a Blob of a row the cache answered with another row (BC's CalcFields of a Blob in a worker waits on
// the rows the test codeunit wrote, the comment at corpus PR 1495 BgTaskQueryCache.al:6534). A row read again after commits
// that wrote it while no commit held it before or after reads the table (BgTaskQueryCache.al:1207). A row read again after a
// commit of a ModifyAll or a DeleteAll of another row follows the BC version (the cells of BQ Tests 12, :7035 and :7042, need
// Base Application to tell the versions apart, so they are not in Probe.al; these assert the answer of BC 27.5, 28.5 and
// 29.0), and is refused where no version is measured; after a Truncate (no cell) it is refused. Read as on main, from the rows
// as they are (no flag): a ReadIsolation read (BC waits on the test session's write, corpus PR 1498, not modelled here). Two
// filter values with the same first characters keep their own entries; a field load after a fetch of another row is not
// refused.

codeunit 97512 "BQ Neighbor Reader"
{
    trigger OnRun()
    var
        Row: Record "BQ Row";
        Copy: Record "BQ Row";
        BlobRow: Record "BQ Blob Row";
        InS: InStream;
        Content: Text;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'copynote':
                begin
                    Row.SetLoadFields(Description);
                    Row.Get(Group, 1);
                    Copy := Row;
                    Seen := Copy.Note;
                end;
            'refetchnote':
                begin
                    Row.SetLoadFields(Description);
                    Row.Get(Group, 1);
                    Row.Get(Group, 2);
                    Seen := Row.Note;
                end;
            'readcommitted':
                begin
                    Row.ReadIsolation := IsolationLevel::ReadCommitted;
                    Row.Get(Group, 1);
                    Seen := Row.Description;
                end;
            'blobread':
                begin
                    BlobRow.Get(Group);
                    BlobRow.CalcFields(Data);
                    BlobRow.Data.CreateInStream(InS);
                    InS.ReadText(Content);
                    Seen := BlobRow.Description + '/' + Content;
                end;
        end;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97511 "BQ Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        BqMode: Codeunit "BQ Mode";

    local procedure Run(var Rows: TestPage "BQ Rows"; Worker: Integer; Mode: Text; Group: Code[20]): Text
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

    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
    end;

    local procedure OpenOn(var Rows: TestPage "BQ Rows"; Group: Code[20])
    begin
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", Group);
        Rows.First();
    end;

    local procedure AddNoted(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
    end;

    local procedure Change(Group: Code[20])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, 1);
        Row.Description := 'second';
        Row.Note := 'n9';
        Row.Modify();
    end;

    local procedure RunGet(var Rows: TestPage "BQ Rows"; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '1');
        Parameters.Add('Mode', 'get');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    [Test]
    procedure CopyOfReplayed_LoadsOmittedField()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB4', 1, 'first', 'n1');
        OpenOn(Rows, 'NB4');
        Assert.AreEqual('first', RunGet(Rows, 'NB4'), 'Get');
        Change('NB4');
        Assert.AreEqual('n1', Run(Rows, Codeunit::"BQ Neighbor Reader", 'copynote', 'NB4'), 'Note of the copy');
        Rows.Close();
    end;

    [Test]
    procedure FetchAfterReplay_LoadsOmittedField()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB5', 1, 'first', 'n1');
        AddNoted('NB5', 2, 'other', 'n2');
        OpenOn(Rows, 'NB5');
        Assert.AreEqual('first', RunGet(Rows, 'NB5'), 'Get');
        Change('NB5');
        Assert.AreEqual('n2', Run(Rows, Codeunit::"BQ Neighbor Reader", 'refetchnote', 'NB5'), 'Note of line 2');
        Rows.Close();
    end;

    [Test]
    procedure ReadIsolation_ReadsTheRows()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddNoted('NB6', 1, 'first', 'n1');
        OpenOn(Rows, 'NB6');
        Assert.AreEqual('first', RunGet(Rows, 'NB6'), 'Get');
        Change('NB6');
        Assert.AreEqual('second', Run(Rows, Codeunit::"BQ Neighbor Reader", 'readcommitted', 'NB6'), 'ReadCommitted Get');
        Rows.Close();
    end;

    [Test]
    procedure LongFilterValues_KeepTheirOwnEntries()
    var
        Rows: TestPage "BQ Rows";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
        Stem: Text[40];
    begin
        BqMode.Set('');
        Stem := 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA';
        AddNoted('NB7', 1, Stem + '1', '');
        AddNoted('NB7', 2, Stem + '2', '');
        AddNoted('NB7', 3, Stem + '2', '');
        OpenOn(Rows, 'NB7');
        Parameters.Add('Group', 'NB7');
        Parameters.Add('Line', '0');
        Parameters.Add('Mode', 'groupdesccount');
        Parameters.Add('Desc', Stem + '1');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Assert.AreEqual('1', Seen, 'Count of the first');
        Parameters.Set('Desc', Stem + '2');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        Assert.AreEqual('2', Seen, 'Count of the second');
        Rows.Close();
    end;

    [Test]
    procedure RowGoneAgain_AfterCommits()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddNoted('NB8H', 1, 'h', '');
        Commit();
        OpenOn(Rows, 'NB8H');
        asserterror begin
            AddNoted('NB8', 1, 'gone', '');
            Assert.AreEqual('gone', RunGet(Rows, 'NB8'), 'Get');
            Error('roll back');
        end;
        AddNoted('NB8', 1, 'back', '');
        Commit();
        Row.Get('NB8', 1);
        Row.Delete();
        Commit();
        Assert.AreEqual('-', RunGet(Rows, 'NB8'), 'Get after the commits');
        Rows.Close();
    end;

    [Test]
    procedure GroupRead_AfterTestReadAndWrite()
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddRow('NB1', 1, 'a');
        Commit();
        Row.SetRange("Group", 'NB1');
        Assert.AreEqual(1, Row.Count(), 'the test''s Count');
        AddRow('NB1', 2, 'b');
        OpenOn(Rows, 'NB1');
        Assert.AreEqual('2', Run(Rows, Codeunit::"BQ Reader", 'count', 'NB1'), 'Count');
        Rows.Close();
    end;

    local procedure AddBlob(No: Code[20]; Description: Text[50]; Content: Text)
    var
        BlobRow: Record "BQ Blob Row";
        OutS: OutStream;
    begin
        BlobRow."No." := No;
        BlobRow.Description := Description;
        BlobRow.Data.CreateOutStream(OutS);
        OutS.WriteText(Content);
        BlobRow.Insert();
    end;

    local procedure SetBlob(No: Code[20]; Description: Text[50]; Content: Text)
    var
        BlobRow: Record "BQ Blob Row";
        OutS: OutStream;
    begin
        BlobRow.Get(No);
        BlobRow.Description := Description;
        BlobRow.Data.CreateOutStream(OutS);
        OutS.WriteText(Content);
        BlobRow.Modify();
    end;

    [Test]
    procedure BlobTable_ReadAgain_AfterChange()
    var
        Rows: TestPage "BQ Rows";
    begin
        BqMode.Set('');
        AddRow('NBB', 1, 'h');
        AddBlob('NBB1', 'first', 'b1');
        Commit();
        OpenOn(Rows, 'NBB');
        Assert.AreEqual('first/b1', Run(Rows, Codeunit::"BQ Neighbor Reader", 'blobread', 'NBB1'), 'Get and CalcFields');
        SetBlob('NBB1', 'second', 'b2');
        Assert.AreEqual('first/b1', Run(Rows, Codeunit::"BQ Neighbor Reader", 'blobread', 'NBB1'), 'Get and CalcFields again');
        Rows.Close();
    end;
}

// The tests a refusal earlier in the codeunit would stop from running (a test codeunit the runner refused a test of runs no
// further test), in a codeunit of their own.
codeunit 97513 "BQ Neighbor Tests 2"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;
        BqMode: Codeunit "BQ Mode";

    local procedure Run(var Rows: TestPage "BQ Rows"; Worker: Integer; Mode: Text; Group: Code[20]): Text
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

    local procedure AddRow(Group: Code[20]; LineNo: Integer; Description: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Insert();
    end;

    local procedure OpenOn(var Rows: TestPage "BQ Rows"; Group: Code[20])
    begin
        Rows.OpenView();
        Rows.Filter.SetFilter("Group", Group);
        Rows.First();
    end;

    local procedure AddNoted(Group: Code[20]; LineNo: Integer; Description: Text[50]; Note: Text[50])
    var
        Row: Record "BQ Row";
    begin
        Row."Group" := Group;
        Row."Line No." := LineNo;
        Row.Description := Description;
        Row.Note := Note;
        Row.Insert();
    end;

    local procedure Change(Group: Code[20])
    var
        Row: Record "BQ Row";
    begin
        Row.Get(Group, 1);
        Row.Description := 'second';
        Row.Note := 'n9';
        Row.Modify();
    end;

    local procedure RunGet(var Rows: TestPage "BQ Rows"; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Line', '1');
        Parameters.Add('Mode', 'get');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure OtherRowBulkCommitted(Group: Code[20]; Bulk: Text)
    var
        Rows: TestPage "BQ Rows";
        Row: Record "BQ Row";
    begin
        BqMode.Set('');
        AddNoted(Group, 1, 'first', '');
        AddNoted(Group, 2, 'other', '');
        Commit();
        OpenOn(Rows, Group);
        Assert.AreEqual('first', RunGet(Rows, Group), 'Get');
        Row.SetRange("Group", Group);
        Row.SetRange("Line No.", 2);
        case Bulk of
            'modifyall':
                Row.ModifyAll(Description, 'other2');
            'deleteall':
                Row.DeleteAll();
            'truncate':
                Row.Truncate();
        end;
        Commit();
        Row.SetRange("Line No.", 1);
        Row.ModifyAll(Description, 'second');
        // BC 27.5, 28.5 and 29.0 end the entry of line 1 at the commit of a ModifyAll or a DeleteAll (BgTaskQueryCache.al:7035,
        // :7042), a local BC 29.1 keeps it (al_version::bulk_commit_ends_cached_row); no cell has the Truncate.
        Assert.AreEqual('second', RunGet(Rows, Group), 'Get after the commit');
        Rows.Close();
    end;

    [Test]
    procedure OtherRowModifyAllCommitted_ThenChanged()
    begin
        OtherRowBulkCommitted('NB9', 'modifyall');
    end;

    [Test]
    procedure OtherRowDeleteAllCommitted_ThenChanged()
    begin
        OtherRowBulkCommitted('NB10', 'deleteall');
    end;

    // Last: a Truncate keeps its table locked to a worker's reads for the rest of the test codeunit on BC.
    [Test]
    procedure OtherRowTruncateCommitted_ThenChanged()
    begin
        OtherRowBulkCommitted('NB11', 'truncate');
    end;
}
