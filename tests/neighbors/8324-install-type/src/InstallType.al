// Review of pull request 8607, round one: install code that leaves the type Update. Only a type a test method of the same
// codeunit left carries; the first test method of a codeunit starts at UpdateNoLocks.
codeunit 50050 "NB IT Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;
}

codeunit 50051 "NB IT First Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FirstTest_StartsAtUpdateNoLocks()
    begin
        if Format(CurrentTransactionType()) <> 'UpdateNoLocks' then
            Error('Type at the start: %1', CurrentTransactionType());
    end;
}
