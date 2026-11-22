// Neighbors of al-runner issue 8637 that no cell of the corpus probe BgTaskQueryCache.al measures (appended to it), on its
// tables 97593 "BQ Wide" and 97594 "BQ Child" and page 97519 "BQ Plain Rows". Each test asserts BC's answer (a local BC 29.1
// container) as 'first reads / reads again'. They are past what the cells of BQ Tests 7 measure: 7651 filters (the 7650 of
// :3408 and one in the OR group), and a FlowField filter next to the key with a filter outside the key, as a range, or with
// the key in filter group 5. Past :3429 (the worker's own record sets filter group 1 while the test session holds none): the
// test session holds a filter in filter group 1 and the worker counts a fresh Record and a RecordRef of the table.

// The worker: FindFirst, FindLast, Count and IsEmpty of the filters a Mode names.
codeunit 97530 "BQ Gate Reader"
{
    trigger OnRun()
    var
        Wide: Record "BQ Wide";
        WideRef: RecordRef;
        Reader: Codeunit "BQ Reader 7";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        case Parameters.Get('Mode') of
            'wide7651':
                begin
                    WideRef.Open(Database::"BQ Wide");
                    Reader.SetWide(WideRef, 7650, Group);
                    WideRef.FilterGroup(-1);
                    WideRef.Field(2).SetFilter('<>zz');
                    WideRef.FilterGroup(0);
                    Results.Add('Seen', Reader.WideForms(WideRef));
                end;
            'flowkeydesc':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange(Description, 'a');
                    Wide.SetRange("Child Count", 1);
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'flowkeyrange':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.SetRange("Child Count", 1, 5);
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'flowkeygroup5':
                begin
                    Wide.FilterGroup(5);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'freshcount':
                begin
                    WideRef.Open(Database::"BQ Wide");
                    Results.Add('Seen', Format(Wide.Count()) + ',' + Format(WideRef.Count()));
                end;
        end;
        Page.SetBackgroundTaskResult(Results);
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

    local procedure AddWide(No: Code[20])
    var
        Wide: Record "BQ Wide";
    begin
        Wide."No." := No;
        Wide.Description := 'a';
        Wide.Insert();
        Wide.SetRange("No.", No);
        if Wide.FindFirst() then;
    end;

    local procedure AddChild(Parent: Code[20]; LineNo: Integer)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := 'K';
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
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

    // The four reads of a Mode; the test inserts the row of table 97593 ('row'), or a line of table 97594 for the row it
    // committed before ('line'); the four reads again.
    local procedure Reads(Mode: Text; Change: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        if Change = 'line' then begin
            AddWide(G);
            Commit();
        end;
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        if Change = 'line' then
            AddChild(G, 1)
        else
            AddWide(G);
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure Filters7651_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads('wide7651', 'row'), 'after the insert');
    end;

    [Test]
    procedure FlowFieldKeyAndDescription_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('flowkeydesc', 'line'), 'after the insert');
    end;

    [Test]
    procedure FlowFieldRange_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('flowkeyrange', 'line'), 'after the insert');
    end;

    [Test]
    procedure FlowFieldKeyInFilterGroup5_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('flowkeygroup5', 'line'), 'after the insert');
    end;

    // The test session sets filter group 1 of table 97593 to F3 7 (an Integer field outside the key; every other row holds 0);
    // the worker counts a fresh Record and a RecordRef of the table (the task of page 97518, on another table); the test
    // inserts a row with F3 7; the worker counts again; the test takes its filter group 1 back. The second counts less the
    // first, as 'Record,RecordRef'.
    [Test]
    procedure TestSessionGroup1_WorkerCountsFresh_InsertAfterFirstRead()
    var
        Rows: TestPage "BQ Single Rows";
        Wide: Record "BQ Wide";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        G: Code[20];
        First: Text;
        Second: Text;
        Before: Integer;
        After: Integer;
        Seen: Text;
        K: Integer;
    begin
        G := NewGroup();
        Wide.FilterGroup(1);
        Wide.SetRange(F3, 7);
        Wide.FilterGroup(0);
        Rows.OpenView();
        Parameters.Add('Group', G);
        Parameters.Add('Mode', 'freshcount');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', First);
        Wide."No." := G;
        Wide.Description := 'a';
        Wide.F3 := 7;
        Wide.Insert();
        if Wide.FindFirst() then;
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', Second);
        Rows.Close();
        Wide.FilterGroup(1);
        Wide.SetRange(F3);
        Wide.FilterGroup(0);
        for K := 1 to 2 do begin
            Evaluate(Before, SelectStr(K, First));
            Evaluate(After, SelectStr(K, Second));
            if K = 2 then
                Seen += ',';
            Seen += Format(After - Before);
        end;
        Assert.AreEqual('0,0', Seen, 'the second counts less the first');
    end;
}
