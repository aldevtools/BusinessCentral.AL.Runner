// The dependency app of the neighbor: a table with a Blob field and an install codeunit that stores its rows, so they
// reach the tests through the install baseline (its captured form and every copy read from its bytes). One row per mark
// a stored row hands on to the record that reads it (`copy_blob`), and two rows without a mark as the control.
table 61110 "BF Blob Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
        field(3; Pic; Media) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 61111 "BF Blob Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        StoreBytesThroughUtf16(1);
        StoreText(2, 'Plain', TextEncoding::UTF8);
        StoreText(3, 'Wide', TextEncoding::UTF16);
        StorePng(4);
        StoreReportOutput(5);
    end;

    // The utf16_binary mark: two Byte values written through a UTF-16 OutStream (the bytes 41 00).
    local procedure StoreBytesThroughUtf16(RowId: Integer)
    var
        Row: Record "BF Blob Row";
        Out: OutStream;
        B: Byte;
    begin
        Row.Id := RowId;
        Row.Data.CreateOutStream(Out, TextEncoding::UTF16);
        B := 65;
        Out.Write(B);
        B := 0;
        Out.Write(B);
        Row.Insert();
    end;

    // The control: a Text written through an OutStream, so the fields the old bytes form dropped hold their defaults.
    local procedure StoreText(RowId: Integer; Value: Text; Encoding: TextEncoding)
    var
        Row: Record "BF Blob Row";
        Out: OutStream;
    begin
        Row.Id := RowId;
        Row.Data.CreateOutStream(Out, Encoding);
        Out.WriteText(Value);
        Row.Insert();
    end;

    // The inserted mark: the content is what the Insert stored, and nothing wrote to it after. A 1x1 PNG.
    local procedure StorePng(RowId: Integer)
    var
        Row: Record "BF Blob Row";
        Out: OutStream;
        Hex: Text;
        B: Byte;
        i: Integer;
    begin
        Hex := '89504E470D0A1A0A0000000D4948445200000001000000010806000000' +
               '1F15C4890000000D49444154789C63F8FFFF3F0005FE02FEA7358184' +
               '0000000049454E44AE426082';
        Row.Id := RowId;
        Row.Data.CreateOutStream(Out);
        i := 1;
        while i < StrLen(Hex) do begin
            B := HexValue(Hex[i]) * 16 + HexValue(Hex[i + 1]);
            Out.Write(B);
            i += 2;
        end;
        Row.Insert();
    end;

    // The render_pending mark: a report saved to the Blob in a rendered format.
    local procedure StoreReportOutput(RowId: Integer)
    var
        Row: Record "BF Blob Row";
        Out: OutStream;
    begin
        Row.Id := RowId;
        Row.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"BF Layout Report", '', ReportFormat::Pdf, Out);
        Row.Insert();
    end;

    local procedure HexValue(C: Char): Integer
    begin
        if (C >= '0') and (C <= '9') then
            exit(C - '0');
        exit(C - 'A' + 10);
    end;
}

report 61112 "BF Layout Report"
{
    UseRequestPage = false;
    DefaultRenderingLayout = BFLayout;

    dataset
    {
        dataitem(Numbers; Integer)
        {
            DataItemTableView = where(Number = const(1));
            column(Number; Number) { }
        }
    }

    rendering
    {
        layout(BFLayout)
        {
            Type = RDLC;
            LayoutFile = './layouts/BFLayout.rdl';
        }
    }
}
