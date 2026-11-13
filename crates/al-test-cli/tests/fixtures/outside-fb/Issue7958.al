// al-runner issue 7958, the reporter's repro (2/2 green on a real BC container). A temporary Blob field stands in for Microsoft's
// Temp Blob, and a temporary data item with the rows 1 to 4 for the Integer table, which the test kit does not have.
report 50140 "Repro Round Dataset"
{
    UsageCategory = None;

    dataset
    {
        dataitem(PriceItem; "Repro Price Row")
        {
            UseTemporary = true;
            column(Number; Number) { }
            column(Price; PriceValue) { DecimalPlaces = 0 : 5; }

            trigger OnAfterGetRecord()
            begin
                case Number of
                    1:
                        PriceValue := Round(2.5, 0.01);
                    2:
                        PriceValue := Round(3, 0.25);
                    3:
                        PriceValue := Round(2.50 * 1, 0.01);
                    4:
                        PriceValue := Round(1.200001, 0.00001);
                end;
            end;
        }
    }

    trigger OnPreReport()
    var
        N: Integer;
    begin
        for N := 1 to 4 do begin
            PriceItem.Number := N;
            PriceItem.Insert();
        end;
    end;

    var
        PriceValue: Decimal;
}

table 50147 "Repro Price Row"
{
    fields
    {
        field(1; Number; Integer) { }
    }
    keys { key(PK; Number) { Clustered = true; } }
}

table 50148 "Repro Blob Holder"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; B; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50149 "Repro Round Scale Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FormatOfRoundResults()
    var
        Results: Text;
    begin
        Results := Format(Round(2.5, 0.01), 0, 9) + '|' + Format(Round(3, 0.25), 0, 9) + '|' +
          Format(Round(2.50 * 1, 0.01), 0, 9) + '|' + Format(Round(1.200001, 0.00001), 0, 9);
        if Results <> '2.5|3|2.5|1.2' then
            Error('Format(..., 0, 9) gave %1, expected 2.5|3|2.5|1.2.', Results);
    end;

    [Test]
    procedure DatasetOfRoundResults()
    var
        TempHolder: Record "Repro Blob Holder" temporary;
        OutStr: OutStream;
        InStr: InStream;
        Xml: Text;
        Line: Text;
        Values: Text;
        StartPos: Integer;
        EndPos: Integer;
    begin
        TempHolder.B.CreateOutStream(OutStr);
        Report.SaveAs(Report::"Repro Round Dataset", '', ReportFormat::Xml, OutStr);
        TempHolder.B.CreateInStream(InStr);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            Xml += Line;
        end;
        StartPos := StrPos(Xml, '<Column name="Price"');
        while StartPos > 0 do begin
            Xml := CopyStr(Xml, StartPos);
            StartPos := StrPos(Xml, '>') + 1;
            EndPos := StrPos(Xml, '</Column>');
            Values += CopyStr(Xml, StartPos, EndPos - StartPos) + '|';
            Xml := CopyStr(Xml, EndPos);
            StartPos := StrPos(Xml, '<Column name="Price"');
        end;
        if Values <> '2.50|3.00|2.50|1.20000|' then
            Error('The XML dataset holds %1, expected 2.50|3.00|2.50|1.20000|.', Values);
    end;
}
