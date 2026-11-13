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

/// BaseAppPageExtCaption.al:271-276 measures Dutch relation text only after the pinned page extension modifies the field
/// caption without an XLIFF unit. The same relation on the unextended page keeps main's English result.
#[test]
fn dutch_relation_text_on_unextended_payment_methods_page_keeps_main_english_result() {
    let mut built = al_testkit::Built::new(al_testkit::compile(PINNED_RELATION));
    assert_eq!(
        al_testkit::text(built.run_in("C", "Text")),
        "Validation error for Field: Bal. Account No.,  Message = 'The field Bal. Account No. of table Payment Method contains a value (ABC) that cannot be found in the related table (G/L Account).'"
    );
}

fn pinned_relation_with_units(mut units: Vec<(String, String)>, translate_related_caption: bool) -> al_testkit::Built {
    units.push((
        format!(
            "{} - {} - {}",
            xliff_part("PageExtension", "LPX Payment Methods Ext"),
            xliff_part("Change", "Bal. Account Type"),
            xliff_part("Property", "Caption")
        ),
        "LPX type gewijzigd".into(),
    ));
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

fn pinned_relation_with_translated_caption() -> al_testkit::Built {
    pinned_relation_with_units(Vec::new(), true)
}

#[test]
fn dutch_relation_text_on_pinned_payment_methods_extension_is_available() {
    let mut built = pinned_relation_with_units(Vec::new(), false);
    assert_eq!(
        al_testkit::text(built.run_in("C", "Text")),
        "Validatiefout voor veld: Bal. Account No., bericht = 'Het veld Bal. Account No. van tabel Payment Method bevat een waarde (ABC) die niet is gevonden in de gerelateerde tabel (G/L Account).'"
    );
}

/// BaseAppPageExtCaption.al:276 pins the TableRelation error's code for the Payment Methods page. An unpinned field-caption
/// translation must make only GetLastErrorText unavailable; GetLastErrorCode remains readable.
#[test]
fn dutch_relation_with_unpinned_caption_keeps_its_pinned_error_code_readable() {
    let mut built = pinned_relation_with_translated_caption();
    assert_eq!(al_testkit::text(built.run_in("C", "Code")), "DB:PrimRecordNotFound");
}

#[test]
fn dutch_relation_with_unpinned_caption_keeps_text_refused() {
    let mut built = pinned_relation_with_translated_caption();
    refused(built.run_in("C", "Text"), "TableRelation Dutch text");
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
    layout { area(Content) { field(Flag; Rec.Flag) { } } }
}
codeunit 69804 C
{
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

/// BaseAppPageExtCaption.al:276 pins Dutch TableRelation text for `Bal. Account No.` on the Payment Methods list only.
#[test]
fn dutch_relation_text_for_another_table_field_keeps_main_english_result() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(DUTCH_VALIDATION, "C", "RelationField")),
        "Validation error for Field: Ref,  Message = 'The field Ref of table DV Row contains a value (MISSING) that cannot be found in the related table (DV Target).'"
    );
}
