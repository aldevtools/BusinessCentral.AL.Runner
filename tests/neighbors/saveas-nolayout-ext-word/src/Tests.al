// One test per shape: the probed shapes expect the texts corpus pull request 1579 measured for the same shape
// (reports/layouts-saveas/SaveAsTriggerErrors.al:764-842); the neighbors expect the same rule and keep main's refusal.
codeunit 97160 "NBX Tests"
{
    Subtype = Test;

    local procedure Statement(Rep: Integer; Site: Integer; Fmt: ReportFormat; Expected: Text)
    var
        S: Codeunit "NBX Site";
    begin
        S.Set(Rep, Site, Fmt);
        ClearLastError();
        if Codeunit.Run(Codeunit::"NBX Statement") then
            Error('the statement returned');
        Check('raised ' + GetLastErrorCode() + '/' + GetLastErrorText(), Expected);
    end;

    local procedure BooleanForm(Rep: Integer; Site: Integer; Fmt: ReportFormat; Expected: Text)
    var
        S: Codeunit "NBX Site";
        TempRow: Record "NBX Row" temporary;
        OutS: OutStream;
    begin
        S.Set(Rep, Site, Fmt);
        ClearLastError();
        TempRow.Data.CreateOutStream(OutS);
        if Report.SaveAs(Rep, '', Fmt, OutS) then
            Error('SaveAs answered true');
        Check('false ' + GetLastErrorCode() + '/' + GetLastErrorText(), Expected);
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;

    local procedure NoLayout(Rep: Integer): Text
    begin
        exit(StrSubstNo('ReportNoLayout/Report %1 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.', Format(Rep, 0, 9)));
    end;

    // The probed shapes.

    [Test]
    procedure NoLayoutExtWord_Control_PdfStatement_RaisesNoLayout()
    begin
        Statement(Report::"NBX No Layout Rpt", 0, ReportFormat::Pdf, 'raised ' + NoLayout(Report::"NBX No Layout Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_OnPreReportError_PdfBoolean_FalseWithNoLayout()
    begin
        BooleanForm(Report::"NBX No Layout Rpt", 1, ReportFormat::Pdf, 'false ' + NoLayout(Report::"NBX No Layout Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_OnAfterGetRecordError_WordStatement_RaisesNoLayout()
    begin
        Statement(Report::"NBX No Layout Rpt", 2, ReportFormat::Word, 'raised ' + NoLayout(Report::"NBX No Layout Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_Control_WordBoolean_FalseWithNoLayout()
    begin
        BooleanForm(Report::"NBX No Layout Rpt", 0, ReportFormat::Word, 'false ' + NoLayout(Report::"NBX No Layout Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_OnAfterGetRecordError_XmlBoolean_FalseWithError()
    begin
        BooleanForm(Report::"NBX No Layout Rpt", 2, ReportFormat::Xml, 'false Dialog/X2');
    end;

    // Neighbors: each keeps main's refusal.

    [Test]
    procedure NoLayoutExtRdlc_OnPreReportError_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBX No Layout Rdlc Rpt", 1, ReportFormat::Pdf, 'false ' + NoLayout(Report::"NBX No Layout Rdlc Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWordAndColumn_OnPreReportError_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBX No Layout Col Rpt", 1, ReportFormat::Pdf, 'false ' + NoLayout(Report::"NBX No Layout Col Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWordAndColumn_OnAfterGetRecordError_XmlBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBX No Layout Col Rpt", 2, ReportFormat::Xml, 'false Dialog/X2');
    end;

    [Test]
    procedure NoLayoutExtWord_WithOnInitReport_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBX No Layout Init Rpt", 1, ReportFormat::Pdf, 'false ' + NoLayout(Report::"NBX No Layout Init Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_OnPreReportError_HtmlBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBX No Layout Rpt", 1, ReportFormat::Html, 'false ' + NoLayout(Report::"NBX No Layout Rpt"));
    end;

    [Test]
    procedure NoLayoutExtWord_VariableSaveAs_PdfBoolean_NotMeasured()
    var
        S: Codeunit "NBX Site";
        Rep: Report "NBX No Layout Rpt";
        TempRow: Record "NBX Row" temporary;
        OutS: OutStream;
    begin
        S.Set(Report::"NBX No Layout Rpt", 1, ReportFormat::Pdf);
        ClearLastError();
        TempRow.Data.CreateOutStream(OutS);
        if Rep.SaveAs('', ReportFormat::Pdf, OutS) then
            Error('SaveAs answered true');
        Check(GetLastErrorCode() + '/' + GetLastErrorText(), NoLayout(Report::"NBX No Layout Rpt"));
    end;
}
