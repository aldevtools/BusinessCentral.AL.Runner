//! al-runner issue 8523: `CurrPage.SaveRecord()` in the OnValidate of a control bound to a page variable, on the untouched
//! new-row line, after page code moved Rec onto a stored row (filter group 2 and Find('-')). The cells corpus pull request 1470
//! measured (StaleUpdateNewLine.al:498, :914, :1242, :1318) save that row through assumption `new-line-saverecord-found-row`; the
//! shapes next to them refuse, and `CurrPage.Update(true)` keeps main's save. The page, its table and its log are the probe's
//! (StaleUpdateNewLine.al:24-364) under other object names; each shape changes one piece of that code or one step.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// The probe's objects (StaleUpdateNewLine.al:24-364), with the code a shape changes as `@name@` slots.
const OBJECTS: &str = r#"
table 99840 "NL Line"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Batch Name"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; "No."; Code[20]) { }
        field(4; Description; Text[30]) { }
    }
    keys
    {
        key(PK; "Batch Name", "Line No.") { Clustered = true; }
    }
    trigger OnInsert()
    begin
        Log.Add('L.OnInsert:' + "Batch Name" + '/' + Format("Line No."));
    end;
    trigger OnModify()
    @on_modify@
    var
        Log: Codeunit "NL Log";
}

page 99841 "NL Page"
{
    PageType = @page_type@;
    SourceTable = "NL Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(CurrentBatch; CurrentBatch)
                {
                    ApplicationArea = All;
                    Caption = 'Batch';
                    trigger OnValidate()
                    @validate@
                }
                field(Boom; Boom)
                {
                    ApplicationArea = All;
                    Caption = 'Boom';
                    trigger OnValidate()
                    begin
                        Log.Add('B:start ' + Format(Rec."Line No."));
                        case Log.BoomMode() of
                            1:
                                CurrPage.Update(true);
                            2:
                                CurrPage.Update(false);
                            3:
                                CurrPage.SaveRecord();
                        end;
                        Log.Add('B:error');
                        Error('LT FU boom');
                    end;
                }
            }
            repeater(Lines)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }
    trigger OnOpenPage()
    begin
        CurrentBatch := 'B3';
        SetBatch();
    end;
    trigger OnAfterGetRecord()
    begin
        Log.Add('R:' + Rec."Batch Name" + '/' + Format(Rec."Line No."));
    end;
    trigger OnAfterGetCurrRecord()
    @agcr@
    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('N:' + Rec."Batch Name" + '/' + Format(Rec."Line No.") + ' below=' + Format(BelowxRec));
    end;
    trigger OnModifyRecord(): Boolean
    @modify_record@
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('I:' + Rec."Batch Name" + '/' + Format(Rec."Line No."));
        exit(true);
    end;
    @page_extra@
    local procedure SetBatch()
    begin
        Rec.FilterGroup := 2;
        Rec.SetRange("Batch Name", CurrentBatch);
        Rec.FilterGroup := 0;
        if Rec.Find('-') then;
    end;
    var
        Log: Codeunit "NL Log";
        CurrentBatch: Code[10];
        Boom: Integer;
}

codeunit 99844 "NL Log"
{
    SingleInstance = true;
    var
        Entries: Text;
        DoFirstSave: Boolean;
        SecondStep: Integer;
        DoTouch: Boolean;
        Boom: Integer;
    procedure Add(Entry: Text)
    begin
        if Entries <> '' then
            Entries += ';';
        Entries += Entry;
    end;
    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
    procedure SetSteps(NewFirstSave: Boolean; NewSecond: Integer; NewTouch: Boolean; NewBoom: Integer)
    begin
        DoFirstSave := NewFirstSave;
        SecondStep := NewSecond;
        DoTouch := NewTouch;
        Boom := NewBoom;
    end;
    procedure FirstSave(): Boolean
    begin
        exit(DoFirstSave);
    end;
    procedure Second(): Integer
    begin
        exit(SecondStep);
    end;
    procedure Touch(): Boolean
    begin
        exit(DoTouch);
    end;
    procedure BoomMode(): Integer
    begin
        exit(Boom);
    end;
}

table 99845 "NL Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 99846 "NL Line2"
{
    fields { field(1; "Batch Name"; Code[10]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Batch Name", "Line No.") { Clustered = true; } }
    trigger OnInsert() var Log: Codeunit "NL Log"; begin Log.Add('L2.OnInsert'); end;
}

@objects@

codeunit 99842 T
{
    Subtype = Test;

    [Test]
    procedure Switch()
    var
        Log: Codeunit "NL Log";
        P: TestPage "NL Page";
        Seen: Text;
    begin
        AddLine('B1', 10000, 'A');
        AddLine('B2', 10000, 'B');
        AddLine('B2', 20000, 'D');
        Log.SetSteps(@steps@, 0);
        P.OpenEdit();
        if P."Line No.".Value() <> '' then;
        @boom@
        P.CurrentBatch.SetValue('B2');
        Seen := Log.Take();
        if StrPos(Seen, 'L.OnModify:B2/10000/') = 0 then
            Error('no modify: %1', Seen);
    end;

    local procedure AddLine(Batch: Code[10]; LineNo: Integer; No: Code[20])
    var
        Line: Record "NL Line";
    begin
        Line."Batch Name" := Batch;
        Line."Line No." := LineNo;
        Line."No." := No;
        Line.Description := 'd' + No;
        Line.Insert();
    end;

    [TryFunction]
    local procedure TryBoom(var P: TestPage "NL Page")
    begin
        P.Boom.SetValue(1);
    end;
}
"#;

/// The probe's batch OnValidate (:76-93).
const VALIDATE: &str = r#"begin
                        Log.Add('V:start ' + Format(Rec."Line No."));
                        if Log.FirstSave() then
                            CurrPage.SaveRecord();
                        SetBatch();
                        if Log.Touch() then
                            Rec.Description := 'code';
                        case Log.Second() of
                            1:
                                CurrPage.SaveRecord();
                            2:
                                CurrPage.Update(true);
                            3:
                                CurrPage.Update(false);
                        end;
                        Log.Add('V:done ' + Format(Rec."Line No."));
                    end;"#;
/// The probe's table OnModify (:45-48).
const ON_MODIFY: &str =
    r#"begin Log.Add('L.OnModify:' + "Batch Name" + '/' + Format("Line No.") + '/' + Description); end;"#;
/// The probe's OnAfterGetCurrRecord (:146-149).
const AGCR: &str = r#"begin Log.Add('C:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;"#;
/// The probe's OnModifyRecord (:156-160).
const MODIFY_RECORD: &str = r#"begin Log.Add('M:' + Rec."Batch Name" + '/' + Format(Rec."Line No.") + '/' + Rec.Description + ' x=' + xRec."Batch Name" + '/' + Format(xRec."Line No.")); exit(true); end;"#;

/// The steps of the log (:337-343): the first SaveRecord, the second step (1 SaveRecord, 2 Update(true), 3 Update(false)), and
/// whether code sets Rec.Description.
const LIST_CELL: &str = "true, 1, false";
const WS_CELL: &str = "false, 1, false";

/// One shape: the probe's code and steps with the pieces it changes, and whether a control's OnValidate failed first (`boom`).
#[derive(Clone, Copy)]
struct Shape {
    page_type: &'static str,
    steps: &'static str,
    validate: &'static str,
    on_modify: &'static str,
    agcr: &'static str,
    modify_record: &'static str,
    page_extra: &'static str,
    objects: &'static str,
    boom: bool,
}

impl Shape {
    fn list(steps: &'static str) -> Self {
        Self {
            page_type: "List",
            steps,
            validate: VALIDATE,
            on_modify: ON_MODIFY,
            agcr: AGCR,
            modify_record: MODIFY_RECORD,
            page_extra: "",
            objects: "",
            boom: false,
        }
    }

    fn worksheet(steps: &'static str) -> Self {
        Self {
            page_type: "Worksheet",
            ..Self::list(steps)
        }
    }

    fn src(&self) -> String {
        OBJECTS
            .replace("@page_type@", self.page_type)
            .replace("@steps@", self.steps)
            .replace("@validate@", self.validate)
            .replace("@on_modify@", self.on_modify)
            .replace("@agcr@", self.agcr)
            .replace("@modify_record@", self.modify_record)
            .replace("@page_extra@", self.page_extra)
            .replace("@objects@", self.objects)
            .replace("@boom@", if self.boom { "if TryBoom(P) then;" } else { "" })
    }

    fn outcome(&self) -> Outcome {
        let o = outcomes_in(&self.src(), "T");
        assert_eq!(o.len(), 1, "{o:?}");
        o.into_iter().next().unwrap().1
    }

    fn name(&self) -> String {
        format!("{} {}", self.page_type, self.steps)
    }
}

#[track_caller]
fn assert_refused(shape: Shape, text: &str) {
    match shape.outcome() {
        Outcome::NotRun(r) => assert!(r.contains(text), "{}: {r}", shape.name()),
        other => panic!("{}: expected the refusal [{text}], got {other:?}", shape.name()),
    }
}

const OUTSIDE: &str =
    "CurrPage.SaveRecord() on the new-row line after page code moved Rec onto a stored row, outside the measured cells";

/// The measured cells: the found row is modified (its OnModify runs), and the List refresh after Update(true) (:1280).
#[test]
fn saverecord_in_the_measured_cell_modifies_the_found_row() {
    for shape in [
        Shape::list(LIST_CELL),
        Shape::list("true, 2, false"),
        Shape::worksheet(WS_CELL),
        Shape::worksheet("true, 1, false"),
        Shape::worksheet("true, 1, true"),
    ] {
        assert_eq!(shape.outcome(), Outcome::Pass, "{}", shape.name());
    }
}

/// A page type, a trigger, an earlier failed step and a List without the first SaveRecord, none measured for SaveRecord: refused.
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
    let subscriber = r#"
codeunit 99843 "NL Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NL Line", 'OnAfterModifyEvent', '', false, false)]
    local procedure OnAfterModify(var Rec: Record "NL Line") begin end;
}"#;
    assert_refused(
        Shape {
            objects: subscriber,
            ..Shape::worksheet(WS_CELL)
        },
        OUTSIDE,
    );
}

/// A batch OnValidate other than the probe's (review of pull request 8536, rounds 1 and 2: another positioning call, the filter
/// group left at 2, a write or Update(false) before the save, an error after it): refused.
#[test]
fn saverecord_from_another_onvalidate_is_refused() {
    let get = "begin Rec.FilterGroup := 2; Rec.SetRange(\"Batch Name\", CurrentBatch); Rec.FilterGroup := 0; if Rec.Get(CurrentBatch, 10000) then; CurrPage.SaveRecord(); end;";
    let find_first = "begin Rec.FilterGroup := 2; Rec.SetRange(\"Batch Name\", CurrentBatch); Rec.FilterGroup := 0; if Rec.FindFirst() then; CurrPage.SaveRecord(); end;";
    let group_left = "begin Rec.FilterGroup := 2; Rec.SetRange(\"Batch Name\", CurrentBatch); if Rec.Find('-') then; CurrPage.SaveRecord(); end;";
    let update_false = "begin SetBatch(); CurrPage.Update(false); CurrPage.SaveRecord(); end;";
    let modify = "begin SetBatch(); Rec.Modify(); CurrPage.SaveRecord(); end;";
    let error_after = "begin SetBatch(); CurrPage.SaveRecord(); Error('after save'); end;";
    for validate in [get, find_first, group_left, update_false, modify, error_after] {
        assert_refused(
            Shape {
                validate,
                ..Shape::worksheet(WS_CELL)
            },
            OUTSIDE,
        );
    }
}

/// An OnModifyRecord other than the probe's (review of pull request 8536, round 2: it answers false, or writes and answers
/// either way): refused.
#[test]
fn saverecord_with_another_onmodifyrecord_is_refused() {
    let answers_false = "begin exit(false); end;";
    let insert_true = "var O: Record \"NL Other\"; begin O.Id := 1; O.Insert(); exit(true); end;";
    let modify_true = "begin Rec.Modify(); exit(true); end;";
    for modify_record in [answers_false, insert_true, modify_true] {
        assert_refused(
            Shape {
                modify_record,
                ..Shape::worksheet(WS_CELL)
            },
            OUTSIDE,
        );
    }
}

/// The table's OnModify bodies of review round 3: it writes a row of another table, of a table with an OnInsert or with a
/// subscriber, or another row of its own table; it sets a field; it raises. Refused on both pages.
const SUB: &str = r#"
table 99847 "NL Sub"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 99848 "NL Sub Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NL Sub", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertSub(var Rec: Record "NL Sub"; RunTrigger: Boolean) var Log: Codeunit "NL Log"; begin Log.Add('S.OnAfterInsert'); end;
}"#;

#[test]
fn saverecord_whose_table_onmodify_is_not_the_probes_is_refused() {
    let log = r#"Log.Add('L.OnModify:' + "Batch Name" + '/' + Format("Line No.") + '/' + Description);"#;
    let bodies = [
        format!("var O: Record \"NL Other\"; begin {log} O.Id := 1; O.Insert(); end;"),
        format!("begin {log} Description := 'om'; end;"),
        format!("begin {log} Error('onmodify boom'); end;"),
        format!(
            "var L2: Record \"NL Line2\"; begin {log} L2.\"Batch Name\" := 'Z'; L2.\"Line No.\" := 1; L2.Insert(true); end;"
        ),
        format!(
            "var Same: Record \"NL Line\"; begin {log} Same.Get('B2', 20000); Same.Description := 'om2'; Same.Modify(); end;"
        ),
        format!("var S: Record \"NL Sub\"; begin {log} S.Id := 1; S.Insert(); end;"),
    ];
    for body in &bodies {
        let on_modify: &'static str = Box::leak(body.clone().into_boxed_str());
        for shape in [Shape::list(LIST_CELL), Shape::worksheet(WS_CELL)] {
            assert_refused(
                Shape {
                    on_modify,
                    objects: SUB,
                    ..shape
                },
                OUTSIDE,
            );
        }
    }
}

/// Review round 3: an OnAfterGetCurrRecord that writes a row of another table during the refresh. Refused on both pages.
#[test]
fn saverecord_whose_onaftergetcurrrecord_writes_is_refused() {
    let agcr = r#"var AgcrOther: Record "NL Other"; begin Log.Add('C:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); if Rec."Batch Name" = 'B2' then begin AgcrOther.Id := 38; if AgcrOther.Insert() then; end; end;"#;
    for shape in [Shape::list(LIST_CELL), Shape::worksheet(WS_CELL)] {
        assert_refused(Shape { agcr, ..shape }, OUTSIDE);
    }
}

/// `CurrPage.Update(true)` outside the cell keeps main's save of the found row (Y50UpdateFromNewLine.al:610): on the Worksheet without
/// the first SaveRecord, after Update(false), on a page with another trigger, and with a table OnModify that writes elsewhere.
#[test]
fn update_true_outside_the_cell_keeps_mains_save() {
    let delete = "trigger OnDeleteRecord(): Boolean begin exit(true); end;";
    let writes = r#"var O: Record "NL Other"; begin Log.Add('L.OnModify:' + "Batch Name" + '/' + Format("Line No.") + '/' + Description); O.Id := 1; O.Insert(); end;"#;
    let update_false_then_true = r#"begin SetBatch(); CurrPage.Update(false); CurrPage.Update(true); end;"#;
    for shape in [
        Shape::worksheet("false, 2, false"),
        Shape {
            page_extra: delete,
            ..Shape::worksheet("false, 2, false")
        },
        Shape {
            on_modify: writes,
            ..Shape::list("true, 2, false")
        },
        Shape {
            validate: update_false_then_true,
            ..Shape::list(WS_CELL)
        },
    ] {
        assert_eq!(shape.outcome(), Outcome::Pass, "{}", shape.name());
    }
}
