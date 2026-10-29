//! `filter('>B')` in a view set by an object property is a comparison, on a Code and on a Text field
//! (filters/lt-uc-quoted-values/UcQuotedValues.al:PageAndQuery_FilterGreaterB `page_cd_ids`, `page_txt_ids` 5,14,15).
//! Only a view given to SetView at run time reads the quoted value as a literal (:SetView_FilterGreaterB).

use al_testkit::text;

const OBJS: &str = r#"
table 53500 "QV Row"
{
    fields { field(1; Id; Integer) { } field(2; Cd; Code[30]) { } field(3; Txt; Text[30]) { } field(4; Head; Code[10]) { } }
    keys { key(PK; Id) { } }
}
table 53501 "QV Head" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
codeunit 53502 "QV Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin if StrPos(T, S + ',') = 0 then T += S + ','; end;
    procedure Dump(): Text begin exit(T); end;
}
page 53503 "QV Cd List"
{
    PageType = List;
    SourceTable = "QV Row";
    SourceTableView = where(Cd = filter('>B'));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
page 53504 "QV Txt List"
{
    PageType = List;
    SourceTable = "QV Row";
    SourceTableView = where(Txt = filter('>B'));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
page 53505 "QV Part"
{
    PageType = ListPart;
    SourceTable = "QV Row";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "QV Log"; begin Log.Add(Format(Rec.Id)); end;
}
page 53506 "QV Host"
{
    PageType = Document;
    SourceTable = "QV Head";
    layout
    {
        area(Content)
        {
            field(C; Rec."Code") { }
            part(P; "QV Part") { SubPageLink = Head = field("Code"); SubPageView = where(Cd = filter('>B')); }
        }
    }
}
query 53507 "QV Cd Query"
{
    elements { dataitem(Row; "QV Row") { DataItemTableFilter = Cd = filter('>B'); column(Id; Id) { } } }
}
query 53508 "QV Txt Query"
{
    elements { dataitem(Row; "QV Row") { DataItemTableFilter = Txt = filter('>B'); column(Id; Id) { } } }
}
"#;

const SEED: &str = "R.DeleteAll();
    R.Id := 1; R.Cd := 'B'; R.Txt := 'B'; R.Head := 'H'; R.Insert();
    R.Id := 2; R.Cd := 'A'; R.Txt := 'A'; R.Head := 'H'; R.Insert();
    R.Id := 3; R.Cd := 'C'; R.Txt := 'C'; R.Head := 'H'; R.Insert();
    R.Id := 4; R.Cd := 'ZZZ'; R.Txt := 'zzz'; R.Head := 'H'; R.Insert();
    R.Id := 5; R.Cd := '>B'; R.Txt := '>b'; R.Head := 'H'; R.Insert();";

fn run(vars: &str, body: &str) -> String {
    let src = format!(
        "{OBJS}\ncodeunit 53600 C {{ procedure F(): Text var R: Record \"QV Row\"; Ids: Text; {vars} begin {SEED} {body} end; }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v))
}

#[test]
fn a_quoted_greater_than_in_source_table_view_is_a_comparison_on_code() {
    let r = run(
        "Pg: TestPage \"QV Cd List\";",
        "Pg.OpenView(); if Pg.First() then repeat Ids += Format(Pg.Id.Value()) + ','; until not Pg.Next(); exit(Ids);",
    );
    assert_eq!(r, "3,4,");
}

#[test]
fn a_quoted_greater_than_in_source_table_view_is_a_comparison_on_text() {
    let r = run(
        "Pg: TestPage \"QV Txt List\";",
        "Pg.OpenView(); if Pg.First() then repeat Ids += Format(Pg.Id.Value()) + ','; until not Pg.Next(); exit(Ids);",
    );
    assert_eq!(r, "3,4,");
}

// A SubPageView reaches the same view parser as SourceTableView: `filter('>B')` stays a comparison there too.
#[test]
fn a_quoted_greater_than_in_sub_page_view_is_a_comparison() {
    let r = run(
        "H: Record \"QV Head\"; Pg: TestPage \"QV Host\"; Log: Codeunit \"QV Log\";",
        "H.Code := 'H'; H.Insert(); Pg.OpenEdit(); exit(Log.Dump());",
    );
    let mut ids: Vec<&str> = r.split(',').filter(|s| !s.is_empty()).collect();
    ids.sort();
    assert_eq!(ids, ["3", "4"]);
}

// :PageAndQuery_FilterGreaterB `query_cd_ids`, `query_txt_ids` 5,14,15: a DataItemTableFilter reads it as a comparison too.
#[test]
fn a_quoted_greater_than_in_data_item_table_filter_is_a_comparison() {
    let r = run(
        "Qc: Query \"QV Cd Query\"; Qt: Query \"QV Txt Query\";",
        "Qc.Open(); while Qc.Read() do Ids += Format(Qc.Id) + ','; Qc.Close(); Ids += '|';
         Qt.Open(); while Qt.Read() do Ids += Format(Qt.Id) + ','; Qt.Close(); exit(Ids);",
    );
    assert_eq!(r, "3,4,|3,4,");
}
