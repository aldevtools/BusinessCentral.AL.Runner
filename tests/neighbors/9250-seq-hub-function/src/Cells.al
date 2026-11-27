// The cells of corpus PR 1580 (tests/al-language/codeunits/run-onrun/SeqNextHubFunction.al) and their neighbours, under
// BC's TestRunnerHub with RequiredTestIsolation = Function. Test a changes the sequence and passes; test b reads it.
codeunit 52500 "N9 Seq"
{
    procedure Prepare(Name: Text; CompanySpecific: Boolean)
    begin
        if NumberSequence.Exists(Name, CompanySpecific) then
            exit;
        NumberSequence.Insert(Name, 1, 1, CompanySpecific);
        if NumberSequence.Next(Name, CompanySpecific) = 0 then;
        if NumberSequence.Next(Name, CompanySpecific) = 0 then;
    end;

    procedure Fresh(Name: Text; CompanySpecific: Boolean)
    begin
        if NumberSequence.Exists(Name, CompanySpecific) then
            NumberSequence.Delete(Name, CompanySpecific);
        NumberSequence.Insert(Name, 1, 1, CompanySpecific);
    end;

    procedure Rel(Name: Text; CompanySpecific: Boolean; Base: BigInteger): Text
    begin
        if not NumberSequence.Exists(Name, CompanySpecific) then
            exit('missing');
        exit('cur=' + Format(NumberSequence.Current(Name, CompanySpecific) - Base));
    end;

    procedure RelNext(Name: Text; CompanySpecific: Boolean; Base: BigInteger): Text
    begin
        if not NumberSequence.Exists(Name, CompanySpecific) then
            exit('missing');
        exit(Format(NumberSequence.Next(Name, CompanySpecific) - Base));
    end;

    procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;
}

// E1 shape: not asserted by the probe.
codeunit 52501 "N9 E1 Existing Next"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base); Seq.RelNext(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52501');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E2.
codeunit 52502 "N9 E2 Existing CompanySpecific"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=1'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '2');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52502');
    end;

    local procedure CS(): Boolean
    begin
        exit(true);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E3.
codeunit 52503 "N9 E3 Existing Next Commit"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then; Commit();
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=1'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '2');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52503');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E4.
codeunit 52504 "N9 E4 Existing Restart"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        NumberSequence.Restart(Name(), Base + 1000, CS());
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=0'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '1');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52504');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E5.
codeunit 52505 "N9 E5 Existing Delete"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        NumberSequence.Delete(Name(), CS());
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=0'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '1');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52505');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// O1.
codeunit 52506 "N9 O1 OnRun Next"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Fresh(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.RelNext(Name(), CS(), Base), '1');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52506');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// O2.
codeunit 52507 "N9 O2 OnRun CompanySpecific"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Fresh(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.RelNext(Name(), CS(), Base), '1');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52507');
    end;

    local procedure CS(): Boolean
    begin
        exit(true);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E4 with CompanySpecific = true: not measured.
codeunit 52508 "N9 Restart CompanySpecific"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        NumberSequence.Restart(Name(), Base + 1000, CS());
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52508');
    end;

    local procedure CS(): Boolean
    begin
        exit(true);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E4 with a Commit after the Restart: not measured.
codeunit 52509 "N9 Restart Commit"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        NumberSequence.Restart(Name(), Base + 1000, CS()); Commit();
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52509');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E4 whose test a fails: not measured.
codeunit 52510 "N9 Restart Failed"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        NumberSequence.Restart(Name(), Base + 1000, CS()); Error('fails');
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52510');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// Range instead of Next: not measured.
codeunit 52511 "N9 Range"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Range(Name(), 2, CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52511');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// Next and Restart in one test: not measured.
codeunit 52512 "N9 Next Then Restart"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then; NumberSequence.Restart(Name(), Base + 1000, CS());
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52512');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// E2, then a third test after test b's Next: not measured.
codeunit 52514 "N9 Second Test Next"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.RelNext(Name(), CS(), Base), '2');
    end;

    [Test]
    procedure C_ReadsAfterSecondNext()
    begin
        Seq.Rel(Name(), CS(), Base);
    end;

    local procedure Name(): Text
    begin
        exit('N9S52514');
    end;

    local procedure CS(): Boolean
    begin
        exit(true);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// test a only reads: nothing to settle.
codeunit 52515 "N9 Untouched"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        Seq.Rel(Name(), CS(), Base);
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=0'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '1');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52515');
    end;

    local procedure CS(): Boolean
    begin
        exit(false);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}


// E2 in the corpus master shape: a Commit at the end of OnRun's Prepare and at the start of test a (Initialize).
codeunit 52516 "N9 E2 Committed"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        Seq.Prepare(Name(), CS());
        Commit();
    end;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next(Name(), CS()) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel(Name(), CS(), Base), 'cur=1'); Seq.Expect(Seq.RelNext(Name(), CS(), Base), '2');
    end;

    local procedure Name(): Text
    begin
        exit('N9S52516');
    end;

    local procedure CS(): Boolean
    begin
        exit(true);
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        Commit();
        if NumberSequence.Exists(Name(), CS()) then
            Base := NumberSequence.Current(Name(), CS());
        IsInitialized := true;
    end;
}

// The sequence is there before the codeunit begins (the install codeunit 52519 makes it); test a takes Next.
codeunit 52517 "N9 Existing Before Codeunit"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    var
        Seq: Codeunit "N9 Seq";
        IsInitialized: Boolean;
        Base: BigInteger;

    [Test]
    procedure A_ChangesAndPasses()
    begin
        Initialize();
        if NumberSequence.Next('N9S52517', false) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Initialize();
        Seq.Expect(Seq.Rel('N9S52517', false, Base), 'cur=1'); Seq.Expect(Seq.RelNext('N9S52517', false, Base), '2');
    end;

    local procedure Initialize()
    begin
        if IsInitialized then
            exit;
        if NumberSequence.Exists('N9S52517', false) then
            Base := NumberSequence.Current('N9S52517', false);
        IsInitialized := true;
    end;
}

// A sequence inserted in test a (as a No. Series with Allow Gaps set up once in Initialize) and used by test b.
codeunit 52518 "N9 Inserted In Test"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    var
        Seq: Codeunit "N9 Seq";

    [Test]
    procedure A_ChangesAndPasses()
    begin
        NumberSequence.Insert('N9S52518', 1, 1, false);
        if NumberSequence.Next('N9S52518', false) = 0 then;
    end;

    [Test]
    procedure B_Reads()
    begin
        Seq.Expect(Seq.RelNext('N9S52518', false, 0), '2');
    end;
}

codeunit 52519 "N9 Install Sequence"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        if not NumberSequence.Exists('N9S52517', false) then
            NumberSequence.Insert('N9S52517', 1, 1, false);
    end;
}
