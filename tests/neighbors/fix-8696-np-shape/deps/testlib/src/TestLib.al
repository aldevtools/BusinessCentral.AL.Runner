// A SingleInstance codeunit of a Microsoft-published app the partner app does not depend on. Its own static subscriber
// changes it when the partner's login subscriber raises the hub's event (the shape of a test library's backup memo).
codeunit 60960 "XS Library Memo"
{
    SingleInstance = true;

    var
        Touched: Integer;

    procedure Get(): Integer
    begin
        exit(Touched);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"XS Touch Hub", 'OnTouched', '', false, false)]
    local procedure OnTouched()
    begin
        Touched += 1;
    end;
}
