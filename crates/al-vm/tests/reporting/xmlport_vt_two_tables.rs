//! A VariableText export whose root holds two table elements (corpus xmlport/lt-dg-xmlport-leads/DgXmlPortLeads.al:193-218,
//! Item106_TwoTablesDefaultSeparator and Item106_TwoTablesNewLineSeparator): the records of a table are joined by the record
//! separator, the table separator takes the place of the record separator between the two tables, and the last record has none.
//! The default table separator reads back as CR/LF twice, `<NewLine>` as CR/LF once.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(port_props: &str, seed: &str) -> String {
    format!(
        r#"
table 69990 "XT Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "XT Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[20]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69992 "XT Line" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Note; Text[20]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69993 "XT Two"
{{
    Format = VariableText; Direction = Export; UseRequestPage = false; {port_props}
    schema
    {{
        textelement(Root)
        {{
            tableelement(ItemRow; "XT Item") {{ fieldelement(Id; ItemRow.Id) {{ }} fieldelement(Name; ItemRow.Name) {{ }} }}
            tableelement(LineRow; "XT Line") {{ fieldelement(Id; LineRow.Id) {{ }} fieldelement(Note; LineRow.Note) {{ }} }}
        }}
    }}
}}
codeunit 69994 C
{{
    procedure Export(): Text
    var B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; Big: BigText; R: Text; Item: Record "XT Item"; Line: Record "XT Line";
    begin
        {seed}
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XT Two", OutS);
        B.Data.CreateInStream(InS, TextEncoding::MSDos);
        Big.Read(InS);
        Big.GetSubText(R, 1, Big.Length());
        exit(R);
    end;

    procedure Getter(): Text
    var Xp: XmlPort "XT Two";
    begin
        exit('[' + Xp.TableSeparator() + ']');
    end;
}}
"#
    )
}

const SEED: &str = "Item.Id := 1; Item.Name := 'Alpha'; Item.Insert(); Item.Id := 2; Item.Name := 'Beta'; Item.Insert(); Line.Id := 7; Line.Note := 'L7'; Line.Insert(); Line.Id := 8; Line.Note := 'L8'; Line.Insert();";

#[test]
fn the_table_separator_replaces_the_record_separator_between_two_tables() {
    // Item106_TwoTablesDefaultSeparator `export`.
    assert_eq!(
        al_testkit::text(run(&src("", SEED), "Export")),
        "\"1\",\"Alpha\"\r\n\"2\",\"Beta\"\r\n\r\n\"7\",\"L7\"\r\n\"8\",\"L8\""
    );
    // Item106_TwoTablesNewLineSeparator `export`.
    assert_eq!(
        al_testkit::text(run(&src("TableSeparator = '<NewLine>';", SEED), "Export")),
        "\"1\",\"Alpha\"\r\n\"2\",\"Beta\"\r\n\"7\",\"L7\"\r\n\"8\",\"L8\""
    );
}

#[test]
fn an_empty_table_among_two_is_refused() {
    let seed = "Item.Id := 1; Item.Name := 'Alpha'; Item.Insert();";
    assert!(matches!(run(&src("", seed), "Export"), Err(Signal::Unsupported(r)) if r.contains("not pinned")));
}

#[test]
fn the_table_separator_reads_back() {
    // Item106_TwoTablesNewLineSeparator `getter`.
    assert_eq!(
        al_testkit::text(run(&src("TableSeparator = '<NewLine>';", SEED), "Getter")),
        "[\\r\\n]"
    );
}
