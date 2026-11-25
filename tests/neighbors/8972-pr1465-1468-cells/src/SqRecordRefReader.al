// Copy of corpus tests/al-language-isolation-probe/src/SqRecordRefReader.al (corpus PRs 1465 and 1468), so the neighbor check
// compares main and head on every cell of those probes (the corpus pin does not reach them yet).
namespace ALLanguage.IsolationProbe;

// Asserts every reading of the RecordRef cells after a failed test (SqRecordRefFail.al; facts in its header). Runs after
// the cell codeunits 76241-76255 (a higher id). Expected values: measured on BC 27.5, 28.5 and 29.0
// under runner 61301 (TestIsolation = Codeunit) and on 28.5 under 130450, the same on each.
codeunit 76239 "LT SQ Ref Reader"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SQ Log";
        Helper: Codeunit "LT SQ Helper";

    // Runs first: what a later test codeunit sees of every cell's sequence (state, then the number Next gives).
    [Test]
    procedure A_ReadsInLaterCodeunit()
    begin
        Helper.Read('R14O later', Log.Seq('R14O'));
        Helper.Read('R14V later', Log.Seq('R14V'));
        Helper.Read('R14P later', Log.Seq('R14P'));
        Helper.Read('R14W later', Log.Seq('R14W'));
        Helper.Read('R14N later', Log.Seq('R14N'));
        Helper.Read('R14E later', Log.Seq('R14E'));
        Helper.Read('R14M later', Log.Seq('R14M'));
        Helper.Read('R14R later', Log.Seq('R14R'));
        Helper.Read('R2O later', Log.Seq('R2O'));
        Helper.Read('R2V later', Log.Seq('R2V'));
        Helper.Read('R2P later', Log.Seq('R2P'));
        Helper.Read('R2W later', Log.Seq('R2W'));
        Helper.Read('R2N later', Log.Seq('R2N'));
        Helper.Read('R2M later', Log.Seq('R2M'));
        Helper.Read('R2R later', Log.Seq('R2R'));
    end;

    [Test]
    procedure R14O_Reading()
    begin
        Check('R14O', 'R14O first no|R14O access|R14O lines|R14O after state|R14O after no|R14O later state|R14O later next',
            'R14O first no=1 | R14O access=last=10000 | R14O lines=10000 | R14O after state=missing | R14O after no=1 | R14O later state=missing | R14O later next=missing');
    end;

    [Test]
    procedure R14V_Reading()
    begin
        Check('R14V', 'R14V first no|R14V access|R14V lines|R14V after state|R14V after no|R14V later state|R14V later next',
            'R14V first no=1 | R14V access=20000 | R14V lines=10000 | R14V after state=missing | R14V after no=1 | R14V later state=missing | R14V later next=missing');
    end;

    [Test]
    procedure R14P_Reading()
    begin
        Check('R14P', 'R14P first no|R14P access|R14P lines|R14P after state|R14P after no|R14P later state|R14P later next',
            'R14P first no=1 | R14P access=20000 | R14P lines=10000 | R14P after state=missing | R14P after no=1 | R14P later state=missing | R14P later next=missing');
    end;

    [Test]
    procedure R14W_Reading()
    begin
        Check('R14W', 'R14W first no|R14W access|R14W lines|R14W after state|R14W after no|R14W later state|R14W later next',
            'R14W first no=1 | R14W access=inserted=Yes | R14W lines=10000 | R14W after state=missing | R14W after no=1 | R14W later state=missing | R14W later next=missing');
    end;

    [Test]
    procedure R14N_Reading()
    begin
        Check('R14N', 'R14N first no|R14N access|R14N lines|R14N after state|R14N after no|R14N later state|R14N later next',
            'R14N first no=1 | R14N access=opened | R14N lines=10000 | R14N after state=cur=1 | R14N after no=2 | R14N later state=missing | R14N later next=missing');
    end;

    [Test]
    procedure R14E_Reading()
    begin
        Check('R14E', 'R14E first no|R14E access|R14E lines|R14E after state|R14E after no|R14E later state|R14E later next',
            'R14E first no=1 | R14E access=none | R14E lines=10000 | R14E after state=missing | R14E after no=1 | R14E later state=missing | R14E later next=missing');
    end;

    [Test]
    procedure R14M_Reading()
    begin
        Check('R14M', 'R14M first no|R14M access|R14M lines|R14M after state|R14M after no|R14M later state|R14M later next',
            'R14M first no=1 | R14M access=inserted 20000 | R14M lines=10000 | R14M after state=missing | R14M after no=1 | R14M later state=missing | R14M later next=missing');
    end;

    [Test]
    procedure R14R_Reading()
    begin
        Check('R14R', 'R14R first no|R14R access|R14R lines|R14R after state|R14R after no|R14R later state|R14R later next',
            'R14R first no=1 | R14R access=last=10000 | R14R lines=10000 | R14R after state=missing | R14R after no=1 | R14R later state=missing | R14R later next=missing');
    end;

    [Test]
    procedure R2O_Reading()
    begin
        Check('R2O', 'R2O first next|R2O access|R2O lines|R2O after state|R2O after next|R2O later state|R2O later next',
            'R2O first next=1 | R2O access=none | R2O lines=none | R2O after state=missing | R2O after next=missing | R2O later state=missing | R2O later next=missing');
    end;

    [Test]
    procedure R2V_Reading()
    begin
        Check('R2V', 'R2V first next|R2V access|R2V lines|R2V after state|R2V after next|R2V later state|R2V later next',
            'R2V first next=1 | R2V access=10000 | R2V lines=none | R2V after state=missing | R2V after next=missing | R2V later state=missing | R2V later next=missing');
    end;

    [Test]
    procedure R2P_Reading()
    begin
        Check('R2P', 'R2P first next|R2P access|R2P lines|R2P after state|R2P after next|R2P later state|R2P later next',
            'R2P first next=1 | R2P access=10000 | R2P lines=none | R2P after state=missing | R2P after next=missing | R2P later state=missing | R2P later next=missing');
    end;

    [Test]
    procedure R2W_Reading()
    begin
        Check('R2W', 'R2W first next|R2W access|R2W lines|R2W after state|R2W after next|R2W later state|R2W later next',
            'R2W first next=1 | R2W access=inserted=Yes | R2W lines=none | R2W after state=missing | R2W after next=missing | R2W later state=missing | R2W later next=missing');
    end;

    [Test]
    procedure R2N_Reading()
    begin
        Check('R2N', 'R2N first next|R2N access|R2N lines|R2N after state|R2N after next|R2N later state|R2N later next',
            'R2N first next=1 | R2N access=opened | R2N lines=none | R2N after state=cur=1 | R2N after next=2 | R2N later state=missing | R2N later next=missing');
    end;

    [Test]
    procedure R2M_Reading()
    begin
        Check('R2M', 'R2M first next|R2M access|R2M lines|R2M after state|R2M after next|R2M later state|R2M later next',
            'R2M first next=1 | R2M access=inserted 10000 | R2M lines=none | R2M after state=missing | R2M after next=missing | R2M later state=missing | R2M later next=missing');
    end;

    [Test]
    procedure R2R_Reading()
    begin
        Check('R2R', 'R2R first next|R2R access|R2R lines|R2R after state|R2R after next|R2R later state|R2R later next',
            'R2R first next=1 | R2R access=none | R2R lines=none | R2R after state=missing | R2R after next=missing | R2R later state=missing | R2R later next=missing');
    end;

    // "key=value | key=value" of the |-separated keys.
    local procedure Reading(Keys: Text): Text
    var
        EntryKey: Text;
        Result: Text;
    begin
        foreach EntryKey in Keys.Split('|') do begin
            if Result <> '' then
                Result += ' | ';
            Result += EntryKey + '=' + Log.Get(EntryKey);
        end;
        exit(Result);
    end;

    // A failure message carries what was observed instead of the expected reading.
    local procedure Check(Cell: Text; Keys: Text; Expected: Text)
    var
        Actual: Text;
    begin
        Actual := Reading(Keys);
        if Actual <> Expected then
            Error('OBSERVE %1: %2', Cell, Actual);
    end;
}
