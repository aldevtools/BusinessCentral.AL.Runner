//! A repeater column whose Visible a page extension's modify() binds to a variable the extension sets to true in
//! OnAfterGetRecord (pages/extensions/PageExtModifyVisibleVariable.al:519-547). Only the measured page counts the value
//! after the open read's first OnAfterGetRecord; every shape one fact away keeps the value frozen at open, as before the rule.

use al_testkit::text;

/// One list page with one page extension, and how the test opens it.
#[derive(Clone, Copy)]
struct Shape {
    name: &'static str,
    /// Further page properties.
    page: &'static str,
    /// Further properties of the column.
    column: &'static str,
    /// Further layout areas.
    layout: &'static str,
    /// Page triggers.
    triggers: &'static str,
    /// The Visible expression of the extension's modify().
    visible: &'static str,
    on_open: &'static str,
    on_record: &'static str,
    on_current: &'static str,
    rows: u8,
    open: &'static str,
    /// What the test does between the open and the read of Visible().
    then: &'static str,
}

/// PageExtModifyVisibleVariable.al:86-108 and :175-204, with one modified column.
const MEASURED: Shape = Shape {
    name: "Measured",
    page: "",
    column: "",
    layout: "",
    triggers: "",
    visible: "V",
    on_open: "O := true;",
    on_record: "V := true;",
    on_current: "C := true;",
    rows: 2,
    open: "OpenEdit",
    then: "",
};

fn source(shapes: &[Shape]) -> String {
    let mut src = String::from(
        r#"
table 69880 "ECV Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
page 69881 "ECV Fact"
{
    PageType = CardPart; SourceTable = "ECV Row";
    layout { area(Content) { field(Name; Rec.Name) { ApplicationArea = All; } } }
}
page 69883 "ECV Card"
{
    PageType = Card; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { ApplicationArea = All; } field(Col; Rec.Name) { ApplicationArea = All; } } }
}
pageextension 69884 "ECV Card Ext" extends "ECV Card"
{
    layout { modify(Col) { Visible = V; } }
    var V: Boolean; O: Boolean; C: Boolean;
    trigger OnOpenPage() begin O := true; end;
    trigger OnAfterGetRecord() begin V := true; end;
    trigger OnAfterGetCurrRecord() begin C := true; end;
}
page 69885 "ECV Own"
{
    PageType = List; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { ApplicationArea = All; } field(Col; Rec.Name) { ApplicationArea = All; Visible = V; } } } }
    var V: Boolean;
    trigger OnAfterGetRecord() begin V := true; end;
}
"#,
    );
    let mut calls = String::new();
    for (i, s) in shapes.iter().enumerate() {
        let (id, name) = (69900 + 2 * i, s.name);
        src += &format!(
            r#"
page {id} "ECV {name}"
{{
    PageType = List; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None; {page}
    layout {{ area(Content) {{ repeater(Rows) {{
        field(IdCtl; Rec.Id) {{ ApplicationArea = All; }}
        field(Col; Rec.Name) {{ ApplicationArea = All; {column} }}
    }} }} {layout} }}
    {triggers}
}}
pageextension {ext} "ECV {name} Ext" extends "ECV {name}"
{{
    layout {{ modify(Col) {{ Visible = {visible}; }} }}
    var V: Boolean; O: Boolean; C: Boolean; N: Integer;
    trigger OnOpenPage() begin {on_open} end;
    trigger OnAfterGetRecord() begin {on_record} end;
    trigger OnAfterGetCurrRecord() begin {on_current} end;
}}
"#,
            ext = id + 1,
            page = s.page,
            column = s.column,
            layout = s.layout,
            triggers = s.triggers,
            visible = s.visible,
            on_open = s.on_open,
            on_record = s.on_record,
            on_current = s.on_current,
        );
        calls += &format!(
            r#"
    procedure {name}(): Text
    var P: TestPage "ECV {name}";
    begin
        Seed({rows});
        P.{open}();
        {then}
        exit(Format(P.Col.Visible()));
    end;
"#,
            rows = s.rows,
            open = s.open,
            then = s.then,
        );
    }
    src + &format!(
        r#"
codeunit 69886 C
{{
    local procedure Seed(Rows: Integer)
    var
        R: Record "ECV Row";
        I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to Rows do begin
            R.Id := I;
            R.Insert();
        end;
    end;
    {calls}
    procedure Card(): Text
    var P: TestPage "ECV Card";
    begin
        Seed(2);
        P.OpenEdit();
        exit(Format(P.Col.Visible()));
    end;

    procedure Own(): Text
    var P: TestPage "ECV Own";
    begin
        Seed(2);
        P.OpenEdit();
        exit(Format(P.Col.Visible()));
    end;
}}
"#
    )
}

fn visible(shapes: &[Shape], routine: &str) -> String {
    text(al_testkit::run_in(&source(shapes), "C", routine))
}

/// PageExtModifyVisibleVariable.al:519-547 (`ColRec=Yes` after OpenEdit and a GoToKey, after OpenView and a Last).
#[test]
fn the_measured_column_counts_its_variable_after_the_first_row_of_the_open() {
    let shapes = [
        MEASURED,
        Shape {
            name: "MeasuredGoToKey",
            then: "P.Col.Visible(); P.GoToKey(2);",
            ..MEASURED
        },
        Shape {
            name: "MeasuredView",
            open: "OpenView",
            ..MEASURED
        },
        Shape {
            name: "MeasuredViewLast",
            open: "OpenView",
            then: "P.Last();",
            ..MEASURED
        },
    ];
    for shape in shapes {
        assert_eq!(visible(&shapes, shape.name), "Yes", "{}", shape.name);
    }
}

/// One fact away from the measured page: the column keeps the value frozen at open, as before the rule.
#[test]
fn neighbors_of_the_measured_column_keep_the_open_time_value() {
    let n = |name, expected: &'static str, shape: Shape| (Shape { name, ..shape }, expected);
    let neighbors = [
        // What the triggers do.
        n(
            "OpenTrueRecordFalse",
            "Yes",
            Shape {
                on_open: "V := true;",
                on_record: "V := false;",
                ..MEASURED
            },
        ),
        n(
            "RowsDiffer",
            "No",
            Shape {
                on_record: "V := Rec.Id = 1;",
                ..MEASURED
            },
        ),
        n(
            "FromTheOneRow",
            "No",
            Shape {
                on_record: "V := Rec.Id = 1;",
                rows: 1,
                ..MEASURED
            },
        ),
        n(
            "Toggled",
            "No",
            Shape {
                on_record: "V := not V;",
                rows: 1,
                ..MEASURED
            },
        ),
        n(
            "TwoStatements",
            "No",
            Shape {
                on_record: "V := true; N := 1;",
                ..MEASURED
            },
        ),
        n(
            "OtherTriggersEmpty",
            "No",
            Shape {
                on_open: "",
                on_current: "",
                ..MEASURED
            },
        ),
        n(
            "SetFalseAtOpen",
            "No",
            Shape {
                on_open: "V := false;",
                ..MEASURED
            },
        ),
        n(
            "SetInCurrentToo",
            "No",
            Shape {
                on_current: "V := true;",
                ..MEASURED
            },
        ),
        // The rows and what the test does after the open.
        n("OneRow", "No", Shape { rows: 1, ..MEASURED }),
        n("ThreeRows", "No", Shape { rows: 3, ..MEASURED }),
        n(
            "EditThenLast",
            "No",
            Shape {
                then: "P.Last();",
                ..MEASURED
            },
        ),
        n(
            "ViewThenGoToKey",
            "No",
            Shape {
                open: "OpenView",
                then: "P.GoToKey(2);",
                ..MEASURED
            },
        ),
        n(
            "TwoMoves",
            "No",
            Shape {
                then: "P.GoToKey(2); P.GoToKey(1);",
                ..MEASURED
            },
        ),
        n(
            "ThenNew",
            "No",
            Shape {
                then: "P.New();",
                ..MEASURED
            },
        ),
        n(
            "ThenFilter",
            "No",
            Shape {
                then: "P.Filter.SetFilter(Id, '2');",
                ..MEASURED
            },
        ),
        n(
            "ThenValueRead",
            "No",
            Shape {
                then: "if P.Col.Value() = '' then;",
                ..MEASURED
            },
        ),
        // The expression and the base column.
        n(
            "NotTheVariable",
            "Yes",
            Shape {
                visible: "not V",
                ..MEASURED
            },
        ),
        n(
            "BaseColumnVisibleFalse",
            "No",
            Shape {
                column: "Visible = false;",
                ..MEASURED
            },
        ),
        n(
            "BaseColumnEditable",
            "No",
            Shape {
                column: "Editable = false;",
                ..MEASURED
            },
        ),
        // The base page.
        n(
            "BasePageTrigger",
            "No",
            Shape {
                triggers: "trigger OnOpenPage() begin end;",
                ..MEASURED
            },
        ),
        n(
            "PageNotEditable",
            "No",
            Shape {
                page: "Editable = false;",
                ..MEASURED
            },
        ),
        n(
            "PageSourceTableView",
            "No",
            Shape {
                page: "SourceTableView = sorting(Id);",
                ..MEASURED
            },
        ),
        n(
            "PageWithFactBox",
            "No",
            Shape {
                layout: r#"area(FactBoxes) { part(Fact; "ECV Fact") { ApplicationArea = All; } }"#,
                ..MEASURED
            },
        ),
    ];
    let shapes: Vec<Shape> = neighbors.iter().map(|(s, _)| *s).collect();
    let src = source(&shapes);
    for (shape, expected) in neighbors {
        assert_eq!(
            text(al_testkit::run_in(&src, "C", shape.name)),
            expected,
            "{}",
            shape.name
        );
    }
    // A card field and a list's own column.
    assert_eq!(text(al_testkit::run_in(&src, "C", "Card")), "No");
    assert_eq!(text(al_testkit::run_in(&src, "C", "Own")), "No");
}
