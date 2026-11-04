//! A field element with an empty value and a text element below it writes no text of its own: the child is indented as element
//! content (corpus streams/AtXmlPortFieldElementEdges.al:162 AT_EmptyFieldElement_ChildText_Export, :178
//! AT_EmptyFieldElement_EmptyChild_Export).

use al_testkit::{run, text};

fn src(child: &str) -> String {
    format!(
        r#"
table 69940 "XE Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69941 "XE Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69942 "XE Port"
{{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Row; "XE Row")
            {{
                fieldelement(Id; Row.Id) {{ }}
                fieldelement(Name; Row.Name) {{ textelement(Child) {{ }} }}
            }}
        }}
    }}
    trigger OnPreXmlPort() begin Child := '{child}'; end;
}}
codeunit 69943 C
{{
    procedure F(): Text
    var B: Record "XE Blob" temporary; R: Record "XE Row"; Xp: XmlPort "XE Port"; OutS: OutStream; InS: InStream; L: Text; T: Text;
    begin
        R.Id := 1; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(L); T += L + '|'; end;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn an_empty_field_element_with_a_text_child_writes_only_the_child() {
    let head =
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Row>|    <Id>1</Id>|    <Name>|";
    assert_eq!(
        text(run(&src("c1"), "F")),
        format!("{head}      <Child>c1</Child>|    </Name>|  </Row>|</Root>|")
    );
    assert_eq!(
        text(run(&src(""), "F")),
        format!("{head}      <Child />|    </Name>|  </Row>|</Root>|")
    );
}
