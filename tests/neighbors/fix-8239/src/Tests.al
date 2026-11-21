// Neighbors of al-runner issue 8239: what the test session reads in memos the partner login subscribers changed.
table 60910 "PX Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; SaaS; Boolean) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        M: Codeunit "Env Memo";
    begin
        SaaS := M.IsSaaS();
    end;
}

codeunit 60911 "PX Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}

codeunit 60912 "PX Login Tests"
{
    Subtype = Test;

    [Test]
    procedure InsertReadsBooleanMemoOfPlatform()
    var
        Row: Record "PX Row";
    begin
        Row."No." := 'PX1';
        Row.Insert(true);
        Row.Get('PX1');
        if Row.SaaS then
            Error('saas');
    end;

    [Test]
    procedure ReadsBooleanMemoOfPlatform()
    var
        M: Codeunit "Env Memo";
    begin
        if M.IsSaaS() then
            Error('saas');
    end;

    [Test]
    procedure ReadsMemoOfOwnAppAndDependentApp()
    var
        M: Codeunit "PXA Count";
    begin
        if M.Get() <> 2 then
            Error('count %1', M.Get());
    end;

    [Test]
    procedure ReadsMicrosoft3703WithDecimal()
    var
        M: Codeunit "Setting Memo";
    begin
        if M.Get() <> 1.5 then
            Error('rate %1', M.Get());
    end;

    [Test]
    procedure ReadsMicrosoft3704WithDecimal()
    var
        M: Codeunit "Other Setting Memo";
    begin
        if M.Get() <> 1.5 then
            Error('rate %1', M.Get());
    end;

    [Test]
    procedure ReadsPartnerMemoWithDecimal()
    var
        M: Codeunit "PXA Dec";
    begin
        if M.Get() <> 2.5 then
            Error('rate %1', M.Get());
    end;

    [Test]
    procedure ReadsMemoOfTwoOtherApps()
    var
        M: Codeunit "Sys Count";
    begin
        if M.Get() <> 2 then
            Error('count %1', M.Get());
    end;

    [Test]
    procedure ReadsMemoOfTwoSubscribersOfOneApp()
    var
        M: Codeunit "PXB Twice";
    begin
        if M.Get() <> 2 then
            Error('count %1', M.Get());
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}
