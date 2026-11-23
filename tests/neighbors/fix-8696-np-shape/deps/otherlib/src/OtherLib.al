// The same as "XS Library Memo", in an app of another publisher than Microsoft that the partner app does not depend on.
codeunit 60970 "XS Other Memo"
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
