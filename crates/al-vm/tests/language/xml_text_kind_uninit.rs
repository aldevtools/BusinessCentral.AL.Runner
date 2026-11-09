//! An unassigned `XmlText`, `XmlCData`, `XmlComment` or `XmlProcessingInstruction` variable: the node-level methods raise
//! `NotInitialized` naming `XmlNode`, the type's own value methods name the type (xml/xml/P90CDataRead.al,
//! P90CommentRead.al, P90PIRead.al, P90TextRead.al: UninitialisedVariable_2, _3; xml/xml/P104TreeB{CData,Comment,PI,Text}.al
//! P104_Remove_Uninitialised). WriteTo of an unassigned `XmlNode` or `XmlDocumentType` names `XmlNode` too
//! (xml/xml/P104Write{Node,DocType,CData,Comment,PI,Text}.al P104_WriteTo_Uninitialised).

use al_testkit::text;

const TABLE: &str = r#"table 60010 "XU Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }"#;

fn errors(ty: &str, ops: &[&str]) -> String {
    let mut body = String::new();
    for op in ops {
        body.push_str(&format!(
            "ClearLastError(); asserterror begin {op}; Error('DONE'); end; R += GetLastErrorText() + '|';\n"
        ));
    }
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{ procedure F(): Text var U: {ty}; X: {ty}; N: XmlNode; Dt: XmlDocumentType; D: XmlDocument; P: XmlElement; E: XmlElement; L: XmlNodeList; S: Text; R: Text; B: Record \"XU Blob\" temporary; OutS: OutStream; Opt: XmlWriteOptions; begin E := XmlElement.Create('e'); B.Data.CreateOutStream(OutS); {body} exit(R); end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const NODE: &str = "Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.";

#[test]
fn node_level_methods_of_an_unassigned_text_kind_name_xml_node() {
    let ops = [
        "U.Remove()",
        "U.AddAfterSelf(E)",
        "U.AddBeforeSelf(E)",
        "U.GetDocument(D)",
        "U.GetParent(P)",
        "U.ReplaceWith(E)",
        "U.SelectNodes('.', L)",
        "U.SelectSingleNode('.', N)",
        "U.WriteTo(S)",
        "U.WriteTo(OutS)",
    ];
    for ty in ["XmlCData", "XmlComment", "XmlProcessingInstruction", "XmlText"] {
        assert_eq!(errors(ty, &ops), format!("{NODE}|").repeat(ops.len()), "{ty}");
    }
}

#[test]
fn the_value_methods_of_an_unassigned_text_kind_name_the_type() {
    for ty in ["XmlCData", "XmlComment", "XmlText"] {
        let want = format!("Microsoft.Dynamics.Nav.Runtime.Nav{ty} variable not initialized.|");
        assert_eq!(
            errors(ty, &["S := U.Value()", "X := U; X.Value('v')"]),
            want.repeat(2),
            "{ty}"
        );
    }
    let want = "Microsoft.Dynamics.Nav.Runtime.NavXmlProcessingInstruction variable not initialized.|";
    assert_eq!(
        errors("XmlProcessingInstruction", &["U.GetData(S)", "X := U; X.SetData('v')"]),
        want.repeat(2)
    );
}

#[test]
fn write_to_of_an_unassigned_node_or_document_type_names_xml_node() {
    let node = [
        "N.WriteTo(S)",
        "N.WriteTo(Opt, S)",
        "N.WriteTo(OutS)",
        "N.WriteTo(Opt, OutS)",
    ];
    assert_eq!(errors("XmlText", &node), format!("{NODE}|").repeat(4));
    let doc_type = ["Dt.WriteTo(OutS)", "Dt.WriteTo(Opt, OutS)"];
    assert_eq!(errors("XmlText", &doc_type), format!("{NODE}|").repeat(2));
}
