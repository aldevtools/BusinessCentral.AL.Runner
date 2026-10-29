//! The key of a table element with LinkTable and no LinkFields below another table element: every pass starts from the
//! SourceTableView key, or the primary key without a view, ascending; a key OnPreXmlItem sets at one pass is gone at the next
//! (corpus writer TU's branch al-runtime-rs-staging-tu, xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al
//! TU_LinkedKey_*, xmlports 90126/90127). A key or direction the code set before the first pass is not pinned.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(mode: i32) -> String {
    format!(
        r#"
table 69970 "LK Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69971 "LK Parent" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69972 "LK Child" {{ fields {{ field(1; Id; Integer) {{ }} field(2; A; Integer) {{ }} field(3; B; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} key(KA; A) {{ }} key(KB; B) {{ }} }} }}
xmlport 69973 "LK Port"
{{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(root)
        {{
            tableelement(par; "LK Parent")
            {{
                XmlName = 'p'; UseTemporary = true;
                fieldattribute(id; par.Id) {{ }}
                trigger OnAfterGetRecord()
                begin
                    if Mode = 2 then c.SetCurrentKey(B);
                end;
                tableelement(c; "LK Child")
                {{
                    XmlName = 'c'; LinkTable = par; MinOccurs = Zero; SourceTableView = sorting(A);
                    fieldattribute(id; c.Id) {{ }}
                    trigger OnPreXmlItem()
                    begin
                        Pass += 1;
                        if (Mode = 1) and (Pass = 1) then c.SetCurrentKey(B);
                    end;
                }}
            }}
        }}
    }}
    var Mode: Integer; Pass: Integer;
    procedure KeyB(DoIt: Boolean)
    begin
        if DoIt then c.SetCurrentKey(B);
    end;
    procedure Setup(M: Integer)
    var I: Integer;
    begin
        Mode := M;
        for I := 1 to 2 do begin par.Id := I; par.Insert(); end;
    end;
}}
codeunit 69974 C
{{
    procedure Export(): Text
    var B: Record "LK Blob" temporary; R: Record "LK Child"; P: XmlPort "LK Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.A := 3; R.B := 2; R.Insert();
        R.Id := 2; R.A := 1; R.B := 3; R.Insert();
        R.Id := 3; R.A := 2; R.B := 1; R.Insert();
        P.Setup({mode});
        B.Data.CreateOutStream(OutS);
        P.SetDestination(OutS);
        P.Export();
        if {mode} >= 3 then begin
            P.KeyB({mode} = 4);
            B.Data.CreateOutStream(OutS);
            P.SetDestination(OutS);
            P.Export();
        end;
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn every_pass_starts_from_the_view_key_again() {
    // By A: 2,3,1; by B: 3,1,2. Pass 1 sets key B, pass 2 is back to the view key (TU_LinkedKey_KeyAtFirstPass).
    let out = al_testkit::text(run(&src(1), "Export"));
    assert!(
        out.ends_with(
            "<root><p id=\"1\"><c id=\"3\" /><c id=\"1\" /><c id=\"2\" /></p><p id=\"2\"><c id=\"2\" /><c id=\"3\" /><c id=\"1\" /></p></root>"
        ),
        "{out}"
    );
}

#[test]
fn a_key_set_before_the_first_pass_is_refused() {
    match run(&src(2), "Export") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a key or direction other than"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_later_export_runs_while_the_key_is_still_the_views_and_refuses_another_key() {
    // A later export keeps the record's key (TS_Again_KeyBetween); TU pins the passes only from the view's key.
    let out = al_testkit::text(run(&src(3), "Export"));
    assert!(out.ends_with("<root><p id=\"1\"><c id=\"2\" /><c id=\"3\" /><c id=\"1\" /></p><p id=\"2\"><c id=\"2\" /><c id=\"3\" /><c id=\"1\" /></p></root>"), "{out}");
    match run(&src(4), "Export") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a key or direction other than"), "{why}"),
        other => panic!("{other:?}"),
    }
}
