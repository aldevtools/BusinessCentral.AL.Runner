// The two shapes of al-runner issue 9292, served by main, with the values merged corpus PR 1634 measures
// (ClearSharedCodeunitDeepFields.al, ClearSharedCodeunitRegisterRun.al):
// 1. field values and a filter on a record that is not temporary three levels below a shared register of the shape of
//    corpus PR 1619 (assumption clear-all-whse-register-store): reset in the plain branch, kept below the SingleInstance
//    codeunit (DeepFields_OnlyDepth3Filter_OwnerClearAll :320).
// 2. a register of the tree of corpus PR 1620 (assumption clear-all-register-tree) whose Run returned before the owner's
//    ClearAll: the globals OnRun set go back to their defaults, temporary rows stay (RegRun_SetsGlobals_OwnerClearAll :202).
codeunit 86954 "W D3"
{
    var
        Rows: Record "CKK Row";
        N: Integer;

    procedure SetF(V: Integer) begin Rows."No." := V; Rows.Txt := 't' + Format(V); Rows.SetRange("No.", V); end;
    procedure State(): Text begin exit(StrSubstNo('%1,%2,%3', Rows."No.", Rows.Txt, Rows.GetFilters())); end;
}

codeunit 86955 "W D2P"
{
    var
        D: Codeunit "W D3";

    procedure SetF(V: Integer) begin D.SetF(V); end;
    procedure State(): Text begin exit(D.State()); end;
}

codeunit 86956 "W D1P"
{
    var
        D: Codeunit "W D2P";

    procedure SetF(V: Integer) begin D.SetF(V); end;
    procedure State(): Text begin exit(D.State()); end;
}

codeunit 86957 "W D2S"
{
    SingleInstance = true;

    var
        D: Codeunit "W D3";

    procedure SetF(V: Integer) begin D.SetF(V); end;
    procedure State(): Text begin exit(D.State()); end;
}

codeunit 86958 "W D1S"
{
    var
        D: Codeunit "W D2S";

    procedure SetF(V: Integer) begin D.SetF(V); end;
    procedure State(): Text begin exit(D.State()); end;
}

codeunit 86959 "W Reg"
{
    TableNo = "CKK Row";

    var
        Cnt: Integer;
        RLbl: Label 'reg label', Comment = 'a comment';
        P: Codeunit "W D1P";
        S: Codeunit "W D1S";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetF(V: Integer) begin Cnt := V; P.SetF(V); S.SetF(V); end;
    procedure State(): Text begin exit(StrSubstNo('%1|%2|%3', Cnt, P.State(), S.State())); end;
}

codeunit 86960 "W Owner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "W Reg";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "W Reg") begin Reg := R; end;
}

codeunit 86961 "W Depth3 Fields Test"
{
    Subtype = Test;

    [Test]
    procedure Depth3FieldsAndFilter_OwnerClearAll()
    var
        Reg: Codeunit "W Reg";
        Owner: Codeunit "W Owner";
        Row: Record "CKK Row" temporary;
    begin
        Reg.SetF(5);
        Owner.SetReg(Reg);
        Row."No." := 1;
        Row.Insert();
        Owner.Run(Row);
        if Reg.State() <> '0|0,,|5,t5,No.: 5' then
            Error('state %1', Reg.State());
    end;
}

codeunit 86962 "T Reg"
{
    TableNo = "CKK Row";

    var
        Cnt: Integer;
        T: Record "CKK Row" temporary;

    trigger OnRun()
    begin
        Cnt := Rec."No.";
        T."No." := Rec."No.";
        T.Insert();
    end;

    procedure State(): Text begin exit(StrSubstNo('%1,%2', Cnt, T.Count())); end;
}

codeunit 86963 "T Owner"
{
    var
        Reg: Codeunit "T Reg";

    procedure RunReg()
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 7;
        Row.Insert();
        Reg.Run(Row);
    end;

    procedure DoClear() begin ClearAll(); end;
    procedure State(): Text begin exit(Reg.State()); end;
}

codeunit 86964 "T Run Before Test"
{
    Subtype = Test;

    [Test]
    procedure TreeRegisterRunReturned_OwnerClearAll()
    var
        Owner: Codeunit "T Owner";
    begin
        Owner.RunReg();
        if Owner.State() <> '7,1' then
            Error('before %1', Owner.State());
        Owner.DoClear();
        if Owner.State() <> '0,1' then
            Error('after %1', Owner.State());
    end;
}
