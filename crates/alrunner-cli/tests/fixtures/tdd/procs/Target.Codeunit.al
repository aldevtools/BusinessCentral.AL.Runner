// The codeunit under test. It declares only Existing; the tests call procedures it does not have yet.
codeunit 50100 "Tdd Target"
{
    procedure Existing(): Integer
    begin
        exit(1);
    end;
}
