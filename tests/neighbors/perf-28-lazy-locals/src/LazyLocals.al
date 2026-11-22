// Neighbors of al-runner issue 8610: a Record local of a stored table is built at the first statement that names
// it instead of at procedure entry. The change must give main's answer in every shape. The tests vary where a local is
// first named (a statement, the header of an if, case, while, repeat or for, the bodies of each, asserterror, exit, an
// argument), what runs between entry and that statement (inserts, an error, a recursion, other locals), the routine kind
// that declares it (procedure, OnRun, table trigger, event subscriber), and the locals that stay eager (temporary, array,
// parameter, return value). Every test answers on main and on head alike.
table 50100 "NB LR Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { InitValue = 'fresh'; }
        field(3; Qty; Integer) { InitValue = 7; }
        field(4; Payload; Blob) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Record "NB LR Log";
        Unused: Record "NB LR Row";
    begin
        if Name = 'logged' then begin
            Log.Init();
            Log.Entry := 0;
            Log.Note := "No.";
            Log.Insert(true);
        end;
    end;
}

table 50101 "NB LR Log"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Entry; Integer) { AutoIncrement = true; }
        field(2; Note; Text[50]) { }
    }

    keys
    {
        key(PK; Entry) { Clustered = true; }
    }
}

codeunit 50102 "NB LR Shapes"
{
    procedure NeverNamed(): Text
    var
        A: Record "NB LR Row";
        B: Record "NB LR Row";
        C: Record "NB LR Log";
    begin
        exit('none named');
    end;

    procedure IfHeaderOnly(RowNo: Code[20]): Boolean
    var
        R: Record "NB LR Row";
    begin
        if R.Get(RowNo) then
            exit(true);
        exit(false);
    end;

    procedure ThenBranchOnly(Take: Boolean): Text
    var
        R: Record "NB LR Row";
    begin
        if Take then
            exit(R.Name);
        exit('skipped');
    end;

    procedure ElseBranchOnly(Take: Boolean): Text
    var
        R: Record "NB LR Row";
    begin
        if Take then
            exit('then')
        else
            exit(Format(R.Qty));
    end;

    procedure BuiltInBranchUsedAfter(Take: Boolean): Text
    var
        R: Record "NB LR Row";
    begin
        if Take then
            R.Name := 'set in branch';
        exit(R.Name);
    end;

    procedure CaseScrutinee(): Integer
    var
        R: Record "NB LR Row";
    begin
        case R.Qty of
            7:
                exit(1);
            else
                exit(2);
        end;
    end;

    procedure CaseBranchAndElse(Which: Integer): Text
    var
        R: Record "NB LR Row";
        S: Record "NB LR Row";
    begin
        case Which of
            1:
                exit(R.Name);
            else
                exit(Format(S.Qty));
        end;
    end;

    procedure WhileCondition(): Integer
    var
        R: Record "NB LR Row";
        N: Integer;
    begin
        while R.Qty > N do
            N += 1;
        exit(N);
    end;

    procedure WhileNestedBody(): Text
    var
        R: Record "NB LR Row";
        N: Integer;
        Seen: Text;
    begin
        while N < 3 do begin
            N += 1;
            if N = 2 then
                Seen += R.Name
            else
                Seen += Format(N);
        end;
        exit(Seen);
    end;

    procedure RepeatUntilOnly(): Integer
    var
        R: Record "NB LR Row";
        N: Integer;
    begin
        repeat
            N += 1;
        until (N >= 5) or (N = R.Qty);
        exit(N);
    end;

    procedure RepeatFindSet(): Text
    var
        R: Record "NB LR Row";
        Seen: Text;
    begin
        if R.FindSet() then
            repeat
                Seen += R."No." + ';';
            until R.Next() = 0;
        exit(Seen);
    end;

    procedure ForBound(): Integer
    var
        R: Record "NB LR Row";
        I: Integer;
        Sum: Integer;
    begin
        for I := 1 to R.Qty do
            Sum += I;
        exit(Sum);
    end;

    procedure ForeachBody(): Text
    var
        R: Record "NB LR Row";
        Codes: List of [Code[20]];
        K: Code[20];
        Seen: Text;
    begin
        Codes.Add('NB-1');
        Codes.Add('NB-MISSING');
        foreach K in Codes do
            if R.Get(K) then
                Seen += R.Name + ';'
            else
                Seen += 'none;';
        exit(Seen);
    end;

    procedure AssertErrorBody(): Text
    var
        R: Record "NB LR Row";
    begin
        asserterror R.Get('NB-MISSING');
        exit(CopyStr(GetLastErrorText(), 1, 40));
    end;

    procedure ExitExpression(): Integer
    var
        R: Record "NB LR Row";
    begin
        exit(R.Count());
    end;

    procedure ByVarArgument(): Text
    var
        R: Record "NB LR Row";
    begin
        Fill(R);
        exit(R.Name + '/' + Format(R.Qty));
    end;

    procedure ByValueArgument(): Text
    var
        R: Record "NB LR Row";
    begin
        exit(Describe(R));
    end;

    procedure CopyBetweenLocals(): Text
    var
        Source: Record "NB LR Row";
        Target: Record "NB LR Row";
    begin
        Source.Get('NB-1');
        Target := Source;
        exit(Target.Name);
    end;

    procedure InitValueAfterInsertsBeforeUse(): Text
    var
        Writer: Record "NB LR Row";
        Late: Record "NB LR Row";
    begin
        Writer.Init();
        Writer."No." := 'NB-LATE';
        Writer.Name := 'written';
        Writer.Qty := 99;
        Writer.Insert();
        exit(Late.Name + '/' + Format(Late.Qty) + '/' + Late."No.");
    end;

    procedure FilterBeforeUse(): Text
    var
        Other: Record "NB LR Row";
        R: Record "NB LR Row";
    begin
        Other.SetRange("No.", 'NB-1');
        exit('[' + R.GetFilters() + '][' + Other.GetFilters() + ']');
    end;

    procedure ErrorBeforeUse(Fail: Boolean): Text
    var
        R: Record "NB LR Row";
    begin
        if Fail then
            Error('NB failed before the record');
        exit(R.Name);
    end;

    procedure RecRefGetTable(): Text
    var
        R: Record "NB LR Row";
        RecRef: RecordRef;
    begin
        RecRef.GetTable(R);
        exit(Format(RecRef.Number) + '/' + Format(RecRef.Field(2).Value));
    end;

    procedure Recursion(Depth: Integer): Text
    var
        R: Record "NB LR Row";
    begin
        if Depth = 0 then
            exit('');
        R.Name := 'depth ' + Format(Depth);
        exit(Recursion(Depth - 1) + R.Name + ';');
    end;

    procedure BlobWriteRead(): Text
    var
        First: Record "NB LR Row";
        Second: Record "NB LR Row";
        Out: OutStream;
        InS: InStream;
        Line: Text;
    begin
        First.Get('NB-1');
        First.Payload.CreateOutStream(Out);
        Out.WriteText('blob line');
        First.Modify();
        Second.Get('NB-1');
        Second.CalcFields(Payload);
        Second.Payload.CreateInStream(InS);
        InS.ReadText(Line);
        exit(Line);
    end;

    procedure TemporaryAround(): Text
    var
        T1: Record "NB LR Row" temporary;
        Stored: Record "NB LR Row";
        T2: Record "NB LR Row" temporary;
    begin
        T1."No." := 'T';
        T1.Insert();
        exit(Format(T1.Count()) + '/' + Format(T2.Count()) + '/' + Format(Stored.IsTemporary()));
    end;

    procedure ArrayOfRecords(): Text
    var
        A: array[2] of Record "NB LR Row";
    begin
        A[2].Get('NB-1');
        exit(A[1].Name + '/' + A[2].Name);
    end;

    procedure ReturnValueRecord() R: Record "NB LR Row"
    begin
        R.Name := 'returned';
    end;

    procedure ClearFirst(): Text
    var
        R: Record "NB LR Row";
    begin
        Clear(R);
        exit(R.Name);
    end;

    procedure InsertLogged(): Integer
    var
        R: Record "NB LR Row";
        Log: Record "NB LR Log";
    begin
        R.Init();
        R."No." := 'NB-LOG';
        R.Name := 'logged';
        R.Insert(true);
        Log.SetRange(Note, 'NB-LOG');
        exit(Log.Count());
    end;

    procedure RaiseEvent(): Text
    var
        Result: Text;
    begin
        OnShape(Result);
        exit(Result);
    end;

    local procedure Fill(var Target: Record "NB LR Row")
    begin
        Target.Get('NB-1');
    end;

    local procedure Describe(Copy: Record "NB LR Row"): Text
    begin
        exit(Copy.Name + '/' + Format(Copy.Qty));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnShape(var Result: Text)
    begin
    end;
}

codeunit 50103 "NB LR Subscriber"
{
    EventSubscriberInstance = StaticAutomatic;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"NB LR Shapes", 'OnShape', '', false, false)]
    local procedure HandleShape(var Result: Text)
    var
        Unused: Record "NB LR Row";
        R: Record "NB LR Row";
    begin
        if R.Get('NB-1') then
            Result := 'subscriber ' + R.Name;
    end;
}

codeunit 50104 "NB LR Run"
{
    trigger OnRun()
    var
        Unused: Record "NB LR Row";
        R: Record "NB LR Row";
    begin
        R.Init();
        R."No." := 'NB-RUN';
        R.Insert();
    end;
}

codeunit 50105 "NB LR Tests"
{
    Subtype = Test;

    var
        Shapes: Codeunit "NB LR Shapes";

    local procedure Seed()
    var
        R: Record "NB LR Row";
        Log: Record "NB LR Log";
    begin
        // The rows of an earlier test of this codeunit are still there.
        R.DeleteAll();
        Log.DeleteAll();
        R.Init();
        R."No." := 'NB-1';
        R.Name := 'one';
        R.Qty := 3;
        R.Insert();
        R.Init();
        R."No." := 'NB-2';
        R.Name := 'two';
        R.Qty := 4;
        R.Insert();
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected "%1", got "%2"', Expected, Actual);
    end;

    [Test]
    procedure Local_NeverNamed()
    begin
        Check(Shapes.NeverNamed(), 'none named');
    end;

    [Test]
    procedure Local_IfHeaderOnly()
    begin
        Seed();
        Check(Format(Shapes.IfHeaderOnly('NB-1')) + Format(Shapes.IfHeaderOnly('NB-MISSING')), 'YesNo');
    end;

    [Test]
    procedure Local_ThenBranchOnly()
    begin
        Check(Shapes.ThenBranchOnly(true) + '|' + Shapes.ThenBranchOnly(false), 'fresh|skipped');
    end;

    [Test]
    procedure Local_ElseBranchOnly()
    begin
        Check(Shapes.ElseBranchOnly(false) + '|' + Shapes.ElseBranchOnly(true), '7|then');
    end;

    [Test]
    procedure Local_BuiltInBranchUsedAfter()
    begin
        Check(Shapes.BuiltInBranchUsedAfter(true) + '|' + Shapes.BuiltInBranchUsedAfter(false), 'set in branch|fresh');
    end;

    [Test]
    procedure Local_CaseScrutinee()
    begin
        Check(Format(Shapes.CaseScrutinee()), '1');
    end;

    [Test]
    procedure Local_CaseBranchAndElse()
    begin
        Check(Shapes.CaseBranchAndElse(1) + '|' + Shapes.CaseBranchAndElse(2), 'fresh|7');
    end;

    [Test]
    procedure Local_WhileCondition()
    begin
        Check(Format(Shapes.WhileCondition()), '7');
    end;

    [Test]
    procedure Local_WhileNestedBody()
    begin
        Check(Shapes.WhileNestedBody(), '1fresh3');
    end;

    [Test]
    procedure Local_RepeatUntilOnly()
    begin
        Check(Format(Shapes.RepeatUntilOnly()), '5');
    end;

    [Test]
    procedure Local_RepeatFindSet()
    begin
        Seed();
        Check(Shapes.RepeatFindSet(), 'NB-1;NB-2;');
    end;

    [Test]
    procedure Local_ForBound()
    begin
        Check(Format(Shapes.ForBound()), '28');
    end;

    [Test]
    procedure Local_ForeachBody()
    begin
        Seed();
        Check(Shapes.ForeachBody(), 'one;none;');
    end;

    [Test]
    procedure Local_AssertErrorBody()
    begin
        Seed();
        Check(Format(StrLen(Shapes.AssertErrorBody()) > 0), 'Yes');
    end;

    [Test]
    procedure Local_ExitExpression()
    begin
        Seed();
        Check(Format(Shapes.ExitExpression()), '2');
    end;

    [Test]
    procedure Local_ByVarArgument()
    begin
        Seed();
        Check(Shapes.ByVarArgument(), 'one/3');
    end;

    [Test]
    procedure Local_ByValueArgument()
    begin
        Check(Shapes.ByValueArgument(), 'fresh/7');
    end;

    [Test]
    procedure Local_CopyBetweenLocals()
    begin
        Seed();
        Check(Shapes.CopyBetweenLocals(), 'one');
    end;

    [Test]
    procedure Local_InitValueAfterInsertsBeforeUse()
    begin
        Check(Shapes.InitValueAfterInsertsBeforeUse(), 'fresh/7/');
    end;

    [Test]
    procedure Local_FilterOnOtherLocalBeforeUse()
    begin
        Check(Shapes.FilterBeforeUse(), '[][No.: NB-1]');
    end;

    [Test]
    procedure Local_ErrorBeforeUse()
    begin
        asserterror Shapes.ErrorBeforeUse(true);
        Check(GetLastErrorText(), 'NB failed before the record');
        Check(Shapes.ErrorBeforeUse(false), 'fresh');
    end;

    [Test]
    procedure Local_RecRefGetTable()
    begin
        Check(Shapes.RecRefGetTable(), '50100/fresh');
    end;

    [Test]
    procedure Local_Recursion()
    begin
        Check(Shapes.Recursion(3), 'depth 1;depth 2;depth 3;');
    end;

    [Test]
    procedure Local_BlobWriteRead()
    begin
        Seed();
        Check(Shapes.BlobWriteRead(), 'blob line');
    end;

    [Test]
    procedure Eager_TemporaryAroundStored()
    begin
        Check(Shapes.TemporaryAround(), '1/0/No');
    end;

    [Test]
    procedure Eager_ArrayOfRecords()
    begin
        Seed();
        Check(Shapes.ArrayOfRecords(), 'fresh/one');
    end;

    [Test]
    procedure Eager_ReturnValueRecord()
    var
        R: Record "NB LR Row";
    begin
        R := Shapes.ReturnValueRecord();
        Check(R.Name, 'returned');
    end;

    [Test]
    procedure Local_ClearFirst()
    begin
        Check(Shapes.ClearFirst(), 'fresh');
    end;

    [Test]
    procedure Trigger_OnInsertLocals()
    begin
        Check(Format(Shapes.InsertLogged()), '1');
    end;

    [Test]
    procedure Subscriber_Locals()
    begin
        Seed();
        Check(Shapes.RaiseEvent(), 'subscriber one');
    end;

    [Test]
    procedure OnRun_Locals()
    var
        R: Record "NB LR Row";
    begin
        Codeunit.Run(Codeunit::"NB LR Run");
        Check(Format(R.Get('NB-RUN')), 'Yes');
    end;

    [Test]
    procedure TestProcedure_LocalNamedLate()
    var
        Early: Record "NB LR Row";
        Late: Record "NB LR Row";
    begin
        Seed();
        Early.Get('NB-2');
        Check(Early.Name + '/' + Late.Name, 'two/fresh');
    end;
}
