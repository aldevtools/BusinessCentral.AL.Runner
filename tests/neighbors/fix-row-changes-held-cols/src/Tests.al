// A carry chain (RequiredTestIsolation = Disabled): the writer leaves rows, the setter leaves a SingleInstance flag,
// and each test of the reader reads both, then fails with what it read. Under --isolation required --strict-order
// --test-suites with the suite "S: 50300 50301 50310 50311", the reader runs again in the SingleInstance lane, on a
// worker thread when AL_TEST_THREADS > 1, from a row delta put on top of the worker's baseline (al-runner issue 8590).
codeunit 50300 "Nb Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure WritesRows()
    var
        Ae: Record "Nb Ae Row";
        Accent: Record "Nb Accent Row";
        Plain: Record "Nb Plain Row";
    begin
        Ae.Name := 'Alpha'; Ae.Insert(); Ae.Name := 'ærø'; Ae.Insert(); Ae.Name := 'Bravo'; Ae.Insert();
        Accent.Name := 'Alpha'; Accent.Insert(); Accent.Name := 'émile'; Accent.Insert();
        Accent.Name := 'Bravo'; Accent.Insert();
        Plain.Name := 'Alpha'; Plain.Insert(); Plain.Name := 'Charlie'; Plain.Insert();
        Plain.Name := 'Bravo'; Plain.Insert();
    end;
}

codeunit 50301 "Nb Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure SetsFlag()
    var
        F: Codeunit "Nb Run Flag";
    begin
        F.Set(1);
    end;
}

codeunit 50310 "Nb Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    var
        F: Codeunit "Nb Run Flag";

    // The pinned shape: an order by a Text key that holds æ.
    [Test]
    procedure AeLastInKeyOrder()
    var
        R: Record "Nb Ae Row";
    begin
        if F.Get() = 0 then;
        R.FindLast();
        Error('last=%1 flag=%2', R.Name, F.Get());
    end;

    // The same rows, a read that orders nothing.
    [Test]
    procedure AeCount()
    var
        R: Record "Nb Ae Row";
    begin
        if F.Get() = 0 then;
        Error('count=%1 flag=%2', R.Count(), F.Get());
    end;

    // The same rows, a lookup of one key that holds no æ.
    [Test]
    procedure AeGetPlainKey()
    var
        R: Record "Nb Ae Row";
    begin
        if F.Get() = 0 then;
        Error('found=%1 flag=%2', R.Get('Bravo'), F.Get());
    end;

    // Another accent than æ in the key.
    [Test]
    procedure AccentLastInKeyOrder()
    var
        R: Record "Nb Accent Row";
    begin
        if F.Get() = 0 then;
        R.FindLast();
        Error('last=%1 flag=%2', R.Name, F.Get());
    end;

    // ASCII only in the key.
    [Test]
    procedure PlainLastInKeyOrder()
    var
        R: Record "Nb Plain Row";
    begin
        if F.Get() = 0 then;
        R.FindLast();
        Error('last=%1 flag=%2', R.Name, F.Get());
    end;
}

// A second codeunit that depends on the flag, so the lane has a second serial session and runs its sessions on worker
// threads.
codeunit 50311 "Nb Other"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure FailsOnTheFlag()
    var
        F: Codeunit "Nb Run Flag";
    begin
        Error('flag=%1', F.Get());
    end;
}
