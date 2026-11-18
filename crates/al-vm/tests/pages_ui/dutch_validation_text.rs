use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_ir::{TranslationFile, xliff_part};

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

#[test]
fn dutch_values_allowed_enum_parse_failure_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "ValuesAllowedEnumParseFailure"),
        "GlobalLanguage 1043",
    );
}

const PINNED_RELATION: &str = r#"
table 15 "G/L Account"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 289 "Payment Method"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Bal. Account Type"; Option) { OptionMembers = "G/L Account",Customer; }
        field(4; "Bal. Account No."; Code[20])
        {
            Caption = 'Bal. Account No.';
            TableRelation = "G/L Account"."No.";
        }
    }
    keys { key(PK; Code) { Clustered = true; } }
}
page 427 "Payment Methods"
{
    PageType = List;
    SourceTable = "Payment Method";
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field("Bal. Account Type"; Rec."Bal. Account Type") { } field("Bal. Account No."; Rec."Bal. Account No.") { } } } }
}
codeunit 69806 C
{
    procedure Code(): Text
    var R: Record "Payment Method"; P: TestPage "Payment Methods";
    begin
        R.Code := 'LPX'; R.Insert();
        GlobalLanguage(1043);
        P.OpenEdit();
        asserterror P."Bal. Account No.".SetValue('missing');
        exit(GetLastErrorCode());
    end;
    procedure Text(): Text
    var R: Record "Payment Method"; P: TestPage "Payment Methods";
    begin
        R.Code := 'LPX'; R.Insert();
        GlobalLanguage(1043);
        P.OpenEdit();
        asserterror P."Bal. Account No.".SetValue('abc');
        exit(GetLastErrorText());
    end;
    procedure ValidateCode(): Text
    var R: Record "Payment Method";
    begin
        R.Code := 'LPX'; R.Insert();
        GlobalLanguage(1043);
        asserterror R.Validate("Bal. Account No.", 'missing');
        exit(GetLastErrorCode());
    end;
}
"#;

const PINNED_RELATION_EXTENSION: &str = r#"
pageextension 62440 "LPX Payment Methods Ext" extends "Payment Methods"
{
    layout
    {
        modify("Bal. Account Type") { Caption = 'LPX Type Modified'; }
        modify("Bal. Account No.") { Caption = 'LPX No Modified'; }
    }
}
"#;

/// The TableRelation error of a TestPage SetValue under 1043 is refused, as on main: BaseAppPageExtCaption.al:276 measures
/// the Dutch sentence on one table only, which states no rule for a page or a table.
#[test]
fn dutch_relation_text_on_unextended_payment_methods_page_keeps_main_refusal() {
    let mut built = al_testkit::Built::new(al_testkit::compile(PINNED_RELATION));
    refused(built.run_in("C", "Text"), "GlobalLanguage 1043");
}

fn pinned_relation(translate_related_caption: bool) -> al_testkit::Built {
    let units = vec![(
        format!(
            "{} - {} - {}",
            xliff_part("PageExtension", "LPX Payment Methods Ext"),
            xliff_part("Change", "Bal. Account Type"),
            xliff_part("Property", "Caption")
        ),
        "LPX type gewijzigd".into(),
    )];
    let source = format!("{PINNED_RELATION}\n{PINNED_RELATION_EXTENSION}");
    let mut lowered = al_testkit::compile(&source);
    let program = std::sync::Arc::make_mut(&mut lowered.program);
    let extension_index = program
        .objects
        .iter()
        .position(|object| object.id == Some(62440))
        .expect("the pinned page extension");
    let extension_app = program.apps.len() as u32;
    program.apps.push(al_ir::AppInfo {
        translations: true,
        translation_files: vec![TranslationFile {
            name: "Translations/nl-NL.xlf".into(),
            target_language: "nl-NL".into(),
            units,
            ..Default::default()
        }],
        ..Default::default()
    });
    program.objects[extension_index].app = extension_app;
    if translate_related_caption {
        let related_index = program
            .objects
            .iter()
            .position(|object| object.id == Some(15))
            .expect("the pinned related table");
        let related_app = program.apps.len() as u32;
        program.apps.push(al_ir::AppInfo {
            translations: true,
            translation_files: vec![TranslationFile {
                name: "Translations/nl-NL.xlf".into(),
                target_language: "nl-NL".into(),
                units: vec![(
                    format!(
                        "{} - {}",
                        xliff_part("Table", "G/L Account"),
                        xliff_part("Property", "Caption")
                    ),
                    "Grootboekrekening".into(),
                )],
                ..Default::default()
            }],
            ..Default::default()
        });
        program.objects[related_index].app = related_app;
    }
    al_testkit::Built::new(lowered)
}

/// The shape of BaseAppPageExtCaption.al:271-276 itself (the extension's `modify` caption without a unit): the Dutch
/// relation sentence stays refused until a probe measures it on a second table.
#[test]
fn dutch_relation_sentence_on_the_probe_shape_stays_refused() {
    let mut built = pinned_relation(false);
    refused(built.run_in("C", "Text"), "GlobalLanguage 1043");
}

/// BaseAppPageExtCaption.al:276 pins `TestValidation` for the relation error of a TestPage; a caption that can translate
/// leaves no part of the case pinned, so its code is not read either: the whole case is refused, as on main.
#[test]
fn dutch_testpage_relation_with_translatable_caption_refuses_its_code() {
    let mut built = pinned_relation(true);
    refused(built.run_in("C", "Code"), "GlobalLanguage 1043");
}

#[test]
fn dutch_testpage_relation_with_translatable_caption_refuses_its_text() {
    let mut built = pinned_relation(true);
    refused(built.run_in("C", "Text"), "GlobalLanguage 1043");
}

/// `Rec.Validate` is no page site: its relation error keeps main's result under 1043, the code readable.
#[test]
fn dutch_record_validate_relation_with_translatable_caption_keeps_its_code() {
    let mut built = pinned_relation(true);
    assert_eq!(
        al_testkit::text(built.run_in("C", "ValidateCode")),
        "DB:PrimRecordNotFound"
    );
}

fn own_page_captionclass_relation() -> al_testkit::Built {
    let source = PINNED_RELATION
        .replace(
            "Caption = 'Bal. Account No.';",
            "Caption = 'Bal. Account No.'; CaptionClass = '3,Account Number';",
        )
        .replace("page 427 \"Payment Methods\"", "page 69807 \"Own Page\"")
        .replace("TestPage \"Payment Methods\"", "TestPage \"Own Page\"");
    let mut lowered = al_testkit::compile(&source);
    let program = std::sync::Arc::make_mut(&mut lowered.program);
    program.apps[0].translations = true;
    program.apps[0].translation_files = vec![TranslationFile {
        name: "Translations/nl-NL.xlf".into(),
        target_language: "nl-NL".into(),
        units: vec![(
            format!(
                "{} - {} - {}",
                xliff_part("Table", "Payment Method"),
                xliff_part("Field", "Bal. Account No."),
                xliff_part("Property", "Caption")
            ),
            "Vertaalde rekening".into(),
        )],
        ..Default::default()
    }];
    al_testkit::Built::new(lowered)
}

/// A CaptionClass field with a translation unit on the app's own page is refused before SetValue can observe a relation error.
#[test]
fn dutch_own_page_captionclass_with_unit_keeps_main_refusal() {
    let mut built = own_page_captionclass_relation();
    refused(built.run_in("C", "Code"), "caption of field Bal. Account No.");
}

const DUTCH_VALIDATION: &str = r#"
table 69800 "DV Target"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 69801 "DV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Flag; Boolean) { }
        field(3; Clock; Time) { }
        field(4; Span; Duration) { }
        field(5; Kind; Option) { OptionMembers = Open,Closed; }
        field(6; Ref; Code[10]) { TableRelation = "DV Target".Code; }
        field(7; EnumKind; Enum "DV Kind") { }
        field(8; Qty; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
enum 69805 "DV Kind" { Extensible = false; value(0; Open) { } value(1; Closed) { } }
page 69802 "DV List"
{
    PageType = List;
    SourceTable = "DV Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Flag; Rec.Flag) { }
                field(Clock; Rec.Clock) { }
                field(Span; Rec.Span) { }
                field(Kind; Rec.Kind) { ValuesAllowed = Open; }
                field(Ref; Rec.Ref) { }
                field(VarFlag; VarFlag) { }
                field(EnumKind; Rec.EnumKind) { }
                field(EnumAllowed; Rec.EnumKind) { ValuesAllowed = Open; }
            }
        }
    }
    var VarFlag: Boolean;
}
page 69803 "DV Card"
{
    PageType = Card;
    SourceTable = "DV Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { field(Flag; Rec.Flag) { } field(Qty; Rec.Qty) { } } }
}
page 69806 "DV Worksheet"
{
    PageType = Worksheet;
    SourceTable = "DV Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Flag; Rec.Flag) { } } } }
}
codeunit 69804 C
{
    procedure WorksheetField(): Text
    var P: TestPage "DV Worksheet";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Flag.SetValue('abc');
        exit('no error');
    end;
    procedure CardIntegerField(): Text
    var P: TestPage "DV Card";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Qty.SetValue('abc');
        exit('no error');
    end;
    procedure Seed()
    var R: Record "DV Row";
    begin
        R.Id := 1;
        R.Insert();
    end;
    procedure EnumField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.EnumKind.SetValue('abc');
        exit('no error');
    end;
    procedure ValuesAllowedEnumParseFailure(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.EnumAllowed.SetValue('abc');
        exit('no error');
    end;
    procedure BooleanField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Flag.SetValue('abc');
        exit('no error');
    end;
    procedure TimeField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Clock.SetValue('abc');
        exit('no error');
    end;
    procedure DurationField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Span.SetValue('-1 day 1 millisecond');
        exit('no error');
    end;
    procedure ValuesAllowedField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Kind.SetValue('Closed');
        exit('no error');
    end;
    procedure VariableControl(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.VarFlag.SetValue('abc');
        exit('no error');
    end;
    procedure CardField(): Text
    var P: TestPage "DV Card";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Flag.SetValue('abc');
        exit('no error');
    end;
    procedure RelationField(): Text
    var P: TestPage "DV List";
    begin
        Seed();
        GlobalLanguage(1043);
        P.OpenEdit();
        P.Ref.SetValue('missing');
        exit('no error');
    end;
}
"#;

/// localization/app-xliff/BaseAppPageExtCaption.al:271-274 pins a Boolean SetValue parse failure on a table field of a list
/// under language 1043.
#[test]
fn dutch_boolean_table_field_list_error_remains_available() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(DUTCH_VALIDATION, "C", "BooleanField")),
        "Validatiefout voor veld: Flag, bericht = 'Your entry of 'abc' is not an acceptable value for 'Flag'.'"
    );
}

/// W2ValueControls' entry errors do not pin Time or Duration; `BaseAppPageExtCaption.al` pins a Boolean/Enum list field only.
#[test]
fn dutch_time_entry_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "TimeField"),
        "GlobalLanguage 1043",
    );
}

#[test]
fn dutch_duration_entry_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "DurationField"),
        "Duration control",
    );
}

#[test]
fn dutch_values_allowed_error_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "ValuesAllowedField"),
        "GlobalLanguage 1043",
    );
}

#[test]
fn dutch_variable_control_error_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "VariableControl"),
        "GlobalLanguage 1043",
    );
}

#[test]
fn dutch_card_field_error_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "CardField"),
        "GlobalLanguage 1043",
    );
}

#[test]
fn dutch_enum_table_field_list_error_remains_available() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(DUTCH_VALIDATION, "C", "EnumField")),
        "Validatiefout voor veld: EnumKind, bericht = 'Your entry of 'abc' is not an acceptable value for 'EnumKind'.'"
    );
}

/// A relation error on another table keeps main's refusal under 1043.
#[test]
fn dutch_relation_text_for_another_table_field_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "RelationField"),
        "GlobalLanguage 1043",
    );
}

/// BaseAppPageExtCaption.al:271-274 pins the bare entry sentence on a PageType List page; a Worksheet has a repeater too,
/// but is not one.
#[test]
fn dutch_boolean_entry_on_a_worksheet_keeps_main_refusal() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "WorksheetField"),
        "GlobalLanguage 1043",
    );
}

/// R7187PageCaption.al:107-122, :152-166 pin the Dutch Integer entry text without a Refresh hint (a draft row); a card
/// with a stored row adds the hint, which is not pinned under 1043.
#[test]
fn dutch_integer_entry_with_a_refresh_hint_is_refused() {
    refused(
        al_testkit::run_in(DUTCH_VALIDATION, "C", "CardIntegerField"),
        "can't be evaluated into type Integer",
    );
}
