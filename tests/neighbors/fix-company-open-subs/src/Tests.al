// Neighbors of the login memo change: an OnAfterLogin subscriber of the app under test calls IsSaaS(), IsSandbox() and
// GetEnableMembershipEntitlement() (the shape of corpus PR 1490, P2XaLoginMemos.al), and tests that read the memo
// directly, through a table trigger, after changing nothing, and not at all. The other neighbors (another facade
// procedure, a call from Microsoft's code, a read without the facade, another SingleInstance global of the login code,
// a memo app not published by Microsoft) change the subscriber, which decides the login for every codeunit of the app:
// they are the Rust tests in crates/al-testrun/src/runtime/login_memo_tests.rs.
table 61000 "FC Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; SaaS; Boolean) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        EnvInfo: Codeunit "Environment Information";
    begin
        SaaS := EnvInfo.IsSaaS();
    end;
}

codeunit 61001 "FC Env Seen"
{
    procedure Record()
    var
        EnvInfo: Codeunit "Environment Information";
        ServerSetting: Codeunit "Server Setting";
    begin
        if EnvInfo.IsSaaS() then;
        if EnvInfo.IsSandbox() then;
        if ServerSetting.GetEnableMembershipEntitlement() then;
    end;
}

codeunit 61002 "FC Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        EnvSeen: Codeunit "FC Env Seen";
    begin
        EnvSeen.Record();
    end;
}

codeunit 61010 "FC Login Memo Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsIsSaaS()
    var
        EnvInfo: Codeunit "Environment Information";
    begin
        if EnvInfo.IsSaaS() then
            Error('IsSaaS is true');
    end;

    [Test]
    procedure ReadsIsOnPrem()
    var
        EnvInfo: Codeunit "Environment Information";
    begin
        if not EnvInfo.IsOnPrem() then
            Error('IsOnPrem is false');
    end;

    [Test]
    procedure ReadsEntitlement()
    var
        ServerSetting: Codeunit "Server Setting";
    begin
        if ServerSetting.GetEnableMembershipEntitlement() then
            Error('entitled');
    end;

    [Test]
    procedure InsertReadsIsSaaS()
    var
        Row: Record "FC Row";
    begin
        Row."No." := 1;
        Row.Insert(true);
        Row.Get(1);
        if Row.SaaS then
            Error('row SaaS is true');
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}
