//! al-runner issue 8523: `CurrPage.SaveRecord()` in the OnValidate of a control bound to a page variable, on the untouched
//! new-row line, after page code moved Rec onto a stored row (filter group 2 and Find('-')). The cells corpus pull request 1470
//! measured (StaleUpdateNewLine.al:498, :914, :1242, :1318) save that row through assumption `new-line-saverecord-found-row`; the
//! shapes next to them refuse, and `CurrPage.Update(true)` keeps main's save.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// The OnValidate of the measured List cell (:1242): a first SaveRecord on the line, the batch switch, then SaveRecord.
const LIST_CELL: &str = "CurrPage.SaveRecord(); SetBatch(); CurrPage.SaveRecord();";
/// The Worksheet cell without the first SaveRecord (:914).
const WS_CELL: &str = "SetBatch(); CurrPage.SaveRecord();";

/// One page and one test, as in the probe: the batch OnValidate's `body`, the page's OnModifyRecord answer, more page code,
/// more objects, and whether a control's OnValidate failed first (`boom`). The test switches the batch, then checks the
/// table's OnModify ran once. It changes no field, as in the probe: the save writes the row as stored.
#[derive(Clone, Copy)]
struct Shape {
    page_type: &'static str,
    body: &'static str,
    modify_answer: &'static str,
    page_extra: &'static str,
    objects: &'static str,
    boom: bool,
}

impl Shape {
    fn list(body: &'static str) -> Self {
        Self {
            page_type: "List",
            body,
            modify_answer: "true",
            page_extra: "",
            objects: "",
            boom: false,
        }
    }

    fn worksheet(body: &'static str) -> Self {
        Self {
            page_type: "Worksheet",
            ..Self::list(body)
        }
    }

    fn src(&self) -> String {
        let Shape {
            page_type,
            body,
            modify_answer,
            page_extra,
            objects,
            boom,
        } = *self;
        let boom = if boom { "if TryBoom(P) then;" } else { "" };
        format!(
            r#"
table 99840 "LT NL Line"
{{
    fields {{ field(1; "Batch Name"; Code[10]) {{ }} field(2; "Line No."; Integer) {{ }} field(4; Description; Text[30]) {{ }} }}
    keys {{ key(PK; "Batch Name", "Line No.") {{ Clustered = true; }} }}
    trigger OnInsert() begin end;
    trigger OnModify() var C: Codeunit "LT NL Count"; begin C.Add(); end;
}}

codeunit 99844 "LT NL Count"
{{
    SingleInstance = true;
    procedure Add() begin N += 1; end;
    procedure Mods(): Integer begin exit(N); end;
    var N: Integer;
}}

page 99841 "LT NL Page"
{{
    PageType = {page_type};
    SourceTable = "LT NL Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {{
        area(Content)
        {{
            group(Header)
            {{
                field(CurrentBatch; CurrentBatch)
                {{
                    trigger OnValidate()
                    begin
                        {body}
                    end;
                }}
                field(Boom; Boom) {{ trigger OnValidate() begin Error('boom'); end; }}
            }}
            repeater(Lines) {{ field("Line No."; Rec."Line No.") {{ }} field(Description; Rec.Description) {{ }} }}
        }}
    }}
    trigger OnOpenPage() begin CurrentBatch := 'B3'; SetBatch(); end;
    trigger OnAfterGetRecord() begin end;
    trigger OnAfterGetCurrRecord() begin end;
    trigger OnNewRecord(BelowxRec: Boolean) begin end;
    trigger OnModifyRecord(): Boolean begin exit({modify_answer}); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin exit(true); end;
    {page_extra}
    local procedure SetBatch()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("Batch Name", CurrentBatch);
        Rec.FilterGroup(0);
        if Rec.Find('-') then;
    end;
    var
        CurrentBatch: Code[10];
        Boom: Text[10];
}}

{objects}

codeunit 99842 T
{{
    Subtype = Test;

    [Test]
    procedure Switch()
    var
        Line: Record "LT NL Line";
        Count: Codeunit "LT NL Count";
        P: TestPage "LT NL Page";
    begin
        Line."Batch Name" := 'B1';
        Line."Line No." := 20000;
        Line.Insert();
        Line."Batch Name" := 'B2';
        Line."Line No." := 10000;
        Line.Insert();
        Line."Line No." := 20000;
        Line.Insert();
        P.OpenEdit();
        if P."Line No.".Value() <> '' then;
        {boom}
        P.CurrentBatch.SetValue('B2');
        if Count.Mods() <> 1 then
            Error('Mods %1', Count.Mods());
    end;

    [TryFunction]
    local procedure TryBoom(var P: TestPage "LT NL Page")
    begin
        P.Boom.SetValue('x');
    end;
}}
"#
        )
    }

    fn outcome(&self) -> Outcome {
        let o = outcomes_in(&self.src(), "T");
        assert_eq!(o.len(), 1, "{o:?}");
        o.into_iter().next().unwrap().1
    }
}

#[track_caller]
fn assert_refused(shape: Shape, text: &str) {
    match shape.outcome() {
        Outcome::NotRun(r) => assert!(r.contains(text), "{}: {r}", shape.body),
        other => panic!("{}: expected the refusal [{text}], got {other:?}", shape.body),
    }
}

const OUTSIDE: &str = "CurrPage.SaveRecord() on the new-row line";

/// The measured cells: the found row is modified once (its OnModify runs).
#[test]
fn saverecord_in_the_measured_cell_modifies_the_found_row() {
    let ws_first = "CurrPage.SaveRecord(); SetBatch(); CurrPage.SaveRecord();";
    let ws_code = "CurrPage.SaveRecord(); SetBatch(); Rec.Description := 'code'; CurrPage.SaveRecord();";
    for shape in [
        Shape::list(LIST_CELL),
        Shape::worksheet(WS_CELL),
        Shape::worksheet(ws_first),
        Shape::worksheet(ws_code),
    ] {
        assert_eq!(shape.outcome(), Outcome::Pass, "{}", shape.body);
    }
}

/// A page type, a trigger and an earlier failed step the probe did not measure for SaveRecord: refused.
#[test]
fn saverecord_next_to_the_cell_is_refused() {
    let delete = "trigger OnDeleteRecord(): Boolean begin exit(true); end;";
    assert_refused(
        Shape {
            page_extra: delete,
            ..Shape::worksheet(WS_CELL)
        },
        OUTSIDE,
    );
    assert_refused(
        Shape {
            page_type: "Document",
            ..Shape::worksheet(WS_CELL)
        },
        OUTSIDE,
    );
    assert_refused(
        Shape {
            boom: true,
            ..Shape::worksheet(WS_CELL)
        },
        OUTSIDE,
    );
    // The List cells all made the first SaveRecord (:1242).
    assert_refused(Shape::list(WS_CELL), OUTSIDE);
}

/// Neighbors of the cell's record state and calls (review of pull request 8536, round 1): Find('+'), a row outside the
/// page's filter, Update(false) or Rec.Modify() before the save, a subscriber of the table's events. Refused.
#[test]
fn saverecord_with_another_row_or_step_is_refused() {
    let find_last = "Rec.FilterGroup(2); Rec.SetRange(\"Batch Name\", CurrentBatch); Rec.FilterGroup(0); if Rec.Find('+') then; CurrPage.SaveRecord();";
    let other_batch = "if Rec.Get('B1', 20000) then; CurrPage.SaveRecord();";
    let update_false = "SetBatch(); CurrPage.Update(false); CurrPage.SaveRecord();";
    let modify = "SetBatch(); Rec.Modify(); CurrPage.SaveRecord();";
    for body in [find_last, other_batch, update_false, modify] {
        assert_refused(Shape::worksheet(body), OUTSIDE);
    }
    let subscriber = r#"
codeunit 99843 "LT NL Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"LT NL Line", 'OnAfterModifyEvent', '', false, false)]
    local procedure OnAfterModify(var Rec: Record "LT NL Line") begin end;
}"#;
    assert_refused(
        Shape {
            objects: subscriber,
            ..Shape::worksheet(WS_CELL)
        },
        OUTSIDE,
    );
}

/// The page's OnModifyRecord answers false in the cell: the save the probe measured did not happen. Refused.
#[test]
fn saverecord_whose_onmodifyrecord_answers_false_is_refused() {
    assert_refused(
        Shape {
            modify_answer: "false",
            ..Shape::worksheet(WS_CELL)
        },
        "whose row was not written",
    );
}

/// An error after the assumed save, in the same OnValidate: what the page shows then is not measured. Refused.
#[test]
fn an_error_after_the_assumed_save_is_refused() {
    let error_after = "SetBatch(); CurrPage.SaveRecord(); Error('after save');";
    let list_error_after = "CurrPage.SaveRecord(); SetBatch(); CurrPage.SaveRecord(); Error('after save');";
    assert_refused(Shape::worksheet(error_after), "fails after CurrPage.SaveRecord()");
    assert_refused(Shape::list(list_error_after), "fails after CurrPage.SaveRecord()");
}

/// `CurrPage.Update(true)` outside the cell keeps main's save of the found row (Y50UpdateFromNewLine.al:610), also after
/// Update(false) and on a page with another trigger.
#[test]
fn update_true_outside_the_cell_keeps_mains_save() {
    let delete = "trigger OnDeleteRecord(): Boolean begin exit(true); end;";
    for shape in [
        Shape {
            page_extra: delete,
            ..Shape::worksheet("SetBatch(); CurrPage.Update(true);")
        },
        Shape::list("SetBatch(); CurrPage.Update(false); CurrPage.Update(true);"),
    ] {
        assert_eq!(shape.outcome(), Outcome::Pass, "{}", shape.body);
    }
}
