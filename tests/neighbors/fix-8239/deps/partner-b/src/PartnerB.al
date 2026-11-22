// Partner app B (depends on A): one memo of its own, changed by both of its OnAfterLogin subscribers; the first one also
// changes "PXA Count" and "PXA Pair" (of app A) and "Sys Count" (of the platform).
codeunit 60922 "PXB Twice"
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

codeunit 60921 "PXB Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Count: Codeunit "PXA Count";
        Sys: Codeunit "Sys Count";
        Twice: Codeunit "PXB Twice";
        Pair: Codeunit "PXA Pair";
    begin
        Count.Bump();
        Pair.SetB();
        Sys.Bump();
        Twice.Bump();
    end;
}

codeunit 60923 "PXB Login 2"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Twice: Codeunit "PXB Twice";
    begin
        Twice.Bump();
    end;
}
