codeunit 50149 "NB XUP Lib"
{
    procedure Seed()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Item.Id := 1;
        Item.Name := 'one';
        Item.Insert();
        Item.Id := 2;
        Item.Name := '  two  ';
        Item.Insert();
    end;

    procedure ReadLines(var InS: InStream): Text
    var
        Line: Text;
        Result: Text;
    begin
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Result += Line + '|';
        end;
        exit(Result);
    end;

    procedure ExportText(PortId: Integer; Enc: TextEncoding): Text
    var
        Src: Record "NB XUP Src" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        Src.Data.CreateOutStream(OutS, Enc);
        Xmlport.Export(PortId, OutS);
        Src.Data.CreateInStream(InS, Enc);
        exit(ReadLines(InS));
    end;

    procedure ImportText(PortId: Integer; Xml: Text)
    var
        Src: Record "NB XUP Src" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        Src.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        Src.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(PortId, InS);
    end;

    procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

}

codeunit 50150 "NB XUP Cells"
{
    Subtype = Test;

    var
        Lib: Codeunit "NB XUP Lib";

    [Test]
    procedure XUP_OneUnused_Export()
    begin
        Lib.Check(Lib.ExportText(50110, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_TwoUnused_Export()
    begin
        Lib.Check(Lib.ExportText(50111, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_NoDefault_Export()
    begin
        Lib.Check(Lib.ExportText(50114, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_DnNoUdn_Export()
    begin
        Lib.Check(Lib.ExportText(50115, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_UdnNoDn_Export()
    begin
        Lib.Check(Lib.ExportText(50116, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:microsoft-dynamics-nav/xmlports/x50116">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_TextAttr_Export()
    begin
        Lib.Check(Lib.ExportText(50117, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" Ver="v1" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_Table_Export()
    begin
        Lib.Seed();
        Lib.Check(Lib.ExportText(50119, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Item Id="1">|    <Name>one</Name>|  </Item>|  <Item Id="2">|    <Name>  two  </Name>|  </Item>|</Items>|');
    end;

    [Test]
    procedure XUP_TwoUnusedUtf16_Export()
    begin
        Lib.Check(Lib.ExportText(50120, TextEncoding::UTF16), '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_Both_Export()
    begin
        Lib.Seed();
        Lib.Check(Lib.ExportText(50121, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Item Id="1">|    <Name>one</Name>|  </Item>|  <Item Id="2">|    <Name>  two  </Name>|  </Item>|</Items>|');
    end;

    [Test]
    procedure XUP_TwoUnused_InstanceExport()
    var
        Port: XmlPort "NB XUP Two Unused";
        Src: Record "NB XUP Src" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        Src.Data.CreateOutStream(OutS);
        Port.SetDestination(OutS);
        Port.Export();
        Src.Data.CreateInStream(InS, TextEncoding::UTF8);
        Lib.Check(Lib.ReadLines(InS), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_Both_ImportOwnExport()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Lib.ImportText(50121, '<?xml version="1.0" encoding="UTF-8" standalone="no"?><Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order"><Item Id="1"><Name>one</Name></Item><Item Id="2"><Name>  two  </Name></Item></Items>');
        Lib.Check(Format(Item.Count()), '2');
        Item.Get(2);
        Lib.Check(Item.Name, 'two');
    end;

    [Test]
    procedure XUP_Both_ImportPlain_Fails()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Commit();
        asserterror Lib.ImportText(50121, '<Items><Item Id="7"><Name>seven</Name></Item></Items>');
        Lib.Check(GetLastErrorText(), 'The element <Items> cannot be found in the object description.');
        Lib.Check(Format(Item.Count()), '0');
    end;

}

codeunit 50151 "NB XUP Export Neighbors"
{
    Subtype = Test;

    var
        Lib: Codeunit "NB XUP Lib";

    [Test]
    procedure NB_TableMinOccurs_Export()
    begin
        Lib.Check(Lib.ExportText(50147, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Item Id="1">|    <Name>one</Name>|  </Item>|  <Item Id="2">|    <Name>  two  </Name>|  </Item>|</Items>|');
    end;

    [Test]
    procedure NB_EmptyPrefix_Export()
    begin
        Lib.Check(Lib.ExportText(50130, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns="urn:nb:xup:e">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_BothLeaf_Export()
    begin
        Lib.Check(Lib.ExportText(50132, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_VarText_Export()
    begin
        Lib.Check(Lib.ExportText(50133, TextEncoding::UTF8), '1,one|');
    end;

    [Test]
    procedure NB_Lax_Export()
    begin
        Lib.Check(Lib.ExportText(50134, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_Inline_Export()
    begin
        Lib.Check(Lib.ExportText(50135, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_Pws_Export()
    begin
        Lib.Check(Lib.ExportText(50136, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_V11_Export()
    begin
        Lib.Check(Lib.ExportText(50137, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_Three_Export()
    begin
        Lib.Check(Lib.ExportText(50138, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:q="urn:nb:q" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_NoEncoding_Export()
    begin
        Lib.Check(Lib.ExportText(50139, TextEncoding::UTF16), '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_TwoLeaves_Export()
    begin
        Lib.Check(Lib.ExportText(50140, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|  <Code>c</Code>|</Response>|');
    end;

    [Test]
    procedure NB_Nested_Export()
    begin
        Lib.Check(Lib.ExportText(50141, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <Body>|    <Status>ok</Status>|  </Body>|</Response>|');
    end;

    [Test]
    procedure NB_PlainTextAttr_Export()
    begin
        Lib.Check(Lib.ExportText(50142, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" Ver="v1" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_AttrNoDn_Export()
    begin
        Lib.Check(Lib.ExportText(50143, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" Ver="v1">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_TableNoDn_Export()
    begin
        Lib.Check(Lib.ExportText(50144, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Item Id="1">|    <Name>one</Name>|  </Item>|  <Item Id="2">|    <Name>  two  </Name>|  </Item>|</Items>|');
    end;

    [Test]
    procedure NB_Utf16NoDn_Export()
    begin
        Lib.Check(Lib.ExportText(50145, TextEncoding::UTF16), '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_OtherPrefixes_Export()
    begin
        Lib.Check(Lib.ExportText(50146, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:a="urn:nb:a" xmlns:b="urn:nb:b" xmlns="urn:nb:xup:order">|  <Status>ok</Status>|</Response>|');
    end;

    [Test]
    procedure NB_InstanceNoDefault_Export()
    var
        Port: XmlPort "NB XUP No Default";
        Src: Record "NB XUP Src" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        Src.Data.CreateOutStream(OutS);
        Port.SetDestination(OutS);
        Port.Export();
        Src.Data.CreateInStream(InS, TextEncoding::UTF8);
        Lib.Check(Lib.ReadLines(InS), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Status>ok</Status>|</Response>|');
    end;
}

codeunit 50152 "NB XUP Import Neighbors"
{
    Subtype = Test;

    var
        Lib: Codeunit "NB XUP Lib";

    [Test]
    procedure NB_Both_ImportPrefixedRoot()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Lib.ImportText(50121, '<xsd:Items xmlns:xsd="http://www.w3.org/2001/XMLSchema"><xsd:Item Id="7"><xsd:Name>seven</xsd:Name></xsd:Item></xsd:Items>');
        Lib.Check(Format(Item.Count()), '1');
    end;

    [Test]
    procedure NB_Both_ImportPrefixedChild()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Lib.ImportText(50121, '<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order"><Item Id="7"><xsd:Name>seven</xsd:Name></Item></Items>');
        Lib.Check(Format(Item.Count()), '1');
    end;

    [Test]
    procedure NB_Both_ImportDefaultOnly()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Lib.ImportText(50121, '<Items xmlns="urn:nb:xup:order"><Item Id="7"><Name>seven</Name></Item></Items>');
        Lib.Check(Format(Item.Count()), '1');
    end;

    [Test]
    procedure NB_Both_ImportPlainWithDecls()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        asserterror Lib.ImportText(50121, '<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><Item Id="7"><Name>seven</Name></Item></Items>');
        Lib.Check(GetLastErrorText(), 'The element <Items> cannot be found in the object description.');
    end;

    [Test]
    procedure NB_ImportDir_Import()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        Lib.ImportText(50131, '<?xml version="1.0" encoding="UTF-8" standalone="no"?><Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order"><Item Id="1"><Name>one</Name></Item><Item Id="2"><Name>  two  </Name></Item></Items>');
        Lib.Check(Format(Item.Count()), '2');
    end;

    [Test]
    procedure NB_Both_ImportMissingName()
    var
        Item: Record "NB XUP Item";
    begin
        Item.DeleteAll();
        asserterror Lib.ImportText(50121, '<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order"><Item Id="7"><Other>x</Other></Item></Items>');
        Lib.Check(Format(Item.Count()), '0');
    end;

}

// The layouts main already served (a prefix used by an element or a field attribute): main and head write BC's text.
codeunit 50153 "NB XUP Main Served"
{
    Subtype = Test;

    var
        Lib: Codeunit "NB XUP Lib";

    [Test]
    procedure XUP_FieldAttr_Export()
    begin
        Lib.Seed();
        Lib.Check(Lib.ExportText(50118, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">|  <Item Id="1">|    <Name>one</Name>|  </Item>|  <Item Id="2">|    <Name>  two  </Name>|  </Item>|</Items>|');
    end;

    [Test]
    procedure XUP_MixRootUsed_Export()
    begin
        Lib.Check(Lib.ExportText(50112, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://www.w3.org/2001/XMLSchema">|  <Status xmlns="urn:nb:xup:order">ok</Status>|</Response>|');
    end;

    [Test]
    procedure XUP_MixChildUsed_Export()
    begin
        Lib.Check(Lib.ExportText(50113, TextEncoding::UTF8), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Response xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:nb:xup:order">|  <xsi:Status>ok</xsi:Status>|</Response>|');
    end;
}
