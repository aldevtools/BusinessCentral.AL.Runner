//! Evaluate into an Enum whose first member is named (`None`) with a blank Caption (`' '`) or an empty one, the shape of
//! al-runner issue 6924 (`Enum "META Record Modification Type"`, value None): the text Format gives for it reads back.

use al_testkit::text;

const SRC: &str = r#"
enum 69811 "NB E Space" { value(0; None) { Caption = ' '; } value(1; Insert) { Caption = 'Insert'; } }
enum 69812 "NB E Empty" { value(0; None) { Caption = ''; } value(1; Insert) { Caption = 'Insert'; } }
enum 69813 "NB E Pinned" { value(0; " ") { Caption = ' '; } value(1; Insert) { Caption = 'Insert'; } }
codeunit 69810 C
{
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
        Err(al_builtins::native::Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// Real BC accepts the text Format gave for a member named None with a blank caption (al-runner issue 6924, a partner test),
// but no corpus test pins it yet (the pinned blank members are named " " or ""): refused until the probe lands
// (corpus pull request 1283, docs/probes-needed.md).
#[test]
fn blank_text_into_a_named_member_with_a_blank_caption_is_refused_not_rejected() {
    for r in ["SpaceRoundTrip", "SpaceFromSpace", "SpaceFromEmpty"] {
        assert!(refused(r).contains("blank caption"), "{r}");
    }
}

// Pinned shapes keep working: a member named " " (corpus format-evaluate/EvalLanguageBoolean.al WA_BlankEnum2) and a
// Evaluate of a blank text into a named member with a blank caption is refused.
#[test]
fn the_pinned_blank_members_still_read_blank_text() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "PinnedFromSpace")), "[ ]Yes|0");
}
