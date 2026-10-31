//! Single pins of misc/lt-y14-triage/Y14Small.al (corpus master, writer Y14): RecordId of a one-field Text key (Format and
//! Evaluate), SetValue of a Decimal on a text control, and XmlPort import into a temporary table element whose table has
//! insert and modify triggers.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50740 "Y Key"
{
    fields { field(1; Key50; Text[50]) { } field(2; Data; Text[30]) { } }
    keys { key(PK; Key50) { Clustered = true; } }
}
table 50741 "Y Trig"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnInsert() var L: Codeunit "Y Log"; begin L.Add('insert(' + Format(Id) + ')'); end;
    trigger OnModify() var L: Codeunit "Y Log"; begin L.Add('modify(' + Format(Id) + ')'); end;
}
codeunit 50742 "Y Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(E: Text) begin T += E + ';'; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
xmlport 50743 "Y Import Temp"
{
    Direction = Import;
    Format = Xml;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "Y Trig")
            {
                UseTemporary = true;
                XmlName = 'Row';
                fieldelement(Id; Row.Id) { }
                fieldelement(Name; Row.Name) { }
                trigger OnAfterInsertRecord() begin Seen += 1; end;
            }
        }
    }
    var Seen: Integer;
    procedure SeenRows(): Integer begin exit(Seen); end;
}
codeunit 50744 C
{
    procedure KeyText(Value: Text): Text
    var Rid: RecordId; Ok: Boolean; Back: Text;
    begin
        Ok := Evaluate(Rid, Value);
        if Ok then Back := Format(Rid);
        exit(Format(Ok) + '=' + Back);
    end;

    procedure FormatLead(): Text
    var K: Record "Y Key";
    begin
        K.Key50 := ' lead'; K.Insert();
        K.Key50 := 'a b'; K.Insert();
        K.FindFirst();
        exit(Format(K.RecordId));
    end;

    procedure EvalForms(): Text
    begin
        exit(KeyText('Y Key: ') + '|' + KeyText('Y Key:  lead') + '|' + KeyText('Y Key: a b ') + '|' + KeyText('Y Key: ""'));
    end;

    procedure ImportTemp(): Text
    var
        T: Record "Y Trig";
        L: Codeunit "Y Log";
        P: XmlPort "Y Import Temp";
        OutS: OutStream;
        InS: InStream;
    begin
        T.Id := 99;
        T.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Row><Id>1</Id><Name>one</Name></Row><Row><Id>2</Id><Name>two</Name></Row></Root>');
        T.Data.CreateInStream(InS, TextEncoding::UTF8);
        L.Take();
        P.SetSource(InS);
        P.Import();
        exit(L.Take() + '|' + Format(T.Count()) + '|' + Format(P.SeenRows()));
    end;
}
"#;

#[test]
fn format_of_a_recordid_quotes_a_text_key_that_starts_with_a_blank() {
    // Y14Small.al:173 `format [ lead]`.
    assert_eq!(text(run_in(SRC, "C", "FormatLead")), "Y Key: \" lead\"");
}

#[test]
fn evaluate_recordid_of_a_one_field_text_key_trims_and_accepts_an_empty_key() {
    // Y14Small.al:205 `eval empty key`, `eval leading blank`, `eval trailing blank`, `eval empty quoted`.
    assert_eq!(
        text(run_in(SRC, "C", "EvalForms")),
        "Yes=Y Key|Yes=Y Key: lead|Yes=Y Key: a b|Yes=Y Key: \"\""
    );
}

#[test]
fn xmlport_import_into_a_temporary_table_element_runs_no_insert_trigger() {
    // Y14Small.al:419 `temp log` '', `temp table rows` 0, `temp seen` 2.
    assert_eq!(text(run_in(SRC, "C", "ImportTemp")), "|0|2");
}
