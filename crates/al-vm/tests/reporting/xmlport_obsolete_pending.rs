//! An XmlPort marked `ObsoleteState = Pending` (with ObsoleteReason and ObsoleteTag) still exports as before
//! (misc/lt-u-obsolete-pending/UObsoleteObjects.al:221-240, UObsoletePending.al:103-140 Pending_QueryReportXmlPort_StillRun).
//! Another ObsoleteState value is not pinned and stays refused.

use al_testkit::run;

fn src(state: &str) -> String {
    format!(
        r#"
table 69980 "XO Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69981 "XO Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69982 "XO Port"
{{
    ObsoleteState = {state};
    ObsoleteReason = 'The XmlPort is replaced.';
    ObsoleteTag = '1.0';
    Direction = Export;
    UseRequestPage = false;
    schema {{ textelement(Root) {{ tableelement(Src; "XO Row") {{ fieldelement(Id; Src.Id) {{ }} }} }} }}
}}
codeunit 69983 C
{{
    procedure Export(): Text
    var Row: Record "XO Row"; B: Record "XO Blob" temporary; OutS: OutStream; InS: InStream; T: Text; L: Text;
    begin
        Row.Id := 7; Row.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XO Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(L); T += L; end;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn an_obsolete_pending_xmlport_still_exports() {
    let out = al_testkit::text(run(&src("Pending"), "Export"));
    assert!(out.contains("<Id>7</Id>"), "{out}");
}

#[test]
fn another_obsolete_state_stays_refused() {
    assert!(matches!(
        run(&src("Moved"), "Export"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
