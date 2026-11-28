codeunit 64916 "N94 XTime"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 KSi";
        N: Integer;
        Tm: Time;

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        Tm := 120000T;
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64917 "N94 XTime Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XTime";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XTime")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64918 "N94 XTemp"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 KSi";
        N: Integer;
        Tmp: Record "N94 Row" temporary;

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        Tmp.SetRange("No.", V);
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64919 "N94 XTemp Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XTemp";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XTemp")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64920 "N94 XList"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 KSi";
        N: Integer;
        L: List of [Text];

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        L.Add('a');
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64921 "N94 XList Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XList";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XList")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64922 "N94 XSiSi"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 Confirm";
        N: Integer;
        

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64923 "N94 XSiSi Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XSiSi";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XSiSi")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64924 "N94 XText"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 TMid";
        N: Integer;
        

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64925 "N94 XText Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XText";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XText")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64926 "N94 XD3Temp"
{
    TableNo = "N94 Row";

    var
        H: Codeunit "N94 TD1";
        N: Integer;
        

    trigger OnRun()
    begin
    end;

    procedure Prime(V: Integer)
    begin
        N := V;
        
        H.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('x[%1]', N) + H.State());
    end;
}

codeunit 64927 "N94 XD3Temp Owner"
{
    TableNo = "N94 Row";

    var
        Reg: Codeunit "N94 XD3Temp";

    trigger OnRun()
    begin
    end;

    procedure SetReg(var R: Codeunit "N94 XD3Temp")
    begin
        Reg := R;
    end;

    procedure DoClearAll()
    begin
        ClearAll();
    end;
}

codeunit 64928 "N94 TImpl"
{
    SingleInstance = true;

    var
        Handled: Boolean;
        Msg: Text;

    procedure Prime(V: Integer)
    begin
        Handled := true;
        Msg := Format(V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('timpl[%1,%2]', Handled, Msg));
    end;
}

codeunit 64929 "N94 TConfirm"
{
    SingleInstance = true;

    var
        Impl: Codeunit "N94 TImpl";

    procedure Prime(V: Integer)
    begin
        Impl.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(Impl.State());
    end;
}

codeunit 64930 "N94 TMid"
{
    var
        Confirm: Codeunit "N94 TConfirm";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        Confirm.Prime(V + 10);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('tmid[%1]', N) + Confirm.State());
    end;
}

codeunit 64931 "N94 TD3"
{
    var
        Tmp: Record "N94 Row" temporary;
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        Tmp.SetRange("No.", V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('td3[%1,%2]', N, Tmp.GetFilter("No.")));
    end;
}

codeunit 64932 "N94 TD2"
{
    var
        D3: Codeunit "N94 TD3";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        D3.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('td2[%1]', N) + D3.State());
    end;
}

codeunit 64933 "N94 TD1"
{
    var
        D2: Codeunit "N94 TD2";
        N: Integer;

    procedure Prime(V: Integer)
    begin
        N := V;
        D2.Prime(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('td1[%1]', N) + D2.State());
    end;
}
