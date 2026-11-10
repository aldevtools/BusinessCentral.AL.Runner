//! A part a Visible expression hid, with no Provider link of its own, on a host page that has a Provider chain elsewhere (issue 6924,
//! a Data Check factbox on Sales Credit Memo). Its triggers do not run when a key is typed on the new host row: the corpus pins
//! hidden parts that take part in a Provider chain (testpage/parts/ProviderLinkedPartsMatrix.al Provider_BothHidden_OpenViewAndGoToKey),
//! and no test pins a hidden part outside every chain on such a sync. Probe pending: probe-hidden-part-plain-sync.

const SRC: &str = r#"
table 69840 "HPS Doc"
{
    fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 69841 "HPS Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Doc No.", "Line No.") { Clustered = true; } }
}
codeunit 69842 "HPS Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
page 69843 "HPS Lines"
{
    PageType = ListPart; SourceTable = "HPS Line";
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
}
page 69844 "HPS Detail"
{
    PageType = ListPart; SourceTable = "HPS Line";
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { ApplicationArea = All; } } } }
}
page 69845 "HPS Factbox"
{
    PageType = CardPart; SourceTable = "HPS Doc";
    layout { area(Content) { field("No."; Rec."No.") { ApplicationArea = All; } } }
    trigger OnAfterGetCurrRecord() var Log: Codeunit "HPS Log"; begin Log.Add('fb(' + Rec."No." + ')'); end;
}
page 69846 "HPS Card"
{
    PageType = Card; SourceTable = "HPS Doc"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "HPS Lines") { ApplicationArea = All; SubPageLink = "Doc No." = field("No."); }
        }
        area(FactBoxes)
        {
            part(Detail; "HPS Detail") { ApplicationArea = All; Provider = Lines; SubPageLink = "Line No." = field("Line No."); }
            part(Fb; "HPS Factbox") { ApplicationArea = All; Visible = FbVisible; SubPageLink = "No." = field("No."); }
        }
    }
    var FbVisible: Boolean;
}
codeunit 69847 C
{
    procedure NewKeyed(): Text
    var P: TestPage "HPS Card"; Log: Codeunit "HPS Log";
    begin
        Log.Take();
        P.OpenNew();
        P."No.".SetValue('D2');
        P.Close();
        exit(Log.Take());
    end;
}
"#;

#[test]
fn a_key_typed_on_a_new_host_runs_no_trigger_of_a_hidden_part_outside_the_chain() {
    let r = al_testkit::run_in(SRC, "C", "NewKeyed");
    // As on 4e20bf926: neither the open of the new host nor the key typed on it runs the hidden part's OnAfterGetCurrRecord.
    assert_eq!(al_testkit::text(r), "");
}
