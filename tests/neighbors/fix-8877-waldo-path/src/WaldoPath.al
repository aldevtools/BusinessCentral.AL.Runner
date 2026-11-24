// al-runner issue 8877, the reporter's own path (his comment on the issue): an install codeunit imports two datasets with
// "AIT AL Test Suite Mgt".ImportTestInputs, then one AI test suite with ImportAITestSuite from a resource read by
// NavApp.GetResource without an encoding. The suite file is UTF-16 LE with a BOM: one AITSuite element with the attributes
// Code, Description, Dataset (the first line's), Capability, Frequency, TestRunnerId and TestType, and ten Line elements with
// CodeunitID, Description and Dataset. No Evaluator and no Language element. A fresh install: the first import of the suite.
// The install checks the rows it stored and raises when one differs, so a passing install baseline (row <baseline>) is the
// result. A test that reads the rows the install stored would pass clean while the install ran through an assumption
// (al-runner issue 8636), so the one test of the app imports a copy of the file with another suite code.
enumextension 50200 "WP Capability" extends "Copilot Capability"
{
    value(50200; "WP Capability") { Caption = 'WP Capability'; }
}

codeunit 50200 "WP Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        AITALTestSuiteMgt: Codeunit "AIT AL Test Suite Mgt";
        WPCheck: Codeunit "WP Check";
        InS: InStream;
        Name: Text;
    begin
        foreach Name in NavApp.ListResources('WP-DS-*.jsonl') do begin
            NavApp.GetResource(Name, InS);
            AITALTestSuiteMgt.ImportTestInputs(Name, InS);
        end;
        NavApp.GetResource('WP-SUITE.xml', InS);
        AITALTestSuiteMgt.ImportAITestSuite(InS);
        WPCheck.Datasets();
        WPCheck.Suite('WP-SUITE', '130451');
    end;
}

codeunit 50204 "WP Check"
{
    local procedure AreEqual(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected <%1>, got <%2>', Expected, Actual);
    end;

    // The datasets imported before the suite: both groups, two inputs each.
    procedure Datasets()
    var
        Grp: Record "Test Input Group";
        Input: Record "Test Input";
    begin
        Grp.Get('WP-DS-ONE');
        Grp.Get('WP-DS-TWO');
        Input.SetRange("Test Input Group Code", 'WP-DS-ONE');
        AreEqual('2', Format(Input.Count()));
        Input.SetRange("Test Input Group Code", 'WP-DS-TWO');
        AreEqual('2', Format(Input.Count()));
    end;

    // The suite row and its ten lines, numbered 10000 apart, with their codeunit, description and dataset.
    procedure Suite(SuiteCode: Code[100]; Runner: Text)
    var
        AITSuite: Record "AIT Test Suite";
        Line: Record "AIT Test Method Line";
        Txt: Text;
    begin
        AITSuite.Get(SuiteCode);
        AreEqual('Waldo path suite/WP-DS-ONE/WP Capability/Manual/' + Runner + '/Copilot',
            AITSuite.Description + '/' + AITSuite."Input Dataset" + '/' + Format(AITSuite."Copilot Capability") + '/' +
            Format(AITSuite."Run Frequency") + '/' + Format(AITSuite."Test Runner Id") + '/' + Format(AITSuite."Test Type"));
        Line.SetRange("Test Suite Code", SuiteCode);
        if Line.FindSet() then
            repeat
                Txt += Format(Line."Line No.") + '/' + Format(Line."Codeunit ID") + '/' + Line.Description + '/' + Line."Input Dataset" + ';';
            until Line.Next() = 0;
        AreEqual(
            '10000/50201/Line 1/WP-DS-ONE;20000/50202/Line 2/WP-DS-ONE;30000/50201/Line 3/WP-DS-ONE;40000/50202/Line 4/WP-DS-ONE;' +
            '50000/50201/Line 5/WP-DS-ONE;60000/50202/Line 6/WP-DS-TWO;70000/50201/Line 7/WP-DS-TWO;80000/50202/Line 8/WP-DS-TWO;' +
            '90000/50201/Line 9/WP-DS-TWO;100000/50202/Line 10/WP-DS-TWO;', Txt);
    end;
}

// The eval codeunits the lines name: test codeunits (the line's codeunit must be one) without tests of their own.
codeunit 50201 "WP Eval One"
{
    Subtype = Test;
}

codeunit 50202 "WP Eval Two"
{
    Subtype = Test;
}

codeunit 50203 "WP Install Tests"
{
    Subtype = Test;

    // The same import from a test, of a copy of the file with another suite code: a first import again. The copy has no
    // TestRunnerId attribute: its OnValidate reads the Test Runner's table "AL Test Suite", which a test may not (ADR-066
    // decision 4). The suite's OnInsert then sets the default runner, "Test Runner - Isol. Codeunit" (130450).
    [Test]
    procedure Suite_ImportOtherCode()
    var
        AITALTestSuiteMgt: Codeunit "AIT AL Test Suite Mgt";
        WPCheck: Codeunit "WP Check";
        InS: InStream;
    begin
        NavApp.GetResource('WP-SUITE-T.xml', InS);
        AITALTestSuiteMgt.ImportAITestSuite(InS);
        WPCheck.Suite('WP-SUITE-T', '130450');
    end;
}
