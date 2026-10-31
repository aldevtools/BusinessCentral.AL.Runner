//! `FindFirstField` and `FindNextField` on a list, then reads of controls whose source is an expression: a function call, a
//! string expression and a page variable that OnAfterGetRecord sets follow the row the search ended on; a search on an
//! arithmetic control with a text finds nothing and leaves the cursor (misc/lt-y13-triage/Y13PageTriage.al:150-186, corpus
//! master).

const SRC: &str = r#"
table 69950 "FE Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Qty; Integer) { } } keys { key(PK; Id) { } } }
page 69951 "FE List"
{
    PageType = List; SourceTable = "FE Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Name; Rec.Name) { }
                field(Fn; Describe()) { }
                field(Dbl; Rec.Qty * 2) { }
                field(Joined; Rec.Name + '/' + Format(Rec.Id)) { }
                field(FromVar; Stored) { }
            }
        }
    }
    var Stored: Text;
    local procedure Describe(): Text begin exit('d:' + Rec.Name); end;
    trigger OnAfterGetRecord() begin Stored := 'v:' + Rec.Name; end;
}
codeunit 69952 C
{
    local procedure Shown(var P: TestPage "FE List"): Text
    begin
        exit(P.Name.Value() + ' ' + P.Fn.Value() + ' ' + P.Dbl.Value() + ' ' + P.Joined.Value() + ' ' + P.FromVar.Value() + ';');
    end;
    procedure F(): Text
    var R: Record "FE Row"; P: TestPage "FE List"; Out: Text;
    begin
        R.Id := 1; R.Name := 'Alpha'; R.Qty := 5; R.Insert();
        R.Id := 2; R.Name := 'Beta'; R.Qty := 7; R.Insert();
        R.Id := 3; R.Name := 'Gamma'; R.Qty := 9; R.Insert();
        P.OpenView();
        Out := Format(P.FindFirstField(P.Name, 'Beta')) + ' ' + Shown(P);
        Out += Format(P.FindNextField(P.Name, 'Gamma')) + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.Fn, 'd:Alpha')) + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.Dbl, '14')) + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.Joined, 'Gamma/3')) + ' ' + Shown(P);
        Out += Format(P.FindFirstField(P.FromVar, 'v:Beta')) + ' ' + Shown(P);
        exit(Out);
    end;
}
"#;

#[test]
fn expression_controls_follow_the_row_find_field_ends_on() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "Yes Beta d:Beta 14 Beta/2 v:Beta;\
         Yes Gamma d:Gamma 18 Gamma/3 v:Gamma;\
         Yes Alpha d:Alpha 10 Alpha/1 v:Alpha;\
         No Alpha d:Alpha 10 Alpha/1 v:Alpha;\
         Yes Gamma d:Gamma 18 Gamma/3 v:Gamma;\
         Yes Beta d:Beta 14 Beta/2 v:Beta;"
    );
}
