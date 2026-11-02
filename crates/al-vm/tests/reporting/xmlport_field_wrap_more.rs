//! A text element right below a table element that holds field elements, text elements of only text attributes, a text element
//! that wraps a table element and a table element directly (corpus xmlport/lt-y67-xmlport-wrapper-in-table/Y67XpWrapperInTable.al:
//! 533-604). The inner tables have no link: every outer row lists all their rows; a wrapped table without rows leaves its
//! wrapper empty, a direct one writes nothing. Skip in the element's OnBeforePassVariable leaves it out with all below it.

fn src(mode: i32, lines: bool, dets: bool) -> String {
    format!(
        r#"
table 69900 "XW Hdr" {{ fields {{ field(1; No; Code[20]) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; No) {{ }} }} }}
table 69901 "XW Line" {{ fields {{ field(1; LineNo; Integer) {{ }} field(2; Item; Text[30]) {{ }} }} keys {{ key(PK; LineNo) {{ }} }} }}
table 69902 "XW Det" {{ fields {{ field(1; DetNo; Integer) {{ }} }} keys {{ key(PK; DetNo) {{ }} }} }}
table 69903 "XW Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 69904 "XW Log" {{ SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E + ';'; end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }}
xmlport 69905 "XW Port"
{{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Resp; "XW Hdr")
            {{
                MinOccurs = Zero;
                textelement(status) {{ MaxOccurs = Once; trigger OnBeforePassVariable() begin Log.Add('status'); status := 'ok'; end; }}
                textelement(membership)
                {{
                    MaxOccurs = Once; MinOccurs = Zero;
                    fieldelement(code; Resp.No) {{ textattribute(cn) {{ trigger OnBeforePassVariable() begin Log.Add('cn'); cn := 'N-' + Resp.No; end; }} }}
                    textelement(loyalty)
                    {{
                        MaxOccurs = Once;
                        textattribute(ln) {{ trigger OnBeforePassVariable() begin Log.Add('ln'); ln := 'L'; end; }}
                        trigger OnBeforePassVariable() begin Log.Add('loyalty'); loyalty := 'lv'; end;
                    }}
                    textelement(attributes)
                    {{
                        MaxOccurs = Once; MinOccurs = Zero;
                        tableelement(Attr; "XW Det") {{ MinOccurs = Zero; fieldattribute(dno; Attr.DetNo) {{ }} trigger OnAfterGetRecord() begin Log.Add('attr ' + Format(Attr.DetNo)); end; }}
                        trigger OnBeforePassVariable() begin Log.Add('attributes'); end;
                    }}
                    tableelement(Entry; "XW Line")
                    {{
                        MinOccurs = Zero;
                        fieldelement(item; Entry.Item) {{ }}
                        trigger OnAfterGetRecord() begin Log.Add('entry ' + Format(Entry.LineNo)); end;
                    }}
                    trigger OnBeforePassVariable() begin Log.Add('membership'); if Mode = 1 then currXMLport.Skip(); end;
                }}
                trigger OnAfterGetRecord() begin Log.Add('resp ' + Resp.No); end;
            }}
        }}
    }}
    var Log: Codeunit "XW Log"; Mode: Integer;
    procedure SetMode(M: Integer) begin Mode := M; end;
}}
codeunit 69906 C
{{
    procedure Export(): Text
    var H: Record "XW Hdr"; Ln: Record "XW Line"; D: Record "XW Det"; Xp: XmlPort "XW Port"; B: Record "XW Blob" temporary; OutS: OutStream; InS: InStream; Line: Text; R: Text; L: Codeunit "XW Log";
    begin
        H.No := 'H1'; H.Name := 'n1'; H.Insert(); H.No := 'H2'; H.Insert();
        if {lines} then begin Ln.LineNo := 10; Ln.Item := 'x'; Ln.Insert(); Ln.LineNo := 20; Ln.Item := 'y'; Ln.Insert(); end;
        if {dets} then begin D.DetNo := 1; D.Insert(); end;
        Xp.SetMode({mode});
        B.Data.CreateOutStream(OutS);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        InS.ReadText(Line);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line.Trim(); end;
        exit(R + ' ' + L.Take());
    end;
}}
"#
    )
}

fn export(mode: i32, lines: bool, dets: bool) -> String {
    al_testkit::text(al_testkit::run_in(&src(mode, lines, dets), "C", "Export"))
}

const ROW: &str = "<code cn=\"N-H?\">H?</code><loyalty ln=\"L\">lv</loyalty>";

#[test]
fn every_outer_row_lists_all_inner_rows() {
    // Temp_AllRows_Export, Real_AllRows_Export.
    let m = |h: &str| {
        format!(
            "<Resp><status>ok</status><membership>{}<attributes><Attr dno=\"1\" /></attributes><Entry><item>x</item></Entry><Entry><item>y</item></Entry></membership></Resp>",
            ROW.replace('?', h)
        )
    };
    let log = |h: &str| format!("resp H{h};status;membership;cn;loyalty;ln;attributes;attr 1;entry 10;entry 20;");
    assert_eq!(
        export(0, true, true),
        format!("<Root>{}{}</Root> {}{}", m("1"), m("2"), log("1"), log("2"))
    );
}

#[test]
fn inner_tables_without_rows_and_skip_of_the_element() {
    // Temp_MissingInnerRows_Export 'neither': `<attributes />`, no Entry; Temp_NoOuterRow_And_SkipWrapper_Export 'skip wrapper'.
    let neither = export(0, false, false);
    assert!(
        neither.starts_with(&format!(
            "<Root><Resp><status>ok</status><membership>{}<attributes /></membership></Resp>",
            ROW.replace('?', "1")
        )),
        "{neither}"
    );
    assert_eq!(
        export(1, true, true),
        "<Root><Resp><status>ok</status></Resp><Resp><status>ok</status></Resp></Root> resp H1;status;membership;resp H2;status;membership;"
    );
}
