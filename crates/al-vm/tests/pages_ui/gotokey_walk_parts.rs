//! GoToKey on a Document host walks the host rows from the first one; its part reads its rows for each host row passed whose
//! lines it does not already show, so a GoToKey onto the row the host stands on reads the part for the first row and then for
//! the target again (pages/parts/QjPartPropagation.al:1183 QL_Both_GoToKey_Counts, :1210
//! QL_Default_GoToKey_Counts `log-gotokey-h2-again`, corpus bump 42).

const SRC: &str = r#"
table 51500 "GW Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 51501 "GW Line" { fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } } keys { key(PK; "Doc No.", "Line No.") { } } }
codeunit 51504 "GW Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ' '; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end;
}
page 51502 "GW Lines"
{
    PageType = ListPart;
    SourceTable = "GW Line";
    layout { area(Content) { repeater(R) { field(LineNoF; Rec."Line No.") { } } } }
    var L: Codeunit "GW Log";
    trigger OnAfterGetCurrRecord() begin L.Add('L.AGCR(' + Format(Rec."Line No.") + ')'); end;
}
page 51503 "GW Host"
{
    PageType = Document;
    SourceTable = "GW Head";
    layout { area(Content) { field(NoF; Rec."No.") { } part(Lines; "GW Lines") { SubPageLink = "Doc No." = field("No."); } } }
    var L: Codeunit "GW Log";
    trigger OnAfterGetCurrRecord() begin L.Add('H.AGCR(' + Rec."No." + ')'); end;
}
codeunit 51505 "GW Test"
{
    var L: Codeunit "GW Log";
    procedure Run(): Text
    var H: Record "GW Head"; Ln: Record "GW Line"; P: TestPage "GW Host"; T: Text;
    begin
        H."No." := 'H1'; H.Insert(); H."No." := 'H2'; H.Insert();
        Ln."Doc No." := 'H1'; Ln."Line No." := 10000; Ln.Insert();
        Ln."Doc No." := 'H2'; Ln."Line No." := 30000; Ln.Insert();
        P.OpenEdit();
        P.GoToKey('H2');
        L.Take();
        P.GoToKey('H2');
        exit(L.Take());
    end;
}
"#;

#[test]
fn gotokey_onto_the_current_row_of_a_document_host_reads_the_part_for_each_row_it_passes() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "GW Test", "Run"));
    assert_eq!(got, "H.AGCR(H1) L.AGCR(10000) H.AGCR(H2) L.AGCR(30000)");
}
