// Option members next to al-runner issue 8657 (an option member named unquoted by a keyword does not compile) that the
// AL compiler takes: keywords quoted, words that are no keyword there unquoted, and the OptionMembers property, which
// takes Key, Area and End unquoted.
table 50000 "NB KO Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option)
        {
            OptionMembers = Zero,Key,Area,End,"If",Local;
        }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50001 "NB KO Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        GlobalOpt: Option Zero,"Key",Exit;

    [Test]
    procedure LocalVariable_QuotedKeywordsAndPlainWords()
    var
        Opt: Option Zero,"Key","End",Local,Exit,If;
    begin
        Opt := Opt::"Key";
        Check(Format(Opt), 'Key');
        Opt := Opt::"End";
        Check(Format(Opt), 'End');
        Opt := Opt::Local;
        Check(Format(Opt), 'Local');
        Opt := Opt::"Exit";
        Check(Format(Opt), 'Exit');
        Opt := Opt::"If";
        Check(Format(Opt), 'If');
        Check(Format(Opt + 0), '5');
    end;

    [Test]
    procedure FirstMember_QuotedKeyword()
    var
        Opt: Option "True",Zero;
    begin
        Check(Format(Opt), 'True');
        Opt := Opt::Zero;
        Check(Format(Opt), 'Zero');
    end;

    [Test]
    procedure GlobalVariable_QuotedKeywordAndPlainWord()
    begin
        GlobalOpt := GlobalOpt::"Key";
        Check(Format(GlobalOpt), 'Key');
        GlobalOpt := GlobalOpt::"Exit";
        Check(Format(GlobalOpt), 'Exit');
    end;

    [Test]
    procedure ParameterAndReturnType_QuotedKeywords()
    begin
        Check(Format(Echo(2)), 'End');
        Check(Name(1), 'Procedure');
    end;

    [Test]
    procedure OptionMembers_KeyAreaAndEndUnquoted()
    var
        Row: Record "NB KO Row";
    begin
        Row.Opt := Row.Opt::Key;
        Check(Format(Row.Opt), 'Key');
        Row.Opt := Row.Opt::Area;
        Check(Format(Row.Opt), 'Area');
        Row.Opt := Row.Opt::"End";
        Check(Format(Row.Opt), 'End');
        Row.Opt := Row.Opt::"If";
        Check(Format(Row.Opt), 'If');
        Row.Opt := Row.Opt::Local;
        Check(Format(Row.Opt), 'Local');
    end;

    local procedure Echo(Value: Integer): Option Zero,"Key","End"
    begin
        exit(Value);
    end;

    local procedure Name(Opt: Option Zero,"Procedure"): Text
    begin
        exit(Format(Opt));
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2.', Expected, Actual);
    end;
}
