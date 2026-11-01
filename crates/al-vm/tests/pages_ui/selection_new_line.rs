//! CurrPage.SetSelectionFilter in OnAfterGetCurrRecord while an editable list stands on its new line (pages/pageops.rs,
//! pages/page_var_view.rs). Each test names the corpus test of pages/lt-y61-page-probes/Y61SelectionComposite.al that pins it.

const SRC: &str = r#"
table 69990 "SN Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69991 "SN Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "SN Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "SN Log"; R: Text;
    begin
        if L.FindSet() then
            repeat
                if R <> '' then R += ';';
                R += L.Txt;
            until L.Next() = 0;
        L.DeleteAll();
        exit(R);
    end;
}
table 69992 "SN Row" { fields { field(1; Id; Integer) { } field(2; "File Name"; Text[30]) { } field(3; Grp; Code[10]) { } } keys { key(PK; Id) { } } }
page 69993 "SN List"
{
    PageType = List; SourceTable = "SN Row"; DelayedInsert = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec."File Name") { } field(GrpCtl; Rec.Grp) { } } } }
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "SN Log Mgt"; begin Rec."File Name" := 'New'; L.Add('new'); end;
    trigger OnAfterGetCurrRecord()
    var L: Codeunit "SN Log Mgt"; Sel: Record "SN Row";
    begin
        CurrPage.SetSelectionFilter(Sel);
        L.Add('agcr id=' + Format(Rec.Id) + ' name=' + Rec."File Name" + ' count=' + Format(Sel.Count()) + ' filters=' + Sel.GetFilters());
    end;
    procedure Fill(G: Code[10]) begin Rec.SetRange(Grp, G); end;
}
codeunit 69994 C
{
    local procedure Seed()
    var Row: Record "SN Row"; i: Integer; L: Codeunit "SN Log Mgt";
    begin
        for i := 1 to 3 do begin
            Row.Init(); Row.Id := i; Row."File Name" := 'f' + Format(i); Row.Grp := 'X';
            if i = 3 then Row.Grp := 'Y';
            Row.Insert();
        end;
        L.Take();
    end;

    procedure NoRow(): Text
    var Pg: Page "SN List"; TP: TestPage "SN List"; L: Codeunit "SN Log Mgt"; Row: Record "SN Row"; R: Text;
    begin
        Seed();
        Pg.Fill('Z');
        TP.Trap();
        Pg.Run();
        R := L.Take() + '|' + TP.IdCtl.Value() + '|' + TP.NameCtl.Value();
        TP.Close();
        exit(R + '|' + L.Take() + '|' + Format(Row.Count()));
    end;

    procedure Rows(): Text
    var Pg: Page "SN List"; TP: TestPage "SN List"; L: Codeunit "SN Log Mgt"; Row: Record "SN Row"; R: Text;
    begin
        Seed();
        Pg.Fill('X');
        TP.Trap();
        Pg.Run();
        R := L.Take();
        TP.Next();
        R += '|' + L.Take();
        R += '|' + Format(TP.Next()) + '|' + L.Take() + '|' + TP.IdCtl.Value() + '|' + TP.NameCtl.Value();
        TP.Close();
        exit(R + '|' + L.Take() + '|' + Format(Row.Count()));
    end;
}
"#;

const NEW: &str = "new;agcr id=0 name=New count=0 filters=Id: 0";

fn run(proc_name: &str) -> Vec<String> {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", proc_name));
    out.split('|').map(str::to_string).collect()
}

/// Y61SelectionComposite.al:391 G612_EditableList_NewLine_OnNewRecordFills_NoRow: the list filtered to no row opens on its new line,
/// OnNewRecord and OnAfterGetCurrRecord run twice, the selection is the blank key (`Id: 0`, count 0) though OnNewRecord filled a field.
#[test]
fn the_new_line_of_a_list_opened_on_no_row_selects_the_blank_key() {
    let out = run("NoRow");
    assert_eq!(out, [&format!("{NEW};{NEW}"), "0", "New", "", "3"]);
}

/// Y61SelectionComposite.al:416 G612_EditableList_NewLine_OnNewRecordFills_Rows: Next past the last row lands on the new line.
#[test]
fn next_onto_the_new_line_selects_the_blank_key() {
    let out = run("Rows");
    assert_eq!(
        out,
        [
            "agcr id=1 name=f1 count=1 filters=Id: 1",
            "agcr id=2 name=f2 count=1 filters=Id: 2",
            "Yes",
            NEW,
            "0",
            "New",
            "",
            "3"
        ]
    );
}
