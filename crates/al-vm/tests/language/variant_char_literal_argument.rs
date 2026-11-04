//! A text literal of one character handed to a Variant parameter is a Text, not a Char (al-runner issue 6924, item 3.8;
//! types/variant/VaTIsChecks.al:179-180, :185, json/JsonObjectBasics.al:485, :489). Assigned to a Variant it is a Char.

use al_testkit::{run, text};

const SRC: &str = r#"
codeunit 69990 C
{
    local procedure Flags(V: Variant): Text
    begin
        if V.IsChar() then exit('Char');
        if V.IsText() then exit('Text');
        if V.IsCode() then exit('Code');
        exit('other');
    end;
    local procedure AddVariant(var O: JsonObject; KeyName: Text; Value: Variant)
    var Ch: Char; T: Text;
    begin
        case true of
            Value.IsChar():
                begin Ch := Value; O.Add(KeyName, Ch); end;
            Value.IsText(), Value.IsCode():
                begin T := Value; O.Add(KeyName, T); end;
        end;
    end;
    procedure F(): Text
    var V: Variant; Ch: Char; C1: Code[1];
    begin
        Ch := ','; C1 := ',';
        V := ',';
        exit(Flags(',') + ',' + Flags('x') + ',' + Flags('ab') + ',' + Flags(Ch) + ',' + Flags(C1) + ',' + Flags(V));
    end;
    procedure J(): Text
    var O: JsonObject; Ch: Char; Txt: Text;
    begin
        Ch := ',';
        AddVariant(O, 'lit', ',');
        AddVariant(O, 'chr', Ch);
        O.WriteTo(Txt);
        exit(Txt);
    end;
}
"#;

#[test]
fn a_one_character_literal_argument_is_a_text_in_the_variant() {
    assert_eq!(text(run(SRC, "F")), "Text,Text,Text,Char,Code,Char");
}

#[test]
fn a_json_helper_that_switches_on_is_char_adds_the_literal_as_text() {
    assert_eq!(text(run(SRC, "J")), r#"{"lit":",","chr":44}"#);
}
