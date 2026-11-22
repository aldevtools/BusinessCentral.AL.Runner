// Names after `::` next to al-runner issue 8649 (one of 32 keywords unquoted after `::` does not compile) that the AL
// compiler takes: keywords quoted, and Local, Key and Area unquoted.
enum 50000 "NB KS Value"
{
    value(0; "Exit") { }
    value(1; "In") { }
    value(2; Local) { }
    value(3; "Key") { }
}

table 50001 "Exit"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option)
        {
            OptionMembers = Zero,"If",Local,"Area";
        }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50002 "NB KS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure EnumVariable_QuotedKeywordsAndPlainWords()
    var
        Value: Enum "NB KS Value";
    begin
        Value := Value::"Exit";
        Check(Format(Value.AsInteger()), '0');
        Value := Value::"In";
        Check(Format(Value.AsInteger()), '1');
        Value := Value::Local;
        Check(Format(Value.AsInteger()), '2');
        Value := Value::Key;
        Check(Format(Value.AsInteger()), '3');
    end;

    [Test]
    procedure EnumType_QuotedKeyword()
    begin
        Check(Format("NB KS Value"::"Exit".AsInteger()), '0');
        Check(Format("NB KS Value"::Local.AsInteger()), '2');
    end;

    [Test]
    procedure OptionField_QuotedKeywordAndPlainWords()
    var
        Row: Record "Exit";
    begin
        Row.Opt := Row.Opt::"If";
        Check(Format(Row.Opt), 'If');
        Row.Opt := Row.Opt::Local;
        Check(Format(Row.Opt), 'Local');
        Row.Opt := Row.Opt::Area;
        Check(Format(Row.Opt), 'Area');
    end;

    [Test]
    procedure OptionVariable_QuotedKeyword()
    var
        Opt: Option Zero,"Do",Local;
    begin
        Opt := Opt::"Do";
        Check(Format(Opt), 'Do');
        Opt := Opt::Local;
        Check(Format(Opt), 'Local');
    end;

    [Test]
    procedure CaseLabel_QuotedKeyword()
    var
        Value: Enum "NB KS Value";
    begin
        Value := Value::"In";
        case Value of
            Value::"Exit":
                Error('Exit');
            Value::"In":
                Check('In', 'In');
            Value::Local:
                Error('Local');
        end;
    end;

    [Test]
    procedure DatabaseId_QuotedKeywordTable()
    begin
        Check(Format(Database::"Exit"), '50001');
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2.', Expected, Actual);
    end;
}
