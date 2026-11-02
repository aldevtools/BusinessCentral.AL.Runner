//! A text element with text attributes (and its own text) and OnBeforePassVariable, below container text elements that are not
//! inside a table element (corpus xmlport/lt-y67-xmlport-textattr-skip/Y67XpTextAttrSkip.al:298-374). Its trigger runs before
//! its attributes'; Skip there leaves out the element with its attributes, whose triggers do not run; Skip in an attribute
//! leaves out only the attribute; an empty text writes the element empty with its attributes. On import the attributes' triggers
//! run first, then the element's, then the containers' (children first).

fn src(mode: i32) -> String {
    format!(
        r#"
table 69984 "XA Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 69980 "XA Log" {{ SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E + ';'; end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }}
xmlport 69981 "XA Export"
{{
    Format = Xml; Direction = Export; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            textelement(Direct)
            {{
                textattribute(ds) {{ trigger OnBeforePassVariable() begin Log.Add('ds'); ds := 'D8'; if Mode = 3 then currXMLport.Skip(); end; }}
                trigger OnBeforePassVariable() begin Log.Add('direct'); Direct := 'dv'; if Mode = 2 then currXMLport.Skip(); end;
            }}
            textelement(Cont)
            {{
                textelement(EP)
                {{
                    textattribute(sc) {{ trigger OnBeforePassVariable() begin Log.Add('sc'); sc := '0088'; end; }}
                    trigger OnBeforePassVariable() begin Log.Add('ep'); if Mode = 4 then EP := '' else EP := 'e'; if Mode = 1 then currXMLport.Skip(); end;
                }}
                trigger OnBeforePassVariable() begin Log.Add('cont'); end;
            }}
        }}
    }}
    var Log: Codeunit "XA Log"; Mode: Integer;
    procedure SetMode(M: Integer) begin Mode := M; end;
}}
xmlport 69982 "XA Import"
{{
    Format = Xml; Direction = Import; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            textelement(Cont)
            {{
                MinOccurs = Zero;
                textelement(EP)
                {{
                    MinOccurs = Zero;
                    textattribute(sc) {{ Occurrence = Optional; trigger OnAfterAssignVariable() begin Log.Add('sc=' + sc); end; }}
                    trigger OnAfterAssignVariable() begin Log.Add('ep=' + EP); end;
                }}
                trigger OnAfterAssignVariable() begin Log.Add('cont=[' + Cont + ']'); end;
            }}
        }}
    }}
    var Log: Codeunit "XA Log";
}}
codeunit 69983 C
{{
    procedure Export(): Text
    var Xp: XmlPort "XA Export"; TempBlob: Record "XA Blob" temporary; OutS: OutStream; InS: InStream; Line: Text; R: Text; L: Codeunit "XA Log";
    begin
        Xp.SetMode({mode});
        TempBlob.Data.CreateOutStream(OutS);
        Xp.SetDestination(OutS);
        Xp.Export();
        TempBlob.Data.CreateInStream(InS, TextEncoding::UTF16);
        InS.ReadText(Line);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line.Trim(); end;
        exit(R + ' ' + L.Take());
    end;

    procedure Import(): Text
    var Xp: XmlPort "XA Import"; TempBlob: Record "XA Blob" temporary; OutS: OutStream; InS: InStream; L: Codeunit "XA Log";
    begin
        TempBlob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Cont><EP sc="9">v</EP></Cont></Root>');
        TempBlob.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xp.SetSource(InS);
        Xp.Import();
        exit(L.Take());
    end;
}}
"#
    )
}

fn export(mode: i32) -> String {
    al_testkit::text(al_testkit::run_in(&src(mode), "C", "Export"))
}

#[test]
fn the_element_trigger_runs_before_its_attributes_and_skip_leaves_it_out() {
    // m0, m1 (Skip in the leaf: `<Cont2 />`, no attribute trigger), m6 (Skip in Direct), m7 (Skip in its attribute), m3 (empty).
    assert_eq!(
        export(0),
        "<Root><Direct ds=\"D8\">dv</Direct><Cont><EP sc=\"0088\">e</EP></Cont></Root> direct;ds;cont;ep;sc;"
    );
    assert_eq!(
        export(1),
        "<Root><Direct ds=\"D8\">dv</Direct><Cont /></Root> direct;ds;cont;ep;"
    );
    assert_eq!(
        export(2),
        "<Root><Cont><EP sc=\"0088\">e</EP></Cont></Root> direct;cont;ep;sc;"
    );
    assert_eq!(
        export(3),
        "<Root><Direct>dv</Direct><Cont><EP sc=\"0088\">e</EP></Cont></Root> direct;ds;cont;ep;sc;"
    );
    assert_eq!(
        export(4),
        "<Root><Direct ds=\"D8\">dv</Direct><Cont><EP sc=\"0088\" /></Cont></Root> direct;ds;cont;ep;sc;"
    );
}

#[test]
fn the_import_runs_the_attributes_then_the_element_then_the_container() {
    // Import_TriggerOrder `full log`: `scheme=0088;sid=S1;ep=v;cont2=[];cont1=[];`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src(0), "C", "Import")),
        "sc=9;ep=v;cont=[];"
    );
}
