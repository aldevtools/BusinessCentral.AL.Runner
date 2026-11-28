//! An xmlport whose Namespaces no element uses, in the Direction = Both table layout of corpus PR 1663
//! (xmlports/shapes-properties/XpUnusedNsPrefix.al XUP_Both_ExportImport :486, assumption `xmlport-unused-namespace-prefix`):
//! the import of its own export and of a plain document run; every other document and every error after the root is read
//! are refused.

use al_testrun::Outcome;

const SRC: &str = r#"
table 50260 "XU Item" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
table 50261 "XU Blob" { fields { field(1; K; Integer) { } field(2; B; Blob) { } } keys { key(PK; K) { } } }
xmlport 50262 "XU Both" { Direction = Both; Format = Xml; UseRequestPage = false; Encoding = UTF8;
    DefaultNamespace = 'urn:xu'; UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';
    schema { textelement(Items) { tableelement(Rw; "XU Item") { XmlName = 'Item';
        fieldattribute(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } } } }
codeunit 50263 T { Subtype = Test;
    local procedure Import(Xml: Text)
    var R: Record "XU Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        R.B.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        R.B.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XU Both", InS);
    end;
    [Test] procedure OwnExport()
    var I: Record "XU Item";
    begin
        Import('<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:xu"><Item Id="1"><Name>one</Name></Item></Items>');
        if I.Count() <> 1 then Error('rows %1', I.Count());
    end;
    [Test] procedure Plain()
    begin
        asserterror Import('<Items><Item Id="7"><Name>seven</Name></Item></Items>');
        if GetLastErrorText() <> 'The element <Items> cannot be found in the object description.' then Error(GetLastErrorText());
    end;
    [Test] procedure DefaultOnly()
    begin
        Import('<Items xmlns="urn:xu"><Item Id="1"><Name>one</Name></Item></Items>');
    end;
    [Test] procedure PlainWithDeclarations()
    begin
        asserterror Import('<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><Item Id="7"><Name>seven</Name></Item></Items>');
    end;
    [Test] procedure ChildInNoNamespace()
    begin
        asserterror Import('<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:xu"><Item xmlns="" Id="7"><Name>seven</Name></Item></Items>');
    end;
    [Test] procedure ErrorAfterTheRoot()
    begin
        asserterror Import('<Items xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="urn:xu"><Item Id="7"><Other>x</Other></Item></Items>');
    end;
}
"#;

#[test]
fn an_unused_prefix_port_imports_only_the_measured_documents() {
    let got = al_testkit::outcomes_in(SRC, "T");
    let get = |name: &str| got.iter().find(|(n, _)| n == name).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(get("OwnExport"), Outcome::Pass);
    assert_eq!(get("Plain"), Outcome::Pass);
    let other =
        "an XmlPort document other than the measured ones, for a port whose Namespaces no element uses (not pinned)";
    for name in ["DefaultOnly", "PlainWithDeclarations", "ChildInNoNamespace"] {
        assert!(
            matches!(get(name), Outcome::NotRun(ref w) if w.contains(other)),
            "{name}: {:?}",
            get(name)
        );
    }
    assert!(
        matches!(get("ErrorAfterTheRoot"), Outcome::NotRun(ref w)
            if w.contains("XmlPort import error in a port whose Namespaces no element uses (not pinned)")),
        "{:?}",
        get("ErrorAfterTheRoot")
    );
}
