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
    Format = VariableText; Direction = Both; UseRequestPage = false; {port_props}
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

    procedure Import(Doc: Text): Text
    var B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; Item: Record "XT Item"; Line: Record "XT Line"; R: Text; Ok: Boolean;
    begin
        Item.Id := 1; Item.Name := 'one'; Item.Insert();
        Line.Id := 1; Line.Note := 'c1'; Line.Insert();
        Commit();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Doc.Replace('~', Crlf()));
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        ClearLastError();
        Ok := Xmlport.Import(Xmlport::"XT Two", InS);
        if not Ok then R := GetLastErrorText() + '|' + GetLastErrorCode() + '|';
        if Item.FindSet() then repeat R += Format(Item.Id) + '/' + Item.Name + ';'; until Item.Next() = 0;
        R += '#';
        if Line.FindSet() then repeat R += Format(Line.Id) + '/' + Line.Note + ';'; until Line.Next() = 0;
        exit(R);
    end;

    local procedure Crlf(): Text
    var T: Text; Ch: Char;
    begin
        Ch := 13; T += Ch; Ch := 10; T += Ch;
        exit(T);
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

// The staged probe xmlport/lt-y83-xmlport-shapes/Y83XpMiscTests.al:VariableText_TwoTables (table separator '#'): `Import(Doc)` seeds
// Item 1/one and Line 1/c1 first and reads `~` in the document as CR/LF.
fn import(doc: &str) -> String {
    let (v, _) = al_testkit::run_with(
        &src("TableSeparator = '#';", SEED),
        "C",
        "Import",
        vec![al_builtins::value::Value::Text(doc.into())],
    )
    .unwrap();
    al_testkit::text(Ok(v))
}

#[test]
fn two_tables_import_a_document_with_the_table_separator() {
    assert_eq!(import("7,seven~#2,c2~"), "1/one;7/seven;#1/c1;2/c2;");
}

#[test]
fn a_document_that_breaks_the_second_table_leaves_nothing() {
    // `two tables doc one`: the table separator is followed by a line break, so the second table starts with an empty record.
    assert_eq!(
        import("7,seven~#~1,x~"),
        "The Element <Note> is expected by Min Occurs value: Once. Element received: <LineRow>.|XmlPortMetadata|1/one;#1/c1;"
    );
    // `two tables doc three`: no second table.
    assert_eq!(
        import("7,seven~"),
        "The Element <LineRow> is expected by Min Occurs value: Once. Element received: <Root>.|XmlPortMetadata|1/one;#1/c1;"
    );
}
