// One test per shape. The texts the probed shapes expect are the ones corpus pull request 1579
// (reports/layouts-saveas/SaveAsTriggerErrors.al) measured for the same shape; the neighbors expect the same rule.
codeunit 97120 "NBS Tests"
{
    Subtype = Test;

    local procedure Statement(Rep: Integer; Site: Integer; Fmt: ReportFormat; Expected: Text)
    var
        S: Codeunit "NBS Site";
    begin
        S.Set(Rep, Site, Fmt);
        ClearLastError();
        if Codeunit.Run(Codeunit::"NBS Statement") then
            Error('the statement returned');
        Check('raised ' + GetLastErrorCode() + '/' + GetLastErrorText(), Expected);
    end;

    local procedure BooleanForm(Rep: Integer; Site: Integer; Fmt: ReportFormat; Expected: Text)
    var
        S: Codeunit "NBS Site";
        TempRow: Record "NBS Row" temporary;
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

    // The probed shapes (assumption saveas-trigger-error-reaches-caller).

    [Test]
    procedure Word_OnPreReportError_PdfStatement_Raises()
    begin
        Statement(Report::"NBS Word Rpt", 1, ReportFormat::Pdf, 'raised Dialog/N1');
    end;

    [Test]
    procedure Word_OnAfterGetRecordError_WordBoolean_FalseWithError()
    begin
        BooleanForm(Report::"NBS Word Rpt", 2, ReportFormat::Word, 'false Dialog/N2');
    end;

    [Test]
    procedure Word_TestFieldOnPreDataItem_XmlBoolean_FalseWithError()
    begin
        BooleanForm(Report::"NBS Word Rpt", 3, ReportFormat::Xml, 'false TestField/Tag must have a value in NBS Row: Code=. It cannot be zero or empty.');
    end;

    [Test]
    procedure Word_OnPostReportError_XmlStatement_Raises()
    begin
        Statement(Report::"NBS Word Rpt", 4, ReportFormat::Xml, 'raised Dialog/N4');
    end;

    [Test]
    procedure RdlcColumnExt_OnPreReportError_PdfBoolean_FalseWithError()
    begin
        BooleanForm(Report::"NBS Rdlc Col Rpt", 1, ReportFormat::Pdf, 'false Dialog/N1');
    end;

    [Test]
    procedure Excel_OnAfterGetRecordError_XmlBoolean_FalseWithError()
    begin
        BooleanForm(Report::"NBS Excel Rpt", 2, ReportFormat::Xml, 'false Dialog/N2');
    end;

    // The probed shapes (assumption saveas-platform-error-before-triggers).

    [Test]
    procedure Excel_OnPreReportError_PdfStatement_RaisesUnsupportedFormat()
    begin
        Statement(Report::"NBS Excel Rpt", 1, ReportFormat::Pdf, 'raised ReportUnsupportedFormat/The Pdf output format is not supported by the current report layout format.');
    end;

    [Test]
    procedure ProcessingOnly_OnPreReportError_WordBoolean_FalseWithProcessingOnly()
    begin
        BooleanForm(Report::"NBS Processing Rpt", 1, ReportFormat::Word, 'false ReportProcessingOnly/Report 97107 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure ProcessingOnly_OnPreReportError_WordStatement_RaisesProcessingOnly()
    begin
        Statement(Report::"NBS Processing Rpt", 1, ReportFormat::Word, 'raised ReportProcessingOnly/Report 97107 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    // The pinned shapes: text/PnBReportSaveAs.al:698; streams/PnH6SaveAs.al:196 (the Boolean Word form above) and :217.

    [Test]
    procedure NoLayout_VariableXml_OnPreReportError_LastErrorHoldsIt()
    var
        S: Codeunit "NBS Site";
        Rep: Report "NBS No Layout Pin Rpt";
        TempRow: Record "NBS Row" temporary;
        OutS: OutStream;
    begin
        S.Set(Report::"NBS No Layout Pin Rpt", 1, ReportFormat::Xml);
        ClearLastError();
        TempRow.Data.CreateOutStream(OutS);
        if Rep.SaveAs('', ReportFormat::Xml, OutS) then
            Error('SaveAs answered true');
        Check(GetLastErrorCode() + '/' + GetLastErrorText(), 'Dialog/N1');
    end;

    [Test]
    procedure NoLayout_VariableXml_OneDataItem_LastError_NotMeasured()
    var
        S: Codeunit "NBS Site";
        Rep: Report "NBS No Layout Rpt";
        TempRow: Record "NBS Row" temporary;
        OutS: OutStream;
    begin
        S.Set(Report::"NBS No Layout Rpt", 1, ReportFormat::Xml);
        ClearLastError();
        TempRow.Data.CreateOutStream(OutS);
        if Rep.SaveAs('', ReportFormat::Xml, OutS) then
            Error('SaveAs answered true');
        Check(GetLastErrorCode() + '/' + GetLastErrorText(), 'Dialog/N1');
    end;

    // Neighbors the rule does not cover: each keeps main's answer.

    [Test]
    procedure Word_OnPostDataItemError_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 7, ReportFormat::Pdf, 'false Dialog/N7');
    end;

    [Test]
    procedure Word_ErrorWithLabel_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 5, ReportFormat::Pdf, 'false Dialog/NBS label error');
    end;

    [Test]
    procedure Word_RecordNotFoundInOnPreReport_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 6, ReportFormat::Pdf, 'false DB:RecordNotFound/The NBS Row does not exist. Identification fields and values: Code=''MISSING''');
    end;

    [Test]
    procedure Word_VariableSaveAs_OnPreReportError_PdfBoolean_NotMeasured()
    var
        S: Codeunit "NBS Site";
        Rep: Report "NBS Word Rpt";
        TempRow: Record "NBS Row" temporary;
        OutS: OutStream;
    begin
        S.Set(Report::"NBS Word Rpt", 1, ReportFormat::Pdf);
        ClearLastError();
        TempRow.Data.CreateOutStream(OutS);
        if Rep.SaveAs('', ReportFormat::Pdf, OutS) then
            Error('SaveAs answered true');
        Check(GetLastErrorCode() + '/' + GetLastErrorText(), 'Dialog/N1');
    end;

    [Test]
    procedure WordTwoItems_OnAfterGetRecordError_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Two Items Rpt", 2, ReportFormat::Pdf, 'false Dialog/N2');
    end;

    [Test]
    procedure WordTriggerExt_ExtensionOnPreReportError_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Ext Rpt", 1, ReportFormat::Pdf, 'false Dialog/N1');
    end;

    [Test]
    procedure Excel_OnPreReportError_HtmlStatement_NotMeasured()
    begin
        Statement(Report::"NBS Excel Rpt", 1, ReportFormat::Html, 'raised ReportUnsupportedFormat/The Html output format is not supported by the current report layout format.');
    end;

    [Test]
    procedure ProcessingOnly_OnPreReportError_ExcelBoolean_FalseWithProcessingOnly()
    begin
        BooleanForm(Report::"NBS Processing Rpt", 1, ReportFormat::Excel, 'false ReportProcessingOnly/Report 97107 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure ProcessingOnly_OnPreReportError_ExcelStatement_NotMeasured()
    begin
        Statement(Report::"NBS Processing Rpt", 1, ReportFormat::Excel, 'raised ReportProcessingOnly/Report 97107 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure NoLayout_StaticXml_OnAfterGetRecordError_LastError_NotMeasured()
    begin
        BooleanForm(Report::"NBS No Layout Rpt", 2, ReportFormat::Xml, 'false Dialog/N2');
    end;

    // The gate conditions of review round 1: each keeps main's refusal.

    [Test]
    procedure Word_ErrorLiteralInOnPreDataItem_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 8, ReportFormat::Pdf, 'false Dialog/N8');
    end;

    [Test]
    procedure Word_TestFieldInOnPreReport_WordBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 9, ReportFormat::Word, 'false TestField/Tag must have a value in NBS Row: Code=. It cannot be zero or empty.');
    end;

    [Test]
    procedure Word_TestFieldWithValueInOnPreDataItem_XmlBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 10, ReportFormat::Xml, 'false TestField/Tag must be equal to ''X''  in NBS Row: Code=. Current value is ''''.');
    end;

    [Test]
    procedure Word_ReportInsertsThenErrors_PdfBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Word Rpt", 11, ReportFormat::Pdf, 'false Dialog/N11');
    end;

    [Test]
    procedure ProcessingOnly_WithOnInitReport_WordBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Processing Init Rpt", 1, ReportFormat::Word, 'false ReportProcessingOnly/Report 97113 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure ProcessingOnly_WithOnInitReport_WordStatement_NotMeasured()
    begin
        Statement(Report::"NBS Processing Init Rpt", 1, ReportFormat::Word, 'raised ReportProcessingOnly/Report 97113 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure ProcessingOnly_WithRequestPageOnInit_ExcelBoolean_NotMeasured()
    begin
        BooleanForm(Report::"NBS Processing Page Rpt", 1, ReportFormat::Excel, 'false ReportProcessingOnly/Report 97114 is a processing-only report. Output cannot be rendered and saved to a file.');
    end;

    [Test]
    procedure Excel_WithOnInitReport_PdfStatement_NotMeasured()
    begin
        Statement(Report::"NBS Excel Init Rpt", 1, ReportFormat::Pdf, 'raised ReportUnsupportedFormat/The Pdf output format is not supported by the current report layout format.');
    end;

    [Test]
    procedure ExcelPlusWord_OnPreReportError_PdfStatement_NotMeasured()
    begin
        Statement(Report::"NBS Excel Word Rpt", 1, ReportFormat::Pdf, 'raised ReportUnsupportedFormat/The Pdf output format is not supported by the current report layout format.');
    end;

    [Test]
    procedure Rdlc_OnPreReportError_PdfBoolean_MainsRule()
    begin
        BooleanForm(Report::"NBS Rdlc Rpt", 1, ReportFormat::Pdf, 'false Dialog/N1');
    end;
}
