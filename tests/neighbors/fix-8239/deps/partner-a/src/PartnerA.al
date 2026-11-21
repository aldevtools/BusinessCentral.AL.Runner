// Partner app A: two memos of its own and the OnAfterLogin subscriber that fills or changes every memo below.
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
    begin
        if Env.IsSaaS() then;
        Count.Bump();
        if Dec.Get() = 0 then;
        Sys.Bump();
        if Setting.Get() = 0 then;
        if Other.Get() = 0 then;
    end;
}
