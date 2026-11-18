//! A table field whose CaptionClass expression reads the caption of that same field (al-runner issue 8329). What BC's nested
//! read gives is not pinned, so the read refuses; it used to run the expression again and again until the stack ran out.

use al_builtins::native::Signal;
use al_builtins::value::Value;

/// The objects of the reporter's repro (al-runner issue 8329), plus neighbors: a field whose expression reads another field's
/// caption, two fields whose expressions read each other's, and (another table) a field whose expression reads its own caption
/// by FieldRef.
const SRC: &str = r#"
table 50100 "ALR Caption Repro"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "End Date"; Date)
        {
            Caption = 'End Date';
            CaptionClass = '3,' + FieldCaption("End Date");
        }
        field(3; "Other Ref"; Date) { Caption = 'Other Ref'; CaptionClass = '3,' + FieldCaption("Code"); }
        field(4; "Ping"; Date) { Caption = 'Ping'; CaptionClass = '3,' + FieldCaption("Pong"); }
        field(5; "Pong"; Date) { Caption = 'Pong'; CaptionClass = '3,' + FieldCaption("Ping"); }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 50102 "ALR Ref Repro"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Ref Self"; Date) { Caption = 'Ref Self'; CaptionClass = '3,' + RefCaption(2); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure RefCaption(No: Integer): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(Rec);
        exit(RecRef.Field(No).Caption);
    end;
}

page 50100 "ALR Caption Repro Card"
{
    PageType = Card;
    SourceTable = "ALR Caption Repro";
    UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } field("End Date"; Rec."End Date") { } } }
}

codeunit 50101 C
{
    procedure SelfFieldCaption(): Text
    var
        Rec: Record "ALR Caption Repro";
    begin
        exit(Rec.FieldCaption("End Date"));
    end;

    procedure SelfTestPageCaption(): Text
    var
        Card: TestPage "ALR Caption Repro Card";
    begin
        Card.OpenNew();
        exit(Card."End Date".Caption);
    end;

    procedure SelfFieldRefCaption(): Text
    var
        RecRef: RecordRef;
    begin
        RecRef.Open(Database::"ALR Caption Repro");
        exit(RecRef.Field(2).Caption);
    end;

    procedure NestedFieldRefCaption(): Text
    var
        Rec: Record "ALR Ref Repro";
    begin
        exit(Rec.FieldCaption("Ref Self"));
    end;

    procedure MutualFieldCaption(): Text
    var
        Rec: Record "ALR Caption Repro";
    begin
        exit(Rec.FieldCaption("Ping"));
    end;

    procedure OtherFieldCaption(): Text
    var
        Rec: Record "ALR Caption Repro";
    begin
        exit(Rec.FieldCaption("Other Ref"));
    end;
}
"#;

fn out(routine: &str) -> String {
    match al_testkit::Built::from_source(SRC).run_in("C", routine) {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

fn assert_refused(routine: &str) {
    let got = out(routine);
    assert!(
        got.starts_with("refused:") && got.contains("its own CaptionClass expression runs"),
        "{routine}: {got}"
    );
}

#[test]
fn field_caption_inside_its_own_caption_class_refuses() {
    assert_refused("SelfFieldCaption");
}

#[test]
fn testpage_caption_of_a_self_referencing_caption_class_refuses() {
    assert_refused("SelfTestPageCaption");
}

/// A FieldRef.Caption reader of the self-reading field goes through the same check.
#[test]
fn field_ref_caption_of_a_self_reading_caption_class_refuses() {
    assert_refused("SelfFieldRefCaption");
}

/// The nested read inside the expression is a FieldRef.Caption of the same field.
#[test]
fn a_nested_field_ref_caption_of_the_same_field_refuses() {
    assert_refused("NestedFieldRefCaption");
}

#[test]
fn caption_classes_that_read_each_other_refuse() {
    assert_refused("MutualFieldCaption");
}

/// Main's result is kept: an expression that reads another field's caption runs that read as before.
#[test]
fn caption_class_reading_another_field_keeps_mains_result() {
    assert_eq!(out("OtherFieldCaption"), "Code");
}
