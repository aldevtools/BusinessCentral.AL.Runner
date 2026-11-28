// A second partner app (publisher LR Partner Two Co): its login subscriber changes Application Area Cache too.
codeunit 61040 "LRQ Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        A: Codeunit "Application Area Cache";
    begin
        A.FillCompany();
    end;
}
