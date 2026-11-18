//! The control captions of a page extended by an app with an nl-NL file, under 1043, that
//! localization/app-xliff/BaseAppPageExtCaption.al:205-219 does not pin: each stays refused, as on main. One control the
//! pin covers (a single extension's `modify` caption with a unit) shows the fixture reaches the translated path.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_ir::{TranslationFile, xliff_part};

const SOURCE: &str = r#"
table 69810 "EC Row"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; A; Text[10]) { }
        field(3; B; Text[10]) { }
        field(4; C; Text[10]) { }
        field(5; D; Text[10]) { }
        field(6; E; Text[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
page 69811 "EC Card"
{
    PageType = Card;
    SourceTable = "EC Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Own; Rec.A) { Caption = 'Own Source'; }
            field(Twice; Rec.B) { }
            field(CaseCtl; Rec.C) { }
            field(Ml; Rec.D) { CaptionML = ENU = 'Ml Source'; }
            field(Single; Rec.E) { }
        }
    }
}
pageextension 69812 "EC Ext One" extends "EC Card"
{
    layout
    {
        modify(Twice) { Caption = 'One Source'; }
        modify(casectl) { Caption = 'Case Source'; }
        modify(Single) { Caption = 'Single Source'; }
    }
}
pageextension 69813 "EC Ext Two" extends "EC Card"
{
    layout
    {
        modify(Twice) { Caption = 'Two Source'; }
    }
}
codeunit 69814 C
{
    procedure Own(): Text
    var P: TestPage "EC Card";
    begin
        GlobalLanguage(1043);
        P.OpenView();
        exit(P.Own.Caption);
    end;
    procedure Twice(): Text
    var P: TestPage "EC Card";
    begin
        GlobalLanguage(1043);
        P.OpenView();
        exit(P.Twice.Caption);
    end;
    procedure CaseCtl(): Text
    var P: TestPage "EC Card";
    begin
        GlobalLanguage(1043);
        P.OpenView();
        exit(P.CaseCtl.Caption);
    end;
    procedure Ml(): Text
    var P: TestPage "EC Card";
    begin
        GlobalLanguage(1043);
        P.OpenView();
        exit(P.Ml.Caption);
    end;
    procedure Single(): Text
    var P: TestPage "EC Card";
    begin
        GlobalLanguage(1043);
        P.OpenView();
        exit(P.Single.Caption);
    end;
}
"#;

fn unit(ext: &str, kind: &str, control: &str, text: &str) -> (String, String) {
    (
        format!(
            "{} - {} - {}",
            xliff_part("PageExtension", ext),
            xliff_part(kind, control),
            xliff_part("Property", "Caption")
        ),
        text.into(),
    )
}

/// The page and table stay in the app that does not translate; both extensions move to one app with an nl-NL file that
/// holds a unit for every caption they set.
fn built() -> al_testkit::Built {
    let mut lowered = al_testkit::compile(SOURCE);
    let program = std::sync::Arc::make_mut(&mut lowered.program);
    let app = program.apps.len() as u32;
    let mut units = vec![
        unit("EC Ext One", "Change", "Twice", "Een"),
        unit("EC Ext Two", "Change", "Twice", "Twee"),
        unit("EC Ext One", "Change", "casectl", "Geval"),
        unit("EC Ext One", "Change", "Single", "Enkel"),
    ];
    units.sort();
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
    for o in &mut program.objects {
        if matches!(o.id, Some(69812 | 69813)) && o.kind == al_ir::ObjKind::PageExtension {
            o.app = app;
        }
    }
    al_testkit::Built::new(lowered)
}

fn refused(r: Result<Value, Signal>, control: &str) {
    match r {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains(&format!("the caption of control {control}")), "{why}")
        }
        other => panic!("expected the caption of {control} refused, got {other:?}"),
    }
}

#[test]
fn a_single_extensions_modify_caption_with_a_unit_reads_it() {
    assert_eq!(al_testkit::text(built().run_in("C", "Single")), "Enkel");
}

#[test]
fn the_own_caption_of_an_extended_page_stays_refused() {
    refused(built().run_in("C", "Own"), "Own");
}

#[test]
fn a_caption_two_extensions_set_stays_refused() {
    refused(built().run_in("C", "Twice"), "Twice");
}

#[test]
fn a_modify_caption_written_in_another_case_stays_refused() {
    refused(built().run_in("C", "CaseCtl"), "CaseCtl");
}

#[test]
fn a_caption_ml_on_an_extended_page_stays_refused() {
    refused(built().run_in("C", "Ml"), "Ml");
}
