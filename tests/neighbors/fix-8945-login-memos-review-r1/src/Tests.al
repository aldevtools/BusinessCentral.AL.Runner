// Neighbors of al-runner issue 8945, review round 1 of al-runner PR 9028: where the partner login changed a Microsoft
// cache (a change undone before the insert event, a setup ask nested in the insert raise), and a cache two partner apps
// changed.
codeunit 61050 "LR Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}

codeunit 61051 "LR Login Tests"
{
    Subtype = Test;

    // Loaded by a direct call and reset, then loaded on the insert event: the state the insert event starts from is
    // the one cell M3 starts from.
    [Test]
    procedure ReadsChangeLogLoadedDirectlyThenOnInsert()
    var
        C: Codeunit "Change Log Management";
    begin
        if not C.IsRead() then
            Error('change log not read');
    end;

    // Asked in a setup ask nested in the insert raise: not measured.
    [Test]
    procedure ReadsApiCacheAskedInsideInsert()
    var
        W: Codeunit "API Webhook Notification Mgt.";
    begin
        if not W.Asked() then
            Error('api cache');
    end;

    // Changed by login subscribers of two partner apps: not measured.
    [Test]
    procedure ReadsAreasFilledByTwoPartnerApps()
    var
        A: Codeunit "Application Area Cache";
    begin
        if A.Areas() <> '#Basic' then
            Error('areas');
    end;
}
