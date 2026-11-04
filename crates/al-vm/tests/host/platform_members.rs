//! Members of platform types that the corpus platform area pins (metadata/platform-tables, P94NavApp.al).

use al_builtins::value::Value;

// metadata/platform-tables/PlTSessionSettings.al:103-131 (Properties_EdgeValues): the properties take assignment syntax.
#[test]
fn session_settings_properties_take_an_assignment() {
    let r = al_testkit::text(al_testkit::run(
        "codeunit 50100 C { procedure F(): Text var S: SessionSettings; begin S.Company := 'X'; S.LanguageId := 0; S.LocaleId := -1; S.TimeZone := ''; S.ProfileId := 'P'; exit(S.Company + Format(S.LanguageId) + Format(S.LocaleId) + S.TimeZone + S.ProfileId); end; }",
        "F",
    ));
    assert_eq!(r, "X0-1P");
}

// metadata/session-system/P94NavApp.al:31-181: outside install code there is no archive.
#[test]
fn navapp_archive_members_outside_install_code() {
    let src = "table 50100 T { fields { field(1; K; Integer) { } } }
codeunit 50101 C {
    procedure F(): Text var R: RecordRef; begin
        NavApp.DeleteArchiveData(0); NavApp.DeleteArchiveData(50100); NavApp.DeleteArchiveData(73999);
        NavApp.RestoreArchiveData(50100); NavApp.RestoreArchiveData(50100, true); NavApp.RestoreArchiveData(0);
        NavApp.LoadPackageData(0); NavApp.LoadPackageData(73999);
        exit('[' + NavApp.GetArchiveVersion() + ']' + Format(NavApp.GetArchiveRecordRef(50100, R)) + Format(R.Number() <> 0)
            + Format(NavApp.IsUnlicensed()) + Format(NavApp.IsUnlicensed(CreateGuid())));
    end;
    procedure Load() begin NavApp.LoadPackageData(50100); end;
}";
    let r = al_testkit::text(al_testkit::run(src, "F"));
    assert_eq!(r, "[]NoNoNoNo");
    // Outside install code LoadPackageData does nothing, for a table of the workspace too
    // (install-upgrade/install-triggers/Y85PackageData.al Y85_Outside_*).
    let r = al_testkit::run(src, "Load");
    assert!(r.is_ok(), "{r:?}");
}

// metadata/platform-tables/PlTFilterPageBuilder.al:242-260 (AddFieldNo_Variants) and :97-114 (AddTable_Errors).
#[test]
fn filter_page_builder_add_field_no_and_an_unknown_table() {
    let src =
        "table 50100 T { fields { field(1; K; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; K) { } } }
codeunit 50101 C {
    procedure F(): Text var Fpb: FilterPageBuilder; begin
        Fpb.AddTable('Rows', 50100);
        Fpb.AddFieldNo('Rows', 1);
        Fpb.AddFieldNo('Rows', 2, 'xyz');
        exit(Fpb.GetView('Rows'));
    end;
    procedure Unknown() var Fpb: FilterPageBuilder; begin Fpb.AddTable('Rows', 99999999); end;
}";
    let r = al_testkit::run(src, "F");
    assert!(
        matches!(&r, Ok(Value::Text(t)) if &**t == "VERSION(1) SORTING(K) WHERE(Name=FILTER(xyz))"),
        "{r:?}"
    );
    let m = al_testkit::error_message(al_testkit::run(src, "Unknown"));
    assert!(
        m.starts_with("You tried to invoke the Table object with the ID 99999999 from the object"),
        "{m}"
    );
}

// metadata/platform-tables/PlTNavApp.al:86-115 (ModuleInfo_Uninitialized), :218-231 (DependencyInfo_Uninitialized) and
// :69-81 (Clear of a filled ModuleInfo).
#[test]
fn an_untouched_or_cleared_module_info_reads_empty() {
    let r = al_testkit::text(al_testkit::run(
        "codeunit 50100 C { procedure F(): Text var I: ModuleInfo; D: ModuleDependencyInfo; begin
            exit(Format(I.Id()) + I.Name() + I.Publisher() + Format(I.AppVersion()) + Format(I.DataVersion()) + Format(I.PackageId())
                + Format(I.Dependencies().Count()) + Format(I.AppVersion().Major()) + Format(D.Id()) + D.Name() + D.Publisher());
        end; }",
        "F",
    ));
    assert_eq!(
        r,
        "{00000000-0000-0000-0000-000000000000}0.0.0.00.0.0.0{00000000-0000-0000-0000-000000000000}00{00000000-0000-0000-0000-000000000000}"
    );
}

// metadata/platform-tables/PlTGuid.al:190-203 (Assignment_FromText): a text that is no guid raises, the Guid keeps its value.
#[test]
fn a_text_that_is_no_guid_raises_on_assignment() {
    let src = "codeunit 50100 C {
        procedure F(): Text var G: Guid; begin G := CreateGuid(); if not TryAssign(G, 'nonsense') then exit(GetLastErrorCode() + Format(IsNullGuid(G))); end;
        [TryFunction] local procedure TryAssign(var G: Guid; T: Text) begin G := T; end;
    }";
    assert_eq!(al_testkit::text(al_testkit::run(src, "F")), "InvalidGuidFormatNo");
}
