codeunit 50275 "Debug Outcome Tests"
{
    Subtype = Test;

    [Test]
    procedure LoopsAndCases()
    var
        Numbers: List of [Integer];
        N: Integer;
        Total: Integer;
        Word: Text;
    begin
        for N := 1 to 5 do
            Numbers.Add(N * N);
        foreach N in Numbers do
            Total += N;
        while Total > 50 do
            Total -= 7;
        repeat
            Total += 1;
        until Total mod 4 = 0;
        case Total of
            48:
                Word := 'forty-eight';
            else
                Word := Format(Total);
        end;
        if Word = '' then
            Error('no word');
    end;

    [Test]
    procedure RecursionPasses()
    var
        Lib: Codeunit "Debug Outcome Lib";
    begin
        if Lib.Fib(10) <> 55 then
            Error('Fib(10) is %1', Lib.Fib(10));
    end;

    [Test]
    procedure ErrorFails()
    var
        X: Integer;
    begin
        X := 3;
        Error('boom %1', X);
    end;

    [Test]
    procedure AssertErrorCatches()
    begin
        asserterror Error('expected %1', 'failure');
        if GetLastErrorText() <> 'expected failure' then
            Error('last error was %1', GetLastErrorText());
    end;

    [Test]
    procedure DivisionByZeroFails()
    var
        A: Integer;
        B: Integer;
    begin
        A := 1;
        A := A div B;
    end;

    [Test]
    procedure TryFunctionReturnsFalse()
    var
        Lib: Codeunit "Debug Outcome Lib";
        Result: Integer;
    begin
        if Lib.TryDivide(6, 0, Result) then
            Error('the division did not fail');
        if not Lib.TryDivide(6, 3, Result) then
            Error('the division failed');
        if Result <> 2 then
            Error('6 div 3 is %1', Result);
    end;

    [Test]
    procedure RecordsAndEvents()
    var
        Row: Record "Debug Outcome Row";
        Lib: Codeunit "Debug Outcome Lib";
        I: Integer;
    begin
        for I := 1 to 3 do begin
            Row.Init();
            Row."No." := I;
            Row.Amount := I * 1.5;
            Row.Insert(true);
        end;
        if Lib.SumRows() <> 9 then
            Error('the sum is %1', Lib.SumRows());
        Row.Get(2);
        if Row.Name <> 'Row 2' then
            Error('the name is %1', Row.Name);
        if Lib.CountRaised() < 1 then
            Error('the subscriber did not run');
    end;

    [Test]
    procedure CodeunitRunReturnsFalse()
    var
        Row: Record "Debug Outcome Row";
    begin
        if Codeunit.Run(Codeunit::"Debug Outcome Runner") then
            Error('the run did not fail');
        if Row.Get(99) then
            Error('the row of the failed run is there');
    end;

    [Test]
    [HandlerFunctions('ConfirmYes')]
    procedure ConfirmIsHandled()
    begin
        if not Confirm('Go on?') then
            Error('the handler said no');
    end;

    [Test]
    procedure MessageWithoutHandlerFails()
    begin
        Message('hello');
    end;

    [Test]
    procedure RowsLeftByEarlierTests()
    var
        Row: Record "Debug Outcome Row";
    begin
        if Row.Count() <> 3 then
            Error('%1 rows are left', Row.Count());
    end;

    [Test]
    procedure StartsASession()
    var
        Id: Integer;
    begin
        StartSession(Id, Codeunit::"Debug Outcome Runner");
    end;

    [Test]
    procedure UsesAnUnsupportedCall()
    var
        RecRef: RecordRef;
        X: Integer;
    begin
        X := 1;
        RecRef.Open(2000000132);
    end;

    [ConfirmHandler]
    procedure ConfirmYes(Question: Text[1024]; var Reply: Boolean)
    begin
        Reply := true;
    end;
}
