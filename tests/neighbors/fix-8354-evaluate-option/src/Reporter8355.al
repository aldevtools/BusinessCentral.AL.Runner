// The reporter's repro of al-runner issue 8355, unchanged.
table 71810 "Eval Opt Two"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Pair; Option)
        {
            OptionMembers = Alpha,Beta;
            OptionCaption = 'Alpha,Beta';
        }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 71861 "Eval Opt Two Test"
{
    Subtype = Test;

    local procedure Try(T: Text)
    var
        R: Record "Eval Opt Two";
        Ok: Boolean;
    begin
        Ok := Evaluate(R.Pair, T);
        Error('%1 %2|%3', Ok, R.Pair, Format(R.Pair, 0, 2));
    end;

    [Test]
    procedure Two()
    begin
        Try('2');
    end;

    [Test]
    procedure NinetyNine()
    begin
        Try('99');
    end;

    [Test]
    procedure MinusOne()
    begin
        Try('-1');
    end;
}
