//! The refresh after `TestField.AssistEdit` (pages/lt-sn-assistedit-refresh/SnAssistEditRefresh.al, writer SN): on a card and on a
//! list whose row holds no edit, OnAfterGetRecord and OnAfterGetCurrRecord run again after the trigger, unless the trigger changed
//! Rec (`set = OnAssistEdit(Set)`).

use al_testkit::{run_in, text};

fn src(page_type: &str) -> String {
    format!(
        r#"
table 50400 R
{{
    fields {{ field(1; "Code"; Code[20]) {{ }} field(2; Name; Text[50]) {{ }} }}
    keys {{ key(PK; Code) {{ Clustered = true; }} }}
}}
codeunit 50401 L
{{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
}}
page 50402 P
{{
    PageType = {page_type};
    SourceTable = R;
    layout
    {{
        area(Content)
        {{
            repeater(Rows)
            {{
                field(NameCtl; Rec.Name) {{ trigger OnAssistEdit() begin Log.Add('AE(Name)'); end; }}
                field(SetCtl; Rec.Name) {{ trigger OnAssistEdit() begin Log.Add('AE(Set)'); Rec.Name := 'assisted'; end; }}
            }}
        }}
    }}
    var Log: Codeunit L;
    trigger OnAfterGetRecord() begin Log.Add('R(' + Rec.Code + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C(' + Rec.Code + ')'); end;
}}
codeunit 50403 T
{{
    procedure Run(): Text
    var
        Row: Record R;
        Pg: TestPage P;
        Log: Codeunit L;
        Out: Text;
    begin
        Row.Code := 'A'; Row.Insert();
        Row.Code := 'B'; Row.Insert();
        Pg.OpenEdit();
        Pg.GoToKey('B');
        Log.Take();
        Pg.NameCtl.AssistEdit();
        Out := 'field=' + Log.Take();
        Pg.SetCtl.AssistEdit();
        Out += ' set=' + Log.Take() + ' value=' + Pg.NameCtl.Value();
        exit(Out);
    end;
}}
"#
    )
    .replace("repeater(Rows)", if page_type == "Card" { "group(Rows)" } else { "repeater(Rows)" })
}

#[test]
fn assistedit_refreshes_the_row_unless_the_trigger_changed_rec() {
    for page_type in ["Card", "List"] {
        assert_eq!(
            text(run_in(&src(page_type), "T", "Run")),
            "field=AE(Name);R(B);C(B) set=AE(Set) value=assisted",
            "{page_type}"
        );
    }
}
