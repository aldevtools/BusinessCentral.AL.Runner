// Partner app A: four memos of its own and the OnAfterLogin subscriber that fills or changes every memo below.
codeunit 60902 "PXA Count"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure Bump()
    begin
        N += 1;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

codeunit 60903 "PXA Dec"
{
    SingleInstance = true;

    var
        Known: Boolean;
        Rate: Decimal;

    procedure Get(): Decimal
    begin
        if not Known then begin
            Rate := 2.5;
            Known := true;
        end;
        exit(Rate);
    end;
}

// Two Booleans, filled on first use (as Environment Information Impl. keeps IsSaaS).
codeunit 60904 "PXA Env Memo"
{
    SingleInstance = true;

    var
        Known: Boolean;
        SaaS: Boolean;

    procedure IsSaaS(): Boolean
    begin
        if not Known then begin
            SaaS := false;
            Known := true;
        end;
        exit(SaaS);
    end;
}

// Two Booleans: app A's subscriber sets FromA, app B's sets FromB.
codeunit 60905 "PXA Pair"
{
    SingleInstance = true;

    var
        FromA: Boolean;
        FromB: Boolean;

    procedure SetA()
    begin
        FromA := true;
    end;

    procedure SetB()
    begin
        FromB := true;
    end;

    procedure Both(): Boolean
    begin
        exit(FromA and FromB);
    end;
}

codeunit 60901 "PXA Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Env: Codeunit "Env Memo";
        Count: Codeunit "PXA Count";
        Dec: Codeunit "PXA Dec";
        Sys: Codeunit "Sys Count";
        Setting: Codeunit "Setting Memo";
        Other: Codeunit "Other Setting Memo";
        PartnerEnv: Codeunit "PXA Env Memo";
        Pair: Codeunit "PXA Pair";
    begin
        if Env.IsSaaS() then;
        if PartnerEnv.IsSaaS() then;
        Pair.SetA();
        Count.Bump();
        if Dec.Get() = 0 then;
        Sys.Bump();
        if Setting.Get() = 0 then;
        if Other.Get() = 0 then;
    end;
}
