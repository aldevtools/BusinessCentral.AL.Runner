//! `currXMLport.FieldSeparator := v` and the other separator and encoding members set by assignment in a VariableText port
//! (corpus xmlport/lt-y67-xmlport-separators/Y67XpSeparators.al:314-472). The assignment is the setter: a Text, a Char or a
//! Code value is used as it is, a token such as `<TAB>` or `<CR/LF>` is written literally, an empty text leaves the
//! separator out. TextEncoding set so changes the bytes of the export. The getter reads back what was assigned.

fn src(apply: &str) -> String {
    format!(
        r#"
table 69990 "XS Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69991 "XS Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69992 "XS Port"
{{
    Format = VariableText; Direction = Both; FieldDelimiter = '"'; FieldSeparator = ','; RecordSeparator = '<LF>';
    TextEncoding = UTF8; UseRequestPage = false;
    schema {{ textelement(Root) {{ tableelement(R; "XS Row") {{ fieldelement(Id; R.Id) {{ }} fieldelement(Name; R.Name) {{ }} }} }} }}
    var Seen: Text; Hash: Char; Tab: Text[1]; Code1: Code[1];
    trigger OnPreXmlPort()
    begin
        Hash := 35; Tab[1] := 9; Code1 := 'Z';
        {apply}
        Seen := '[' + currXMLport.FieldSeparator + '|' + currXMLport.FieldDelimiter + '|' + currXMLport.RecordSeparator + ']';
    end;
    procedure GetSeen(): Text begin exit(Seen); end;
}}
codeunit 69993 C
{{
    procedure Export(): Text
    var Row: Record "XS Row"; B: Record "XS Blob" temporary; Xp: XmlPort "XS Port"; OutS: OutStream; InS: InStream; Byte: Byte; T: Text; N: Integer; Tx: Text[1]; I: Integer;
    begin
        Row.Id := 1; Row.Name := 'a'; Row.Insert();
        Row.Id := 2; Row.Name := 'b'; Row.Insert();
        B.Data.CreateOutStream(OutS);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin InS.Read(Byte); N += 1; if (Byte >= 32) and (Byte < 127) then begin Tx[1] := Byte; T += Tx; end else if N > 3 then begin I := Byte; T += '{{' + Format(I) + '}}'; end; end;
        exit(Format(N) + ' ' + T + ' ' + Xp.GetSeen());
    end;

    procedure Import(Content: Text): Text
    var Row: Record "XS Row"; B: Record "XS Blob" temporary; Xp: XmlPort "XS Port"; OutS: OutStream; InS: InStream; T: Text; Ch: Char;
    begin
        Ch := 10;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Content.Replace('~', Format(Ch)));
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xp.SetSource(InS);
        Xp.Import();
        if Row.FindSet() then repeat T += '[' + Format(Row.Id) + '|' + Row.Name + ']'; until Row.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

fn export(apply: &str) -> String {
    al_testkit::text(al_testkit::run_in(&src(apply), "C", "Export"))
}

#[test]
fn an_assigned_field_separator_is_used_as_it_is() {
    // Y67XpSeparators.al:314 FieldSeparator_AssignForms_Export: m1 text, m5 Char, m6 Code[1], m4 Tab char, m2 several chars.
    assert_eq!(
        export("currXMLport.FieldSeparator := ';';"),
        "18 \"1\";\"a\"{10}\"2\";\"b\" [;|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldSeparator := Hash;"),
        "18 \"1\"#\"a\"{10}\"2\"#\"b\" [#|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldSeparator := Code1;"),
        "18 \"1\"Z\"a\"{10}\"2\"Z\"b\" [Z|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldSeparator := Tab[1];"),
        "18 \"1\"{9}\"a\"{10}\"2\"{9}\"b\" [\t|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldSeparator := '::';"),
        "20 \"1\"::\"a\"{10}\"2\"::\"b\" [::|\"|\\n]"
    );
}

#[test]
fn an_assigned_token_is_written_literally_and_an_empty_one_leaves_the_separator_out() {
    // m8 '<TAB>', m9 '', m13 FieldDelimiter '<None>', m16 RecordSeparator '<None>', m18 '<CR/LF>'.
    assert_eq!(
        export("currXMLport.FieldSeparator := '<TAB>';"),
        "26 \"1\"<TAB>\"a\"{10}\"2\"<TAB>\"b\" [<TAB>|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldSeparator := '';"),
        "16 \"1\"\"a\"{10}\"2\"\"b\" [|\"|\\n]"
    );
    assert_eq!(
        export("currXMLport.FieldDelimiter := '<None>';"),
        "58 <None>1<None>,<None>a<None>{10}<None>2<None>,<None>b<None> [,|<None>|\\n]"
    );
    assert_eq!(
        export("currXMLport.RecordSeparator := '<None>';"),
        "23 \"1\",\"a\"<None>\"2\",\"b\" [,|\"|<None>]"
    );
    assert_eq!(
        export("currXMLport.RecordSeparator := '<CR/LF>';"),
        "24 \"1\",\"a\"<CR/LF>\"2\",\"b\" [,|\"|<CR/LF>]"
    );
    assert_eq!(export("currXMLport.FieldDelimiter := '';"), "10 1,a{10}2,b [,||\\n]");
}

#[test]
fn an_assigned_text_encoding_changes_the_bytes() {
    // TextEncoding_AssignForms_Export: m21 UTF16 (FF FE, two bytes a character), m23 MSDos without a byte order mark.
    let utf16 = export("currXMLport.TextEncoding := TextEncoding::UTF16;");
    assert!(utf16.starts_with("32 "), "{utf16}");
    let msdos = export("currXMLport.TextEncoding := TextEncoding::MSDos;");
    assert!(msdos.starts_with("15 "), "{msdos}");
}

#[test]
fn an_assigned_separator_splits_an_import() {
    // Assign_Import: a Char field separator, a Char record separator, `<CR/LF>` assigned does not split LF-only lines.
    let import = |apply: &str, content: &str| {
        al_testkit::text(
            al_testkit::run_with(
                &src(apply),
                "C",
                "Import",
                vec![al_builtins::value::Value::Text(content.into())],
            )
            .map(|r| r.0),
        )
    };
    assert_eq!(import("currXMLport.FieldSeparator := Hash;", "1#a~2#b"), "[1|a][2|b]");
    assert_eq!(import("currXMLport.RecordSeparator := Hash;", "1,a#2,b"), "[1|a][2|b]");
}
