// Copy of corpus tests/al-language-isolation-probe/src/SqBindFailed.al (corpus PRs 1465 and 1468), so the neighbor check
// compares main and head on every cell of those probes (the corpus pin does not reach them yet).
namespace ALLanguage.IsolationProbe;

// P5 (AL Runner issue 8337): what a failed test leaves bound of a manual event subscription. The reporter's three codeunits
// (91506 "P161 Events", 91507 "P161 Manual Sub", 91558 "P161 Binding"), renamed, plus a binding held in a local variable
// and an UnbindSubscription of the global in the test after the failed one. Every test also writes its reading to
// "LT SQ Log"; "LT SQ Reader" (76089) states them.

codeunit 76069 "LT SQ Bind Events"
{
    [IntegrationEvent(false, false)]
    procedure OnPing(var Hits: Integer)
    begin
    end;

    procedure Ping(): Integer
    var
        Hits: Integer;
    begin
        OnPing(Hits);
        exit(Hits);
    end;
}

codeunit 76070 "LT SQ Manual Sub"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LT SQ Bind Events", 'OnPing', '', false, false)]
    local procedure HandlePing(var Hits: Integer)
    begin
        Hits += 1;
    end;
}

// The reporter's test codeunit: each test fails on purpose, its error text carries the value.
codeunit 76071 "LT SQ P5 Global Binding"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Sub: Codeunit "LT SQ Manual Sub";
        Log: Codeunit "LT SQ Log";

    [Test]
    procedure BindInFirst()
    var
        Events: Codeunit "LT SQ Bind Events";
        Hits: Integer;
    begin
        BindSubscription(Sub);
        Hits := Events.Ping();
        Log.Put('P5a first hits', Format(Hits));
        Error('MEASURED hits=%1', Hits);
    end;

    [Test]
    procedure StillBoundInSecond()
    var
        Events: Codeunit "LT SQ Bind Events";
        Hits: Integer;
    begin
        Hits := Events.Ping();
        Log.Put('P5a second hits', Format(Hits));
        Error('MEASURED hits=%1', Hits);
    end;

    [Test]
    procedure BindTwice()
    var
        Events: Codeunit "LT SQ Bind Events";
        Sub2: Codeunit "LT SQ Manual Sub";
        Hits: Integer;
    begin
        BindSubscription(Sub2);
        asserterror BindSubscription(Sub2);
        Hits := Events.Ping();
        Log.Put('P5a third hits', Format(Hits));
        Log.Put('P5a third error', GetLastErrorText());
        Error('MEASURED hits=%1 err=%2', Hits, GetLastErrorText());
    end;
}

// A binding held in a local variable of the failed test.
codeunit 76072 "LT SQ P5 Local Binding"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";

    [Test]
    procedure P5b_BindsLocalFails()
    var
        Events: Codeunit "LT SQ Bind Events";
        LocalSub: Codeunit "LT SQ Manual Sub";
    begin
        BindSubscription(LocalSub);
        Log.Put('P5b first hits', Format(Events.Ping()));
        Error('P5b fails on purpose with a local binding');
    end;

    [Test]
    procedure P5b_ReadsHits()
    var
        Events: Codeunit "LT SQ Bind Events";
    begin
        Log.Put('P5b second hits', Format(Events.Ping()));
    end;
}

// A global binding left by a failed test, unbound by the next test.
codeunit 76073 "LT SQ P5 Unbind After Fail"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Sub: Codeunit "LT SQ Manual Sub";
        Log: Codeunit "LT SQ Log";

    [Test]
    procedure P5c_BindsGlobalFails()
    var
        Events: Codeunit "LT SQ Bind Events";
    begin
        BindSubscription(Sub);
        Log.Put('P5c first hits', Format(Events.Ping()));
        Error('P5c fails on purpose with a global binding');
    end;

    [Test]
    procedure P5c_UnbindsGlobal()
    var
        Events: Codeunit "LT SQ Bind Events";
    begin
        Log.Put('P5c unbind', Format(UnbindSubscription(Sub)));
        Log.Put('P5c hits after unbind', Format(Events.Ping()));
    end;
}
