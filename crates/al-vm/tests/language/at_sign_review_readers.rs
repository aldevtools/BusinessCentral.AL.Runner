//! Review neighbors of the direct CalcFields cells of AtSignAloneTypes.al.

use al_builtins::native::Signal;

const SRC: &str = r#"
enum 69630 "At Sign Review Kind"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}
enum 69631 "At Sign Review Extended Kind"
{
    Extensible = true;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}
enumextension 69631 "At Sign Review Kind Ext" extends "At Sign Review Extended Kind"
{
    value(50000; Extra) { Caption = 'Extra'; }
}
enum 69632 "At Sign Review Open Kind"
{
    Extensible = true;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}
enum 69633 "At Sign Review Out Of Order Kind"
{
    Extensible = false;
    value(1; Alpha) { Caption = 'First'; }
    value(0; None) { }
    value(2; Beta) { Caption = 'Second'; }
}
enum 69634 "At Sign Review Blank Kind"
{
    Extensible = false;
    value(0; " ") { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}
enum 69635 "At Sign Review Caption ML Kind"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { CaptionML = ENU = 'First'; }
    value(2; Beta) { CaptionML = ENU = 'Second'; }
}
table 69630 "At Sign Review Reader Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { }
        field(3; Tm; Time) { }
        field(4; Uid; Guid) { }
        field(5; Opt; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Beta'; }
        field(6; Other Opt; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Gamma'; }
        field(7; Ml Opt; Option) { OptionMembers = A,B; OptionCaptionML = ENU = 'Alpha,Beta'; }
        field(8; Kind; Enum "At Sign Review Kind") { }
        field(9; Extended Kind; Enum "At Sign Review Extended Kind") { }
        field(10; Plain Opt; Option) { OptionMembers = A,B; }
        field(11; Blank Opt; Option) { OptionMembers = A,B; OptionCaption = 'Al pha,Beta'; }
        field(15; Empty Opt; Option) { OptionMembers = A,B; OptionCaption = ',Beta'; }
        field(16; Caption Ml Kind; Enum "At Sign Review Caption ML Kind") { }
        field(12; Open Kind; Enum "At Sign Review Open Kind") { }
        field(13; Order Kind; Enum "At Sign Review Out Of Order Kind") { }
        field(14; Blank Kind; Enum "At Sign Review Blank Kind") { }
    }
    keys { key(PK; "No.") { } }
}
table 69631 "At Sign Review Reader Host"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Time Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), Tm = filter('@'))); }
        field(3; "Time Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), Tm = filter('<>@'))); }
        field(4; "Time Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("At Sign Review Reader Row".Amount where("No." = field("No."), Tm = filter('@'))); }
        field(5; "Guid Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), Uid = filter('@'))); }
        field(6; "Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), Opt = filter('@'))); }
        field(7; "Other Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Other Opt" = filter('@'))); }
        field(8; "Ml Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Ml Opt" = filter('@'))); }
        field(9; "Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), Kind = filter('@'))); }
        field(10; "Extended Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Extended Kind" = filter('@'))); }
        field(11; "Plain Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Plain Opt" = filter('@'))); }
        field(12; "Blank Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Blank Opt" = filter('@'))); }
        field(13; "Open Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Open Kind" = filter('@'))); }
        field(14; "Order Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Order Kind" = filter('@'))); }
        field(15; "Blank Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Blank Kind" = filter('@'))); }
        field(16; "Empty Option Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Empty Opt" = filter('@'))); }
        field(17; "Caption Ml Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("At Sign Review Reader Row" where("No." = field("No."), "Caption Ml Kind" = filter('@'))); }
    }
    keys { key(PK; "No.") { } }
}
query 69630 "At Sign Review Time"
{
    elements
    {
        dataitem(Host; "At Sign Review Reader Host")
        {
            column("No"; "No.") { }
            column(TimeAt; "Time Count At") { }
        }
    }
}
codeunit 69630 "At Sign Review Readers"
{
    local procedure NewHost(var Host: Record "At Sign Review Reader Host"): Code[20]
    var Row: Record "At Sign Review Reader Row";
    begin
        Row."No." := 'R';
        Row.Amount := 10;
        Row.Insert();
        Host."No." := Row."No.";
        Host.Insert();
        exit(Row."No.");
    end;

    procedure AutoCalc()
    var Host: Record "At Sign Review Reader Host"; G: Code[20];
    begin
        G := NewHost(Host);
        Host.SetAutoCalcFields("Time Count At");
        Host.SetRange("No.", G);
        Host.FindFirst();
    end;

    procedure DirectShapes()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Time Count At");
        asserterror Host.CalcFields("Guid Count At");
        asserterror Host.CalcFields("Plain Option Count At");
        asserterror Host.CalcFields("Option Count At");
        asserterror Host.CalcFields("Enum Count At");
    end;

    procedure FlowFilter()
    var Host: Record "At Sign Review Reader Host"; G: Code[20];
    begin
        G := NewHost(Host);
        Host.SetRange("No.", G);
        Host.SetRange("Time Count At", 0);
        Host.Count();
    end;

    procedure QueryRead()
    var
        Host: Record "At Sign Review Reader Host";
        Q: Query "At Sign Review Time";
    begin
        NewHost(Host);
        Q.Open();
        Q.Read();
        Q.Close();
    end;

    procedure TemporaryHost()
    var Host: Record "At Sign Review Reader Host" temporary;
    begin
        Host."No." := 'T';
        Host.Insert();
        asserterror Host.CalcFields("Time Count At");
    end;

    procedure Language1031Sum()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Time Sum At");
    end;

    procedure Language1030NegatedCount()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1030);
        asserterror Host.CalcFields("Time Count Not At");
    end;

    procedure LanguageCells()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Time Count At");
        GlobalLanguage(1030);
        asserterror Host.CalcFields("Time Count At");
    end;

    procedure TranslatedCaption()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        GlobalLanguage(1031);
        asserterror Host.CalcFields("Option Count At");
    end;

    procedure OtherCaption()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Other Option Count At");
    end;

    procedure CaptionML()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Ml Option Count At");
    end;

    procedure EnumCaptionML()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Caption Ml Enum Count At");
    end;

    procedure ExtendedEnum()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Extended Enum Count At");
    end;

    procedure BlankCaption()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Blank Option Count At");
    end;

    procedure EmptyCaption()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Empty Option Count At");
    end;

    procedure OpenEnum()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Open Enum Count At");
    end;

    procedure OutOfOrderEnum()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Order Enum Count At");
    end;

    procedure BlankEnum()
    var Host: Record "At Sign Review Reader Host";
    begin
        NewHost(Host);
        asserterror Host.CalcFields("Blank Enum Count At");
    end;
}
"#;

fn refused(procedure: &str) -> String {
    match al_testkit::run_in(SRC, "At Sign Review Readers", procedure) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{procedure}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn exact_new_shapes_run_through_direct_calcfields() {
    let result = al_testkit::run_in(SRC, "At Sign Review Readers", "DirectShapes");
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_new_type_reader_other_than_direct_calcfields_is_refused() {
    let why = refused("AutoCalc");
    assert!(why.contains("reader of the CalcFormula"), "{why}");
}

#[test]
fn a_new_type_in_a_flowfield_filter_is_refused() {
    let why = refused("FlowFilter");
    assert!(why.contains("reader of the CalcFormula"), "{why}");
}

#[test]
fn a_new_type_in_a_query_is_refused() {
    let why = refused("QueryRead");
    assert!(why.contains("reader of the CalcFormula"), "{why}");
}

#[test]
fn a_new_type_on_a_temporary_host_is_refused() {
    let why = refused("TemporaryHost");
    assert!(why.contains("reader of the CalcFormula"), "{why}");
}

#[test]
fn language_1031_is_limited_to_count_at() {
    let why = refused("Language1031Sum");
    assert!(why.contains("session of language 1031"), "{why}");
}

#[test]
fn language_1030_is_limited_to_at_without_negation() {
    let why = refused("Language1030NegatedCount");
    assert!(why.contains("session of language 1030"), "{why}");
}

#[test]
fn the_count_at_language_cells_still_run() {
    assert!(al_testkit::run_in(SRC, "At Sign Review Readers", "LanguageCells").is_ok());
}

#[test]
fn a_translated_caption_is_refused() {
    let mut lowered = al_testkit::compile(SRC);
    std::sync::Arc::get_mut(&mut lowered.program).unwrap().apps[0].translations = true;
    let mut built = al_testkit::Built::new(lowered);
    let result = built.run_in("At Sign Review Readers", "TranslatedCaption");
    let Err(Signal::Unsupported(why)) = result else {
        panic!("expected a refusal, got {result:?}");
    };
    assert!(why.contains("translation"), "{why}");
}

#[test]
fn a_caption_list_outside_the_exact_allowlist_is_refused() {
    let why = refused("OtherCaption");
    assert!(why.contains("over a Option field (not measured)"), "{why}");
}

#[test]
fn caption_ml_is_refused() {
    for procedure in ["CaptionML", "EnumCaptionML"] {
        let why = refused(procedure);
        assert!(
            why.contains("over a Option field (not measured)") || why.contains("over a Enum field (not measured)"),
            "{why}"
        );
    }
}

#[test]
fn an_extended_enum_is_refused() {
    let why = refused("ExtendedEnum");
    assert!(why.contains("over a Enum field (not measured)"), "{why}");
}

#[test]
fn an_empty_or_blank_caption_is_refused() {
    for procedure in ["BlankCaption", "EmptyCaption"] {
        let why = refused(procedure);
        assert!(why.contains("over a Option field (not measured)"), "{why}");
    }
}

#[test]
fn an_extensible_enum_without_an_extension_is_refused() {
    let why = refused("OpenEnum");
    assert!(why.contains("over a Enum field (not measured)"), "{why}");
}

#[test]
fn enum_values_out_of_declaration_order_are_refused() {
    let why = refused("OutOfOrderEnum");
    assert!(why.contains("over a Enum field (not measured)"), "{why}");
}

#[test]
fn a_blank_enum_member_is_refused() {
    let why = refused("BlankEnum");
    assert!(why.contains("over a Enum field (not measured)"), "{why}");
}

#[test]
fn an_enum_from_a_dependency_is_refused() {
    let dep = al_testkit::AppSource {
        id: "dep".into(),
        name: "dep".into(),
        dependencies: vec![],
        files: vec![(
            "dep.al".into(),
            r#"
enum 69640 "At Sign Review Dependency Kind"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}
"#
            .into(),
        )],
    };
    let app = al_testkit::AppSource {
        id: "test".into(),
        name: "test".into(),
        dependencies: vec!["dep".into()],
        files: vec![(
            "test.al".into(),
            r#"
table 69640 "At Sign Review Dependency Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Enum "At Sign Review Dependency Kind") { }
    }
    keys { key(PK; "No.") { } }
}
table 69641 "At Sign Review Dependency Host"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; At; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("At Sign Review Dependency Row" where("No." = field("No."), Kind = filter('@')));
        }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69640 "At Sign Review Dependency"
{
    procedure Run()
    var Host: Record "At Sign Review Dependency Host";
    begin
        Host."No." := 'D';
        Host.Insert();
        asserterror Host.CalcFields(At);
    end;
}
"#
            .into(),
        )],
    };
    let mut built = al_testkit::Built::new(al_testkit::compile_apps(vec![dep, app]));
    let result = built.run_in("At Sign Review Dependency", "Run");
    let Err(Signal::Unsupported(why)) = result else {
        panic!("expected a refusal, got {result:?}");
    };
    assert!(why.contains("over a Enum field (not measured)"), "{why}");
}
