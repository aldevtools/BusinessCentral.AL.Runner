//! ProviderPartOutsideFactBoxes.al:419-421, :482-506: an empty Provider does not select a blank-key detail;
//! a new host with a base FactBox chain reads blank parts and stores the host after its key is typed.

use al_testkit::{run, text};

const SOURCE: &str = r#"
table 69970 "PF Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 69971 "PF Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Doc No.", "Line No.") { } }
}
table 69972 "PF Detail"
{
    fields { field(1; "Line Ref"; Integer) { } field(2; Note; Text[30]) { } }
    keys { key(PK; "Line Ref") { } }
}
codeunit 69973 "PF Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Value: Text) begin Entries += Value + '|'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Clear(Entries); exit(Result); end;
}
page 69974 "PF Lines"
{
    PageType = ListPart; SourceTable = "PF Line";
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnAfterGetCurrRecord() var Log: Codeunit "PF Log"; begin Log.Add('line=' + Format(Rec."Line No.")); end;
}
page 69975 "PF Details"
{
    PageType = CardPart; SourceTable = "PF Detail";
    layout { area(Content) { field(Note; Rec.Note) { } } }
    trigger OnAfterGetCurrRecord() var Log: Codeunit "PF Log"; begin Log.Add('detail=' + Rec.Note); end;
}
page 69976 "PF Host"
{
    PageType = Card; SourceTable = "PF Head";
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes)
        {
            part(Lines; "PF Lines") { SubPageLink = "Doc No." = field("No."); }
            part(Detail; "PF Details") { Provider = Lines; SubPageLink = "Line Ref" = field("Line No."); }
        }
    }
}
codeunit 69977 C
{
    local procedure Seed()
    var Head: Record "PF Head"; Line: Record "PF Line"; Detail: Record "PF Detail"; Log: Codeunit "PF Log";
    begin
        Head."No." := 'H1'; Head.Insert(); Head."No." := 'H2'; Head.Insert();
        Line."Doc No." := 'H1'; Line."Line No." := 10; Line.Insert();
        Detail."Line Ref" := 10; Detail.Note := 'ten'; Detail.Insert();
        Detail."Line Ref" := 0; Detail.Note := 'wrong blank match'; Detail.Insert();
        Log.Take();
    end;
    procedure EmptyProvider(): Text
    var P: TestPage "PF Host"; Result: Text;
    begin
        Seed(); P.OpenView(); P.GoToKey('H2');
        Result := P.Detail.Note.Value;
        if P.Detail.First() then Error('Empty provider exposed a stored detail.');
        P.GoToKey('H1'); Result += '/' + P.Detail.Note.Value;
        P.Close(); exit(Result);
    end;
    procedure NewHost(): Text
    var P: TestPage "PF Host"; Head: Record "PF Head"; Log: Codeunit "PF Log"; Result: Text;
    begin
        Seed(); P.OpenNew();
        Result := P.Detail.Note.Value + ':' + Log.Take();
        P."No.".SetValue('H3'); Result += ':' + Log.Take();
        P.Close();
        if not Head.Get('H3') then Error('The new host was not stored.');
        exit(Result);
    end;
}
"#;

#[test]
fn an_empty_provider_excludes_the_blank_key_detail_and_recovers_on_a_real_row() {
    assert_eq!(text(run(SOURCE, "EmptyProvider")), "/ten");
}

#[test]
fn a_new_factbox_chain_reads_blank_once_and_stores_the_typed_host() {
    assert_eq!(text(run(SOURCE, "NewHost")), ":line=0|detail=|:line=0|");
}

#[test]
fn an_extension_linked_listpart_does_not_match_a_blank_key_detail() {
    let src = SOURCE
        .replace("area(FactBoxes)", "area(Content)")
        .replace("PageType = CardPart", "PageType = ListPart")
        .replace(
            "layout { area(Content) { field(Note; Rec.Note) { } } }",
            "layout { area(Content) { repeater(R) { field(Note; Rec.Note) { } } } }",
        )
        .replace(
            "trigger OnAfterGetCurrRecord() var Log: Codeunit \"PF Log\"; begin Log.Add('line=' + Format(Rec.\"Line No.\")); end;",
            "",
        )
        .replace(
            "trigger OnAfterGetCurrRecord() var Log: Codeunit \"PF Log\"; begin Log.Add('detail=' + Rec.Note); end;",
            "",
        )
        .replace(
            "part(Detail; \"PF Details\") { Provider = Lines; SubPageLink = \"Line Ref\" = field(\"Line No.\"); }",
            "",
        )
        .replace(
            "var P: TestPage \"PF Host\"; Result: Text;",
            "var P: TestPage \"PF Host\"; Result: Text; Head: Record \"PF Head\";",
        )
        .replace("P.GoToKey('H2');", "Head.Get('H2'); P.GoToRecord(Head);");
    let src = format!(
        "{src}\npageextension 69978 \"PF Host Ext\" extends \"PF Host\" {{ layout {{ addafter(Lines) {{ \
         part(Detail; \"PF Details\") {{ Provider = Lines; SubPageLink = \"Line Ref\" = field(\"Line No.\"); }} }} }} }}"
    );
    assert_eq!(text(run(&src, "EmptyProvider")), "/ten");
}
