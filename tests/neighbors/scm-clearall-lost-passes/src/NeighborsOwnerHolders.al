// Review round 1 of runner PR 9382: the owner of the never-called register tree held by another variable besides the one
// that runs it (assumption clear-all-uncalled-register-tree). Corpus PR 1650 runs the owner only through the one declared
// variable that holds it (ClearSharedCodeunitNeverCalledShapes.al LocalProc :617, LocalOwner :682). Each test adds one
// holder: a var or by-value parameter copy of the holder's global (path 4 base), the test's own variable set into the holder
// (path 1), or a SingleInstance codeunit's global beside the running local (path 5). The last test runs the owner's chain on
// another instance of its codeunit. Each refuses, as on main.
codeunit 86917 "N Holder Shared Owner"
{
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Modes";

    procedure SetOwner(var O: Codeunit "N Owner Modes") begin Owner := O; end;

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

    procedure PostByVar(): Integer
    begin
        Owner.SetReg(Reg);
        RunByVar(Owner);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;

    local procedure RunByVar(var SalesPost: Codeunit "N Owner Modes")
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        SalesPost.Run(Row);
    end;

    procedure PostByValue(): Integer
    begin
        Owner.SetReg(Reg);
        RunByValue(Owner);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;

    local procedure RunByValue(SalesPost: Codeunit "N Owner Modes")
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        SalesPost.Run(Row);
    end;

    procedure PostFn(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Owner.SetReg(Reg);
        GetOwner().Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;

    local procedure GetOwner(): Codeunit "N Owner Modes"
    begin
        exit(Owner);
    end;

    procedure PostLocalKept(): Integer
    var
        Row: Record "CKK Row" temporary;
        LocalOwner: Codeunit "N Owner Modes";
        Keeper: Codeunit "N Owner Keeper";
    begin
        Row."No." := 1;
        Row.Insert();
        LocalOwner.SetReg(Reg);
        Keeper.Keep(LocalOwner);
        LocalOwner.Run(Row);
        Reg.SetCnt(5);
        exit(LocalOwner.GetCnt());
    end;
}

codeunit 86918 "N Owner Keeper"
{
    SingleInstance = true;

    var
        Kept: Codeunit "N Owner Modes";

    procedure Keep(var O: Codeunit "N Owner Modes") begin Kept := O; end;
}

codeunit 86919 "N Owners ByVar"
{
    Subtype = Test;

    [Test]
    procedure Refused_HolderGlobalRunThroughVarParameter()
    var
        H: Codeunit "N Holder Shared Owner";
    begin
        if H.PostByVar() <> 5 then
            Error('not shared');
    end;
}

codeunit 86936 "N Owners ByValue"
{
    Subtype = Test;

    [Test]
    procedure Refused_HolderGlobalRunThroughValueParameter()
    var
        H: Codeunit "N Holder Shared Owner";
    begin
        if H.PostByValue() <> 5 then
            Error('not shared');
    end;
}

codeunit 86937 "N Owners Test"
{
    Subtype = Test;

    [Test]
    procedure Refused_OwnerAlsoHeldByTheTest()
    var
        H: Codeunit "N Holder Shared Owner";
        O: Codeunit "N Owner Modes";
    begin
        H.SetOwner(O);
        if H.Post() <> 5 then
            Error('not shared');
    end;
}

codeunit 87006 "N Owners FnResult"
{
    Subtype = Test;

    // The Run's receiver is the result of a procedure, not a variable: refused, as on main.
    [Test]
    procedure Refused_OwnerRunThroughProcedureResult()
    var
        H: Codeunit "N Holder Shared Owner";
    begin
        if H.PostFn() <> 5 then
            Error('not shared');
    end;
}

codeunit 86938 "N Owners Single"
{
    Subtype = Test;

    [Test]
    procedure Refused_LocalOwnerAlsoHeldBySingleInstanceGlobal()
    var
        H: Codeunit "N Holder Shared Owner";
    begin
        if H.PostLocalKept() <> 5 then
            Error('not shared');
    end;
}

// The owner's OnRun runs RunWithCheck and ClearAllVariables on another instance of its own codeunit, whose register the
// holder shares (path 6 base otherwise). 1650 runs the whole chain on the instance run: refused, as on main.
codeunit 69161 "N Owner Other Instance"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";
        Other: Codeunit "N Owner Other Instance";

    trigger OnRun() begin Other.RunWithCheck(); end;
    internal procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetOtherReg(var R: Codeunit "N Reg Base") begin Other.SetReg(R); end;
    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure GetOtherCnt(): Integer begin exit(Other.GetCnt()); end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86939 "N Holder Other Instance"
{
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Other Instance";

    procedure Post(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Owner.SetOtherReg(Reg);
        Owner.Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetOtherCnt());
    end;
}

codeunit 87005 "N Owners Other Instance"
{
    Subtype = Test;

    [Test]
    procedure Refused_ChainOnAnotherOwnerInstance()
    var
        H: Codeunit "N Holder Other Instance";
    begin
        if H.Post() <> 5 then
            Error('not shared');
    end;
}
