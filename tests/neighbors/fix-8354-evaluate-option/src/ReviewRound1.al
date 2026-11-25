// Review round 1 of the fix (runner PR 9044): the statement error of a field whose member has a name and an empty caption,
// a format number, blank members of two kinds in one field, and Enum fields whose blank member is not the one measured. Each
// test sits in a codeunit of its own: a refused test marks the later tests of its codeunit.
codeunit 71930 "NB8354 R1 Statement"
{
    Subtype = Test;

    [Test]
    procedure NamedEmptyCap_Statement()
    var
        R: Record "NB8354 Row";
    begin
        R.NamedEmptyCap := 2;
        Evaluate(R.NamedEmptyCap, '');
        Error('%1', R.NamedEmptyCap);
    end;
}

codeunit 71931 "NB8354 R1 Option Format"
{
    Subtype = Test;

    [Test]
    procedure Reporter_Format9()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        Ok := Evaluate(R.Reporter, '', 9);
        Ord := R.Reporter;
        Error('%1|%2', Ok, Ord);
    end;
}

codeunit 71932 "NB8354 R1 Enum Format"
{
    Subtype = Test;

    [Test]
    procedure Last_Format9()
    var
        E: Enum "NB8354 Last";
        Ok: Boolean;
    begin
        E := E::Beta;
        Ok := Evaluate(E, '', 9);
        Error('%1|%2', Ok, E.AsInteger());
    end;
}

codeunit 71933 "NB8354 R1 Mix"
{
    Subtype = Test;

    [Test]
    procedure MixFalse_Empty()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        R.MixNoneEmpty := 2;
        Ok := Evaluate(R.MixNoneEmpty, '');
        Ord := R.MixNoneEmpty;
        Error('%1|%2', Ok, Ord);
    end;
}

codeunit 71934 "NB8354 R1 SpaceMid Field"
{
    Subtype = Test;

    [Test]
    procedure SpaceMid_Field()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
    begin
        R.SpaceMid := R.SpaceMid::Beta;
        Ok := Evaluate(R.SpaceMid, '');
        Error('%1|%2', Ok, R.SpaceMid.AsInteger());
    end;
}

codeunit 71935 "NB8354 R1 TwoBlank Field"
{
    Subtype = Test;

    [Test]
    procedure TwoBlank_Field()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
    begin
        R.TwoBlank := R.TwoBlank::Beta;
        Ok := Evaluate(R.TwoBlank, '');
        Error('%1|%2', Ok, R.TwoBlank.AsInteger());
    end;
}
