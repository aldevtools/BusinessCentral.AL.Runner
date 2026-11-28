// Neighbors of the call paths that open corpus PR 1650 measures for the never-called register tree (assumption
// clear-all-uncalled-register-tree): ClearSharedCodeunitNeverCalledShapes.al measures each of four facts of
// "Whse.-Activity-Post" running "Sales-Post" alone (holder TableNo, owner run from a local procedure, owner in a local,
// internal RunWithCheck) and all four together (holder entered by Run, three local procedures). Every other combination,
// entry and ClearAll path refuses, as on main. The two Assumed_ tests are cut-down copies of two measured shapes.
codeunit 86972 "N Owner Check Modes"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";
        Mode: Integer;

    trigger OnRun()
    begin
        if Mode = 1 then
            ClearAllPublic()
        else
            RunWithCheck();
    end;

    internal procedure RunWithCheck()
    begin
        if Mode = 2 then
            ClearAll()
        else
            ClearAllVariables();
    end;

    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearAllPublic() begin ClearAllVariables(); end;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

// OnRun -> Locals local procedures -> the owner, a local of the last (3: the measured Activity shape; 4: BC's own chain
// Code -> PostWhseActivityLine -> PostSourceDoc -> PostSourceDocument). ByVar: the owner is a local of PostPassing,
// passed by var to RunPassed, which runs it.
codeunit 86973 "N Holder Activity Modes"
{
    TableNo = "CKK Row";
    var
        WhseJnlRegisterLine: Codeunit "N Reg Base";
        Locals: Integer;
        ByVar: Boolean;
        Got: Integer;

    trigger OnRun()
    begin
        if Locals = 2 then
            PostSourceDoc()
        else
            Code();
    end;

    local procedure Code()
    begin
        if ByVar then
            PostPassing()
        else
            if Locals = 4 then
                PostWhseActivityLine()
            else
                PostSourceDoc();
    end;

    local procedure PostWhseActivityLine() begin PostSourceDoc(); end;
    local procedure PostSourceDoc() begin PostSourceDocument(); end;

    local procedure PostSourceDocument()
    var
        SalesHeader: Record "CKK Row" temporary;
        SalesPost: Codeunit "N Owner Check Modes";
    begin
        SalesHeader."No." := 1;
        SalesHeader.Insert();
        SalesPost.SetReg(WhseJnlRegisterLine);
        SalesPost.Run(SalesHeader);
        WhseJnlRegisterLine.SetCnt(5);
        Got := SalesPost.GetCnt();
    end;

    local procedure PostPassing()
    var
        SalesPost: Codeunit "N Owner Check Modes";
    begin
        RunPassed(SalesPost);
    end;

    local procedure RunPassed(var SalesPost: Codeunit "N Owner Check Modes")
    var
        SalesHeader: Record "CKK Row" temporary;
    begin
        SalesHeader."No." := 1;
        SalesHeader.Insert();
        SalesPost.SetReg(WhseJnlRegisterLine);
        SalesPost.Run(SalesHeader);
        WhseJnlRegisterLine.SetCnt(5);
        Got := SalesPost.GetCnt();
    end;

    procedure Set(L: Integer; V: Boolean) begin Locals := L; ByVar := V; end;
    procedure GetGot(): Integer begin exit(Got); end;
}

// A holder with a TableNo whose public Post runs the owner with the internal RunWithCheck (owner in a global); its OnRun
// calls Post.
codeunit 86974 "N Holder TableNo Internal"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Check Modes";
        PlainOwner: Codeunit "N Owner Base";
        Got: Integer;
        Plain: Boolean;

    trigger OnRun()
    begin
        Got := Post();
    end;

    procedure Post(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        if Plain then begin
            PlainOwner.SetReg(Reg);
            PlainOwner.Run(Row);
            Reg.SetCnt(5);
            exit(PlainOwner.GetCnt());
        end;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;

    procedure SetPlain(P: Boolean) begin Plain := P; end;
    procedure GetGot(): Integer begin exit(Got); end;
}

// A plain holder: public Post -> local PostSourceDocument, whose local owner it runs (local procedure and local owner).
codeunit 86975 "N Holder Local Pair"
{
    var
        Reg: Codeunit "N Reg Base";

    procedure Post(): Integer begin exit(PostSourceDocument()); end;

    local procedure PostSourceDocument(): Integer
    var
        Row: Record "CKK Row" temporary;
        SalesPost: Codeunit "N Owner Base";
    begin
        Row."No." := 1;
        Row.Insert();
        SalesPost.SetReg(Reg);
        SalesPost.Run(Row);
        Reg.SetCnt(5);
        exit(SalesPost.GetCnt());
    end;
}

// A plain holder that runs "N Owner Check Modes" from its public Post, the owner in a global.
codeunit 86989 "N Holder Check Modes"
{
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Check Modes";

    procedure Post(M: Integer): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        Row."No." := 1;
        Row.Insert();
        Owner.SetMode(M);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Reg.SetCnt(5);
        exit(Owner.GetCnt());
    end;
}

// A plain codeunit between the test and the holder ("Whse.-Act.-Post (Yes/No)" holds "Whse.-Activity-Post" in a global).
codeunit 86990 "N Mid"
{
    var
        WhseActivityPost: Codeunit "N Holder Activity Modes";

    procedure Activity(): Integer
    var
        Row: Record "CKK Row" temporary;
    begin
        WhseActivityPost.Set(3, false);
        WhseActivityPost.Run(Row);
        exit(WhseActivityPost.GetGot());
    end;

    procedure Base(): Integer
    var
        H: Codeunit "N Holder Base";
    begin
        H.Post(false, false);
        H.SetCnt(5);
        exit(H.OwnerCnt());
    end;
}

codeunit 86991 "N Shape FourLocals"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(RunActivity(4, false));
    end;

    [Test]
    procedure Refused_ActivityFourLocals()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86992 "N Shape TwoLocals"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(RunActivity(2, false));
    end;

    [Test]
    procedure Refused_ActivityTwoLocals()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86993 "N Shape OwnerByVar"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(RunActivity(3, true));
    end;

    [Test]
    procedure Refused_ActivityOwnerPassedByVar()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86994 "N Shape GuardedHolder"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(3, false);
        if H.Run(Row) then;
        exit(H.GetGot());
    end;

    [Test]
    procedure Refused_ActivityGuardedRun()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86995 "N Shape HolderById"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        Codeunit.Run(Codeunit::"N Holder Activity Modes", Row);
        exit(5);
    end;

    [Test]
    procedure Refused_ActivityByCodeunitId()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86996 "N Shape ActivityFromMid"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(M.Activity());
    end;

    [Test]
    procedure Refused_ActivityRunFromAPlainCodeunit()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86997 "N Shape BaseFromMid"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(M.Base());
    end;

    [Test]
    procedure Refused_BaseRunFromAPlainCodeunit()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86998 "N Shape TableNoInternal"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(T.Post());
    end;

    [Test]
    procedure Refused_TableNoHolderAndInternal()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 86999 "N Shape LocalPair"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(P.Post());
    end;

    [Test]
    procedure Refused_LocalProcedureAndLocalOwner()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 87000 "N Shape InternalClearAll"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(C.Post(2));
    end;

    [Test]
    procedure Refused_InternalWithClearAllInIt()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 87001 "N Shape PublicCheck"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(C.Post(1));
    end;

    [Test]
    procedure Refused_PublicProcedureBeforeClearAll()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 87002 "N Shape TableNoGuarded"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        T.SetPlain(true);
        if T.Run(Row) then;
        exit(T.GetGot());
    end;

    [Test]
    procedure Refused_TableNoHolderGuardedRun()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 87003 "N Shape Activity"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        exit(RunActivity(3, false));
    end;

    [Test]
    procedure Assumed_ActivityShape()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}

codeunit 87004 "N Shape TableNoRun"
{
    Subtype = Test;

    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        Row: Record "CKK Row" temporary;
    begin
        H.Set(Locals, ByVar);
        H.Run(Row);
        exit(H.GetGot());
    end;

    local procedure Cell(): Integer
    var
        H: Codeunit "N Holder Activity Modes";
        M: Codeunit "N Mid";
        T: Codeunit "N Holder TableNo Internal";
        P: Codeunit "N Holder Local Pair";
        C: Codeunit "N Holder Check Modes";
        Row: Record "CKK Row" temporary;
    begin
        T.SetPlain(true);
        T.Run(Row);
        exit(T.GetGot());
    end;

    [Test]
    procedure Assumed_TableNoHolderEnteredByRun()
    begin
        if Cell() <> 5 then
            Error('not shared');
    end;
}
