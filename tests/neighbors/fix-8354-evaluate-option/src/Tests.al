codeunit 71910 "NB8354 Option Tests"
{
    Subtype = Test;

    local procedure Show(Ok: Boolean; Ord: Integer)
    begin
        Error('%1|%2', Ok, Ord);
    end;

    local procedure Tab(): Text
    var
        T: Text;
    begin
        T[1] := 9;
        exit(T);
    end;

    local procedure Opt(FieldNo: Integer; Word: Text)
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
    begin
        R.Reporter := R.Reporter::First;
        case FieldNo of
            2: begin Ok := Evaluate(R.Reporter, Word); Show(Ok, R.Reporter); end;
            3: begin R.FirstEmpty := 2; Ok := Evaluate(R.FirstEmpty, Word); Show(Ok, R.FirstEmpty); end;
            4: begin R.EmptyNamedCap := 2; Ok := Evaluate(R.EmptyNamedCap, Word); Show(Ok, R.EmptyNamedCap); end;
            5: begin R.SpaceEmptyCap := 2; Ok := Evaluate(R.SpaceEmptyCap, Word); Show(Ok, R.SpaceEmptyCap); end;
            6: begin R.NamedEmptyCap := 2; Ok := Evaluate(R.NamedEmptyCap, Word); Show(Ok, R.NamedEmptyCap); end;
            7: begin R.NamedSpaceCapMid := 2; Ok := Evaluate(R.NamedSpaceCapMid, Word); Show(Ok, R.NamedSpaceCapMid); end;
            8: begin R.EmptySpaceCapMid := 2; Ok := Evaluate(R.EmptySpaceCapMid, Word); Show(Ok, R.EmptySpaceCapMid); end;
            9: begin R.Pinned := 2; Ok := Evaluate(R.Pinned, Word); Show(Ok, R.Pinned); end;
            10: begin R.NoBlank := 2; Ok := Evaluate(R.NoBlank, Word); Show(Ok, R.NoBlank); end;
            11: begin R.NoCaption := 2; Ok := Evaluate(R.NoCaption, Word); Show(Ok, R.NoCaption); end;
            12: begin Ok := Evaluate(R.ReporterML, Word); Show(Ok, R.ReporterML); end;
            13: begin Ok := Evaluate(R.Pair, Word); Show(Ok, R.Pair); end;
        end;
    end;

    [Test] procedure Reporter_Empty() begin Opt(2, ''); end;
    [Test] procedure Reporter_Space() begin Opt(2, ' '); end;
    [Test] procedure Reporter_TwoSpaces() begin Opt(2, '  '); end;
    [Test] procedure Reporter_Tab() begin Opt(2, Tab()); end;
    [Test] procedure Reporter_Name() begin Opt(2, 'Skip'); end;
    [Test] procedure Reporter_Caption() begin Opt(2, 'Skip it'); end;

    [Test]
    procedure Reporter_Statement()
    var
        R: Record "NB8354 Row";
    begin
        R.Reporter := R.Reporter::Second;
        Evaluate(R.Reporter, '');
        Error('%1', R.Reporter);
    end;

    [Test]
    procedure Reporter_FieldRef()
    var
        R: Record "NB8354 Row";
        RecRef: RecordRef;
        FldRef: FieldRef;
        Ok: Boolean;
    begin
        RecRef.GetTable(R);
        FldRef := RecRef.Field(2);
        Ok := FldRef.Evaluate('');
        Error('%1|%2', Ok, FldRef.Value);
    end;

    [Test] procedure Reporter_CaptionML() begin Opt(12, ''); end;
    [Test] procedure FirstEmpty_Empty() begin Opt(3, ''); end;
    [Test] procedure EmptyNamedCap_Empty() begin Opt(4, ''); end;
    [Test] procedure SpaceEmptyCap_Empty() begin Opt(5, ''); end;
    [Test] procedure NamedEmptyCap_Empty() begin Opt(6, ''); end;
    [Test] procedure NamedSpaceCapMid_Empty() begin Opt(7, ''); end;
    [Test] procedure EmptySpaceCapMid_Empty() begin Opt(8, ''); end;
    [Test] procedure Pinned_Empty() begin Opt(9, ''); end;
    [Test] procedure Pinned_Tab() begin Opt(9, Tab()); end;
    [Test] procedure NoBlank_Empty() begin Opt(10, ''); end;
    [Test] procedure NoCaption_Empty() begin Opt(11, ''); end;
    [Test] procedure Pair_Two() begin Opt(13, '2'); end;
    [Test] procedure Pair_NinetyNine() begin Opt(13, '99'); end;
    [Test] procedure Pair_MinusOne() begin Opt(13, '-1'); end;
}

codeunit 71911 "NB8354 Enum Tests"
{
    Subtype = Test;

    local procedure Tab(): Text
    var
        T: Text;
    begin
        T[1] := 9;
        exit(T);
    end;

    local procedure Last(Word: Text)
    var
        E: Enum "NB8354 Last";
        Ok: Boolean;
    begin
        E := E::Beta;
        Ok := Evaluate(E, Word);
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test] procedure Last_Empty() begin Last(''); end;
    [Test] procedure Last_Space() begin Last(' '); end;
    [Test] procedure Last_Tab() begin Last(Tab()); end;
    [Test] procedure Last_Name() begin Last('blank'); end;

    [Test]
    procedure Last_Statement()
    var
        E: Enum "NB8354 Last";
    begin
        E := E::Beta;
        Evaluate(E, '');
        Error('%1', E.AsInteger());
    end;

    [Test]
    procedure Last_Language1034()
    begin
        GlobalLanguage(1034);
        Last('');
    end;

    [Test]
    procedure Last_Field()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
    begin
        R.Kind := R.Kind::Beta;
        Ok := Evaluate(R.Kind, '');
        Error('%1|%2', Ok, R.Kind.AsInteger());
    end;

    [Test]
    procedure SpaceMid_Empty()
    var
        E: Enum "NB8354 Space Mid";
        Ok: Boolean;
    begin
        E := E::Beta;
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure TwoBlank_Empty()
    var
        E: Enum "NB8354 Two Blank";
        Ok: Boolean;
    begin
        E := E::Beta;
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure EmptyMid_Empty()
    var
        E: Enum "NB8354 Empty Mid";
        Ok: Boolean;
    begin
        E := E::Beta;
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure FirstSpace_Empty()
    var
        E: Enum "NB8354 First Space";
        Ok: Boolean;
    begin
        E := E::First;
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure FirstSpace_Statement()
    var
        E: Enum "NB8354 First Space";
    begin
        E := E::First;
        Evaluate(E, ' ');
        Error('%1', E.AsInteger());
    end;

    [Test]
    procedure FirstSpace_Language1031()
    var
        E: Enum "NB8354 First Space";
        Ok: Boolean;
    begin
        GlobalLanguage(1031);
        E := E::First;
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure FirstEmpty_Space()
    var
        E: Enum "NB8354 First Empty";
        Ok: Boolean;
    begin
        E := E::First;
        Ok := Evaluate(E, ' ');
        Error('%1|%2', Ok, E.AsInteger());
    end;

    [Test]
    procedure FirstEmpty_Statement()
    var
        E: Enum "NB8354 First Empty";
    begin
        E := E::First;
        Evaluate(E, '');
        Error('%1', E.AsInteger());
    end;
}
