// The stand-in login code refuses after it raised the login (deps/platform). An OnAfterLogin subscriber of the app
// under test calls IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement() (the shape of corpus 5da42777,
// tests/al-language-xapp-a/TiXaIsSaaSLogin.al, and corpus PR 1490, P2XaLoginMemos.al). Neighbors: test codeunits under
// RequiredTestIsolation = Disabled that read no memo (a test that reads nothing, one that writes and reads a row of its
// own) and that read the memo, and one under Codeunit isolation that reads the memo. The neighbors that change the
// memos (another member of a served class, a member the runtime does not serve) decide the login for every codeunit of
// the app: they are the Rust tests in crates/al-testrun/src/runtime/disabled_login_memo_tests.rs. tools/neighbor-diff.sh runs
// the app under Microsoft's runner 130450, which takes no account of RequiredTestIsolation (QuCommitIsolation.al:39-40):
// there every codeunit keeps main's result. The Disabled rows differ under `al-runner test --isolation required`.
table 61000 "DRL Row"
{
    fields
    {
        field(1; "No."; Integer) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 61001 "DRL Env Seen"
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

codeunit 61002 "DRL Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        EnvSeen: Codeunit "DRL Env Seen";
    begin
        EnvSeen.Record();
    end;
}

codeunit 61010 "DRL Disabled No Memo Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure ReadsNothing()
    begin
    end;

    [Test]
    procedure OwnRow()
    var
        Row: Record "DRL Row";
    begin
        Row."No." := 1;
        Row.Insert();
        Row.Get(1);
    end;
}

codeunit 61011 "DRL Disabled Memo Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure ReadsIsSaaS()
    var
        EnvInfo: Codeunit "Environment Information";
    begin
        if not EnvInfo.IsSaaS() then
            Error('IsSaaS is false');
    end;
}

codeunit 61012 "DRL Codeunit Memo Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure ReadsIsSaaS()
    var
        EnvInfo: Codeunit "Environment Information";
    begin
        if not EnvInfo.IsSaaS() then
            Error('IsSaaS is false');
    end;
}
