// Neighbors of al-runner issue 8637 that no cell of the corpus probe BgTaskQueryCache.al measures (appended to it), on its
// tables 97593 "BQ Wide" and 97594 "BQ Child", page 97519 "BQ Plain Rows" and a table of their own. Each test asserts BC's
// answer (a local BC 29.1 container) as 'first reads / reads again'. They are past what the cells of BQ Tests 8 measure:
// 26215 filters (on a table of 103 fields; the cells reach 26214 at :4243), and a FlowField filter next to the key set to
// its one value twice, in filter groups 0 and 2 (the cells set each key field by one filter, :4253-:4286). Past :3429 (the
// worker's own record sets filter group 1 while the test session holds none): the test session holds a filter in filter
// group 1 and the worker counts a fresh Record and a RecordRef of the table. Past BQ Tests 8b (:4781, :4794, :4830): a
// FlowField filter of an open range (1..), of an & inside a | (>0&<5|9), a case-sensitive wildcard (K*) and @ alone on a
// lookup, and a sum whose field() is a FlowField of the row after an uncommitted note.

table 97532 "BQ Gate Widest"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; F3; Integer) { }
        field(4; F4; Integer) { }
        field(5; F5; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
        field(8; F8; Integer) { }
        field(9; F9; Integer) { }
        field(10; F10; Integer) { }
        field(11; F11; Integer) { }
        field(12; F12; Integer) { }
        field(13; F13; Integer) { }
        field(14; F14; Integer) { }
        field(15; F15; Integer) { }
        field(16; F16; Integer) { }
        field(17; F17; Integer) { }
        field(18; F18; Integer) { }
        field(19; F19; Integer) { }
        field(20; F20; Integer) { }
        field(21; F21; Integer) { }
        field(22; F22; Integer) { }
        field(23; F23; Integer) { }
        field(24; F24; Integer) { }
        field(25; F25; Integer) { }
        field(26; F26; Integer) { }
        field(27; F27; Integer) { }
        field(28; F28; Integer) { }
        field(29; F29; Integer) { }
        field(30; F30; Integer) { }
        field(31; F31; Integer) { }
        field(32; F32; Integer) { }
        field(33; F33; Integer) { }
        field(34; F34; Integer) { }
        field(35; F35; Integer) { }
        field(36; F36; Integer) { }
        field(37; F37; Integer) { }
        field(38; F38; Integer) { }
        field(39; F39; Integer) { }
        field(40; F40; Integer) { }
        field(41; F41; Integer) { }
        field(42; F42; Integer) { }
        field(43; F43; Integer) { }
        field(44; F44; Integer) { }
        field(45; F45; Integer) { }
        field(46; F46; Integer) { }
        field(47; F47; Integer) { }
        field(48; F48; Integer) { }
        field(49; F49; Integer) { }
        field(50; F50; Integer) { }
        field(51; F51; Integer) { }
        field(52; F52; Integer) { }
        field(53; F53; Integer) { }
        field(54; F54; Integer) { }
        field(55; F55; Integer) { }
        field(56; F56; Integer) { }
        field(57; F57; Integer) { }
        field(58; F58; Integer) { }
        field(59; F59; Integer) { }
        field(60; F60; Integer) { }
        field(61; F61; Integer) { }
        field(62; F62; Integer) { }
        field(63; F63; Integer) { }
        field(64; F64; Integer) { }
        field(65; F65; Integer) { }
        field(66; F66; Integer) { }
        field(67; F67; Integer) { }
        field(68; F68; Integer) { }
        field(69; F69; Integer) { }
        field(70; F70; Integer) { }
        field(71; F71; Integer) { }
        field(72; F72; Integer) { }
        field(73; F73; Integer) { }
        field(74; F74; Integer) { }
        field(75; F75; Integer) { }
        field(76; F76; Integer) { }
        field(77; F77; Integer) { }
        field(78; F78; Integer) { }
        field(79; F79; Integer) { }
        field(80; F80; Integer) { }
        field(81; F81; Integer) { }
        field(82; F82; Integer) { }
        field(83; F83; Integer) { }
        field(84; F84; Integer) { }
        field(85; F85; Integer) { }
        field(86; F86; Integer) { }
        field(87; F87; Integer) { }
        field(88; F88; Integer) { }
        field(89; F89; Integer) { }
        field(90; F90; Integer) { }
        field(91; F91; Integer) { }
        field(92; F92; Integer) { }
        field(93; F93; Integer) { }
        field(94; F94; Integer) { }
        field(95; F95; Integer) { }
        field(96; F96; Integer) { }
        field(97; F97; Integer) { }
        field(98; F98; Integer) { }
        field(99; F99; Integer) { }
        field(100; F100; Integer) { }
        field(101; F101; Integer) { }
        field(102; F102; Integer) { }
        field(103; F103; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker: FindFirst, FindLast, Count and IsEmpty of the filters a Mode names, or the Counts of 'freshcount'.
codeunit 97530 "BQ Gate Reader"
{
    trigger OnRun()
    var
        Wide: Record "BQ Wide";
        WideRef: RecordRef;
        Calc: Record "BQ Calc";
        Reader: Codeunit "BQ Reader 8";
        Reader8b: Codeunit "BQ Reader 8b";
        Mode: Text;
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        K: Integer;
        FieldNo: Integer;
        FilterGroupNo: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        if Mode.StartsWith('8b-') then begin
            Results.Add('Seen', Reader8b.Forms(CopyStr(Mode, 4), Group));
            Page.SetBackgroundTaskResult(Results);
            exit;
        end;
        case Mode of
            'many26215':
                begin
                    WideRef.Open(Database::"BQ Gate Widest");
                    for K := 0 to 26214 do begin
                        FieldNo := K mod 103 + 1;
                        FilterGroupNo := K div 103;
                        if FilterGroupNo > 0 then
                            FilterGroupNo += 1;
                        WideRef.FilterGroup(FilterGroupNo);
                        case FieldNo of
                            1:
                                WideRef.Field(1).SetRange(Group);
                            2:
                                WideRef.Field(2).SetFilter('<>zz');
                            else
                                WideRef.Field(FieldNo).SetRange(0);
                        end;
                    end;
                    WideRef.FilterGroup(0);
                    Results.Add('Seen', Reader.RefForms(WideRef));
                end;
            'keytwice':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(2);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'openrange', 'nested':
                begin
                    Wide.SetRange("No.", Group);
                    if Mode = 'openrange' then
                        Wide.SetFilter("Child Count", '1..')
                    else
                        Wide.SetFilter("Child Count", '>0&<5|9');
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'wildcs':
                begin
                    Calc.SetRange("No.", Group);
                    Calc.SetFilter("Line Kind", 'K*');
                    Results.Add('Seen', Reader.CalcForms(Calc));
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

    // The row a Mode reads committed (table 97593, 97597, or 97591 with its line 1 of table 97592); the four reads; the test
    // inserts a line of table 97594, or for table 97591 a note of table 97595, without committing; the four reads again.
    local procedure Reads(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Busy: Record "BQ Busy";
        Line: Record "BQ Noted Line";
        Note: Record "BQ Note";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        case true of
            Mode.Contains('calc'), Mode = 'wildcs':
                begin
                    Calc."No." := G;
                    Calc.Description := 'a';
                    Calc.Insert();
                end;
            Mode.Contains('amount'):
                begin
                    Busy."No." := G;
                    Busy.Description := 'a';
                    Busy.Insert();
                    Line.Parent := G;
                    Line."Line No." := 1;
                    Line.Amount := 5;
                    Line.Insert();
                end;
            else begin
                Wide."No." := G;
                Wide.Description := 'a';
                Wide.Insert();
            end;
        end;
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        if Mode.Contains('amount') then begin
            Note.Parent := G;
            Note."Line No." := 1;
            Note.Insert();
            Note.SetRange(Parent, G);
            if Note.FindFirst() then;
        end else
            AddChild(G, 1);
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure FlowFieldOpenRange_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('openrange'), '1..');
    end;

    [Test]
    procedure FlowFieldAndInsideOr_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('nested'), '>0&<5|9');
    end;

    [Test]
    procedure FlowFieldCaseSensitiveWildcard_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('wildcs'), 'K*');
    end;

    [Test]
    procedure FlowFieldOverAFlowFieldOfTheRow_NoteAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads('8b-amount'), 'Amount At Note Count 5');
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

    // The four reads of 26215 filters on table 97532 on an empty set; the test inserts a row they keep; the four reads again.
    [Test]
    procedure Filters26215_InsertAfterFirstRead()
    var
        Rows: TestPage "BQ Plain Rows";
        Widest: Record "BQ Gate Widest";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Rows.OpenView();
        Seen := Run(Rows, 'many26215', G);
        Widest."No." := G;
        Widest.Description := 'a';
        Widest.Insert();
        Widest.SetRange("No.", G);
        if Widest.FindFirst() then;
        Seen += ' / ' + Run(Rows, 'many26215', G);
        Rows.Close();
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Seen, 'after the insert');
    end;

    // The row of table 97593 committed without a line; the four reads of "Child Count" 1 next to "No." set to the row twice;
    // the test inserts a line; the four reads again.
    [Test]
    procedure FlowFieldKeyTwice_LineAfterFirstRead()
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Wide."No." := G;
        Wide.Description := 'a';
        Wide.Insert();
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, 'keytwice', G);
        AddChild(G, 1);
        Seen += ' / ' + Run(Rows, 'keytwice', G);
        Rows.Close();
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Seen, 'after the insert');
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

    // Last: the runner refuses @ alone on a Code field before any read.
    [Test]
    procedure FlowFieldAtSignAlone_LineAfterFirstRead()
    begin
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', Reads('8b-calcat'), '@');
    end;
}
