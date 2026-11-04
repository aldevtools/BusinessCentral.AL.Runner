//! FieldDelimiter = '<>' (the empty separator token) of a VariableText port (AL Runner issue 6965; corpus
//! xmlports/shapes-properties/XpSeparatorTokens.al on branch al-runtime-rs-staging-r6963).

const SRC: &str = r#"
table 69980 "XE Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69981 "XE Row" { fields { field(1; Id; Integer) { } field(2; Street; Text[30]) { } } keys { key(PK; Id) { } } }
xmlport 69982 "XE Port"
{
    Direction = Import; Format = VariableText; UseRequestPage = false;
    FieldDelimiter = '<>'; FieldSeparator = '<TAB>'; TextEncoding = UTF8;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XE Row")
            {
                fieldelement(Id; Row.Id) { }
                fieldelement(Street; Row.Street) { }
            }
        }
    }
}
codeunit 69983 C
{
    procedure Import(): Text
    var B: Record "XE Blob" temporary; R: Record "XE Row"; OutS: OutStream; InS: InStream; T: Text; Tab: Text[1];
    begin
        Tab[1] := 9;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('1' + Tab + 'Hauptstrasse');
        OutS.WriteText();
        OutS.WriteText('2' + Tab + 'Süd');
        OutS.WriteText();
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XE Port", InS);
        if R.FindSet() then
            repeat T += Format(R.Id) + '=' + R.Street + ','; until R.Next() = 0;
        exit(T);
    end;
}
"#;

#[test]
fn empty_delimiter_token_imports_plain_values() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Import")),
        "1=Hauptstrasse,2=Süd,"
    );
}

/// A port with the given FieldDelimiter and FieldSeparator over a table of Id and Name; `Export` writes the rows 1 'Alpha' and
/// 2 'Be ta' and gives the bytes (CR, LF and TAB as \r, \n and \t, a byte above 127 as {N}); `Import` reads `Data` (with `~`
/// for a TAB and `|` for CR LF) and gives the rows.
fn port(fd: &str, fs: &str) -> String {
    format!(
        r#"
table 69984 "XT Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69985 "XT Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69986 "XT Port"
{{
    Direction = Both; Format = VariableText; UseRequestPage = false; TextEncoding = UTF8;
    FieldDelimiter = '{fd}'; FieldSeparator = '{fs}';
    schema {{ textelement(Root) {{ tableelement(Row; "XT Row") {{ fieldelement(Id; Row.Id) {{ }} fieldelement(Name; Row.Name) {{ }} }} }} }}
}}
codeunit 69987 C
{{
    procedure Export(): Text
    var R: Record "XT Row"; B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; Byte: Byte; T: Text; Tx: Text[1]; I: Integer;
    begin
        R.Id := 1; R.Name := 'Alpha'; R.Insert();
        R.Id := 2; R.Name := 'Be ta'; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XT Port", OutS);
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(Byte);
            case Byte of
                9: T += '\t';
                10: T += '\n';
                13: T += '\r';
                32..126: begin Tx[1] := Byte; T += Tx; end;
                else begin I := Byte; T += '{{' + Format(I) + '}}'; end;
            end;
        end;
        exit(T);
    end;

    procedure Import(Data: Text): Text
    var R: Record "XT Row"; B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; T: Text; Tab: Text[1]; CrLf: Text[2];
    begin
        Tab[1] := 9; CrLf[1] := 13; CrLf[2] := 10;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Data.Replace('~', Tab).Replace('|', CrLf));
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XT Port", InS);
        if R.FindSet() then
            repeat T += Format(R.Id) + '=[' + R.Name + '];'; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

fn export(fd: &str, fs: &str) -> String {
    al_testkit::text(al_testkit::run_in(&port(fd, fs), "C", "Export"))
}

fn import(fd: &str, fs: &str, data: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(
        &port(fd, fs),
        "C",
        "Import",
        vec![al_builtins::value::Value::Text(data.into())],
    )
    .map(|(v, _)| v)
}

/// XpSeparatorTokens.al (corpus branch al-runtime-rs-staging-r6963) VariableText_Export_*: `<>`, `<tab>` and `<none>` are
/// written as these characters; the tokens as their characters.
#[test]
fn export_writes_the_tokens_the_probe_measured() {
    let bom = "{239}{187}{191}";
    let cases = [
        ("<>", "<TAB>", "<>1<>\\t<>Alpha<>\\r\\n<>2<>\\t<>Be ta<>"),
        ("<>", ",", "<>1<>,<>Alpha<>\\r\\n<>2<>,<>Be ta<>"),
        ("<TAB>", ";", "\\t1\\t;\\tAlpha\\t\\r\\n\\t2\\t;\\tBe ta\\t"),
        ("<None>", "<>", "1<>Alpha\\r\\n2<>Be ta"),
        ("<None>", "<LF>", "1\\nAlpha\\r\\n2\\nBe ta"),
        ("<None>", "<CR>", "1\\rAlpha\\r\\n2\\rBe ta"),
        ("<None>", "<CR/LF>", "1\\r\\nAlpha\\r\\n2\\r\\nBe ta"),
        ("<None>", "<NewLine>", "1\\r\\nAlpha\\r\\n2\\r\\nBe ta"),
        ("<None>", "<tab>", "1<tab>Alpha\\r\\n2<tab>Be ta"),
        ("<None>", ";<TAB>", "1;\\tAlpha\\r\\n2;\\tBe ta"),
        ("<None>", "<TAB><TAB>", "1\\t\\tAlpha\\r\\n2\\t\\tBe ta"),
        ("<None>", "<None>", "1Alpha\\r\\n2Be ta"),
        ("<>", "<None>", "<>1<><>Alpha<>\\r\\n<>2<><>Be ta<>"),
        (
            "<none>",
            "<TAB>",
            "<none>1<none>\\t<none>Alpha<none>\\r\\n<none>2<none>\\t<none>Be ta<none>",
        ),
    ];
    for (fd, fs, want) in cases {
        assert_eq!(
            export(fd, fs),
            format!("{bom}{want}"),
            "FieldDelimiter {fd} FieldSeparator {fs}"
        );
    }
}

/// XpSeparatorTokens.al VariableText_Import_*: values with and without the `<>` around them read the same; a literal separator
/// that is not in the line leaves the whole line to the first field.
#[test]
fn import_reads_values_as_the_probe_measured() {
    let ok = [
        (
            "<>",
            "<TAB>",
            "<>1<>~<>Hauptstrasse<>|<>2<>~<>Süd<>|",
            "1=[Hauptstrasse];2=[Süd];",
        ),
        ("<>", ",", "1,Alpha|2,Be ta|", "1=[Alpha];2=[Be ta];"),
        ("<TAB>", ";", "~1~;~Alpha~|~2~;~Be;ta~|", "1=[Alpha];2=[Be;ta];"),
        ("<None>", ";<TAB>", "1;~Alpha|2;~Be ta|", "1=[Alpha];2=[Be ta];"),
        ("<None>", "<TAB><TAB>", "1~~Alpha|2~~Be ta|", "1=[Alpha];2=[Be ta];"),
        ("<none>", "<TAB>", "1~Alpha|", "1=[Alpha];"),
    ];
    for (fd, fs, data, want) in ok {
        assert_eq!(
            al_testkit::text(import(fd, fs, data)),
            want,
            "FieldDelimiter {fd} FieldSeparator {fs}"
        );
    }
    let failing = [
        (
            "<>",
            "<TAB>",
            "\"1\"~\"Hauptstrasse\"|",
            "The value \"\"1\"\" can't be evaluated into type Integer.",
        ),
        (
            "<None>",
            "<>",
            "1Alpha|",
            "The value \"1Alpha\" can't be evaluated into type Integer.",
        ),
        (
            "<None>",
            "<tab>",
            "1~Alpha|",
            "The value \"1\tAlpha\" can't be evaluated into type Integer.",
        ),
    ];
    for (fd, fs, data, want) in failing {
        assert_eq!(
            al_testkit::error_message(import(fd, fs, data)),
            want,
            "FieldDelimiter {fd} FieldSeparator {fs}"
        );
    }
}
