//! XmlElement.Create and XmlAttribute.Create with names that are not plain XML names (al-runner issue 7964 item 1; corpus PR
//! 1382, xml/XmlNameValidity.al:113-241, not pinned yet, assumption `xml-create-invalid-name`): the empty name raises the
//! Argument error, the measured invalid names the XmlException with its position. Valid names keep main's result; a name
//! outside the measured cells stays refused.

use al_builtins::assumptions::{Assumption, take_hits};

const SRC: &str = r#"
codeunit 69976 C
{
    [TryFunction]
    local procedure TryElement(Name: Text; var Result: Text)
    var
        E: XmlElement;
        S: Text;
    begin
        E := XmlElement.Create(Name);
        E.WriteTo(S);
        Result := 'ok name=' + E.Name() + ' local=' + E.LocalName() + ' ns=' + E.NamespaceUri() + ' xml=' + S;
    end;

    [TryFunction]
    local procedure TryElementNs(Name: Text; Ns: Text; var Result: Text)
    var
        E: XmlElement;
        S: Text;
    begin
        E := XmlElement.Create(Name, Ns);
        E.WriteTo(S);
        Result := 'ok name=' + E.Name() + ' local=' + E.LocalName() + ' ns=' + E.NamespaceUri() + ' xml=' + S;
    end;

    [TryFunction]
    local procedure TryAttribute(Name: Text; var Result: Text)
    var
        A: XmlAttribute;
        S: Text;
    begin
        A := XmlAttribute.Create(Name, 'v');
        A.WriteTo(S);
        Result := 'ok name=' + A.Name() + ' local=' + A.LocalName() + ' ns=' + A.NamespaceUri() + ' xml=' + S;
    end;

    [TryFunction]
    local procedure TryAttributeNs(Name: Text; Ns: Text; var Result: Text)
    var
        A: XmlAttribute;
        S: Text;
    begin
        A := XmlAttribute.Create(Name, Ns, 'v');
        A.WriteTo(S);
        Result := 'ok name=' + A.Name() + ' local=' + A.LocalName() + ' ns=' + A.NamespaceUri() + ' xml=' + S;
    end;

    procedure Run(Kind: Integer; Name: Text; Ns: Text): Text
    var
        R: Text;
        Ok: Boolean;
    begin
        ClearLastError();
        case Kind of
            1: Ok := TryElement(Name, R);
            2: Ok := TryElementNs(Name, Ns, R);
            3: Ok := TryAttribute(Name, R);
            4: Ok := TryAttributeNs(Name, Ns, R);
        end;
        if Ok then
            exit(R);
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
}
"#;

const ARGUMENT: &str = "Argument|Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLArgumentException' was thrown.";

fn xml(message: &str, position: u32) -> String {
    format!(
        "Xml:Xml|The following exception was encountered when processing XML data: '{message}' at line 0 and position {position}."
    )
}

fn begin(c: char, hex: &str) -> String {
    xml(&format!("Name cannot begin with the '{c}' character, hexadecimal value {hex}."), 1)
}

fn included(c: char, hex: &str, position: u32) -> String {
    xml(&format!("The '{c}' character, hexadecimal value {hex}, cannot be included in a name."), position)
}

fn run(b: &mut al_testkit::Built, kind: i32, name: &str, ns: &str) -> Result<String, String> {
    use al_builtins::value::Value;
    let (oi, ri) = b.routine_in("C", "Run");
    match b.call(oi, ri, vec![Value::Integer(kind), Value::Text(name.into()), Value::Text(ns.into())]) {
        Ok((Value::Text(t), _)) => Ok(t.to_string()),
        other => Err(format!("{other:?}")),
    }
}

#[test]
fn the_measured_names_raise_the_measured_errors_flagged() {
    let mut b = al_testkit::Built::from_source(SRC);
    let colon4 = included(':', "0x3A", 4);
    let colon2 = included(':', "0x3A", 2);
    let space5 = included(' ', "0x20", 5);
    let one = begin('1', "0x31");
    let nine = begin('9', "0x39");
    let cases: Vec<(i32, &str, &str, String)> = vec![
        // :113 Element_Create_IssueNames
        (1, "", "", ARGUMENT.into()),
        (1, "1.0", "", one.clone()),
        (1, "urn:Example-Namespace:TEST", "", colon4.clone()),
        // :130 Element_Create_NeighborNames
        (1, "9abc", "", nine.clone()),
        (1, "p:name", "", colon2.clone()),
        (1, "name with space", "", space5.clone()),
        // :145 Element_Create_ColonAndDotNames
        (1, ".a", "", begin('.', "0x2E")),
        (1, ":a", "", begin(':', "0x3A")),
        (1, "a:", "", colon2.clone()),
        // :162 Element_Create_WithNamespace
        (2, "", "urn:x", ARGUMENT.into()),
        (2, "1.0", "urn:x", one.clone()),
        (2, "urn:Example-Namespace:TEST", "urn:x", colon4.clone()),
        // :177 Element_Create_WithNamespace_Prefix
        (2, "p:name", "urn:x", colon2.clone()),
        (2, "p:name", "", colon2.clone()),
        (2, "name with space", "urn:x", space5.clone()),
        // :194 Attribute_Create_Names, :209 Attribute_Create_NeighborNames
        (3, "", "", ARGUMENT.into()),
        (3, "1.0", "", one.clone()),
        (3, "urn:Example-Namespace:TEST", "", colon4.clone()),
        (3, "9abc", "", nine),
        (3, "p:name", "", colon2.clone()),
        (3, "name with space", "", space5.clone()),
        // :226 Attribute_Create_WithNamespace, :241 Attribute_Create_WithNamespace_Prefix
        (4, "", "urn:x", ARGUMENT.into()),
        (4, "1.0", "urn:x", one),
        (4, "urn:Example-Namespace:TEST", "urn:x", colon4),
        (4, "p:name", "urn:x", colon2),
        (4, "name with space", "urn:x", space5),
    ];
    for (kind, name, ns, expected) in cases {
        take_hits();
        assert_eq!(run(&mut b, kind, name, ns), Ok(expected), "{kind} {name:?} {ns:?}");
        assert!(take_hits().contains(&Assumption::XmlCreateInvalidName), "{kind} {name:?} {ns:?} must mark");
    }
}

#[test]
fn valid_names_keep_mains_result_unflagged() {
    let mut b = al_testkit::Built::from_source(SRC);
    for (kind, name, ns, expected) in [
        (1, "name", "", "ok name=name local=name ns= xml=<name />"),
        (1, "a.b-c_1", "", "ok name=a.b-c_1 local=a.b-c_1 ns= xml=<a.b-c_1 />"),
        (2, "name", "urn:x", "ok name=:name local=name ns=urn:x xml=<name xmlns=\"urn:x\" />"),
        (3, "name", "", "ok name=name local=name ns= xml=name=\"v\""),
        (4, "name", "urn:x", "ok name=p0:name local=name ns=urn:x xml=p0:name=\"v\""),
    ] {
        take_hits();
        assert_eq!(run(&mut b, kind, name, ns).as_deref(), Ok(expected), "{kind} {name:?}");
        assert!(!take_hits().contains(&Assumption::XmlCreateInvalidName), "{kind} {name:?}");
    }
}

#[test]
fn names_outside_the_measured_cells_stay_refused() {
    let mut b = al_testkit::Built::from_source(SRC);
    for (kind, name, ns) in [
        // A first character the probe did not try, or tried only at another entry point.
        (1, "-a", ""),
        (2, "9abc", "urn:x"),
        (3, ".a", ""),
        (4, ":a", "urn:x"),
        // A bad character after something other than ASCII letters, or one the probe did not try.
        (1, "a1:b", ""),
        (1, "a/b", ""),
        // A namespace the probe did not use.
        (2, "1.0", "urn:other"),
        (4, "1.0", ""),
    ] {
        let r = run(&mut b, kind, name, ns);
        assert!(r.as_ref().is_err_and(|e| e.contains("Unsupported")), "{kind} {name:?} {ns:?}: {r:?}");
    }
}
