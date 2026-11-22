// The two cells past 60000 filters of al-runner issue 8637, run here and not in the neighbor app
// tests/neighbors/8637-bg-cache-filtered-reads: on the CI runners main's al-runner (with #8490) runs each past the neighbor
// check's 20 s CPU limit, so the check could not compare that folder. BgQueryCache_Run_AllForms_100000Filters_FirstAnswer is
// open corpus PR 1495, BgTaskQueryCache.al:5564 at 37bffb8a (BQ Tests 9; the cache answers all four reads for 100000 filters
// on table 97596); Filters100001_InsertAfterFirstRead is the neighbor one filter past it (on table 97532 of 503 fields BC
// answers from the cache too; the runner keeps main's answer and reads the rows as they are). Compiled with the neighbor
// app's Probe.al and Neighbors.al; the helpers are those of "BQ Tests 9" and "BQ Gate Tests".
codeunit 97529 "BQ Heavy Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Reader: Integer; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Reader, Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The four reads of a Mode of `Reader` with no row committed; the test inserts the row of table 97596 (`Widest` false) or
    // 97532 without committing; the four reads again: 'first / again'.
    local procedure Reads(Reader: Integer; Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Widest: Record "BQ Widest";
        GateWidest: Record "BQ Gate Widest";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, Reader, Mode, G);
        if Reader = Codeunit::"BQ Reader 9" then begin
            Widest."No." := G;
            Widest.Description := 'a';
            Widest.Insert();
            Widest.SetRange("No.", G);
            if Widest.FindFirst() then;
        end else begin
            GateWidest."No." := G;
            GateWidest.Description := 'a';
            GateWidest.Insert();
            GateWidest.SetRange("No.", G);
            if GateWidest.FindFirst() then;
        end;
        Seen += ' / ' + Run(Rows, Reader, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    // 100000 filters on table 97596.
    [Test]
    procedure BgQueryCache_Run_AllForms_100000Filters_FirstAnswer()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads(Codeunit::"BQ Reader 9", 'widest-100000'), '100000 filters');
    end;

    [Test]
    procedure Filters100001_InsertAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads(Codeunit::"BQ Gate Reader", 'many100001'), 'after the insert');
    end;
}
