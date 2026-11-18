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
    for (member, name) in [
        (1, "Add"),
        (2, "ContainsKey"),
        (3, "Get"),
        (4, "Get"),
        (5, "Remove"),
        (6, "Set"),
    ] {
        let why = refused("XmlKey", member);
        assert!(
            why.contains(&format!("Dictionary.{name}")) && why.contains("XML"),
            "{member}: {why}"
        );
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
        assert!(
            why.contains(&format!("List.{name}")) && why.contains("JsonToken"),
            "{member}: {why}"
        );
    }
}

#[test]
fn a_dictionary_of_text_keys_keeps_its_answers() {
    assert_eq!(text(run_in(SRC, "T", "TextKeys")), "No|Yes|3|No|Yes|1");
}

/// A Dictionary keyed by, and a List of, values of type `ty` made by `make`; `body` runs on `D`, `L` and `X`.
fn json(ty: &str, make: &str, body: &str) -> Result<Value, Signal> {
    let src = format!(
        "codeunit 50335 J {{ procedure F(): Text var D: Dictionary of [{ty}, Integer]; L: List of [{ty}]; X: {ty}; O: JsonObject; A: JsonArray; V: JsonValue; \
         begin {make} {body} end; }}"
    );
    run_in(&src, "J", "F")
}

const JSON_KINDS: [(&str, &str); 4] = [
    ("JsonObject", "O.ReadFrom('{\"a\":1}'); X := O;"),
    ("JsonArray", "A.Add(1); X := A;"),
    ("JsonValue", "V.SetValue(1); X := V;"),
    ("JsonToken", "X := O.AsToken();"),
];

// al-runner issue 8173: how BC compares two JSON values is not measured (corpus probe P2): a key compare and a list search
// refuse for every JSON type.
#[test]
fn a_json_dictionary_key_or_list_search_refuses_when_it_compares() {
    for (ty, make) in JSON_KINDS {
        for body in [
            "D.Add(X, 1); exit(Format(D.ContainsKey(X)));",
            "L.Add(X); exit(Format(L.Contains(X)));",
            "L.Add(X); exit(Format(L.IndexOf(X)));",
        ] {
            match json(ty, make, body) {
                Err(Signal::Unsupported(why)) => assert!(why.contains("not measured"), "{ty} {body}: {why}"),
                other => panic!("{ty} {body}: {other:?}"),
            }
        }
    }
}

// Nothing is compared on an empty list or dictionary: the answers stay.
#[test]
fn an_empty_json_list_or_dictionary_keeps_its_answers() {
    for (ty, make) in JSON_KINDS {
        let body = "exit(Format(L.Contains(X)) + '|' + Format(L.IndexOf(X)) + '|' + Format(L.Remove(X)) + '|' + Format(D.ContainsKey(X)));";
        assert_eq!(text(json(ty, make, body)), "No|0|No|No", "{ty}");
    }
}
