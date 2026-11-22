// Neighbors of al-runner issue 8642: Record.SetFilter(F, '<>%1', V) with a wildcard text V that has a blank at one end or is
// longer than the Code field (corpus PR 1503, assumption filter-placeholder-wildcard-edges), and the callers, record states,
// languages, argument types and texts next to it, which keep main's answer or refuse.
table 60951 "GK Filter Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cd; Code[20]) { }
        field(3; Txt; Text[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 60952 "GK Filter Lib"
{
    procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;

    procedure Fill(var R: Record "GK Filter Row")
    begin
        R.DeleteAll();
        Add(R, 1, 'XYZ-1');
        Add(R, 2, 'XYZ-*');
        Add(R, 3, 'ABC-1');
    end;

    local procedure Add(var R: Record "GK Filter Row"; Id: Integer; V: Text[30])
    begin
        R.Init();
        R.Id := Id;
        R.Cd := V;
        R.Txt := V;
        R.Insert();
    end;

    procedure Ids(var R: Record "GK Filter Row") Result: Text
    begin
        if R.FindSet() then
            repeat
                Result += Format(R.Id) + ' ';
            until R.Next() = 0;
    end;
}

codeunit 60951 "GK Filter Tests"
{
    Subtype = Test;

    [Test]
    procedure Measured_Code_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'XYZ-* ');
        L.Check(R.GetFilter(Cd) + '|' + L.Ids(R), '<>XYZ-*|3 ');
    end;

    [Test]
    procedure Measured_Text_BlankBefore()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Txt, '<>%1', ' XYZ-*');
        L.Check(R.GetFilter(Txt) + '|' + L.Ids(R), '<>'' XYZ-*''|1 2 3 ');
    end;

    [Test]
    procedure Measured_Code_TooLong()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        asserterror R.SetFilter(Cd, '<>%1', 'XYZ-ABCDEFGHIJKLMNOP*');
        L.Check(GetLastErrorCode(), 'StringLengthExceeded');
    end;

    [Test]
    procedure MainKept_EqualForm_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '%1', 'XYZ-* ');
        L.Check(R.GetFilter(Cd) + '|' + L.Ids(R), 'XYZ-*|1 2 ');
    end;

    [Test]
    procedure MainKept_BlankInside()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'XYZ- *');
        L.Check(R.GetFilter(Cd) + '|' + L.Ids(R), '<>XYZ- *|1 2 3 ');
    end;

    [Test]
    procedure MainKept_Text_TooLong()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Txt, '<>%1', 'XYZ-ABCDEFGHIJKLMNOPQRSTUVWXYZ1234*');
        L.Check(R.GetFilter(Txt) + '|' + L.Ids(R), '<>XYZ-ABCDEFGHIJKLMNOPQRSTUVWXYZ1234*|1 2 3 ');
    end;

    [Test]
    procedure MainKept_Written_TrailingBlank()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Txt, '<>XYZ-* ');
        L.Check(R.GetFilter(Txt) + '|' + L.Ids(R), '<>XYZ-*|3 ');
    end;

    [Test]
    procedure MainKept_Reporter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'XYZ-*');
        L.Check(R.GetFilter(Cd) + '|' + L.Ids(R), '<>XYZ-*|3 ');
    end;
}

// One refused shape per codeunit: a refusal stops the rest of its codeunit.
codeunit 60953 "GK R60953"
{
    Subtype = Test;

    [Test]
    procedure Refused_FieldRef_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
        Ref: RecordRef;
        Fld: FieldRef;
    begin
        L.Fill(R);
        Ref.Open(Database::"GK Filter Row");
        Fld := Ref.Field(2);
        Fld.SetFilter('<>%1', 'XYZ-* ');
        L.Check(Fld.GetFilter(), '<>XYZ-*');
    end;
}

codeunit 60954 "GK R60954"
{
    Subtype = Test;

    [Test]
    procedure Refused_Temporary_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row" temporary;
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'XYZ-* ');
        L.Check(R.GetFilter(Cd), '<>XYZ-*');
    end;
}

codeunit 60955 "GK R60955"
{
    Subtype = Test;

    [Test]
    procedure Refused_FilterGroup2_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.FilterGroup(2);
        R.SetFilter(Cd, '<>%1', 'XYZ-* ');
        L.Check(R.GetFilter(Cd), '<>XYZ-*');
    end;
}

codeunit 60956 "GK R60956"
{
    Subtype = Test;

    [Test]
    procedure Refused_German_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        GlobalLanguage(1031);
        R.SetFilter(Cd, '<>%1', 'XYZ-* ');
        L.Check(R.GetFilter(Cd), '<>XYZ-*');
    end;
}

codeunit 60957 "GK R60957"
{
    Subtype = Test;

    [Test]
    procedure Refused_Lowercase_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'xyz-* ');
        L.Check(R.GetFilter(Cd), '<>XYZ-*');
    end;
}

codeunit 60958 "GK R60958"
{
    Subtype = Test;

    [Test]
    procedure Refused_TwoPlaceholders_BlankAfter()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1&<>%2', 'XYZ-* ', 'ABC-*');
        L.Check(R.GetFilter(Cd), '<>XYZ-*&<>ABC-*');
    end;
}

codeunit 60959 "GK R60959"
{
    Subtype = Test;

    [Test]
    procedure Refused_BlankAndTooLong()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        asserterror R.SetFilter(Cd, '<>%1', 'XYZ-ABCDEFGHIJKLMNOP* ');
        L.Check(GetLastErrorCode(), 'StringLengthExceeded');
    end;
}

codeunit 60960 "GK R60960"
{
    Subtype = Test;

    [Test]
    procedure Refused_CodeArgument_TooLong()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
        V: Code[30];
    begin
        V := 'XYZ-ABCDEFGHIJKLMNOP*';
        asserterror R.SetFilter(Cd, '<>%1', V);
        L.Check(GetLastErrorCode(), 'StringLengthExceeded');
    end;
}

codeunit 60961 "GK R60961"
{
    Subtype = Test;

    [Test]
    procedure Refused_QuestionMark_TooLong()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        asserterror R.SetFilter(Cd, '<>%1', 'XYZ-ABCDEFGHIJKLMNOP?');
        L.Check(GetLastErrorCode(), 'StringLengthExceeded');
    end;
}
