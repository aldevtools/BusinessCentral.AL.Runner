// Shapes next to al-runner issue 8629 (`local` is a name in code) that main already compiles: the modifier `local` before
// a procedure, right after a var section and after a statement block; a procedure and a table field named Local; and a
// variable or parameter named Local used quoted. None of them may change.
table 50000 "NB LN Row"
{
    fields
    {
        field(1; "No."; Code[20])
        {
        }
        field(2; Local; Integer)
        {
        }
    }
    keys
    {
        key(PK; "No.")
        {
            Clustered = true;
        }
    }
}

codeunit 50001 "NB LN Code"
{
    var
        Counter: Integer;
    local procedure AfterGlobals(): Integer
    begin
        exit(Counter + 1);
    end;

    procedure Calls(): Integer
    var
        Other: Integer;
    begin
        Other := AfterGlobals();
        exit(Other + "Local"() + AfterBlock());
    end;
    local procedure AfterBlock(): Integer
    begin
        exit(10);
    end;

    procedure Local(): Integer
    begin
        exit(100);
    end;

    procedure QuotedUse(): Integer
    var
        Other, Local: Integer;
    begin
        Other := 1;
        "Local" := 2;
        exit(Other + "Local");
    end;

    procedure QuotedParameter(Local: Integer): Integer
    begin
        exit("Local" * 3);
    end;

    procedure FieldNamedLocal(): Integer
    var
        Row: Record "NB LN Row";
    begin
        Row."No." := 'A';
        Row.Local := 5;
        Row.Insert();
        Row.Get('A');
        exit(Row.Local);
    end;
}

codeunit 50002 "NB LN Tests"
{
    Subtype = Test;

    [Test]
    procedure LocalModifierAroundVarSectionsAndBlocks()
    var
        C: Codeunit "NB LN Code";
    begin
        if C.Calls() <> 111 then
            Error('got %1', C.Calls());
    end;

    [Test]
    procedure LocalUsedQuoted()
    var
        C: Codeunit "NB LN Code";
    begin
        if C.QuotedUse() <> 3 then
            Error('got %1', C.QuotedUse());
    end;

    [Test]
    procedure ParameterNamedLocalUsedQuoted()
    var
        C: Codeunit "NB LN Code";
    begin
        if C.QuotedParameter(4) <> 12 then
            Error('got %1', C.QuotedParameter(4));
    end;

    [Test]
    procedure FieldNamedLocal()
    var
        C: Codeunit "NB LN Code";
    begin
        if C.FieldNamedLocal() <> 5 then
            Error('got %1', C.FieldNamedLocal());
    end;
}