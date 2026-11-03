//! A Legacy (`FormatEvaluate = Legacy`) Xml port reads a Boolean as Evaluate does, and under language 1031 its error is the German
//! one (corpus xmlport/lt-dg-xmlport-leads/DgXmlPortLeads.al:Item107_BoolImportByLanguage `legacy_1031_yes`).

const SRC: &str = r#"
table 69995 "XG Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69996 "XG Item" { fields { field(1; Id; Integer) { } field(2; Flag; Boolean) { } } keys { key(PK; Id) { } } }
xmlport 69997 "XG Port"
{
    Format = Xml; Direction = Import; FormatEvaluate = Legacy; UseRequestPage = false;
    schema { textelement(Root) { tableelement(ItemRow; "XG Item") { XmlName = 'Item'; fieldelement(Id; ItemRow.Id) { } fieldelement(Flag; ItemRow.Flag) { } } } }
}
codeunit 69998 C
{
    procedure F(): Text
    var B: Record "XG Blob" temporary; OutS: OutStream; InS: InStream; Saved: Integer; R: Text;
    begin
        Saved := GlobalLanguage();
        GlobalLanguage(1031);
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Item><Id>1</Id><Flag>Yes</Flag></Item></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        ClearLastError();
        if not Xmlport.Import(Xmlport::"XG Port", InS) then
            R := GetLastErrorText() + '|' + GetLastErrorCode();
        GlobalLanguage(Saved);
        exit(R);
    end;
}
"#;

#[test]
fn the_evaluate_error_of_a_legacy_boolean_is_german_under_1031() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "Der Wert „Yes“ kann nicht in Typ Boolean ausgewertet werden.|Evaluate"
    );
}

const GERMAN_DATE: &str = "Das Format Date „31.12.2024“ stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format „jjjj-mm-tt“ verwenden.|Evaluate";

#[test]
fn the_error_of_a_date_nobody_reads_is_german_under_1031() {
    // Y83XpMiscTests.al:DateDecimal_XML_German `31.12.2024` (needs the corpus bump), a port without FormatEvaluate and a Legacy one.
    for src in [SRC.replace("FormatEvaluate = Legacy; ", ""), SRC.to_string()] {
        let src = src
            .replace("field(2; Flag; Boolean)", "field(2; Flag; Date)")
            .replace("<Flag>Yes</Flag>", "<Flag>31.12.2024</Flag>");
        assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), GERMAN_DATE);
    }
}
