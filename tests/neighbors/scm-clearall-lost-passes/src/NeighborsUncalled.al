// Neighbors of the never-called register tree of open corpus PR 1650 (assumption clear-all-uncalled-register-tree):
// the base shape ("N Reg Base": a TableNo register with an Integer and a plain codeunit that holds a SingleInstance codeunit
// with a record that is not temporary, no store shape before 1650), then one fact flipped per test. Each test codeunit holds
// one test, so a refusal does not make the next test "not run".
codeunit 86902 "N A" { var U: Codeunit "N U"; }
codeunit 86903 "N U" { SingleInstance = true; var R: Record "CKK Row"; }
codeunit 86906 "N A Text" { var U: Codeunit "N U"; T: Text[10]; }
codeunit 86908 "N U Int" { SingleInstance = true; var R: Record "CKK Row"; I: Integer; }
codeunit 86909 "N A UInt" { var U: Codeunit "N U Int"; }
codeunit 86911 "N C" { var I: Integer; }
codeunit 86912 "N B" { var C: Codeunit "N C"; }
codeunit 86913 "N A Deep" { var U: Codeunit "N U"; B: Codeunit "N B"; }

codeunit 86901 "N Reg Base"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        
        A: Codeunit "N A";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86920 "N Owner Base"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Base";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86921 "N Holder Base"
{
    var
        Reg: Codeunit "N Reg Base";
        Reg2: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86904 "N Reg Dec"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        D: Decimal;
        A: Codeunit "N A";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86922 "N Owner Dec"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Dec";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Dec") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86923 "N Holder Dec"
{
    var
        Reg: Codeunit "N Reg Dec";
        Reg2: Codeunit "N Reg Dec";
        Owner: Codeunit "N Owner Dec";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86905 "N Reg Tmp"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        T: Record "CKK Row" temporary;
        A: Codeunit "N A";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86924 "N Owner Tmp"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Tmp";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Tmp") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86925 "N Holder Tmp"
{
    var
        Reg: Codeunit "N Reg Tmp";
        Reg2: Codeunit "N Reg Tmp";
        Owner: Codeunit "N Owner Tmp";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86907 "N Reg AText"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        
        A: Codeunit "N A Text";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86926 "N Owner AText"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg AText";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg AText") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86927 "N Holder AText"
{
    var
        Reg: Codeunit "N Reg AText";
        Reg2: Codeunit "N Reg AText";
        Owner: Codeunit "N Owner AText";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86910 "N Reg UInt"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        
        A: Codeunit "N A UInt";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86928 "N Owner UInt"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg UInt";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg UInt") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86929 "N Holder UInt"
{
    var
        Reg: Codeunit "N Reg UInt";
        Reg2: Codeunit "N Reg UInt";
        Owner: Codeunit "N Owner UInt";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86914 "N Reg Deep"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        
        A: Codeunit "N A Deep";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86930 "N Owner Deep"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Deep";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Deep") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86931 "N Holder Deep"
{
    var
        Reg: Codeunit "N Reg Deep";
        Reg2: Codeunit "N Reg Deep";
        Owner: Codeunit "N Owner Deep";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86915 "N Reg Normal"
{
    TableNo = "CKK Row";
    Subtype = Normal;
    var
        Cnt: Integer;
        
        A: Codeunit "N A";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86932 "N Owner Normal"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Normal";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Normal") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86933 "N Holder Normal"
{
    var
        Reg: Codeunit "N Reg Normal";
        Reg2: Codeunit "N Reg Normal";
        Owner: Codeunit "N Owner Normal";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86916 "N Reg Si1"
{
    TableNo = "CKK Row";
    
    var
        Cnt: Integer;
        
        A: Codeunit "N U";

    trigger OnRun()
    begin
        Cnt += 1;
    end;

    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}

codeunit 86934 "N Owner Si1"
{
    TableNo = "CKK Row";
    var
        Reg: Codeunit "N Reg Si1";

    trigger OnRun()
    begin
        RunWithCheck();
    end;

    local procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearDirect() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "N Reg Si1") begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86935 "N Holder Si1"
{
    var
        Reg: Codeunit "N Reg Si1";
        Reg2: Codeunit "N Reg Si1";
        Owner: Codeunit "N Owner Si1";

    procedure Post(TwoHolders: Boolean; Direct: Boolean)
    var
        Row: Record "CKK Row" temporary;
    begin
        if TwoHolders then
            Reg2 := Reg;
        Owner.SetReg(Reg);
        if Direct then
            Owner.ClearDirect()
        else begin
            Row."No." := 1;
            Row.Insert();
            Owner.Run(Row);
        end;
    end;

    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86940 "N Uncalled Base"
{
    Subtype = Test;

    [Test]
    procedure Base_NeverCalled_StaysShared()
    var
        H: Codeunit "N Holder Base";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86941 "N Uncalled Direct"
{
    Subtype = Test;

    [Test]
    procedure Refused_ClearAllFromDirectCall()
    var
        H: Codeunit "N Holder Base";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, true);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86942 "N Uncalled Local"
{
    Subtype = Test;

    [Test]
    procedure Refused_HolderIsTestLocal()
    var
        
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        Owner.SetReg(Reg);
        Row."No." := 1;
        Row.Insert();
        Owner.Run(Row);
        Reg.SetCnt(6);
        if Owner.GetCnt() <> 6 then
            Error('not shared: %1', Owner.GetCnt());
    end;
}

codeunit 86943 "N Uncalled Two"
{
    Subtype = Test;

    [Test]
    procedure Refused_TwoOtherHolders()
    var
        H: Codeunit "N Holder Base";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(true, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86944 "N Uncalled Dec"
{
    Subtype = Test;

    [Test]
    procedure Refused_RegisterDecimal()
    var
        H: Codeunit "N Holder Dec";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86945 "N Uncalled Tmp"
{
    Subtype = Test;

    [Test]
    procedure Refused_RegisterTempRecord()
    var
        H: Codeunit "N Holder Tmp";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86946 "N Uncalled AText"
{
    Subtype = Test;

    [Test]
    procedure Refused_PlainNodeText()
    var
        H: Codeunit "N Holder AText";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86947 "N Uncalled UInt"
{
    Subtype = Test;

    [Test]
    procedure Refused_SingleInstanceNodeInteger()
    var
        H: Codeunit "N Holder UInt";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86948 "N Uncalled Deep"
{
    Subtype = Test;

    [Test]
    procedure Refused_PlainNodeThreeDeep()
    var
        H: Codeunit "N Holder Deep";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86949 "N Uncalled Normal"
{
    Subtype = Test;

    [Test]
    procedure Refused_RegisterSubtypeNormal()
    var
        H: Codeunit "N Holder Normal";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86950 "N Uncalled Si1"
{
    Subtype = Test;

    [Test]
    procedure Refused_SingleInstanceOneDeep()
    var
        H: Codeunit "N Holder Si1";
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Base";
        Row: Record "CKK Row" temporary;
    begin
        H.Post(false, false);
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}

codeunit 86951 "N Owner Plain"
{
    var
        Reg: Codeunit "N Reg Base";
        Mode: Integer;

    procedure SetReg(var R: Codeunit "N Reg Base") begin Reg := R; end;
    procedure DoClear() begin ClearAll(); end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}

codeunit 86952 "N Holder Plain"
{
    var
        Reg: Codeunit "N Reg Base";
        Owner: Codeunit "N Owner Plain";

    procedure Post() begin Owner.SetReg(Reg); Owner.DoClear(); end;
    procedure SetCnt(V: Integer) begin Reg.SetCnt(V); end;
    procedure OwnerCnt(): Integer begin exit(Owner.GetCnt()); end;
}

codeunit 86953 "N Uncalled Plain Owner"
{
    Subtype = Test;

    [Test]
    procedure Refused_OwnerWithoutTableNo()
    var
        H: Codeunit "N Holder Plain";
    begin
        H.Post();
        H.SetCnt(5);
        if H.OwnerCnt() <> 5 then
            Error('not shared: %1', H.OwnerCnt());
    end;
}
