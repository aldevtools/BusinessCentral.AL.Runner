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

    procedure Opt(FieldNo: Integer; Word: Text)
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

    procedure Last(Word: Text)
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

codeunit 71912 "NB8354 Option FieldRef"
{
    Subtype = Test;

    [Test]
    procedure Pair_FieldRef()
    var
        R: Record "NB8354 Row";
        RecRef: RecordRef;
        FldRef: FieldRef;
        Ok: Boolean;
    begin
        RecRef.GetTable(R);
        FldRef := RecRef.Field(13);
        Ok := Evaluate(FldRef, '2');
        Error('%1|%2', Ok, FldRef.Value);
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
        Ok := Evaluate(FldRef, '');
        Error('%1|%2', Ok, FldRef.Value);
    end;
}

codeunit 71913 "NB8354 Option CaptionML"
{
    Subtype = Test;

    [Test] procedure Reporter_CaptionML() var H: Codeunit "NB8354 Option Tests"; begin H.Opt(12, ''); end;
}

codeunit 71914 "NB8354 Enum Lang 1034"
{
    Subtype = Test;

    [Test]
    procedure Last_Language1034()
    var
        H: Codeunit "NB8354 Enum Tests";
    begin
        GlobalLanguage(1034);
        H.Last('');
    end;
}

codeunit 71915 "NB8354 Enum Lang 1031"
{
    Subtype = Test;

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
}

codeunit 71916 "NB8354 Enum FieldRef"
{
    Subtype = Test;

    [Test]
    procedure Last_FieldRef()
    var
        R: Record "NB8354 Row";
        RecRef: RecordRef;
        FldRef: FieldRef;
        Ok: Boolean;
    begin
        R.Kind := R.Kind::Beta;
        RecRef.GetTable(R);
        FldRef := RecRef.Field(R.FieldNo(Kind));
        Ok := Evaluate(FldRef, '');
        Error('%1|%2', Ok, Format(FldRef.Value, 0, 9));
    end;
}

codeunit 71917 "NB8354 Enum Global"
{
    Subtype = Test;

    var
        GlobalLast: Enum "NB8354 Last";

    [Test]
    procedure Last_Global()
    var
        Ok: Boolean;
    begin
        GlobalLast := GlobalLast::Beta;
        Ok := Evaluate(GlobalLast, '');
        Error('%1|%2', Ok, GlobalLast.AsInteger());
    end;
}

codeunit 71918 "NB8354 Enum Parameter"
{
    Subtype = Test;

    [Test]
    procedure Last_Parameter()
    var
        E: Enum "NB8354 Last";
    begin
        E := E::Beta;
        EvaluateInto(E);
    end;

    local procedure EvaluateInto(var E: Enum "NB8354 Last")
    var
        Ok: Boolean;
    begin
        Ok := Evaluate(E, '');
        Error('%1|%2', Ok, E.AsInteger());
    end;
}

// Al-runner issue 8355: an Option field whose OptionCaption repeats its member names. The texts, forms and fields of corpus PR
// 1577 OBC_Option_PairOrdinals, and their neighbors.
codeunit 71919 "NB8355 Pair Tests"
{
    Subtype = Test;

    local procedure Pair(Start: Integer; Word: Text)
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        R.Pair := Start;
        Ok := Evaluate(R.Pair, Word);
        Ord := R.Pair;
        Error('%1|%2', Ok, Ord);
    end;

    local procedure PairStatement(Start: Integer; Word: Text)
    var
        R: Record "NB8354 Row";
        Ord: Integer;
    begin
        R.Pair := Start;
        Evaluate(R.Pair, Word);
        Ord := R.Pair;
        Error('no error|%1', Ord);
    end;

    [Test] procedure Pair_PlusTwo() begin Pair(0, '+2'); end;
    [Test] procedure Pair_MaxInteger() begin Pair(0, '2147483647'); end;
    [Test] procedure Pair_PastMaxInteger() begin Pair(0, '2147483648'); end;
    [Test] procedure Pair_MinusOneFromBeta() begin Pair(1, '-1'); end;
    [Test] procedure Pair_StatementNinetyNine() begin PairStatement(0, '99'); end;
    [Test] procedure Pair_StatementMinusOne() begin PairStatement(1, '-1'); end;

    // Neighbors: texts, forms, members and places the probe did not measure.
    [Test] procedure Pair_One() begin Pair(0, '1'); end;
    [Test] procedure Pair_Name() begin Pair(0, 'beta'); end;
    [Test] procedure Pair_Empty() begin Pair(1, ''); end;
    [Test] procedure Pair_Three() begin Pair(0, '3'); end;
    [Test] procedure Pair_SpaceTwo() begin Pair(0, ' 2'); end;
    [Test] procedure Pair_StatementTwo() begin PairStatement(0, '2'); end;

    [Test]
    procedure Pair_FormatNumber9()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        Ok := Evaluate(R.Pair, '2', 9);
        Ord := R.Pair;
        Error('%1|%2', Ok, Ord);
    end;

    [Test]
    procedure Pair_Language1031()
    begin
        GlobalLanguage(1031);
        Pair(0, '2');
    end;

    [Test]
    procedure Trio_Three()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        Ok := Evaluate(R.Trio, '3');
        Ord := R.Trio;
        Error('%1|%2', Ok, Ord);
    end;

    [Test]
    procedure OtherPair_Two()
    var
        R: Record "NB8354 Row";
        Ok: Boolean;
        Ord: Integer;
    begin
        Ok := Evaluate(R.OtherPair, '2');
        Ord := R.OtherPair;
        Error('%1|%2', Ok, Ord);
    end;
}
