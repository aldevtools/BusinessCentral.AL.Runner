// Fix round 1 of PR 8664: shapes next to the measured texts that must refuse or keep main's answer.

codeunit 60964 "GK R60964"
{
    Subtype = Test;

    [Test]
    procedure MainKept_DateTimeLocal_Hour24()
    var
        V: DateTime;
    begin
        Error('result [%1]', Evaluate(V, '01/04/25 24:00'));
    end;
}

codeunit 60965 "GK R60965"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateTimeLocal_Hour23PM()
    var
        V: DateTime;
    begin
        Error('result [%1]', Evaluate(V, '01/04/25 23:00 PM'));
    end;
}

codeunit 60966 "GK R60966"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateTimeLocal_TClock25()
    var
        V: DateTime;
    begin
        Error('result [%1]', Evaluate(V, '01/04/25T25:00'));
    end;
}

codeunit 60967 "GK R60967"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateLocal_MixedDigitParts()
    var
        V: Date;
    begin
        Error('result [%1]', Evaluate(V, '1/04/2025 12:00 PM'));
    end;
}

codeunit 60968 "GK R60968"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateLocal_OneDigit_OtherTime()
    var
        V: Date;
    begin
        Error('result [%1]', Evaluate(V, '1/4/25 11:59 PM'));
    end;
}

codeunit 60969 "GK R60969"
{
    Subtype = Test;

    [Test]
    procedure Refused_DateLocal_Hour11AM()
    var
        V: Date;
    begin
        Error('result [%1]', Evaluate(V, '01/04/25 11:00 AM'));
    end;
}

report 60970 "GK Region Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    trigger OnPreReport()
    var
        V: Date;
    begin
        CurrReport.FormatRegion := 'de-DE';
        Error('result [%1] [%2]', Evaluate(V, '01/04/25 12:00 PM'), Format(V, 0, 9));
    end;
}

codeunit 60971 "GK R60971"
{
    Subtype = Test;

    [Test]
    procedure Refused_Report_FormatRegion_DateTimeDropped()
    begin
        Report.Run(Report::"GK Region Report");
    end;
}

codeunit 60972 "GK R60972"
{
    Subtype = Test;

    [Test]
    procedure Refused_Code_BlankAfter_Pattern20()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Cd, '<>%1', 'XYZ-ABCDEFGHIJKLMNO* ');
        Error('filter [%1]', R.GetFilter(Cd));
    end;
}

codeunit 60973 "GK R60973"
{
    Subtype = Test;

    [Test]
    procedure Refused_Text_RowWithBlank()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.Init();
        R.Id := 4;
        R.Txt := 'XYZ-1 ';
        R.Insert();
        R.SetFilter(Txt, '<>%1', 'XYZ-* ');
        Error('ids [%1]', L.Ids(R));
    end;
}

codeunit 60974 "GK R60974"
{
    Subtype = Test;

    [Test]
    procedure Refused_Text_BlankAfter_GetView()
    var
        L: Codeunit "GK Filter Lib";
        R: Record "GK Filter Row";
    begin
        L.Fill(R);
        R.SetFilter(Txt, '<>%1', 'XYZ-* ');
        Error('view [%1]', R.GetView());
    end;
}
