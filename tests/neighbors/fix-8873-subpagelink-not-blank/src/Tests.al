
codeunit 50110 "NB SPNB Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit "NB SPNB Assert";

    local procedure Seed()
    var
        H: Record "Repro Header";
        L: Record "NB SPNB Line";
    begin
        H.DeleteAll();
        L.DeleteAll();
        H."No." := 'H1';
        H.Insert();
        H."No." := 'H2';
        H.Insert();
        AddLine('H1', 1, '', '', 0);
        AddLine('H1', 2, 'A1', 'N1', 1);
        AddLine('H1', 3, 'A2', '', 2);
        AddLine('H1', 4, '', 'N4', 0);
        AddLine('H2', 5, 'A5', 'N5', 3);
        AddLine('H2', 6, '', '', 0);
    end;

    local procedure AddLine(HeaderNo: Code[10]; LineNo: Integer; Arrival: Code[10]; NoteText: Text[30]; Q: Integer)
    var
        L: Record "NB SPNB Line";
    begin
        L."Header No." := HeaderNo;
        L."Line No." := LineNo;
        L."Arrival No." := Arrival;
        L.Note := NoteText;
        L.Qty := Q;
        L.Insert();
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Int Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Wide Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Quoted Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Union Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    local procedure OpenOn(var Card: TestPage "NB SPNB Const Card"; HeaderNo: Code[10])
    var
        H: Record "Repro Header";
    begin
        H.Get(HeaderNo);
        Card.OpenView();
        Card.GoToRecord(H);
    end;

    [Test]
    procedure CodeSpaced_Rows()
    var
        Card: TestPage "NB SPNB Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.CodeSpaced.First() then
            repeat
                Result += Card.CodeSpaced.LineNo.Value + ',';
            until not Card.CodeSpaced.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Assert.AreEqual('<>''''', Card.CodeSpaced.Filter.GetFilter("Arrival No."), 'filter');
        Card.Close();
    end;

    [Test]
    procedure CodeSpaced_OtherHost_Rows()
    var
        Card: TestPage "NB SPNB Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H2');
        if Card.CodeSpaced.First() then
            repeat
                Result += Card.CodeSpaced.LineNo.Value + ',';
            until not Card.CodeSpaced.Next();
        Assert.AreEqual('5,', Result, 'rows');
        Card.Close();
    end;

    [Test]
    procedure CodeTight_Rows()
    var
        Card: TestPage "NB SPNB Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.CodeTight.First() then
            repeat
                Result += Card.CodeTight.LineNo.Value + ',';
            until not Card.CodeTight.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Assert.AreEqual('<>''''', Card.CodeTight.Filter.GetFilter("Arrival No."), 'filter');
        Card.Close();
    end;

    [Test]
    procedure CodeAlone_Rows()
    var
        Card: TestPage "NB SPNB Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.CodeAlone.First() then
            repeat
                Result += Card.CodeAlone.LineNo.Value + ',';
            until not Card.CodeAlone.Next();
        Assert.AreEqual('2,3,5,', Result, 'rows');
        Assert.AreEqual('<>''''', Card.CodeAlone.Filter.GetFilter("Arrival No."), 'filter');
        Card.Close();
    end;

    [Test]
    procedure TextSpaced_Rows()
    var
        Card: TestPage "NB SPNB Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.TextSpaced.First() then
            repeat
                Result += Card.TextSpaced.LineNo.Value + ',';
            until not Card.TextSpaced.Next();
        Assert.AreEqual('2,4,', Result, 'rows');
        Assert.AreEqual('<>''''', Card.TextSpaced.Filter.GetFilter(Note), 'filter');
        Card.Close();
    end;

    [Test]
    procedure RunPageLink_Code_Rows()
    var
        Card: TestPage "NB SPNB Card";
        List: TestPage "NB SPNB List";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        List.Trap();
        Card.RunCode.Invoke();
        if List.First() then
            repeat
                Result += List.LineNo.Value + ',';
            until not List.Next();
        Assert.AreEqual('2,3,5,0,', Result, 'rows');
        Assert.AreEqual('<>''''', List.Filter.GetFilter("Arrival No."), 'filter');
        List.Close();
        Card.Close();
    end;

    [Test]
    procedure RunPageLink_Text_Rows()
    var
        Card: TestPage "NB SPNB Card";
        List: TestPage "NB SPNB List";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        List.Trap();
        Card.RunText.Invoke();
        if List.First() then
            repeat
                Result += List.LineNo.Value + ',';
            until not List.Next();
        Assert.AreEqual('2,4,5,0,', Result, 'rows');
        Assert.AreEqual('<>''''', List.Filter.GetFilter(Note), 'filter');
        List.Close();
        Card.Close();
    end;

    [Test]
    procedure Integer_KeepsMain()
    var
        Card: TestPage "NB SPNB Int Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.IntPart.First() then
            repeat
                Result += Card.IntPart.LineNo.Value + ',';
            until not Card.IntPart.Next();
        Assert.AreEqual('1,2,3,4,', Result, 'rows');
        Card.Close();
    end;

    [Test]
    procedure RunPageLink_Integer_KeepsMain()
    var
        Card: TestPage "NB SPNB Int Card";
        List: TestPage "NB SPNB List";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        List.Trap();
        Card.RunInt.Invoke();
        if List.First() then
            repeat
                Result += List.LineNo.Value + ',';
            until not List.Next();
        Assert.AreEqual('1,2,3,4,5,6,', Result, 'rows');
        Assert.AreEqual('', List.Filter.GetFilter(Qty), 'filter');
        List.Close();
        Card.Close();
    end;

    [Test]
    procedure TwoBlanks_KeepsMain()
    var
        Card: TestPage "NB SPNB Wide Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.WidePart.First() then
            repeat
                Result += Card.WidePart.LineNo.Value + ',';
            until not Card.WidePart.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Card.Close();
    end;

    [Test]
    procedure QuotedString_KeepsMain()
    var
        Card: TestPage "NB SPNB Quoted Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.QuotedPart.First() then
            repeat
                Result += Card.QuotedPart.LineNo.Value + ',';
            until not Card.QuotedPart.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Card.Close();
    end;

    [Test]
    procedure Union_KeepsMain()
    var
        Card: TestPage "NB SPNB Union Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.UnionPart.First() then
            repeat
                Result += Card.UnionPart.LineNo.Value + ',';
            until not Card.UnionPart.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Card.Close();
    end;

    [Test]
    procedure Const_KeepsMain()
    var
        Card: TestPage "NB SPNB Const Card";
        Result: Text;
    begin
        Seed();
        OpenOn(Card, 'H1');
        if Card.ConstPart.First() then
            repeat
                Result += Card.ConstPart.LineNo.Value + ',';
            until not Card.ConstPart.Next();
        Assert.AreEqual('2,3,', Result, 'rows');
        Card.Close();
    end;
}
