// A small stand-in for the corpus fixture of the same name (tests/al-language/_fixtures/ProbeObserver.Codeunit.al): Verify fails when the
// observed name/value pairs differ from the expected ones. Lets src/RowVersionWrites.al (corpus PR 1509, verbatim) run as a neighbor app.
codeunit 69160 "Probe Observer"
{
    var
        Pairs: Text;
        Expected: Text;

    procedure Observe(ObservedName: Text; ObservedValue: Text)
    begin
        Pairs += ObservedName + '=' + ObservedValue + ';';
    end;

    procedure Expect(ExpectedName: Text; ExpectedValue: Text)
    begin
        Expected += ExpectedName + '=' + ExpectedValue + ';';
    end;

    procedure Verify()
    var
        Got: Text;
        Want: Text;
    begin
        Got := Pairs;
        Want := Expected;
        Pairs := '';
        Expected := '';
        if Got <> Want then
            Error('expected [%1] but was [%2]', Want, Got);
    end;
}
