codeunit 50272 "Debug Outcome Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Debug Outcome Lib", 'OnAfterSum', '', false, false)]
    local procedure CountSum(Total: Decimal)
    var
        Lib: Codeunit "Debug Outcome Lib";
    begin
        if Total > 0 then
            Lib.NoteRaised();
    end;
}
