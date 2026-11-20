//! al-runner issue 8523: `CurrPage.SaveRecord()` in the OnValidate of a control bound to a page variable, on the untouched
//! new-row line, after page code moved Rec onto a stored row (filter group 2 and Find('-')). The cells corpus pull request 1470
//! measured (StaleUpdateNewLine.al:498) save that row through assumption `new-line-saverecord-found-row`; the shapes next to
//! them refuse, and `CurrPage.Update(true)` keeps main's save.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// One page and one test: `page_type`, `extra` page code, the batch OnValidate's `step`, and whether a control's OnValidate
/// failed first (`boom`).
fn src(page_type: &str, extra: &str, step: &str, boom: bool) -> String {
    let boom = if boom { "if TryBoom(P) then;" } else { "" };
    format!(
        r#"
table 99840 "LT NL Line"
{{
    fields {{ field(1; "Batch Name"; Code[10]) {{ }} field(2; "Line No."; Integer) {{ }} field(3; Mods; Integer) {{ }} }}
    keys {{ key(PK; "Batch Name", "Line No.") {{ Clustered = true; }} }}
    trigger OnModify() begin Mods += 1; end;
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
                        SetBatch();
                        {step}
                    end;
                }}
                field(Boom; Boom) {{ trigger OnValidate() begin Error('boom'); end; }}
            }}
            repeater(Lines) {{ field("Line No."; Rec."Line No.") {{ }} field(Mods; Rec.Mods) {{ }} }}
        }}
    }}
    trigger OnOpenPage() begin CurrentBatch := 'B3'; SetBatch(); end;
    trigger OnAfterGetRecord() begin end;
    trigger OnAfterGetCurrRecord() begin end;
    trigger OnNewRecord(BelowxRec: Boolean) begin end;
    trigger OnModifyRecord(): Boolean begin exit(true); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin exit(true); end;
    {extra}
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

codeunit 99842 T
{{
    Subtype = Test;

    [Test]
    procedure Switch()
    var
        Line: Record "LT NL Line";
        P: TestPage "LT NL Page";
    begin
        Line."Batch Name" := 'B2';
        Line."Line No." := 10000;
        Line.Insert();
        Line."Line No." := 20000;
        Line.Insert();
        P.OpenEdit();
        if P."Line No.".Value() <> '' then;
        {boom}
        P.CurrentBatch.SetValue('B2');
        Line.Get('B2', 10000);
        if Line.Mods <> 1 then
            Error('Mods %1', Line.Mods);
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

fn outcome(src: &str) -> Outcome {
    let o = outcomes_in(src, "T");
    assert_eq!(o.len(), 1, "{o:?}");
    o.into_iter().next().unwrap().1
}

fn assert_refused(o: &Outcome) {
    match o {
        Outcome::NotRun(r) => assert!(r.contains("CurrPage.SaveRecord() on the new-row line"), "{r}"),
        other => panic!("expected the refusal, got {other:?}"),
    }
}

/// The measured cell, on a Worksheet and on a List: the found row is modified once (its OnModify runs).
#[test]
fn saverecord_in_the_measured_cell_modifies_the_found_row() {
    for page_type in ["Worksheet", "List"] {
        assert_eq!(
            outcome(&src(page_type, "", "CurrPage.SaveRecord();", false)),
            Outcome::Pass,
            "{page_type}"
        );
    }
}

/// A page type, a trigger and an earlier failed step the probe did not measure for SaveRecord: refused.
#[test]
fn saverecord_next_to_the_cell_is_refused() {
    let delete = "trigger OnDeleteRecord(): Boolean begin exit(true); end;";
    assert_refused(&outcome(&src("Worksheet", delete, "CurrPage.SaveRecord();", false)));
    assert_refused(&outcome(&src("Document", "", "CurrPage.SaveRecord();", false)));
    assert_refused(&outcome(&src("Worksheet", "", "CurrPage.SaveRecord();", true)));
}

/// `CurrPage.Update(true)` outside the cell keeps main's save of the found row (Y50UpdateFromNewLine.al:610).
#[test]
fn update_true_outside_the_cell_keeps_mains_save() {
    let delete = "trigger OnDeleteRecord(): Boolean begin exit(true); end;";
    assert_eq!(
        outcome(&src("Worksheet", delete, "CurrPage.Update(true);", false)),
        Outcome::Pass
    );
}
