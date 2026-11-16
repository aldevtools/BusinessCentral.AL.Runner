//! al-runner issue 8165: `Contains`, `IndexOf`, `LastIndexOf` and `Remove` on a list of XML values are not measured:
//! they refuse when the searched value or an element is an XML value. Lists of other types keep their answers.

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50332 T
{
    procedure Node(Member: Integer): Text
    var
        L: List of [XmlNode];
        D: XmlDocument;
        E: XmlElement;
        N: XmlNode;
    begin
        E := XmlElement.Create('a');
        D := XmlDocument.Create();
        D.Add(E);
        N := E.AsXmlNode();
        L.Add(N);
        exit(Search(L, N, Member));
    end;

    procedure UnassignedNode(Member: Integer): Text
    var
        L: List of [XmlNode];
        N: XmlNode;
    begin
        L.Add(N);
        exit(Search(L, N, Member));
    end;

    procedure EmptyList(Member: Integer): Text
    var
        L: List of [XmlNode];
        N: XmlNode;
    begin
        N := XmlElement.Create('a').AsXmlNode();
        exit(Search(L, N, Member));
    end;

    procedure Element(Member: Integer): Text
    var
        L: List of [XmlElement];
        E: XmlElement;
    begin
        E := XmlElement.Create('a');
        L.Add(E);
        case Member of
            1: exit(Format(L.Contains(E)));
            2: exit(Format(L.IndexOf(E)));
            3: exit(Format(L.LastIndexOf(E)));
            4: exit(Format(L.Remove(E)));
        end;
    end;

    procedure Document(Member: Integer): Text
    var
        L: List of [XmlDocument];
        D: XmlDocument;
    begin
        D := XmlDocument.Create();
        L.Add(D);
        case Member of
            1: exit(Format(L.Contains(D)));
            2: exit(Format(L.IndexOf(D)));
            3: exit(Format(L.LastIndexOf(D)));
            4: exit(Format(L.Remove(D)));
        end;
    end;

    procedure Attribute(Member: Integer): Text
    var
        L: List of [XmlAttribute];
        A: XmlAttribute;
    begin
        A := XmlAttribute.Create('x', 'y');
        L.Add(A);
        case Member of
            1: exit(Format(L.Contains(A)));
            2: exit(Format(L.IndexOf(A)));
            3: exit(Format(L.LastIndexOf(A)));
            4: exit(Format(L.Remove(A)));
        end;
    end;

    local procedure Search(var L: List of [XmlNode]; N: XmlNode; Member: Integer): Text
    begin
        case Member of
            1: exit(Format(L.Contains(N)));
            2: exit(Format(L.IndexOf(N)));
            3: exit(Format(L.LastIndexOf(N)));
            4: exit(Format(L.Remove(N)));
        end;
    end;

    procedure Texts(): Text
    var
        L: List of [Text];
    begin
        L.Add('a');
        L.Add('b');
        L.Add('a');
        exit(Format(L.Contains('a')) + '|' + Format(L.IndexOf('b')) + '|' + Format(L.LastIndexOf('a')) + '|' + Format(L.Remove('a')) + '|' + Format(L.Contains('c')) + '|' + Format(L.Count()));
    end;

    procedure Integers(): Text
    var
        L: List of [Integer];
    begin
        L.Add(1);
        L.Add(2);
        exit(Format(L.Contains(2)) + '|' + Format(L.IndexOf(2)) + '|' + Format(L.LastIndexOf(3)) + '|' + Format(L.Remove(1)) + '|' + Format(L.Count()));
    end;

    procedure JsonObjects(): Text
    var
        L: List of [JsonObject];
        J: JsonObject;
    begin
        L.Add(J);
        exit(Format(L.Contains(J)) + '|' + Format(L.IndexOf(J)));
    end;
}
"#;

fn refused(routine: &str, member: i32) -> String {
    match al_testkit::run_with(SRC, "T", routine, vec![al_builtins::value::Value::Integer(member)]) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine} {member}: {other:?}"),
    }
}

#[test]
fn a_search_of_a_list_of_xml_values_refuses() {
    // An empty list too (main answered No / 0 there): every shape is refused until corpus PR 1422 is pinned.
    for routine in [
        "Node",
        "UnassignedNode",
        "EmptyList",
        "Element",
        "Document",
        "Attribute",
    ] {
        for (member, name) in [(1, "Contains"), (2, "IndexOf"), (3, "LastIndexOf"), (4, "Remove")] {
            let why = refused(routine, member);
            assert!(
                why.contains(&format!("List.{name}")) && why.contains("XML"),
                "{routine}: {why}"
            );
        }
    }
}

#[test]
fn a_search_of_a_list_of_other_values_keeps_its_answer() {
    assert_eq!(text(run_in(SRC, "T", "Texts")), "Yes|2|3|Yes|No|2");
    assert_eq!(text(run_in(SRC, "T", "Integers")), "Yes|2|0|Yes|1");
    assert_eq!(text(run_in(SRC, "T", "JsonObjects")), "Yes|1");
}
