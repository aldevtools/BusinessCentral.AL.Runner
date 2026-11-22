codeunit 61101 "SAI Tests"
{
    Subtype = Test;

    var
        H: Codeunit "SAI Helper";
        Assert: Codeunit "SAI Assert";
        EmptyErr: Label 'The report couldn’t be generated, because it was empty. Adjust your filters and try again.', Locked = true;

    // The reporter's repro (al-runner issue 8668): BC 29 fails at SaveAs with the empty-report error.
    [Test]
    procedure Repro_TwoItemsNoRows_PdfStatement()
    begin
        asserterror H.SaveStatement(Report::"SAI Word Break", ReportFormat::Pdf);
        Assert.Equal(EmptyErr, GetLastErrorText(), 'statement error');
        Assert.Equal('ReportResultEmpty', GetLastErrorCode(), 'statement code');
    end;

    [Test]
    procedure Measured_WordTwo_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Two", ReportFormat::Pdf), 'pdf');
        Assert.Equal(EmptyErr, GetLastErrorText(), 'text');
    end;

    [Test]
    procedure Measured_WordTwo_Empty_Word()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Two", ReportFormat::Word), 'word');
    end;

    [Test]
    procedure Measured_WordTwo_Empty_Html()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Two", ReportFormat::Html), 'html');
    end;

    [Test]
    procedure Measured_WordBreak_Pdf()
    begin
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Break", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Measured_WordThree_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Three", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Measured_WordThree_Empty_Word()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Three", ReportFormat::Word), 'word');
    end;

    [Test]
    procedure Measured_WordThree_Empty_Html()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Three", ReportFormat::Html), 'html');
    end;

    [Test]
    procedure Measured_RdlcTwo_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=Yes len=0 code=', H.SaveUsed(Report::"SAI Rdlc Two", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Measured_RdlcTwo_Empty_Word()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=Yes len=0 code=', H.SaveUsed(Report::"SAI Rdlc Two", ReportFormat::Word), 'word');
    end;

    // Not checked by the entry: the data item triggers. The items skip every record, so no dataset row.
    [Test]
    procedure NotChecked_WordSkip_Pdf()
    begin
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Skip", ReportFormat::Pdf), 'pdf');
    end;

    // Neighbors the new rows do not take.
    [Test]
    procedure Refused_WordTwo_Empty_WordStatement()
    begin
        H.Fill(0, 0, 0);
        asserterror H.SaveStatement(Report::"SAI Word Two", ReportFormat::Word);
        Assert.Equal('ReportResultEmpty', GetLastErrorCode(), 'code');
    end;

    [Test]
    procedure Refused_WordTwo_Empty_HtmlStatement()
    begin
        H.Fill(0, 0, 0);
        asserterror H.SaveStatement(Report::"SAI Word Two", ReportFormat::Html);
        Assert.Equal('ReportResultEmpty', GetLastErrorCode(), 'code');
    end;

    [Test]
    procedure Refused_WordThree_Empty_PdfStatement()
    begin
        H.Fill(0, 0, 0);
        asserterror H.SaveStatement(Report::"SAI Word Three", ReportFormat::Pdf);
        Assert.Equal('ReportResultEmpty', GetLastErrorCode(), 'code');
    end;

    [Test]
    procedure Refused_WordFour_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Four", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Refused_WordNested_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Nested", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Refused_WordTrigger_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Trigger", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Refused_WordLegacy_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Legacy", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Refused_WordExt_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportResultEmpty', H.SaveUsed(Report::"SAI Word Ext Base", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Refused_WordTwo_Empty_RecordRef()
    var
        Blob: Record "SAI Blob";
        A: Record "SAI A";
        RecRef: RecordRef;
        OutS: OutStream;
    begin
        H.Fill(0, 0, 0);
        RecRef.GetTable(A);
        Blob.Data.CreateOutStream(OutS);
        Assert.IsFalse(Report.SaveAs(Report::"SAI Word Two", '', ReportFormat::Pdf, OutS, RecRef), 'answer');
    end;

    [Test]
    procedure Refused_WordTwo_Empty_RequestParameters()
    var
        Blob: Record "SAI Blob";
        OutS: OutStream;
    begin
        H.Fill(0, 0, 0);
        Blob.Data.CreateOutStream(OutS);
        Assert.IsFalse(Report.SaveAs(Report::"SAI Word Two", '<?xml version="1.0" standalone="yes"?><ReportParameters></ReportParameters>', ReportFormat::Pdf, OutS), 'answer');
    end;

    [Test]
    procedure Main_RdlcThree_Empty_Pdf()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=Yes len=0 code=', H.SaveUsed(Report::"SAI Rdlc Three", ReportFormat::Pdf), 'pdf');
    end;

    [Test]
    procedure Main_RdlcTwo_Empty_PdfStatement()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal(0, H.SaveStatement(Report::"SAI Rdlc Two", ReportFormat::Pdf), 'len');
    end;

    // Main's answers: unchanged.
    [Test]
    procedure Main_WordTwo_OneRow_Pdf()
    begin
        H.Fill(0, 1, 0);
        Assert.IsTrue(H.SaveUsed(Report::"SAI Word Two", ReportFormat::Pdf).StartsWith('ok=Yes'), 'pdf');
    end;

    [Test]
    procedure Main_WordTwo_Empty_Excel()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportUnsupportedFormat', H.SaveUsed(Report::"SAI Word Two", ReportFormat::Excel), 'excel');
    end;

    [Test]
    procedure Main_RdlcTwo_Empty_Html()
    begin
        H.Fill(0, 0, 0);
        Assert.Equal('ok=No len=0 code=ReportUnsupportedFormat', H.SaveUsed(Report::"SAI Rdlc Two", ReportFormat::Html), 'html');
    end;

    [Test]
    procedure Main_WordTwo_Empty_Xml()
    begin
        H.Fill(0, 0, 0);
        Assert.IsTrue(H.SaveUsed(Report::"SAI Word Two", ReportFormat::Xml).StartsWith('ok=Yes'), 'xml');
    end;
}

codeunit 61102 "SAI Assert"
{
    procedure Equal(Expected: Variant; Actual: Variant; What: Text)
    begin
        if Format(Expected) <> Format(Actual) then
            Error('%1: expected <%2>, got <%3>', What, Expected, Actual);
    end;

    procedure IsTrue(Cond: Boolean; What: Text)
    begin
        if not Cond then
            Error('%1: expected true', What);
    end;

    procedure IsFalse(Cond: Boolean; What: Text)
    begin
        if Cond then
            Error('%1: expected false', What);
    end;
}
