codeunit 50899 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DependentExtensionTargetsRowsAddedByDependency()
    var
        Trace: Codeunit "Cross Report Trace";
        Rows: Record "Cross Report Number";
        Number: Integer;
        Buffer: Record "Cross Report Number" temporary;
        Output: OutStream;
        Input: InStream;
        Line: Text;
        Xml: Text;
    begin
        for Number := 1 to 4 do begin
            Rows.Number := Number;
            Rows.Insert();
        end;
        Buffer.Data.CreateOutStream(Output);
        Report.SaveAs(Report::"Cross Report Base", '', ReportFormat::Xml, Output);
        Buffer.Data.CreateInStream(Input, TextEncoding::UTF8);
        while not Input.EOS() do begin
            Input.ReadText(Line);
            Xml += Line;
        end;
        if not Xml.Contains('<Column name="DependentNo">2</Column>') then
            Error('The dependent column did not contain the owner row value.');
        if Trace.Read() <> 'owner=2|dependent=2|before|footer|after=2|' then
            Error('Cross report observation: %1', Trace.Read());
    end;
}
