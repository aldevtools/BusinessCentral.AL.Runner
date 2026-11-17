//! al-runner issue 8173: a Dictionary key of an XML handle and a search of a list of JsonToken values are not measured: the
//! key compare and the list search refuse for them. Keys and lists of other types keep their answers.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run_in, run_with, text};

const SRC: &str = r#"
codeunit 50334 T
{
    procedure XmlKey(Member: Integer): Text
    var
        D: Dictionary of [XmlNode, Integer];
        N: XmlNode;
        I: Integer;
    begin
        N := XmlElement.Create('a').AsXmlNode();
        D.Add(N, 1);
        case Member of
            1: exit(Format(D.Add(N, 2)));
            2: exit(Format(D.ContainsKey(N)));
            3: exit(Format(D.Get(N)));
            4: exit(Format(D.Get(N, I)));
            5: exit(Format(D.Remove(N)));
            6: begin D.Set(N, 2); exit(Format(D.Count())); end;
        end;
    end;

    procedure XmlKeyNoCompare(): Text
    var
        D: Dictionary of [XmlNode, Integer];
        N: XmlNode;
    begin
        N := XmlElement.Create('a').AsXmlNode();
        D.Add(N, 1);
        exit(Format(D.Count()) + '|' + Format(D.Keys().Count()) + '|' + Format(D.Values().Get(1)));
    end;

    procedure TokenList(Member: Integer): Text
    var
        L: List of [JsonToken];
        O: JsonObject;
        T: JsonToken;
    begin
        T := O.AsToken();
        L.Add(T);
        case Member of
            1: exit(Format(L.Contains(T)));
            2: exit(Format(L.IndexOf(T)));
            3: exit(Format(L.LastIndexOf(T)));
            4: exit(Format(L.Remove(T)));
        end;
    end;

    procedure TextKeys(): Text
    var
        D: Dictionary of [Text, Integer];
        I: Integer;
    begin
        D.Add('a', 1);
        D.Set('b', 2);
        D.Set('a', 3);
        exit(Format(D.Add('a', 4)) + '|' + Format(D.ContainsKey('b')) + '|' + Format(D.Get('a')) + '|' + Format(D.Get('c', I)) + '|' + Format(D.Remove('b')) + '|' + Format(D.Count()));
    end;
}
"#;

fn refused(routine: &str, member: i32) -> String {
    match run_with(SRC, "T", routine, vec![Value::Integer(member)]) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine} {member}: {other:?}"),
    }
}

#[test]
fn a_dictionary_key_of_an_xml_handle_refuses_when_it_is_compared() {
    for (member, name) in [(1, "Add"), (2, "ContainsKey"), (3, "Get"), (4, "Get"), (5, "Remove"), (6, "Set")] {
        let why = refused("XmlKey", member);
        assert!(why.contains(&format!("Dictionary.{name}")) && why.contains("XML"), "{member}: {why}");
    }
}

#[test]
fn a_dictionary_of_xml_keys_that_compares_nothing_keeps_its_answer() {
    assert_eq!(text(run_in(SRC, "T", "XmlKeyNoCompare")), "1|1|1");
}

#[test]
fn a_search_of_a_list_of_json_tokens_refuses() {
    for (member, name) in [(1, "Contains"), (2, "IndexOf"), (3, "LastIndexOf"), (4, "Remove")] {
        let why = refused("TokenList", member);
        assert!(why.contains(&format!("List.{name}")) && why.contains("JsonToken"), "{member}: {why}");
    }
}

#[test]
fn a_dictionary_of_text_keys_keeps_its_answers() {
    assert_eq!(text(run_in(SRC, "T", "TextKeys")), "No|Yes|3|No|Yes|1");
}
