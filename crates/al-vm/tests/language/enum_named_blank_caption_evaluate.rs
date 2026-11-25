//! Evaluate into an Enum whose first member is named (`None`) with a blank Caption (`' '`) or an empty one, the shape of
//! al-runner issue 6924 (`Enum "META Record Modification Type"`, value None): the text Format gives for it reads back.

use al_testkit::text;

const SRC: &str = r#"
enum 69811 "NB E Space" { value(0; None) { Caption = ' '; } value(1; Insert) { Caption = 'Insert'; } }
enum 69812 "NB E Empty" { value(0; None) { Caption = ''; } value(1; Insert) { Caption = 'Insert'; } }
enum 69815 "NB E Last" { Extensible = false; value(0; Alpha) { Caption = 'Alpha cap'; } value(5; Beta) { Caption = 'Beta cap'; } value(10; Blank) { Caption = ''; } }
enum 69816 "NB E Mid" { value(0; Alpha) { Caption = 'Alpha cap'; } value(1; None) { Caption = ' '; } value(2; Beta) { Caption = 'Beta cap'; } }
enum 69813 "NB E Pinned" { value(0; " ") { Caption = ' '; } value(1; Insert) { Caption = 'Insert'; } }
table 69814 "NB Option Row"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = None,First,Second; OptionCaption = ' ,First cap,Second cap'; }
        field(3; Pinned; Option) { OptionMembers = " ",First,Second; OptionCaption = ' ,First cap,Second cap'; }
        field(4; Reporter; Option) { OptionMembers = First,Second,Third,,,Skip; OptionCaption = 'First one,Second one,Third one,,,Skip it'; }
        field(5; Last; Enum "NB E Last") { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69810 C
{
    var GlobalLast: Enum "NB E Last";

    procedure LastField(): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Last := "NB E Last"::Beta;
        R := Evaluate(Row.Last, '');
        exit(Format(R));
    end;

    procedure LastGlobal(): Text
    var R: Boolean;
    begin
        GlobalLast := "NB E Last"::Beta;
        R := Evaluate(GlobalLast, '');
        exit(Format(R));
    end;

    procedure LastParameter(): Text
    var F: Enum "NB E Last";
    begin
        exit(EvaluateInto(F));
    end;

    local procedure EvaluateInto(var F: Enum "NB E Last"): Text
    var R: Boolean;
    begin
        R := Evaluate(F, '');
        exit(Format(R));
    end;

    procedure LastFieldRef(): Text
    var Row: Record "NB Option Row"; RecRef: RecordRef; FldRef: FieldRef; R: Boolean;
    begin
        RecRef.GetTable(Row);
        FldRef := RecRef.Field(5);
        R := Evaluate(FldRef, '');
        exit(Format(R));
    end;

    procedure SpaceRoundTrip(): Text
    var E: Enum "NB E Space"; F: Enum "NB E Space"; T: Text; R: Boolean;
    begin
        T := Format(E);
        F := "NB E Space"::Insert;
        R := Evaluate(F, T);
        exit('[' + T + ']' + Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure SpaceFromSpace(): Text
    var F: Enum "NB E Space"; R: Boolean;
    begin
        F := "NB E Space"::Insert;
        R := Evaluate(F, ' ');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure SpaceFromEmpty(): Text
    var F: Enum "NB E Space"; R: Boolean;
    begin
        F := "NB E Space"::Insert;
        R := Evaluate(F, '');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure EmptyRoundTrip(): Text
    var E: Enum "NB E Empty"; F: Enum "NB E Empty"; T: Text; R: Boolean;
    begin
        T := Format(E);
        F := "NB E Empty"::Insert;
        R := Evaluate(F, T);
        exit('[' + T + ']' + Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure PinnedFromSpace(): Text
    var E: Enum "NB E Pinned"; F: Enum "NB E Pinned"; T: Text; R: Boolean; J: JsonObject; Tok: JsonToken;
    begin
        T := Format(E);
        J.Add('Type', T);
        J.Get('Type', Tok);
        F := "NB E Pinned"::Insert;
        R := Evaluate(F, Tok.AsValue().AsText());
        exit('[' + T + ']' + Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure EmptyFromEmpty(): Text
    var F: Enum "NB E Empty"; R: Boolean;
    begin
        F := "NB E Empty"::Insert;
        R := Evaluate(F, '');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure EmptyFromName(): Text
    var F: Enum "NB E Empty"; R: Boolean;
    begin
        F := "NB E Empty"::Insert;
        R := Evaluate(F, 'none');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure OptionFromEmpty(): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Kind := Row.Kind::Second;
        R := Evaluate(Row.Kind, '');
        exit(Format(R) + '|' + Format(Row.Kind, 0, 9));
    end;

    procedure OptionFromSpace(): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Kind := Row.Kind::Second;
        R := Evaluate(Row.Kind, ' ');
        exit(Format(R) + '|' + Format(Row.Kind, 0, 9));
    end;

    procedure OptionFromName(): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Kind := Row.Kind::Second;
        R := Evaluate(Row.Kind, 'None');
        exit(Format(R) + '|' + Format(Row.Kind, 0, 9));
    end;

    procedure PinnedOptionFromEmpty(): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Pinned := Row.Pinned::Second;
        R := Evaluate(Row.Pinned, '');
        exit(Format(R) + '|' + Format(Row.Pinned, 0, 9));
    end;

    procedure ReporterOption(T: Text): Text
    var Row: Record "NB Option Row"; R: Boolean;
    begin
        Row.Reporter := Row.Reporter::First;
        R := Evaluate(Row.Reporter, T);
        exit(Format(R) + '|' + Format(Row.Reporter, 0, 9));
    end;

    procedure ReporterOptionStatement(): Text
    var Row: Record "NB Option Row";
    begin
        Row.Reporter := Row.Reporter::First;
        Evaluate(Row.Reporter, '');
        exit(Format(Row.Reporter, 0, 9));
    end;

    procedure ReporterOptionFieldRef(): Text
    var Row: Record "NB Option Row"; RecRef: RecordRef; FldRef: FieldRef; R: Boolean;
    begin
        RecRef.GetTable(Row);
        FldRef := RecRef.Field(4);
        R := Evaluate(FldRef, '');
        exit(Format(R));
    end;

    procedure LastFromEmpty(): Text
    var F: Enum "NB E Last"; R: Boolean;
    begin
        F := "NB E Last"::Beta;
        R := Evaluate(F, '');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure LastFromTab(): Text
    var F: Enum "NB E Last"; R: Boolean; T: Text;
    begin
        T[1] := 9;
        F := "NB E Last"::Beta;
        R := Evaluate(F, T);
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure MidFromEmpty(): Text
    var F: Enum "NB E Mid"; R: Boolean;
    begin
        F := "NB E Mid"::Beta;
        R := Evaluate(F, '');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;

    procedure EmptyStatement(): Text
    var F: Enum "NB E Empty";
    begin
        F := "NB E Empty"::Insert;
        Evaluate(F, '');
        exit(Format(F.AsInteger()));
    end;

    procedure EmptyFromSpace(): Text
    var F: Enum "NB E Empty"; R: Boolean;
    begin
        F := "NB E Empty"::Insert;
        R := Evaluate(F, ' ');
        exit(Format(R) + '|' + Format(F.AsInteger()));
    end;
}
"#;

fn refused(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// A first member named None with Caption `' '` or `''` reads the blank text, on the assumption
// `enum-blank-text-named-blank-caption` (corpus PR 1306, format-evaluate/EvalNamedBlankCaption.al:208-211, :383-385, merged,
// not yet pinned). The round trip of the `''` caption goes through the text Format gives for it, which is not blank.
#[test]
fn blank_text_into_a_first_member_with_a_blank_caption_names_it() {
    for r in [
        "SpaceRoundTrip",
        "SpaceFromSpace",
        "SpaceFromEmpty",
        "EmptyFromSpace",
        "EmptyFromEmpty",
        "EmptyRoundTrip",
    ] {
        assert!(text(al_testkit::run_in(SRC, "C", r)).ends_with("Yes|0"), "{r}");
    }
    assert_eq!(text(al_testkit::run_in(SRC, "C", "SpaceRoundTrip")), "[ ]Yes|0");
}

// Pinned shapes keep working: a member named " " (corpus format-evaluate/EvalLanguageBoolean.al WA_BlankEnum2).
#[test]
fn the_pinned_blank_members_still_read_blank_text() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "PinnedFromSpace")), "[ ]Yes|0");
}

// The first member with a name and a blank Caption, wherever it is (al-runner issue 8356, the reporter's repro, BC 28.4 and
// 29.0: true and member 10; corpus PR 1577 OBC_Enum_Last, OBC_Enum_SpaceMid: a TAB, a middle member, `''` as a statement), on
// the same assumption.
#[test]
fn blank_text_into_the_first_named_member_with_a_blank_caption_names_it() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "LastFromEmpty")), "Yes|10");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "LastFromTab")), "Yes|10");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "MidFromEmpty")), "Yes|1");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EmptyStatement")), "0");
    // An Enum field named directly (OBC_Enum_Field).
    assert_eq!(text(al_testkit::run_in(SRC, "C", "LastField")), "Yes");
}

// Measured on an Enum variable of the routine's `var` section and an Enum field only: a FieldRef, a global and a parameter
// refuse.
#[test]
fn blank_text_into_a_named_blank_caption_member_of_another_place_is_refused() {
    for r in ["LastFieldRef", "LastGlobal", "LastParameter"] {
        assert!(refused(r).contains("blank caption"), "{r}");
    }
}

// A member with a name and the caption `' '` of an Option field is named by a blank text (corpus PR 1577
// OBC_Option_NamedSpaceCapMid), on the assumption `option-blank-text-blank-captions`.
#[test]
fn blank_text_into_an_option_field_whose_named_member_has_a_space_caption_names_it() {
    for r in ["OptionFromEmpty", "OptionFromSpace"] {
        assert_eq!(text(al_testkit::run_in(SRC, "C", r)), "Yes|0", "{r}");
    }
}

// Members named "" with caption `''` in the middle (al-runner issue 8354, the reporter's repro, BC 28.4 and 29.0; corpus PR
// 1577 OBC_Option_Reporter): `''` and `' '` give false and keep the value, `''` as a statement raises the error of the field,
// on the assumption `option-blank-text-blank-captions`. A FieldRef is not measured: refused.
#[test]
fn blank_text_into_option_members_named_empty_with_empty_captions_is_rejected() {
    for t in ["", " "] {
        let r = al_testkit::run_with(
            SRC,
            "C",
            "ReporterOption",
            vec![al_builtins::value::Value::Text(t.into())],
        )
        .map(|(v, _)| v);
        assert_eq!(text(r), "No|0", "[{t}]");
    }
    match al_testkit::run_in(SRC, "C", "ReporterOptionStatement") {
        Err(al_builtins::native::Signal::Error(e)) => assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "'' is not an option. The existing options are: First one,Second one,Third one,,,Skip it",
                "InvalidOptionString"
            )
        ),
        other => panic!("expected the error, got {other:?}"),
    }
    assert!(refused("ReporterOptionFieldRef").contains("blank member names and captions"));
}

// A member name still reads (types/ProbeTypeFormsAndFilters.al Enum_Evaluate_Forms; recordref-fieldref/P128EvalBasic.al
// P128_Evaluate_IntoOptionField_NumericCaptions `Two`), and so does the blank text into a member named " " with a blank
// caption (format-evaluate/P131EvalOption.al:107-108).
#[test]
fn the_pinned_forms_of_these_shapes_still_read() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EmptyFromName")), "Yes|0");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "OptionFromName")), "Yes|0");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "PinnedOptionFromEmpty")), "Yes|0");
}
