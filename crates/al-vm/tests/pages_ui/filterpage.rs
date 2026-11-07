//! `FilterPageBuilder`: an in-memory builder of named filter controls. Each test names the corpus
//! test (pages/filterpagebuilder/TestFilterPageBuilder.al) that pins it.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const FIXTURES: &str = r#"
table 60000 "ALT Universal"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(3; "Integer Field"; Integer) { }
        field(14; "Option Field"; Option) { OptionMembers = " ",Draft,Active,Closed; }
    }
    keys { key(PK; "Entry No.") { } }
}

table 60001 "ALT Composite"
{
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { } }
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{FIXTURES}\ncodeunit 50100 C {{ {body} }}");
    al_testkit::run_in(&src, "C", "F")
}

fn f(vars_and_body: &str) -> String {
    match run(&format!("procedure F(): Text {vars_and_body}")) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn refusal(vars_and_body: &str) -> String {
    match run(&format!("procedure F(): Text {vars_and_body}")) {
        Err(Signal::Unsupported(r)) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// :AddTable_ReturnsTheControlName, :AddRecord_..., :AddRecordRef_...
#[test]
fn the_add_methods_return_the_name_and_count_the_control() {
    assert_eq!(
        f("var B: FilterPageBuilder; U: Record \"ALT Universal\"; R: RecordRef; B2: FilterPageBuilder; B3: FilterPageBuilder;
        begin
            R.Open(60000);
            exit(B.AddTable('Universal', Database::\"ALT Universal\") + '|' + Format(B.Count()) + '|' +
                 B2.AddRecord('FromRecord', U) + '|' + Format(B2.Count()) + '|' +
                 B3.AddRecordRef('FromRef', R) + '|' + Format(B3.Count()));
        end;"),
        "Universal|1|FromRecord|1|FromRef|1"
    );
}

// :AddTable_ThreeDistinctNames_CountIsThree, :SameNameSameTable_IsIdempotent
#[test]
fn names_are_the_identity() {
    assert_eq!(
        f("var B: FilterPageBuilder; begin
            B.AddTable('One', Database::\"ALT Universal\");
            B.AddTable('Two', Database::\"ALT Composite\");
            B.AddTable('Three', Database::\"ALT Universal\");
            B.AddTable('Three', Database::\"ALT Universal\");
            exit(Format(B.Count()));
        end;"),
        "3"
    );
}

// :SameNameDifferentTable_Throws names the control; the rest of the text is not pinned
#[test]
fn a_name_cannot_be_reused_for_another_table() {
    assert_eq!(
        f("var B: FilterPageBuilder; begin
            B.AddTable('Clash', Database::\"ALT Universal\");
            asserterror B.AddTable('Clash', Database::\"ALT Composite\");
            exit(Format(StrPos(GetLastErrorText(), 'Clash') > 0) + Format(B.Count()));
        end;"),
        "Yes1"
    );
}

// :AddTable_ZeroTableId_Throws, :AddTable_ReturnCaptured_BadTableId_ReturnsEmpty
#[test]
fn a_bad_table_id_raises_unless_the_result_is_used() {
    assert_eq!(
        f("var B: FilterPageBuilder; N: Text; begin
            asserterror B.AddTable('Bad', 0);
            N := GetLastErrorText();
            exit(N + '|' + Format(B.Count()) + '|' + B.AddTable('Trapped', 0) + '|' + Format(B.Count()));
        end;"),
        "The filter page table ID must be a positive number greater than zero.|0||0"
    );
}

// :AddField_KnownControl_ReturnsTrue, :AddField_UnknownControl_ReturnsFalse
#[test]
fn add_field_answers_whether_the_control_exists() {
    assert_eq!(
        f("var B: FilterPageBuilder; R: RecordRef; Fld: FieldRef; begin
            R.Open(60000);
            B.AddRecordRef('Ref', R);
            Fld := R.Field(3);
            exit(Format(B.AddField('Ref', Fld)) + '|' + Format(B.AddField('NoSuchControl', Fld)) + '|' + Format(B.Count()));
        end;"),
        "Yes|No|1"
    );
}

// :AddField_DefaultFilter_ReachesTheView, :AddField_NoDefaultFilter_ViewHasNoFilterValue
#[test]
fn the_default_filter_of_add_field_reaches_the_view() {
    assert_eq!(
        f(
            "var B: FilterPageBuilder; R: RecordRef; Fld: FieldRef; WithF, WithoutF: Text; begin
            R.Open(60000);
            B.AddRecordRef('Ref', R);
            Fld := R.Field(3);
            WithoutF := B.GetView('Ref');
            B.AddField('Ref', Fld, '>41');
            WithF := B.GetView('Ref');
            exit(Format(StrPos(WithF, '>41') > 0) + Format(StrPos(WithoutF, '>41') > 0));
        end;"
        ),
        "YesNo"
    );
}

// :SetView_ThenGetView_RoundTripsTheFilter
#[test]
fn set_view_round_trips() {
    assert_eq!(
        f("var B: FilterPageBuilder; begin
            B.AddTable('Universal', Database::\"ALT Universal\");
            exit(Format(B.SetView('Universal', 'WHERE(\"Integer Field\"=FILTER(>4711))')) + '|' + B.GetView('Universal'));
        end;"),
        "Yes|VERSION(1) SORTING(Entry No.) WHERE(Integer Field=FILTER(>4711))"
    );
}

// :GetView_UnknownControl_ReturnsEmpty, :SetView_UnknownControl_Throws, :SetView_ReturnCaptured_...,
// :SetView_EmptyView_ReturnsFalse
#[test]
fn get_view_and_set_view_differ_on_an_unknown_name() {
    assert_eq!(
        f("var B: FilterPageBuilder; Captured: Boolean; Err: Text; begin
            B.AddTable('Known', Database::\"ALT Universal\");
            asserterror B.SetView('Unknown', 'WHERE(\"Integer Field\"=FILTER(>1))');
            Err := GetLastErrorText();
            Captured := B.SetView('Unknown', 'WHERE(\"Integer Field\"=FILTER(>1))');
            exit('[' + B.GetView('Unknown') + ']|' + Format(StrPos(Err, 'Unknown') > 0) + '|' + Format(Captured) +
                 '|' + Format(B.SetView('Known', '')));
        end;"),
        "[]|Yes|No|No"
    );
}

// :GetView_CaptionsFlag_RendersAnOptionFilterDifferently; without names the filter takes the numeric form `FieldN=1(...)`
// (metadata/platform-tables/PlTFilterPageBuilder.al:177-180 GetSetView_RoundTrip).
#[test]
fn use_names_spells_an_option_filter_as_a_name_or_an_ordinal() {
    assert_eq!(
        f("var B: FilterPageBuilder; R: RecordRef; Fld: FieldRef; begin
            R.Open(60000);
            B.AddRecordRef('Ref', R);
            Fld := R.Field(14);
            B.AddField('Ref', Fld, 'Active');
            exit(B.GetView('Ref', true) + '|' + B.GetView('Ref', false));
        end;"),
        "VERSION(1) SORTING(Entry No.) WHERE(Option Field=FILTER(Active))|VERSION(1) SORTING(Field1) WHERE(Field14=1(2))"
    );
}

// :Name_IsOneBasedAndInInsertionOrder, :Name_IndexZero_Throws, :Name_IndexPastEnd_Throws
#[test]
fn name_is_one_based_and_range_checked() {
    assert_eq!(
        f("var B: FilterPageBuilder; Unused: Text; E0, E2: Text; begin
            B.AddTable('Alpha', Database::\"ALT Universal\");
            B.AddTable('Beta', Database::\"ALT Composite\");
            asserterror Unused := B.Name(0);
            E0 := GetLastErrorText();
            Clear(B);
            B.AddTable('Alpha', Database::\"ALT Universal\");
            asserterror Unused := B.Name(2);
            E2 := GetLastErrorText();
            exit(E0 + '|' + E2);
        end;"),
        "The filter control index value 0 is out of range. The value must be greater than or equal to 1 and less than or equal to 2.|\
         The filter control index value 2 is out of range. The value must be greater than or equal to 1 and less than or equal to 1."
    );
    assert_eq!(
        f("var B: FilterPageBuilder; begin
            B.AddTable('Alpha', Database::\"ALT Universal\");
            B.AddTable('Beta', Database::\"ALT Composite\");
            exit(B.Name(1) + B.Name(2));
        end;"),
        "AlphaBeta"
    );
}

// :PageCaption_FreshBuilder_IsNonEmptyDefault, :Assigned_ReplacesTheDefault, :LastWriteWins
#[test]
fn the_page_caption_has_a_default_and_the_last_write_wins() {
    assert_eq!(
        f("var B: FilterPageBuilder; D: Text; begin
            D := B.PageCaption();
            B.PageCaption('First caption');
            B.PageCaption('Second caption');
            exit(Format(D <> '') + '|' + B.PageCaption());
        end;"),
        "Yes|Second caption"
    );
}

// :Assignment_CopiesTheControlCollection, :CopiesTheExistingViews, :ViewOnCopyDoesReachOriginal
#[test]
fn assignment_copies_the_controls_and_shares_the_records() {
    assert_eq!(
        f("var A, C: FilterPageBuilder; begin
            A.AddTable('Shared', Database::\"ALT Universal\");
            A.SetView('Shared', 'WHERE(\"Integer Field\"=FILTER(>4711))');
            C := A;
            C.AddTable('OnlyOnCopy', Database::\"ALT Composite\");
            exit(Format(C.Count()) + '|' + Format(A.Count()) + '|' + A.Name(1) + '|' + C.GetView('Shared'));
        end;"),
        "2|1|Shared|VERSION(1) SORTING(Entry No.) WHERE(Integer Field=FILTER(>4711))"
    );
    assert_eq!(
        f("var A, C: FilterPageBuilder; begin
            A.AddTable('Universal', Database::\"ALT Universal\");
            C := A;
            C.SetView('Universal', 'WHERE(\"Integer Field\"=FILTER(>4711))');
            exit(A.GetView('Universal'));
        end;"),
        "VERSION(1) SORTING(Entry No.) WHERE(Integer Field=FILTER(>4711))"
    );
}

// :Clear_EmptiesTheControls, :Clear_KeepsTheAssignedCaption, :Clear_ThenReuse_Works
#[test]
fn clear_empties_the_controls_but_keeps_the_caption() {
    assert_eq!(
        f("var B: FilterPageBuilder; begin
            B.AddTable('Alpha', Database::\"ALT Universal\");
            B.PageCaption('Pick your rows');
            Clear(B);
            exit(Format(B.Count()) + '|[' + B.GetView('Alpha') + ']|' + B.PageCaption() + '|' +
                 B.AddTable('Alpha', Database::\"ALT Composite\") + '|' + Format(B.Count()));
        end;"),
        "0|[]|Pick your rows|Alpha|1"
    );
}

// :Variant_RoundTripsThroughAVariant, :Format_ReturnsANonEmptyRendering
#[test]
fn a_builder_survives_a_variant_and_formats() {
    assert_eq!(
        f("var B, Back: FilterPageBuilder; V: Variant; begin
            B.AddTable('Alpha', Database::\"ALT Universal\");
            V := B;
            Back := V;
            exit(Format(Back.Count()) + Back.Name(1) + '|' + Format(Format(B) <> ''));
        end;"),
        "1Alpha|Yes"
    );
}

// RunModal needs a client, which the corpus never provides: refused (docs/probes-needed.md)
#[test]
fn run_modal_is_refused() {
    let why = refusal(
        "var B: FilterPageBuilder; begin
            B.AddTable('Alpha', Database::\"ALT Universal\");
            B.RunModal();
            exit('');
        end;",
    );
    assert!(why.contains("RunModal"), "{why}");
}
