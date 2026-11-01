//! How many times a page open runs the CaptionClass expressions with an empty record, for more fields and page types than one or
//! two fields on a Card or a List (pages/caption_runs.rs). Each test names the corpus test of
//! pages/lt-y61-page-probes/Y61CaptionClassCount.al that pins it.

const SRC: &str = r#"
codeunit 69930 "CK Log"
{
    SingleInstance = true;
    var Empty: Dictionary of [Integer, Integer]; Rows: Integer;
    procedure Hit(n: Integer; Id: Integer)
    var c: Integer;
    begin
        if Id <> 0 then begin Rows += 1; exit; end;
        if Empty.ContainsKey(n) then c := Empty.Get(n);
        Empty.Set(n, c + 1);
    end;
    procedure Take() R: Text
    var n: Integer;
    begin
        foreach n in Empty.Keys() do R += Format(n) + ':' + Format(Empty.Get(n)) + ' ';
        R += 'rows=' + Format(Rows);
        Clear(Empty); Rows := 0;
    end;
}
table 69931 "CK Four"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; F1; Decimal) { CaptionClass = Cc(1); } field(3; F2; Decimal) { CaptionClass = Cc(2); }
        field(4; F3; Decimal) { CaptionClass = Cc(3); } field(5; F4; Decimal) { CaptionClass = Cc(4); }
    }
    keys { key(PK; Id) { } }
    procedure Cc(n: Integer): Text[80] var L: Codeunit "CK Log"; begin L.Hit(n, Id); exit('3,Cap' + Format(n)); end;
}
table 69932 "CK Three"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; F1; Decimal) { CaptionClass = Cc(); } field(3; F2; Decimal) { CaptionClass = Cc(); } field(4; F3; Decimal) { CaptionClass = Cc(); }
    }
    keys { key(PK; Id) { } }
    procedure Cc(): Text[80] var L: Codeunit "CK Log"; begin L.Hit(0, Id); exit('3,Same'); end;
}
table 69933 "CK One"
{
    fields { field(1; Id; Integer) { } field(2; F1; Decimal) { CaptionClass = Cc(); } }
    keys { key(PK; Id) { } }
    procedure Cc(): Text[80] var L: Codeunit "CK Log"; begin L.Hit(1, Id); exit('3,One'); end;
}
page 69934 "CK Four List"
{
    PageType = List; SourceTable = "CK Four"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(F1; Rec.F1) { } field(F2; Rec.F2) { } field(F3; Rec.F3) { } field(F4; Rec.F4) { } } } }
}
page 69935 "CK Three List"
{
    PageType = List; SourceTable = "CK Three"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(F1; Rec.F1) { } field(F2; Rec.F2) { } field(F3; Rec.F3) { } } } }
}
page 69936 "CK One Worksheet"
{
    PageType = Worksheet; SourceTable = "CK One"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(F1; Rec.F1) { } } } }
}
page 69937 "CK One ListPlus"
{
    PageType = ListPlus; SourceTable = "CK One"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(G) { field(F1; Rec.F1) { } } } }
}
codeunit 69938 C
{
    procedure FourList(): Text
    var R: Record "CK Four"; P: TestPage "CK Four List"; L: Codeunit "CK Log"; T: Text; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(); end;
        L.Take();
        P.OpenEdit();
        T := L.Take();
        R.Get(2);
        P.GoToRecord(R);
        T += '|' + L.Take();
        P.Close();
        exit(T + '|' + L.Take());
    end;

    procedure ThreeList(): Text
    var R: Record "CK Three"; P: TestPage "CK Three List"; L: Codeunit "CK Log"; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(); end;
        L.Take();
        P.OpenEdit();
        exit(L.Take());
    end;

    procedure OneWorksheet(): Text
    var R: Record "CK One"; P: TestPage "CK One Worksheet"; L: Codeunit "CK Log"; T: Text; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(); end;
        L.Take();
        P.OpenEdit();
        T := L.Take();
        R.Get(2);
        P.GoToRecord(R);
        exit(T + '|' + L.Take());
    end;

    procedure OneListPlus(): Text
    var R: Record "CK One"; P: TestPage "CK One ListPlus"; L: Codeunit "CK Log"; T: Text; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(); end;
        L.Take();
        P.OpenEdit();
        T := L.Take();
        R.Get(2);
        P.GoToRecord(R);
        exit(T + '|' + L.Take());
    end;
}
"#;

fn run(proc_name: &str) -> String {
    let mut built = al_testkit::Built::from_source(SRC);
    al_testkit::text(built.run_in("C", proc_name))
}

/// Y61CaptionClassCount.al:468 G615_List_4DifferentFields: 10 empty runs and one row run per field; GoToRecord and Close run none.
#[test]
fn a_list_with_four_expression_fields() {
    assert_eq!(run("FourList"), "1:10 2:10 3:10 4:10 rows=4|rows=0|rows=0");
}

/// Y61CaptionClassCount.al:540 G615_List_3Fields_SameText: 9 empty runs per field.
#[test]
fn a_list_with_three_fields_of_one_expression() {
    assert_eq!(run("ThreeList"), "0:27 rows=3");
}

/// Y61CaptionClassCount.al:639 G615_Worksheet_1Field: as a List.
#[test]
fn a_worksheet_with_one_expression_field() {
    assert_eq!(run("OneWorksheet"), "1:7 rows=1|rows=0");
}

/// Y61CaptionClassCount.al:663 G615_ListPlus_1Field pins 8 runs on the open and one on GoToRecord; the moves of a ListPlus are not
/// built for it, so the open stays refused.
#[test]
fn a_list_plus_with_one_expression_field_in_a_group_is_refused() {
    let mut built = al_testkit::Built::from_source(SRC);
    let r = built.run_in("C", "OneListPlus");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
