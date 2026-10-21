// Stubs on receivers other than codeunit variables, on bare calls and for missing overloads (ADR-051 decision 2).
table 50170 "Tdd Rec Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }

    procedure UsesRec(): Integer
    begin
        exit(Rec.ViaRec());
    end;
}

page 50171 "Tdd Rec Card"
{
    SourceTable = "Tdd Rec Item";
}

report 50172 "Tdd Rec Report"
{
    ProcessingOnly = true;
}

codeunit 50173 "Tdd Rec Target"
{
    procedure Existing(): Integer
    begin
        exit(1);
    end;
}

codeunit 50180 "Tdd Receiver Tests"
{
    Subtype = Test;

    [Test]
    procedure OnARecord()
    var
        Item: Record "Tdd Rec Item";
    begin
        if Item.OnRecord() <> 0 then
            Error('the stub returned a value');
    end;

    [Test]
    procedure OnRecInTheTable()
    var
        Item: Record "Tdd Rec Item";
    begin
        if Item.UsesRec() <> 0 then
            Error('the stub returned a value');
    end;

    [Test]
    procedure OnAPage()
    var
        Card: Page "Tdd Rec Card";
    begin
        if Card.OnPage() then
            Error('the stub returned true');
    end;

    [Test]
    procedure OnAReport()
    var
        Rep: Report "Tdd Rec Report";
    begin
        Rep.OnReport(5);
    end;

    [Test]
    procedure BareLocalCall()
    begin
        if NotWrittenYet(2) <> '' then
            Error('the stub returned a value');
    end;

    [Test]
    procedure NewOverload()
    var
        Target: Codeunit "Tdd Rec Target";
    begin
        if Target.Existing(5) <> 0 then
            Error('the stub returned a value');
        if Target.Existing() <> 1 then
            Error('the existing overload did not run');
    end;
}
