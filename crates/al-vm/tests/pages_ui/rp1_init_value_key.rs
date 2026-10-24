//! A page whose SourceTableView sorts by a secondary key, over a table whose primary key has an InitValue: Rec stands on a
//! primary key no row has when the page opens and OnOpenPage does not touch it. A List or Card opened for edit or view opens
//! on the first row of the view (corpus pages/lt-outside-worksheet/RmInitValueKey.al:231, :250, :330, :349; al-runner issue
//! 5760). OnOpenPage that moves Rec onto a primary key no row has stays refused (not pinned).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69970 "IK Line"
{
    fields
    {
        field(1; "Group Code"; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = A,B,C; InitValue = B; }
        field(3; "Line No."; Integer) { }
        field(4; SortOrder; Integer) { }
    }
    keys { key(PK; "Group Code", Kind, "Line No.") { } key(BySort; SortOrder) { } }
}
codeunit 69971 "IK Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(S: Text) begin E += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := E; E := ''; exit(R); end;
}
page 69972 "IK List"
{
    PageType = List; SourceTable = "IK Line"; SourceTableView = sorting(SortOrder) order(ascending); ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnOpenPage() begin Opened := true; end;
    trigger OnAfterGetRecord() var L: Codeunit "IK Log"; begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "IK Log"; begin L.Add('C:' + Format(Rec."Line No.")); end;
    var Opened: Boolean;
}
page 69973 "IK Card"
{
    PageType = Card; SourceTable = "IK Line"; SourceTableView = sorting(SortOrder) order(ascending); ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(LineNo; Rec."Line No.") { } } }
    trigger OnOpenPage() begin Opened := true; end;
    trigger OnAfterGetRecord() var L: Codeunit "IK Log"; begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "IK Log"; begin L.Add('C:' + Format(Rec."Line No.")); end;
    var Opened: Boolean;
}
page 69974 "IK Moved"
{
    PageType = List; SourceTable = "IK Line"; SourceTableView = sorting(SortOrder) order(ascending); ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnOpenPage() begin Rec."Line No." := 7; end;
}
codeunit 69975 C
{
    local procedure Seed()
    var L: Record "IK Line"; Log: Codeunit "IK Log";
    begin
        Ins('G', L.Kind::A, 1, 20); Ins('G', L.Kind::B, 2, 10); Ins('G', L.Kind::C, 3, 30); Ins('G', L.Kind::A, 4, -5);
        Log.Take();
    end;
    local procedure Ins(G: Code[20]; K: Option; N: Integer; S: Integer)
    var L: Record "IK Line";
    begin
        L."Group Code" := G; L.Kind := K; L."Line No." := N; L.SortOrder := S; L.Insert();
    end;
    procedure ListEdit(): Text
    var P: TestPage "IK List"; L: Codeunit "IK Log"; S: Text;
    begin
        Seed();
        P.OpenEdit();
        S := P.LineNo.Value() + '|' + L.Take();
        S += '|' + Format(P.Next()) + P.LineNo.Value() + '|' + L.Take();
        exit(S);
    end;
    procedure GoTo(): Text
    var P: TestPage "IK List"; L: Codeunit "IK Log"; Line: Record "IK Line";
    begin
        Seed();
        P.OpenEdit();
        P.Next();
        L.Take();
        Line.Get('G', Line.Kind::C, 3);
        P.GoToRecord(Line);
        exit(P.LineNo.Value() + '|' + L.Take());
    end;
    procedure CardView(): Text
    var P: TestPage "IK Card"; L: Codeunit "IK Log";
    begin
        Seed();
        P.OpenView();
        exit(P.LineNo.Value() + '|' + L.Take());
    end;
    procedure Moved()
    var P: TestPage "IK Moved";
    begin
        Seed();
        P.OpenEdit();
    end;
}
"#;

#[test]
fn a_list_opens_on_the_first_row_of_its_view() {
    assert_eq!(
        text(run_in(SRC, "C", "ListEdit")),
        "4|R:4;R:2;R:1;R:3;R:4;R:4;R:4;C:4;|Yes2|R:2;R:2;R:2;C:2;"
    );
}

#[test]
fn go_to_record_reads_each_row_of_its_scan_once_more_in_the_key() {
    assert_eq!(
        text(run_in(SRC, "C", "GoTo")),
        "3|R:4;R:4;R:4;C:4;R:2;R:2;R:2;C:2;R:1;R:1;R:1;C:1;R:3;R:3;R:3;C:3;"
    );
}

#[test]
fn a_card_opens_on_the_first_row_of_its_view() {
    assert_eq!(text(run_in(SRC, "C", "CardView")), "4|R:4;C:4;");
}

#[test]
fn an_onopenpage_that_moves_rec_onto_a_key_no_row_has_is_refused() {
    assert!(matches!(
        run_in(SRC, "C", "Moved"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
