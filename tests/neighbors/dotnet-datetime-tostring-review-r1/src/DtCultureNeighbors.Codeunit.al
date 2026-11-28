// Review round 1 of al-runner pull request 9427: ToString() of a DotNet System.DateTime after AL changed the .NET
// formatting state, or under a language or format region the probe (corpus pull request 1653, DotNetDateTimeKind.al)
// did not run with. Neighbor_* keep main's text (.NET's, with U+202F before AM or PM) and carry no flag. Served_* end in
// the formatting state the probe measured and get the plain space, flagged.
dotnet
{
    assembly("mscorlib")
    {
        type("System.DateTime"; DtCuDateTime) { }
        type("System.Globalization.CultureInfo"; DtCuCulture) { }
        type("System.Globalization.DateTimeFormatInfo"; DtCuFormat) { }
        type("System.Threading.Thread"; DtCuThread) { }
    }
}

codeunit 50482 "DtCulture Neighbors"
{
    Subtype = Test;

    local procedure Noon(var Dn: DotNet DtCuDateTime)
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
    end;

    local procedure Narrow(): Text
    var
        C: Char;
    begin
        C := 8239;
        exit(Format(C));
    end;

    local procedure Equal(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('Expected <%1>, got <%2>', Expected, Actual);
    end;

    // A new en-US culture object set as the current culture: not the state the probe ran in.
    [Test]
    procedure Neighbor_NewEnUsCurrentCulture()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Saved: DotNet DtCuCulture;
    begin
        Saved := Culture.CurrentCulture;
        Culture.CurrentCulture := Culture.CultureInfo('en-US');
        Noon(Dn);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Dn.ToString());
        Culture.CurrentCulture := Saved;
    end;

    // The current culture's long-time pattern set in place to another one with U+202F before tt (Copilot's case). At noon
    // its text equals the start pattern's; the state does not.
    [Test]
    procedure Neighbor_LongTimePatternInPlace()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Fmt: DotNet DtCuFormat;
        Before: Text;
    begin
        Culture := Culture.CurrentCulture;
        Fmt := Culture.DateTimeFormat;
        Before := Fmt.LongTimePattern;
        Fmt.LongTimePattern := 'H:mm:ss' + Narrow() + 'tt';
        Noon(Dn);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Dn.ToString());
        Fmt.LongTimePattern := Before;
    end;

    // The long-time pattern set to the value it already has (.NET's en-US one, with U+202F): the measured state.
    [Test]
    procedure Served_LongTimePatternSameValue()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Fmt: DotNet DtCuFormat;
    begin
        Culture := Culture.CurrentCulture;
        Fmt := Culture.DateTimeFormat;
        Fmt.LongTimePattern := 'h:mm:ss' + Narrow() + 'tt';
        Noon(Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // The current culture's long-time pattern set back to the value it had: the measured state again.
    [Test]
    procedure Served_LongTimePatternSetBack()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Fmt: DotNet DtCuFormat;
        Before: Text;
    begin
        Culture := Culture.CurrentCulture;
        Fmt := Culture.DateTimeFormat;
        Before := Fmt.LongTimePattern;
        Fmt.LongTimePattern := 'HH:mm:ss';
        Fmt.LongTimePattern := Before;
        Noon(Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // Another culture set and the startup culture object set back: the measured state again.
    [Test]
    procedure Served_CurrentCultureSetBack()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Saved: DotNet DtCuCulture;
    begin
        Saved := Culture.CurrentCulture;
        Culture.CurrentCulture := Culture.CultureInfo('de-DE');
        Culture.CurrentCulture := Saved;
        Noon(Dn);
        Equal('1/15/2023 12:00:00 PM', Dn.ToString());
    end;

    // The thread's culture set through System.Threading.Thread.
    [Test]
    procedure Neighbor_ThreadCurrentCulture()
    var
        Dn: DotNet DtCuDateTime;
        Culture: DotNet DtCuCulture;
        Saved: DotNet DtCuCulture;
        Thread: DotNet DtCuThread;
    begin
        Saved := Culture.CurrentCulture;
        Thread := Thread.CurrentThread;
        Thread.CurrentCulture := Culture.CultureInfo('en-US');
        Noon(Dn);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Dn.ToString());
        Thread.CurrentCulture := Saved;
    end;

    // GlobalLanguage other than en-US (1033): not the language the probe ran with.
    [Test]
    procedure Neighbor_GlobalLanguageGerman()
    var
        Dn: DotNet DtCuDateTime;
        Shown: Text;
    begin
        GlobalLanguage(1031);
        Noon(Dn);
        Shown := Dn.ToString();
        GlobalLanguage(1033);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Shown);
    end;

    // A report trigger after CurrReport.FormatRegion := 'fr-FR'.
    [Test]
    procedure Neighbor_ReportFormatRegion()
    var
        Store: Codeunit "DtCulture Store";
    begin
        Store.SetRegion('fr-FR');
        Report.Run(Report::"DtCulture Region Report", false);
        Store.SetRegion('');
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Store.Shown());
    end;

    // A report trigger with no format region and the session language: a report context the probe did not run in.
    [Test]
    procedure Neighbor_ReportNoRegion()
    var
        Store: Codeunit "DtCulture Store";
    begin
        Store.SetRegion('');
        Report.Run(Report::"DtCulture Region Report", false);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Store.Shown());
    end;

    // A trigger of a report's request page: report code that runs outside the report triggers.
    [Test]
    [HandlerFunctions('CancelRequestPage')]
    procedure Neighbor_ReportRequestPageTrigger()
    var
        Store: Codeunit "DtCulture Store";
    begin
        Store.SetShown('');
        Report.Run(Report::"DtCulture Request Report", true);
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Store.Shown());
    end;

    // A procedure of a report object called on a variable, outside a run: the session's state.
    [Test]
    procedure Served_ReportProcedureOutsideRun()
    var
        Rep: Report "DtCulture Region Report";
    begin
        Equal('1/15/2023 12:00:00 PM', Rep.NoonText());
    end;

    // RunRequestPage: the request page's trigger runs with no report run.
    [Test]
    [HandlerFunctions('CancelRequestPage')]
    procedure Neighbor_RunRequestPage()
    var
        Store: Codeunit "DtCulture Store";
    begin
        Store.SetShown('');
        Report.RunRequestPage(Report::"DtCulture Request Report");
        Equal('1/15/2023 12:00:00' + Narrow() + 'PM', Store.Shown());
    end;

    [RequestPageHandler]
    procedure CancelRequestPage(var Request: TestRequestPage "DtCulture Request Report")
    begin
        Request.Cancel().Invoke();
    end;
}

report 50483 "DtCulture Region Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    procedure NoonText(): Text
    var
        Dn: DotNet DtCuDateTime;
    begin
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
        exit(Dn.ToString());
    end;

    trigger OnPreReport()
    var
        Store: Codeunit "DtCulture Store";
        Dn: DotNet DtCuDateTime;
    begin
        if Store.Region() <> '' then
            CurrReport.FormatRegion := Store.Region();
        Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
        Store.SetShown(Dn.ToString());
    end;
}

report 50485 "DtCulture Request Report"
{
    ProcessingOnly = true;

    requestpage
    {
        trigger OnOpenPage()
        var
            Store: Codeunit "DtCulture Store";
            Dn: DotNet DtCuDateTime;
        begin
            Dn := Dn.DateTime(2023, 1, 15, 12, 0, 0);
            Store.SetShown(Dn.ToString());
        end;
    }
}

codeunit 50484 "DtCulture Store"
{
    SingleInstance = true;

    var
        RegionText: Text;
        ShownText: Text;

    procedure SetRegion(NewRegion: Text)
    begin
        RegionText := NewRegion;
    end;

    procedure Region(): Text
    begin
        exit(RegionText);
    end;

    procedure SetShown(NewShown: Text)
    begin
        ShownText := NewShown;
    end;

    procedure Shown(): Text
    begin
        exit(ShownText);
    end;
}
