// Neighbors of the call path and the holder that open corpus PR 1650 measures for the never-called register tree
// (assumption clear-all-uncalled-register-tree, ClearSharedCodeunitNeverCalledTree.al "CNC Owner" :287-347, "CNC Holder"
// :349-419): the owner's OnRun reaches its ClearAll through two local procedures; a public procedure of a plain holder
// that holds the register and the owner in globals runs the owner by the statement form. One fact flipped per test. The
// four Assumed_ tests flip one of the four facts ClearSharedCodeunitNeverCalledShapes.al measures alone (owner in a local,
// run from a local procedure, internal RunWithCheck, holder with a TableNo); the others refuse as on main. The last test
// has all four facts but enters the holder by a public procedure, not by Run: refused (NeighborsShapes.al has the rest).
codeunit 86976 "N Owner Modes"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";
        Mode: Integer;

    trigger OnRun()
    begin
        if Mode = 1 then
            ClearAll()
        else
            RunWithCheck();
    end;

    local procedure RunWithCheck()
    begin
        if Mode = 2 then
            ClearAll()
        else
            ClearAllVariables();
    end;

    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86977 "N Owner Internal"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";

    trigger OnRun() begin RunWithCheck(); end;
    internal procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86978 "N Holder Modes"
{
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Modes";
        OwnerInternal: Codeunit "N Owner Internal";

    procedure Post(Mode: Integer): Integer
    var
        Row: Record "CKK Row" temporary;
        LocalOwner: Codeunit "N Owner Modes";
    begin
        Row."No." := 1;
        Row.Insert();
        case Mode of
            1, 2:
                begin
                    Owner.SetMode(Mode);
                    Owner.SetReg(Reg);
                    Owner.Run(Row);
                end;
            3:
                begin
                    Owner.SetReg(Reg);
                    if Owner.Run(Row) then;
                end;
            4:
                begin
                    LocalOwner.SetReg(Reg);
                    LocalOwner.Run(Row);
                    Reg.SetCnt(5);
                    exit(LocalOwner.GetCnt());
                end;
            5:
                begin
                    OwnerInternal.SetReg(Reg);
                    OwnerInternal.Run(Row);
                    Reg.SetCnt(5);
                    exit(OwnerInternal.GetCnt());
                end;
        end;
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;

    procedure PostViaLocal(): Integer
    begin
        exit(RunFromLocal());
    end;

    local procedure RunFromLocal(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;
}

codeunit 86979 "N Holder TableNo"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Modes";

    procedure Post(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;
}

codeunit 86985 "N Holder Activity Post"
{
    TableNo = "CKK Row";
    var
        WhseJnlRegisterLine: Codeunit "N Reg Base";

    procedure Post(): Integer
    begin
        exit(PostSourceDocument());
    end;

    local procedure PostSourceDocument(): Integer
    var
        SalesHeader: Record "CKK Row" temporary;
        SalesPost: Codeunit "N Owner Internal";
    begin
        SalesHeader."No." := 1;
        SalesHeader.Insert();
        SalesPost.SetReg(WhseJnlRegisterLine);
        SalesPost.Run(SalesHeader);
        WhseJnlRegisterLine.SetCnt(5);
        exit(SalesPost.GetCnt());
    end;
}

codeunit 86980 "N Path OnRun"
{
    Subtype = Test;

    [Test]
    procedure Refused_ClearAllInOnRun()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.Post(1) <> 5 then
            Error('not shared');
    end;
}

codeunit 86981 "N Path OneLocal"
{
    Subtype = Test;

    [Test]
    procedure Refused_OneLocalProcedure()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.Post(2) <> 5 then
            Error('not shared');
    end;
}

codeunit 86982 "N Path Guarded"
{
    Subtype = Test;

    [Test]
    procedure Refused_GuardedRun()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.Post(3) <> 5 then
            Error('not shared');
    end;
}

codeunit 86983 "N Path OwnerLocal"
{
    Subtype = Test;

    [Test]
    procedure Assumed_OwnerHeldByALocal()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.Post(4) <> 5 then
            Error('not shared');
    end;
}

codeunit 86984 "N Path HolderLocalProc"
{
    Subtype = Test;

    [Test]
    procedure Assumed_RunFromALocalProcedure()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.PostViaLocal() <> 5 then
            Error('not shared');
    end;
}

codeunit 86986 "N Path Internal"
{
    Subtype = Test;

    [Test]
    procedure Assumed_InternalRunWithCheck()
    var
        H: Codeunit "N Holder Modes";
    begin
        if H.Post(5) <> 5 then
            Error('not shared');
    end;
}

codeunit 86987 "N Path HolderTableNo"
{
    Subtype = Test;

    [Test]
    procedure Assumed_HolderWithTableNo()
    var
        H: Codeunit "N Holder TableNo";
    begin
        if H.Post() <> 5 then
            Error('not shared');
    end;
}

codeunit 86988 "N Path Activity Post"
{
    Subtype = Test;

    [Test]
    procedure Refused_ActivityPostRunsSalesPost()
    var
        H: Codeunit "N Holder Activity Post";
    begin
        if H.Post() <> 5 then
            Error('not shared');
    end;
}
