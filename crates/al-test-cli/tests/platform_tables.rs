//! Platform tables (ADR-013 decision 7, ADR-019): the registry of native providers is bound to
//! the real declarations in the platform `System` app, and a provider never stands in for AL
//! that exists.
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app` and the Microsoft apps).

use al_builtins::platform::{Provider, REGISTRY};
use al_deps::package::read_package;
use al_syntax::{ast, scan::scan};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;
use std::path::{Path, PathBuf};

fn packages() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    assert!(
        dir.join("System.app").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    dir
}

fn system_app_files() -> Vec<(String, String)> {
    let bytes = std::fs::read(packages().join("System.app")).unwrap();
    read_package(&bytes).expect("System.app reads").1
}

fn symbols() -> std::collections::BTreeSet<String> {
    ["BC27PLUS".to_string()].into()
}

#[test]
fn every_registry_entry_is_a_bodiless_table_of_the_system_app_with_that_id_and_name() {
    let files = system_app_files();
    let syms = symbols();
    for (id, name, provider) in REGISTRY {
        let mut found = Vec::new();
        for (path, src) in &files {
            if !src.contains(&id.to_string()) {
                continue;
            }
            let parse = al_syntax::parse(src, &syms);
            let root = ast::SourceFile::cast(parse.syntax()).unwrap();
            for o in root.objects() {
                if o.kind() == "table" && o.id() == Some(*id) {
                    found.push((path.clone(), o));
                }
            }
        }
        assert_eq!(
            found.len(),
            1,
            "{name} ({id}) must be declared once in System.app: {found:?}",
            found = found.iter().map(|f| &f.0).collect::<Vec<_>>()
        );
        let (path, o) = &found[0];
        assert!(
            o.name().eq_ignore_ascii_case(name),
            "{path}: the registry names {id} `{name}`, the declaration `{}`",
            o.name()
        );
        // A platform table has no AL body: the platform has the behaviour (ADR-013 decision 7).
        assert!(
            ast::routines_in(o.syntax()).is_empty(),
            "{path}: {name} has code, so it is AL that exists and may not be replaced natively ({provider:?})"
        );
    }
}

#[test]
#[ignore = "scans 9500 files: cargo test --release -p al-test-cli --test platform_tables -- --ignored"]
fn no_other_app_supplies_a_table_under_a_registered_id_or_name() {
    let syms = symbols();
    for entry in std::fs::read_dir(packages()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "app") || path.file_name().is_some_and(|n| n == "System.app") {
            continue;
        }
        let Ok((_, files)) = read_package(&std::fs::read(&path).unwrap()) else {
            continue;
        };
        for (file, src) in files {
            for o in scan(&src, &syms).objects.iter().filter(|o| o.kind == "table") {
                for (id, name, _) in REGISTRY {
                    assert!(
                        o.id != Some(*id) && !o.name.eq_ignore_ascii_case(name),
                        "{}: {file} declares table `{}` ({:?}), which the platform registry owns ({name}, {id})",
                        path.display(),
                        o.name,
                        o.id
                    );
                }
            }
        }
    }
}

/// Outcomes of the test codeunit in `main`, compiled over the real platform `System` app.
fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    outcomes_with(main, al_sema::check::Roots::default())
}

/// [`outcomes`] with the demand roots chosen: `eager` compiles every table of the System app.
fn outcomes_with(main: &str, roots: al_sema::check::Roots) -> Vec<(String, Outcome)> {
    outcomes_app("main", main, roots)
}

/// [`outcomes_with`] with the id of the app under test chosen (a platform table lists apps by Guid).
fn outcomes_app(app_id: &str, main: &str, roots: al_sema::check::Roots) -> Vec<(String, Outcome)> {
    outcomes_bc(app_id, main, roots, "27.5.0.0")
}

/// [`outcomes_app`] run as BC build `bc` (the System app stays the one of the corpus packages).
fn outcomes_bc(app_id: &str, main: &str, roots: al_sema::check::Roots, bc: &str) -> Vec<(String, Outcome)> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, name: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files,
    };
    let (_, l) = al_sema::lower::compile_apps_with(
        vec![
            app(app_id, "main", &[], vec![("main.al".into(), main.into())]),
            app(al_sema::workspace::PLATFORM_APP_ID, "System", &[], system_app_files()),
        ],
        vec![AppKind::Workspace, AppKind::Dependency],
        &symbols(),
        None,
        roots,
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    // The two sources above carry no manifest, so no version; the app tables list an app only with one.
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut().filter(|a| a.version.is_empty()) {
        a.version = "1.0.0.0".into();
    }
    let engine = Engine::with_target(
        std::sync::Arc::new(program),
        blocked,
        Some(al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap())),
    );
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    run_codeunit(&engine, &l, oi)
}

fn not_run(o: &[(String, Outcome)], test: &str) -> String {
    match o.iter().find(|(n, _)| n == test).map(|(_, o)| o) {
        Some(Outcome::NotRun(why)) => why.clone(),
        other => panic!("{test} was expected to be refused, got {other:?}"),
    }
}

#[test]
fn a_platform_table_with_no_provider_is_refused_where_code_uses_it_and_names_table_and_id() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure UsesATableWithNoProvider() var P: Record "License Information"; begin if P.IsEmpty() then; end; }"#,
    );
    let why = not_run(&o, "UsesATableWithNoProvider");
    assert!(
        why.contains("platform table 'License Information' (2000000040) has no provider"),
        "{why}"
    );
}

// A name that ends in a dot (`Metadata Permission Set Rel.`, 23 of Microsoft's permission tests) is a
// platform table with no provider, not an object outside the workspace.
#[test]
fn a_platform_table_whose_name_ends_in_a_dot_names_the_missing_provider() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure UsesIt() var P: Record "Metadata Permission Set Rel."; begin if P.IsEmpty() then; end; }"#,
    );
    let why = not_run(&o, "UsesIt");
    assert!(
        why.contains("platform table 'Metadata Permission Set Rel.' (") && why.contains("has no provider"),
        "{why}"
    );
}

#[test]
fn a_virtual_platform_table_is_read_but_cannot_be_written() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ReadsTheIntegerTable() var I: Record Integer; begin I.SetRange(Number, 3, 5); if I.Count() <> 3 then Error('count %1', I.Count()); end;
  [Test] procedure WritesTheIntegerTable() var I: Record Integer; begin I.Number := 7; I.Insert(); end; }"#,
    );
    assert!(
        o.iter()
            .any(|(n, o)| n == "ReadsTheIntegerTable" && *o == Outcome::Pass),
        "{o:?}"
    );
    let why = not_run(&o, "WritesTheIntegerTable");
    assert!(why.contains("virtual table 'Integer'"), "{why}");
}

// Microsoft's Tests-ERM `Initialize` sets Feature Key ReminderTermsCommunicationTexts to None and calls Modify (243 tests were
// refused with "Modify on virtual table 'Feature Key'"). Enabled is the only writable column; a change stays for later reads
// (assumed, docs/probes-needed.md); another column is still refused.
#[test]
fn a_feature_key_enabled_can_be_modified_and_reads_back_and_another_column_is_refused() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ModifyUnchanged() var K: Record "Feature Key"; begin K.FindFirst(); K.Enabled := K.Enabled; K.Modify(); end;
  [Test] procedure ModifyEnabledReadsBack() var K: Record "Feature Key"; W: Record "Feature Key"; N: Option; begin
    K.FindFirst(); if K.Enabled = K.Enabled::None then K.Enabled := K.Enabled::"All Users" else K.Enabled := K.Enabled::None;
    N := K.Enabled; K.Modify(); W.Get(K.ID); if W.Enabled <> N then Error('read back %1', W.Enabled); end;
  [Test] procedure ModifyAnotherColumn() var K: Record "Feature Key"; begin K.FindFirst(); K."Is One Way" := not K."Is One Way"; K.Modify(); end; }"#,
    );
    for t in ["ModifyUnchanged", "ModifyEnabledReadsBack"] {
        assert!(o.iter().any(|(n, o)| n == t && *o == Outcome::Pass), "{o:?}");
    }
    let why = not_run(&o, "ModifyAnotherColumn");
    assert!(why.contains("virtual table 'Feature Key'"), "{why}");
}

// platform/lt-aw-fresh-tenant/AwTests.al:557-576: on the corpus CI (Linux) a Feature Key change survives the rollback of an
// asserterror. That is a Linux suspect until the Windows nightly confirms it: the rollback of an uncommitted change is refused.
#[test]
fn a_rollback_of_an_uncommitted_feature_key_change_is_refused() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure RolledBack() var K: Record "Feature Key"; begin
    asserterror begin K.FindFirst(); if K.Enabled = K.Enabled::None then K.Enabled := K.Enabled::"All Users" else K.Enabled := K.Enabled::None;
      K.Modify(); Error('undo'); end; end;
  [Test] procedure UnchangedRolledBack() var K: Record "Feature Key"; begin
    asserterror begin K.FindFirst(); K.Modify(); Error('undo'); end; end; }"#,
    );
    let why = not_run(&o, "RolledBack");
    assert!(why.contains("Feature Key"), "{why}");
    passes(&o, "UnchangedRolledBack");
}

#[test]
fn a_column_nothing_pins_is_refused_where_code_reads_it() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ReadsAPinnedColumn() var F: Record Field; begin F.SetRange(TableNo, 50000); if F.IsEmpty() then; end;
  [Test] procedure ReadsAnUnpinnedColumn() var R: Record "Report Metadata"; L: Integer; begin L := R.MaxRows; end; }"#,
    );
    // The corpus pins every Field column up to 28 (record/PZFieldObsoleteRows.al:39-58); Report Metadata
    // MaxRows (19) is read by no corpus test.
    let why = not_run(&o, "ReadsAnUnpinnedColumn");
    assert!(why.contains("column 19 of platform table 'Report Metadata'"), "{why}");
    assert!(
        !matches!(o.iter().find(|(n, _)| n == "ReadsAPinnedColumn").map(|(_, o)| o), Some(Outcome::NotRun(w)) if w.contains("column")),
        "{o:?}"
    );
}

// Business Foundation's table extensions on "Source Code" declare fields with `ObsoleteState = Moved`; Base
// Application's Dimension Management scans the Field table for all tables. A moved field has no row in the Field
// table, a pending one has (types/PWMisc.al:FieldTable_MovedAndPendingField, BC 27.5 and 29.0).
#[test]
fn a_moved_field_has_no_row_in_the_field_table_and_a_scan_of_all_fields_runs() {
    let o = outcomes(
        r#"table 50001 "Moved Host" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
tableextension 50002 "Moved Ext" extends "Moved Host" { fields {
  field(50000; Gone; Boolean) { ObsoleteState = Moved; ObsoleteReason = 'x'; ObsoleteTag = '1'; MovedTo = '437dbf0e-84ff-417a-965d-ed2bb9650972'; }
  field(50001; Soon; Boolean) { ObsoleteState = Pending; ObsoleteReason = 'x'; ObsoleteTag = '1'; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ScansAllFields() var F: Record Field; begin F.SetRange(Class, F.Class::Normal); if F.FindSet() then repeat until F.Next() = 0; end;
  [Test] procedure MovedHasNoRow() var F: Record Field; begin F.SetRange(TableNo, 50001); F.SetRange("No.", 50000); if F.FindFirst() then Error('moved field listed'); end;
  [Test] procedure PendingHasARow() var F: Record Field; begin F.SetRange(TableNo, 50001); F.SetRange("No.", 50001); if not F.FindFirst() then Error('pending field missing'); end; }"#,
    );
    passes(&o, "ScansAllFields");
    passes(&o, "MovedHasNoRow");
    passes(&o, "PendingHasARow");
}

// The Field table answers the rowversion row (No. 0, `timestamp`, BigInteger, Normal, Len 8, Enabled) only to a FieldName
// filter, on a table with a key over SystemRowVersion and on one without (types/lt-aj-probes/AjMisc.al:510-521, helper
// QtFieldTableAnswers :414-429). Get(table, 0), a No. = 0 filter, a Type filter, a FieldName filter on another name and a
// TableNo-only scan do not see it (same helper; RaFieldScan :613-641; metadata/W6FieldKeyRows.al:263-270). Only the shape
// TableNo plus FieldName = 'timestamp' (:405-406) is pinned: another filter next to them, or a pattern, is refused.
#[test]
fn the_rowversion_row_of_the_field_table_answers_only_a_fieldname_filter() {
    let o = outcomes(
        r#"table 50003 "Rv Host" { fields { field(1; Id; Integer) { } field(2; Txt; Text[10]) { } }
  keys { key(PK; Id) { } key(Rv; SystemRowVersion) { } } }
table 50004 "No Rv" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50000 "T" { Subtype = Test;
  local procedure Answers(Tab: Integer): Text var F: Record Field; s: Text; begin
    if F.Get(Tab, 0) then s := 'found|' else s := 'not found|';
    F.SetRange(TableNo, Tab); F.SetRange(FieldName, 'timestamp');
    if F.FindSet() then repeat s += Format(F."No.") + ':' + F.FieldName + ':' + Format(F.Type) + ':' + Format(F.Class) + ':' + Format(F.Len) + ':' + Format(F.Enabled) + ';'; until F.Next() = 0;
    F.SetRange(FieldName, 'SystemRowVersion'); s += '|' + Format(F.Count());
    F.SetRange(FieldName); F.SetRange("No.", 0); s += '|' + Format(F.Count());
    F.SetRange("No."); F.SetRange(Type, F.Type::BigInteger); s += '|' + Format(F.Count());
    F.SetRange(Type); F.FindFirst(); s += '|' + Format(F."No.");
    exit(s); end;
  [Test] procedure WithKey() begin if Answers(50003) <> 'not found|0:timestamp:BigInteger:Normal:8:Yes;|0|0|0|1' then Error(Answers(50003)); end;
  [Test] procedure WithoutKey() begin if Answers(50004) <> 'not found|0:timestamp:BigInteger:Normal:8:Yes;|0|0|0|1' then Error(Answers(50004)); end;
  [Test] procedure OtherName() var F: Record Field; begin F.SetRange(TableNo, 50003); F.SetRange(FieldName, 'Txt'); F.FindFirst(); if F."No." <> 2 then Error('wrong'); end;
  [Test] procedure Pattern() var F: Record Field; begin F.SetRange(TableNo, 50003); F.SetFilter(FieldName, 'time*'); if F.Count() = 0 then; end;
  [Test] procedure ExtraFilter() var F: Record Field; begin F.SetRange(TableNo, 50004); F.SetRange(FieldName, 'timestamp'); F.SetRange(Class, F.Class::Normal); if F.Count() = 0 then; end;
  [Test] procedure UnpinnedColumn() var F: Record Field; begin F.SetRange(TableNo, 50004); F.SetRange(FieldName, 'timestamp'); F.FindFirst(); if F."Field Caption" = '' then; end; }"#,
    );
    for t in ["WithKey", "WithoutKey", "OtherName"] {
        passes(&o, t);
    }
    for t in ["Pattern", "ExtraFilter", "UnpinnedColumn"] {
        let why = not_run(&o, t);
        assert!(why.contains("timestamp"), "{t}: {why}");
    }
}

#[test]
fn registry_providers_are_virtual_or_stored() {
    for (_, name, p) in REGISTRY {
        assert!(matches!(p, Provider::Virtual(_) | Provider::Stored), "{name}");
    }
}

fn passes(o: &[(String, Outcome)], test: &str) {
    match o.iter().find(|(n, _)| n == test).map(|(_, o)| o) {
        Some(Outcome::Pass) => {}
        other => panic!("{test} was expected to pass, got {other:?}"),
    }
}

#[test]
fn the_session_starts_with_its_company_user_and_super_row() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure CompanyRow() var C: Record Company; begin if not C.Get(CompanyName()) then Error('no company row'); end;
  [Test] procedure UserRow() var U: Record User; begin if not U.Get(UserSecurityId()) then Error('no user row'); if U."User Name" <> UserId() then Error('name %1', U."User Name"); end;
  [Test] procedure SuperRow() var A: Record "Access Control"; begin A.SetRange("User Security ID", UserSecurityId()); A.SetRange("Role ID", 'SUPER'); if A.IsEmpty() then Error('no SUPER'); end; }"#,
    );
    for t in ["CompanyRow", "UserRow", "SuperRow"] {
        passes(&o, t);
    }
}

#[test]
fn record_links_are_rows_of_the_record_link_table() {
    let o = outcomes(
        r#"table 50001 "Host" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure AddHasDelete() var H: Record "Host"; L: Record "Record Link"; Id: Integer; begin
    H."No." := 1; H.Insert();
    if H.HasLinks() then Error('links before');
    Id := H.AddLink('https://example.com', 'D');
    if not H.HasLinks() then Error('no links after AddLink');
    L.Get(Id);
    if L.URL1 <> 'https://example.com' then Error('url');
    H.DeleteLinks();
    if H.HasLinks() then Error('links after DeleteLinks');
  end; }"#,
    );
    passes(&o, "AddHasDelete");
}

// Gen. Jnl.-Post Line PostBankAcc does `BankAccLedgEntry.CopyLinks(GenJnlLine)` for a journal line without links, in
// about 330 Microsoft tests: the copy of nothing is nothing, not the "There is no Record Link within the filter" error
// an unused FindSet would raise.
#[test]
fn copying_the_links_of_a_record_without_links_copies_nothing() {
    let o = outcomes(
        r#"table 50001 "Host" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure CopyNothing() var A: Record "Host"; B: Record "Host"; begin
    A."No." := 1; A.Insert(); B."No." := 2; B.Insert();
    B.CopyLinks(A);
    if B.HasLinks() then Error('links appeared');
    A.AddLink('https://example.com', 'D');
    B.CopyLinks(A);
    if not B.HasLinks() then Error('links not copied');
  end; }"#,
    );
    passes(&o, "CopyNothing");
}

#[test]
fn date_and_time_zone_and_windows_language_answer_from_their_pins() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure MondayWeek() var D: Record Date; begin D.SetRange("Period Type", D."Period Type"::Week); D.SetRange("Period Start", DMY2Date(19, 1, 2026)); if not D.FindFirst() then Error('no week'); if D."Period No." <> 4 then Error('week %1', D."Period No."); end;
  [Test] procedure ZonesNumbered() var Z: Record "Time Zone"; N: Integer; begin if Z.Count() < 2 then Error('few'); Z.FindSet(); repeat N += 1; if Z."No." <> N then Error('gap'); until Z.Next() = 0; end;
  [Test] procedure English() var W: Record "Windows Language"; begin if not W.Get(1033) then Error('no 1033'); if W."Language Tag" <> 'en-US' then Error('tag'); if W.Get(999999) then Error('999999'); end; }"#,
    );
    for t in ["MondayWeek", "ZonesNumbered", "English"] {
        passes(&o, t);
    }
}

// Windows Language: record/TestWindowsLanguageTable.al:9-13 pins the columns of the culture (Language ID, Primary Language
// ID, Name, Abbreviated Name, Primary CodePage, Language Tag). platform/lt-ag-contoso/AgWindowsLanguage.al pins more: the row
// of each of the DemoTool's 54 codes, found by Abbreviated Name with a count of 1 (QX_WindowsLanguage_ByAbbreviatedName_Part1-3),
// "Globally Enabled" true on every row (its count is the table's, :116-118) and "STX File Exist" true for 1030, 1031, 1033,
// 1036 and 2057 (:110-114), "Localization Exist" true for 1031, 1033, 1036, 1040 and 2057
// (errors/lt-ms-refusals-128/P128WindowsLanguage.al:46-50). Any other column, and "STX File Exist" of any other row, is refused, not answered with a blank
// (the DemoTool's Demo Data Setup."Data Language ID" TableRelation filters on both).
#[test]
fn windows_language_answers_the_pinned_columns_and_refuses_the_rest() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure PinnedColumns() var W: Record "Windows Language"; begin W.SetFilter("Language ID", '1031|1033|2057'); W.SetRange("Abbreviated Name", 'ENU'); if not W.FindFirst() then Error('no ENU'); if W."Language ID" <> 1033 then Error('id'); end;
  [Test] procedure FilterOnGloballyEnabledAndStx() var W: Record "Windows Language"; begin W.SetRange("Language ID", 1033); W.SetRange("Globally Enabled", true); W.SetRange("STX File Exist", true); if not W.FindFirst() then Error('no 1033'); if not W."STX File Exist" then Error('stx'); end;
  [Test] procedure ByAbbreviatedName() var W: Record "Windows Language"; N: Integer; begin W.SetCurrentKey("Abbreviated Name"); W.SetFilter("Abbreviated Name", 'NOR'); if W.Count() <> 1 then Error('count'); W.FindSet(); repeat N += 1; until W.Next() = 0; if N <> 1 then Error('walk'); if W."Language ID" <> 1044 then Error('id'); if W.Name <> 'Norwegian Bokmål (Norway)' then Error('name'); if W."Primary CodePage" <> '850' then Error('cp'); if W."Language Tag" <> 'nb-NO' then Error('tag'); end;
  [Test] procedure ReadOfSTXFileExistOfAnotherRow() var W: Record "Windows Language"; begin W.Get(3079); if W."STX File Exist" then; end;
  [Test] procedure FilterOnEnabled() var W: Record "Windows Language"; begin W.SetRange("Language ID", 1033); W.SetRange(Enabled, true); if W.FindFirst() then; end;
  [Test] procedure StxCountOfAKnownRange() var W: Record "Windows Language"; begin W.SetFilter("Language ID", '1031|3079'); W.SetRange("STX File Exist", true); if W.Count() = 0 then; end;
  [Test] procedure LocalizationOf1040() var W: Record "Windows Language"; begin W.Get(1040); if not W."Localization Exist" then Error('loc'); end;
  [Test] procedure LocalizationOf1030() var W: Record "Windows Language"; begin W.Get(1030); if W."Localization Exist" then; end;
  [Test] procedure AnAbbreviationNotPinned() var W: Record "Windows Language"; begin W.SetFilter("Abbreviated Name", 'ARA'); if W.FindFirst() then; end; }"#,
    );
    for t in [
        "PinnedColumns",
        "FilterOnGloballyEnabledAndStx",
        "ByAbbreviatedName",
        "LocalizationOf1040",
    ] {
        passes(&o, t);
    }
    for t in [
        "ReadOfSTXFileExistOfAnotherRow",
        "FilterOnEnabled",
        "StxCountOfAKnownRange",
        "AnAbbreviationNotPinned",
        "LocalizationOf1030",
    ] {
        let why = not_run(&o, t);
        assert!(
            why.contains("Windows Language") || why.contains("not pinned"),
            "{t}: {why}"
        );
    }
}

// License Permission: the two rows platform/lt-platform-tables/PtCloudTables.al:60-76 pins. Any other object is refused,
// because the table lists the permissions of the license and nothing pins them (Cash Flow Wksh.-Register Batch reads
// Codeunit 'Update Analysis View', which is not pinned).
#[test]
fn license_permission_answers_its_two_pinned_rows_and_refuses_the_rest() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure FirstRow() var L: Record "License Permission"; begin if not L.FindFirst() then Error('empty'); if (L."Object Type" <> L."Object Type"::TableData) or (L."Object Number" <> 0) or (L."Read Permission" <> L."Read Permission"::" ") then Error('first'); end;
  [Test] procedure PinnedTableRow() var L: Record "License Permission"; begin L.SetRange("Object Type", L."Object Type"::Table); L.SetRange("Object Number", 99999); if L.Count() <> 1 then Error('count'); if not L.FindFirst() then Error('find'); if (L."Read Permission" <> L."Read Permission"::Yes) or (L."Execute Permission" <> L."Execute Permission"::Yes) or (L."Limited Usage Permission" <> L."Limited Usage Permission"::" ") then Error('perms'); if not L.Get(L."Object Type"::Table, 99999) then Error('get'); end;
  [Test] procedure ACodeunitIsNotPinned() var L: Record "License Permission"; begin L.Get(L."Object Type"::Codeunit, 410); end;
  [Test] procedure ACountOfEverythingIsNotPinned() var L: Record "License Permission"; begin if L.Count() = 0 then; end; }"#,
    );
    passes(&o, "FirstRow");
    passes(&o, "PinnedTableRow");
    for t in ["ACodeunitIsNotPinned", "ACountOfEverythingIsNotPinned"] {
        let why = not_run(&o, t);
        assert!(why.contains("License Permission"), "{t}: {why}");
    }
}

// Language Selection: every row and column of a fresh tenant (platform/lt-platform-rows-fresh/PlatformRowsLanguageSelection.al:76,
// 212 rows from BC 29 and 208 before, :574-586). Base Application's Customer.UpdateFormatRegion reads it by Language ID (DemoTool).
// The Name of a row in another session language is not pinned: refused.
#[test]
fn language_selection_lists_the_pinned_rows_per_major() {
    let src = r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Count29() var L: Record "Language Selection"; begin if L.Count() <> 212 then Error('count %1', L.Count()); end;
  [Test] procedure Count27() var L: Record "Language Selection"; begin if L.Count() <> 208 then Error('count %1', L.Count()); end;
  [Test] procedure FirstRow() var L: Record "Language Selection"; begin L.FindFirst(); if (L."Language Tag" <> 'af-ZA') or (L.Name <> 'Afrikaans (South Africa)') or (L."Language ID" <> 1078) or (L."Abbreviated Name" <> 'AFK') or L.Enabled then Error('first %1', L."Language Tag"); end;
  [Test] procedure ByLanguageId() var L: Record "Language Selection"; begin L.SetRange("Language ID", 1031); L.FindFirst(); if (L."Language Tag" <> 'de-DE') or (L."Primary Language ID" <> 1031) or not L.Enabled then Error('de %1', L."Language Tag"); end;
  [Test] procedure ZhTwOn29() var L: Record "Language Selection"; begin L.SetRange("Language Tag", 'zh-TW'); if L.IsEmpty() then Error('no zh-TW'); end;
  [Test] procedure ZhTwNotOn27() var L: Record "Language Selection"; begin L.SetRange("Language Tag", 'zh-TW'); if not L.IsEmpty() then Error('zh-TW'); end;
  [Test] procedure OtherLanguage() var L: Record "Language Selection"; begin GlobalLanguage(1031); if L.FindFirst() then; end; }"#;
    let o29 = outcomes_bc("main", src, al_sema::check::Roots::default(), "29.0.0.0");
    for t in ["Count29", "FirstRow", "ByLanguageId", "ZhTwOn29"] {
        passes(&o29, t);
    }
    let why = not_run(&o29, "OtherLanguage");
    assert!(why.contains("Language Selection"), "{why}");
    let o27 = outcomes_bc("main", src, al_sema::check::Roots::default(), "27.5.0.0");
    for t in ["Count27", "FirstRow", "ZhTwNotOn27"] {
        passes(&o27, t);
    }
}

#[test]
fn a_permission_set_is_listed_with_its_caption_and_assignable_default() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
permissionset 50002 "My Set" { Caption = 'My Set Caption'; Assignable = true; Permissions = tabledata "Thing" = RIMD; }
permissionset 50003 "No Assign" { Permissions = tabledata "Thing" = r; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Listed() var M: Record "Metadata Permission Set"; begin M.SetRange("Role ID", 'MY SET'); M.FindFirst(); if M.Name <> 'My Set Caption' then Error('name'); if not M.Assignable then Error('assignable'); end;
  [Test] procedure UndeclaredIsNotAssignable() var M: Record "Metadata Permission Set"; begin M.SetRange("Role ID", 'NO ASSIGN'); M.FindFirst(); if M.Assignable then Error('assignable'); if M.Name <> 'NO ASSIGN' then Error('name %1', M.Name); end;
  [Test] procedure Grants() var P: Record "Metadata Permission"; begin P.SetRange("Role ID", 'MY SET'); if P.Count() <> 1 then Error('count'); P.FindFirst(); if P."Insert Permission" <> P."Insert Permission"::Yes then Error('insert'); if P."Object Type" <> P."Object Type"::"Table Data" then Error('type'); end; }"#,
    );
    for t in ["Listed", "UndeclaredIsNotAssignable", "Grants"] {
        passes(&o, t);
    }
}

#[test]
fn this_ready_isolation_member_and_filter_group_setter_lower() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  local procedure Twice(N: Integer): Integer begin exit(N * 2); end;
  [Test] procedure ThisCall() begin if this.Twice(2) <> 4 then Error('this'); end;
  [Test] procedure Setters() var R: Record "Thing"; begin R.ReadIsolation := R.ReadIsolation::ReadUnCommitted; R.FilterGroup := 2; if R.FilterGroup <> 2 then Error('group'); end; }"#,
    );
    passes(&o, "ThisCall");
    passes(&o, "Setters");
}

/// Compiling every table of the System app (eager) must not make the Key table panic on a platform
/// table that has no provider: it has no usable members, so it has no fields and no keys, and its
/// Key rows are refused like those of any table nothing compiled. The tables with a real
/// declaration still answer (corpus record/TestKeyVirtualTable.al:67-80).
#[test]
fn the_key_table_survives_an_eager_compile_of_platform_tables_without_a_provider() {
    let o = outcomes_with(
        r#"table 50001 "Probe" { fields { field(1; A; Integer) { } field(2; B; Integer) { } }
  keys { key(PK; A) { Clustered = true; } key(S; B, A) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure CountsTheDeclaredKeysAndTheSystemIdKey() var K: Record "Key"; begin
    K.SetRange(TableNo, Database::"Probe");
    if K.Count() <> 3 then Error('count %1', K.Count()); end; }"#,
        al_sema::check::Roots {
            eager: true,
            ..Default::default()
        },
    );
    passes(&o, "CountsTheDeclaredKeysAndTheSystemIdKey");
}

/// Change Log Management assigns `xRecRef.ReadIsolation` on a RecordRef parameter (Base
/// Application, ChangeLogManagement.Codeunit.al). Assigning it compiles and runs on a parameter
/// and on a local RecordRef, as the corpus pins for a plain one (record/TestRecordLock.al:86,
/// session/TestBCLockingContracts.al:14, both `*_IsCallable`).
#[test]
fn read_isolation_is_assignable_on_a_recordref_parameter_and_a_local_one() {
    let o = outcomes(
        r#"table 50002 "Probe" { fields { field(1; A; Integer) { } } keys { key(PK; A) { } } }
codeunit 50001 "H" {
  procedure OnParam(var RecRef: RecordRef; xRecRef: RecordRef) begin xRecRef.ReadIsolation := xRecRef.ReadIsolation::ReadCommitted; end;
  procedure OnLocal() var R: RecordRef; begin R.Open(50002); R.ReadIsolation := R.ReadIsolation::ReadCommitted; end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Param() var H: Codeunit H; R: RecordRef; begin R.Open(50002); H.OnParam(R, R); end;
  [Test] procedure Local() var H: Codeunit H; begin H.OnLocal(); end; }"#,
    );
    passes(&o, "Param");
    passes(&o, "Local");
}

/// `Database::"X"` is the table's id, a constant of its declaration. A platform table nothing
/// provides rows for is refused where code reads or writes it as a record, not where code only
/// names its id (Base Application's Change Log Management lists such tables by id).
#[test]
fn the_id_of_a_platform_table_with_no_provider_can_be_named() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure NamesTheId() var Id: Integer; begin Id := Database::"License Information"; if Id <> 2000000040 then Error('id %1', Id); end;
  [Test] procedure ReadingTheRowsIsStillRefused() var P: Record "License Information"; begin if P.IsEmpty() then; end; }"#,
    );
    passes(&o, "NamesTheId");
    let why = not_run(&o, "ReadingTheRowsIsStillRefused");
    assert!(why.contains("has no provider"), "{why}");
}

/// Report Layout List, Report Metadata and Report Data Items (handlers/TestSourceReportLayoutFlags.al,
/// TestPrecompiledReportLayouts.al, TestReportMetadataColumns.al).
#[test]
fn a_report_is_listed_with_its_layouts_metadata_and_data_items() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } field(2; Kind; Option) { OptionMembers = A,B; } field(3; Name; Text[30]) { } }
  keys { key(PK; "No.") { Clustered = true; } key(ByName; Name, Kind) { } } }
report 50002 "R" { UseRequestPage = false; ProcessingOnly = false;
  dataset { dataitem(Thing; Thing) { DataItemTableView = sorting(Name, Kind) order(descending); RequestFilterFields = Name, "No.";
    dataitem(Child; Thing) { DataItemLink = "No." = field("No."); } } }
  rendering { layout(L1) { Type = Excel; LayoutFile = 'a.xlsx'; ExcelLayoutMultipleDataSheets = true; }
    layout(L2) { Type = Word; LayoutFile = 'b.docx'; ObsoleteState = Pending; ObsoleteReason = 'x'; ObsoleteTag = '1'; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Layouts() var L: Record "Report Layout List"; begin L.SetRange("Report ID", 50002); if L.Count() <> 2 then Error('count %1', L.Count());
    L.SetRange(Name, 'L1'); L.FindFirst(); if L."Layout Format" <> L."Layout Format"::Excel then Error('format'); if L.IsObsolete then Error('obsolete');
    if L.ExcelLayoutMultipleDataSheets <> L.ExcelLayoutMultipleDataSheets::"Multiple data sheets" then Error('sheets');
    L.SetRange(Name, 'L2'); L.FindFirst(); if not L.IsObsolete then Error('not obsolete');
    // A layout with no Caption or Summary reads its name for both (session/PWPlatformTables.al:126-139).
    if L.Caption <> 'L2' then Error('caption [%1]', L.Caption); if L.Description <> 'L2' then Error('description'); end;
  [Test] procedure Metadata() var M: Record "Report Metadata"; begin M.Get(50002); if M.ProcessingOnly then Error('processing'); if M.UseRequestPage then Error('request page'); if M.FirstDataItemTableID <> Database::Thing then Error('table'); end;
  [Test] procedure DataItems() var D: Record "Report Data Items"; begin D.SetRange("Report ID", 50002); if D.Count() <> 2 then Error('count'); D.SetRange(Name, 'Thing'); D.FindFirst();
    if D.Name <> 'Thing' then Error('name'); if D."Indentation Level" <> 0 then Error('indent'); if D."Request Filter Fields" <> '3,1' then Error('rff %1', D."Request Filter Fields");
    if D."Sorting Fields" <> '3,2' then Error('sorting %1', D."Sorting Fields"); if D."Data Item Table View" <> 'SORTING(Field3,Field2) ORDER(1)' then Error('view %1', D."Data Item Table View");
    D.SetRange(Name, 'Child'); D.FindFirst(); if D.Name <> 'Child' then Error('child'); if D."Indentation Level" <> 1 then Error('child indent'); end; }"#,
    );
    for t in ["Layouts", "Metadata", "DataItems"] {
        passes(&o, t);
    }
}

/// Report Metadata TransactionType (11) is UpdateNoLocks for a report that declares none
/// (metadata/lt-report-metadata-columns/ReportMetadata.al:210, :236, :262). A declared one is pinned by no
/// corpus test: reading it is refused.
#[test]
fn report_metadata_transaction_type_is_update_no_locks_when_not_declared() {
    let o = outcomes(
        r#"report 50002 "Plain" { ProcessingOnly = true; }
report 50003 "Snap" { ProcessingOnly = true; TransactionType = Snapshot; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Undeclared() var M: Record "Report Metadata"; begin M.Get(50002); if Format(M.TransactionType) <> 'UpdateNoLocks' then Error('got %1', M.TransactionType); end;
  [Test] procedure Declared() var M: Record "Report Metadata"; begin M.Get(50003); if Format(M.TransactionType) = '' then; end; }"#,
    );
    passes(&o, "Undeclared");
    let why = not_run(&o, "Declared");
    assert!(why.contains("TransactionType"), "{why}");
}

/// Report Metadata EnableExternalAssemblies (8) is No for a report that declares none (report/lt-y-report-props/
/// YReportPropsTests.al:45); a declared one is pinned by no corpus test, so reading it is refused.
#[test]
fn report_metadata_enable_external_assemblies_is_refused_when_declared() {
    let o = outcomes(
        r#"report 50002 "Plain" { ProcessingOnly = true; }
report 50003 "Ext" { ProcessingOnly = true; EnableExternalAssemblies = true; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Undeclared() var M: Record "Report Metadata"; begin M.Get(50002); if M.EnableExternalAssemblies then Error('yes'); end;
  [Test] procedure Declared() var M: Record "Report Metadata"; begin M.Get(50003); if M.EnableExternalAssemblies then; end; }"#,
    );
    passes(&o, "Undeclared");
    let why = not_run(&o, "Declared");
    assert!(why.contains("EnableExternalAssemblies"), "{why}");
}

/// Field ExternalName (10) is blank and SQLDataType (23) Varchar for a Text field that declares neither
/// (record/PZFieldObsoleteRows.al:48, :52). A declared ExternalName and the SQLDataType of any other type are
/// pinned by no corpus test: reading them is refused.
#[test]
fn field_external_name_and_sql_data_type_are_answered_only_where_pinned() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } field(2; Name; Text[30]) { } field(3; Ext; Text[30]) { ExternalName = 'ext_col'; } }
  keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure TextField() var F: Record Field; begin F.Get(50001, 2); if F.ExternalName <> '' then Error('ext'); if Format(F.SQLDataType) <> 'Varchar' then Error('sql %1', F.SQLDataType); end;
  [Test] procedure DeclaredExternalName() var F: Record Field; begin F.Get(50001, 3); if F.ExternalName = '' then; end;
  [Test] procedure IntegerSqlDataType() var F: Record Field; begin F.Get(50001, 1); if Format(F.SQLDataType) = '' then; end; }"#,
    );
    passes(&o, "TextField");
    let why = not_run(&o, "DeclaredExternalName");
    assert!(why.contains("ExternalName"), "{why}");
    let why = not_run(&o, "IntegerSqlDataType");
    assert!(why.contains("SQLDataType"), "{why}");
}

/// XmlPort Metadata and Query Metadata (xmlport/TestXmlPortMetadataVirtualTable.al,
/// query/TestQueryMetadataVirtualTable.al).
#[test]
fn xmlports_and_queries_are_listed_from_their_declarations() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
xmlport 50002 "X" { Caption = 'X Caption'; Direction = Export; Format = VariableText; UseRequestPage = false; TransactionType = Browse; TextEncoding = UTF16;
  schema { textelement(Root) { tableelement(Row; Thing) { fieldelement(No; Row."No.") { } } } } }
query 50003 "Q" { QueryType = API; APIPublisher = 'pub'; APIGroup = 'grp'; APIVersion = 'v2.0'; EntityName = 'q'; EntitySetName = 'qs';
  elements { dataitem(Thing; Thing) { column(No; "No.") { } } } }
query 50004 "Plain" { elements { dataitem(Thing; Thing) { column(No; "No.") { } } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure XmlPort() var M: Record "XmlPort Metadata"; begin M.Get(50002); if M.Name <> 'X' then Error('name'); if M.Caption <> 'X Caption' then Error('caption'); if M.UseRequestPage then Error('request page');
    if M.Direction <> M.Direction::Export then Error('direction'); if M.Format <> M.Format::VariableText then Error('format'); if M.TransactionType <> M.TransactionType::Browse then Error('tx'); if M.TextEncoding <> M.TextEncoding::UTF16 then Error('enc'); if M.Get(50009) then Error('invented'); end;
  [Test] procedure Query() var M: Record "Query Metadata"; P: Record "Query Metadata"; begin M.Get(50003); if M.APIPublisher <> 'pub' then Error('publisher'); if M.APIVersion <> 'v2.0' then Error('version'); if M.EntitySetName <> 'qs' then Error('set');
    P.Get(50004); if P.APIPublisher <> '' then Error('plain publisher'); if P.Caption <> 'Plain' then Error('caption %1', P.Caption); end; }"#,
    );
    passes(&o, "XmlPort");
    passes(&o, "Query");
}

/// An xmlport that leaves `Encoding` out reads UTF16 (xmlport/TestPlatformMetadataDefaultsContracts.al:93),
/// not UTF8; one that declares it is not pinned and its row is refused.
#[test]
fn an_xmlport_without_an_encoding_property_reads_utf16() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
xmlport 50002 "X" { schema { textelement(Root) { tableelement(Row; Thing) { fieldelement(No; Row."No.") { } } } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Default() var M: Record "XmlPort Metadata"; begin M.Get(50002); if M.Encoding <> M.Encoding::UTF16 then Error('encoding'); end; }"#,
    );
    passes(&o, "Default");
}

/// "Page Metadata"."CardPageID" is the id of the page named in CardPageId and 0 when the page states none
/// (handlers/TestPageCloseTriggersSelectionCardPageContracts.al:305-315).
#[test]
fn a_pages_card_page_id_is_the_stated_page_or_zero() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50002 "Card" { PageType = Card; SourceTable = Thing; CardPageId = "Card"; layout { area(Content) { field(No; Rec."No.") { } } } }
page 50003 "List" { PageType = List; SourceTable = Thing; CardPageId = "Card"; layout { area(Content) { repeater(R) { field(No; Rec."No.") { } } } } }
page 50004 "Plain List" { PageType = List; SourceTable = Thing; layout { area(Content) { repeater(R) { field(No; Rec."No.") { } } } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure CardPage() var M: Record "Page Metadata"; begin
    M.Get(50003); if M.CardPageID <> 50002 then Error('list %1', M.CardPageID);
    M.Get(50002); if M.CardPageID <> 50002 then Error('card %1', M.CardPageID);
    M.Get(50004); if M.CardPageID <> 0 then Error('plain %1', M.CardPageID); end; }"#,
    );
    passes(&o, "CardPage");
}

/// "Page Metadata" reads a property as the page's extensions leave it: one extension's value wins over the page's
/// (pageext/lt-z-page-ext-props/ZPageExtProps.al:629, 685-1110), an extension of another page changes nothing, and two
/// extensions that set it are not pinned, so the row is refused.
#[test]
fn a_page_metadata_property_is_the_one_extension_value_and_two_extensions_are_refused() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50002 "Card" { PageType = Card; SourceTable = Thing; layout { area(Content) { field(No; Rec."No.") { } } } }
page 50003 "Other" { PageType = Card; SourceTable = Thing; InsertAllowed = false; layout { area(Content) { field(No; Rec."No.") { } } } }
page 50004 "Twice" { PageType = Card; SourceTable = Thing; layout { area(Content) { field(No; Rec."No.") { } } } }
pageextension 50010 "Card Ext" extends "Card" { InsertAllowed = false; }
pageextension 50011 "Twice A" extends "Twice" { ModifyAllowed = false; }
pageextension 50012 "Twice B" extends "Twice" { ModifyAllowed = false; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure OneExtension() var M: Record "Page Metadata"; begin
    M.Get(50002); if M.InsertAllowed then Error('card'); if not M.ModifyAllowed then Error('card modify');
    M.Get(50003); if M.InsertAllowed then Error('other'); end;
  [Test] procedure TwoExtensions() var M: Record "Page Metadata"; begin
    M.Get(50004); end; }"#,
    );
    passes(&o, "OneExtension");
    let why = match o.iter().find(|(n, _)| n == "TwoExtensions").map(|(_, o)| o) {
        Some(Outcome::NotRun(why) | Outcome::Fail(why)) => why.clone(),
        other => panic!("TwoExtensions was expected to be refused, got {other:?}"),
    };
    assert!(why.contains("2 page extensions set ModifyAllowed"), "{why}");
}

/// Page Action (record/TestPageActionVirtualTable.al): the area is a container row, a group and its
/// actions nest under it.
#[test]
fn a_pages_actions_are_rows_nested_under_their_area_and_group() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50002 "P" { PageType = Card; SourceTable = Thing; layout { area(Content) { field(No; Rec."No.") { } } }
  actions { area(Processing) { group(G) { action(A) { RunObject = page P; } action(B) { } } } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Nesting() var G: Record "Page Action"; A: Record "Page Action"; C: Record "Page Action"; begin
    G.SetRange("Page ID", 50002); G.SetRange(Name, 'G'); G.FindFirst();
    C.SetRange("Page ID", 50002); C.SetRange("Action ID", G."Parent Action ID"); C.FindFirst();
    if C.Indentation <> 0 then Error('container'); if G.Indentation <> 1 then Error('group');
    if C."Action Type" = G."Action Type" then Error('types');
    A.SetRange("Page ID", 50002); A.SetRange(Name, 'A'); A.FindFirst();
    if A.Indentation <> 2 then Error('action'); if A."Parent Action ID" <> G."Action ID" then Error('parent'); if A.RunObjectID <> 50002 then Error('run object');
    A.SetRange(Name, 'B'); A.FindFirst(); if A.RunObjectID <> 0 then Error('plain run object'); end; }"#,
    );
    passes(&o, "Nesting");
}

/// Event Subscription (record/TestEventSubscriptionVirtualTable.al): rows of compiled subscribers.
#[test]
fn event_subscribers_are_rows_keyed_by_codeunit_and_function() {
    let o = outcomes(
        r#"codeunit 50001 "Pub" { [IntegrationEvent(false, false)] procedure OnThing() begin end; [BusinessEvent(false)] procedure OnBusiness() begin end; }
codeunit 50002 "Sub" {
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Pub", 'OnThing', '', false, false)] local procedure H1() begin end;
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Pub", 'OnBusiness', '', false, false)] local procedure H2() begin end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var E: Record "Event Subscription"; begin
    E.SetRange("Subscriber Codeunit ID", 50002); if E.Count() <> 2 then Error('count %1', E.Count());
    E.Get(50002, 'H1'); if E."Published Function" <> 'OnThing' then Error('function'); if E."Event Type" <> E."Event Type"::Integration then Error('type');
    if E."Publisher Object Type" <> E."Publisher Object Type"::Codeunit then Error('object type'); if E."Publisher Object ID" <> 50001 then Error('object id'); if not E.Active then Error('active');
    E.Get(50002, 'H2'); if E."Event Type" <> E."Event Type"::Business then Error('business');
    if E.Get(50002, 'Nope') then Error('invented'); E.SetRange("Subscriber Codeunit ID", 50001); if not E.IsEmpty() then Error('publisher'); end; }"#,
    );
    passes(&o, "Rows");
}

/// A table trigger event bound to a field is published as `<event> - <field>`
/// (xmlport/TestPlatformMetadataDefaultsContracts.al:113-124).
#[test]
fn a_validate_subscription_on_a_field_names_the_field_in_the_published_function() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } field(2; Txt; Text[10]) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50002 "Sub" {
  [EventSubscriber(ObjectType::Table, Database::"Thing", 'OnAfterInsertEvent', '', false, false)] local procedure H1() begin end;
  [EventSubscriber(ObjectType::Table, Database::"Thing", 'OnBeforeValidateEvent', 'Txt', false, false)] local procedure H2() begin end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var E: Record "Event Subscription"; begin
    E.Get(50002, 'H1'); if E."Published Function" <> 'OnAfterInsertEvent' then Error('plain %1', E."Published Function");
    E.Get(50002, 'H2'); if E."Published Function" <> 'OnBeforeValidateEvent - Txt' then Error('field %1', E."Published Function"); end; }"#,
    );
    passes(&o, "Rows");
}

/// Table Relations Metadata (record/TestTableRelationsMetadataVirtualTable.al), including a
/// condition that is a `filter(...)`, which the runtime's own relation check does not understand.
#[test]
fn table_relations_are_rows_per_arm_and_condition() {
    let o = outcomes(
        r#"table 50001 "Parent" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50002 "Parent B" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50003 "Child" { fields { field(1; "Entry No."; Integer) { } field(2; Kind; Option) { OptionMembers = A,B; } field(3; Doc; Option) { OptionMembers = X,Y; }
    field(4; Plain; Code[20]) { TableRelation = Parent.Code; } field(5; Soft; Code[20]) { }
    field(6; Cond; Code[20]) { TableRelation = if (Kind = const(A)) Parent.Code else "Parent B".Code; }
    field(7; Filtered; Code[20]) { TableRelation = if (Kind = const(A), Doc = filter(<> X)) Parent where(Code = filter(<> '')); } }
  keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var R: Record "Table Relations Metadata"; begin
    R.SetRange("Table ID", 50003); R.SetRange("Field No.", 4); if R.Count() <> 1 then Error('plain'); R.FindFirst(); if R."Related Table ID" <> 50001 then Error('table'); if R."Related Field No." <> 1 then Error('field');
    R.SetRange("Field No.", 5); if not R.IsEmpty() then Error('soft');
    R.SetRange("Field No.", 6); if R.Count() <> 2 then Error('cond'); R.FindSet(); if R."Relation No." <> 1 then Error('first'); if R."Condition Field No." <> 2 then Error('cond field'); R.Next(); if R."Related Table ID" <> 50002 then Error('else');
    R.SetRange("Field No.", 7); if R.Count() <> 2 then Error('two conditions %1', R.Count()); R.FindFirst(); if R."Condition Field No." <> 2 then Error('first condition'); R.Next(); if R."Condition Field No." <> 3 then Error('second condition'); end;
  [Test] procedure FieldRelation() var F: Record "Field"; begin F.Get(50003, 7); if F.RelationTableNo <> 50001 then Error('filtered relation %1', F.RelationTableNo); if F.RelationFieldNo <> 1 then Error('related field'); end;
  [Test] procedure ConditionalRelation() var F: Record "Field"; begin F.Get(50003, 6); if (F.RelationTableNo <> 0) or (F.RelationFieldNo <> 0) then Error('conditional relation reads 0/0'); end; }"#,
    );
    passes(&o, "FieldRelation");
    // record/TestPlatformMetadataDeclarationContracts.al:262-264.
    passes(&o, "ConditionalRelation");
    passes(&o, "Rows");
}

/// A tableextension `modify(..) { TableRelation }` on a field whose base relation is unconditional: the Field row keeps
/// the base's table when the extension adds an unconditional relation (tableextension/TestTableExtFieldTableRelation.al:266)
/// or a conditional one with an else arm (:532). An extension relation without an else arm is not pinned: refused.
/// A conditional base relation reads 0 (:368, :409, :450, :491).
#[test]
fn field_row_of_an_unconditional_base_relation_modified_by_an_extension() {
    let o = outcomes(
        r#"table 50001 "A" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50002 "B" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50003 "Base" { fields { field(1; Id; Integer) { } field(2; Kind; Option) { OptionMembers = X,Y,Z; }
    field(3; Uncond; Code[20]) { TableRelation = A; } field(4; Else; Code[20]) { TableRelation = A; } field(5; NoElse; Code[20]) { TableRelation = A; }
    field(6; CondBase; Code[20]) { TableRelation = if (Kind = const(X)) A; } }
  keys { key(PK; Id) { Clustered = true; } } }
tableextension 50004 "Ext" extends "Base" { fields { modify(Uncond) { TableRelation = B; }
    modify(Else) { TableRelation = if (Kind = const(Y)) B else B; } modify(NoElse) { TableRelation = if (Kind = const(Y)) B; } modify(CondBase) { TableRelation = B; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Uncond() var F: Record "Field"; begin F.Get(50003, 3); if (F.RelationTableNo <> 50001) or (F.RelationFieldNo <> 1) then Error('uncond %1', F.RelationTableNo); end;
  [Test] procedure ElseArm() var F: Record "Field"; begin F.Get(50003, 4); if F.RelationTableNo <> 50001 then Error('else %1', F.RelationTableNo); end;
  [Test] procedure NoElse() var F: Record "Field"; begin F.Get(50003, 5); if F.RelationTableNo <> 50001 then Error('noelse %1', F.RelationTableNo); end;
  [Test] procedure CondBase() var F: Record "Field"; begin F.Get(50003, 6); if (F.RelationTableNo <> 0) or (F.RelationFieldNo <> 0) then Error('condbase %1', F.RelationTableNo); end; }"#,
    );
    passes(&o, "Uncond");
    passes(&o, "ElseArm");
    assert!(
        not_run(&o, "NoElse").contains("TableRelation"),
        "{:?}",
        not_run(&o, "NoElse")
    );
    // A conditional base relation reads 0 whatever the extension adds (:368, :409, :450, :491).
    passes(&o, "CondBase");
}

/// The code coverage tables hold nothing until a recording starts; after one they hold what ran
/// (codeunit/TestCodeCoverageTable.al, TestCodeCoveragePerTest.al). `Line No.` is a source line.
#[test]
fn code_coverage_tables_hold_what_was_recorded_and_delete_all_empties_them() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  procedure Initialize() var C: Record "Code Coverage"; R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin CodeCoverageLog(false, false); C.DeleteAll(); R.DeleteAll(); L.DeleteAll(); end;
  procedure Branch(V: Integer): Integer
  begin
    if V > 1 then
      exit(1);
    exit(0);
  end;
  [Test] procedure EmptyAtFirst() var C: Record "Code Coverage"; begin if not C.IsEmpty() then Error('rows'); end;
  [Test] procedure RowsAfterRecording()
  var C: Record "Code Coverage";
  begin Initialize();
    CodeCoverageLog(true, false); Branch(5); CodeCoverageLog(false, false);
    C.SetRange("Object Type", C."Object Type"::Codeunit); C.SetRange("Object ID", 50000);
    C.SetRange("Line Type", C."Line Type"::"Trigger/Function"); C.SetRange("Line No.", 3);
    if C.IsEmpty() then Error('declaration row');
    C.SetRange("Line Type", C."Line Type"::Code); C.SetRange("Line No.", 6);
    C.FindFirst(); if C."No. of Hits" = 0 then Error('hit');
    C.SetRange("Line No.", 7);
    C.FindFirst(); if C."No. of Hits" <> 0 then Error('not hit');
  end;
  [Test] procedure NothingRecordedWhileOff()
  var C: Record "Code Coverage";
  begin Initialize();
    Branch(5); C.SetRange("Object ID", 50000);
    if not C.IsEmpty() then Error('rows');
  end;
  [Test] procedure DeleteAllEmptiesIt() var C: Record "Code Coverage"; begin Initialize(); CodeCoverageLog(true, false); Branch(1); CodeCoverageLog(false, false); C.DeleteAll(); if not C.IsEmpty() then Error('rows'); end;
  [Test] procedure AFilteredDeleteAllLeavesTheOtherRows() var C: Record "Code Coverage"; begin Initialize(); CodeCoverageLog(true, false); Branch(1); CodeCoverageLog(false, false); C.SetRange("Object ID", 1); C.DeleteAll(); C.Reset(); if C.IsEmpty() then Error('gone'); end;
  [Test] procedure ARecordingAfterDeleteAllStartsFromNothing()
  var C: Record "Code Coverage";
  begin Initialize();
    CodeCoverageLog(true, false); Branch(1); CodeCoverageLog(false, false); C.DeleteAll();
    CodeCoverageLog(true, false); CodeCoverageLog(false, false);
    C.SetRange("Object ID", 50000); C.SetRange("Line No.", 6);
    if not C.IsEmpty() then Error('old rows came back');
  end; }"#,
    );
    passes(&o, "EmptyAtFirst");
    passes(&o, "RowsAfterRecording");
    passes(&o, "NothingRecordedWhileOff");
    passes(&o, "DeleteAllEmptiesIt");
    passes(&o, "AFilteredDeleteAllLeavesTheOtherRows");
    passes(&o, "ARecordingAfterDeleteAllStartsFromNothing");
}

/// codeunit/TestCodeCoverageLineTextContracts.al: the `begin` and `end` lines have `Empty` rows, every row's `Line` holds the
/// platform's text for an app without source, and the status is Covered, Not Covered or Non Applicable.
#[test]
fn code_coverage_rows_of_an_app_without_source_hold_the_platform_text_and_a_status() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  procedure Branch(V: Integer): Integer
  begin
    if V > 1 then
      exit(1);
    exit(0);
  end;
  [Test] procedure Rows()
  var C: Record "Code Coverage"; Rows: Text;
  begin
    CodeCoverageLog(false, false); C.DeleteAll();
    CodeCoverageLog(true, false); Branch(5); CodeCoverageLog(false, false);
    C.SetRange("Object ID", 50000); C.SetRange("Line No.", 2, 7);
    C.FindSet();
    repeat
      if not C.Line.StartsWith('The extension that declares this object has its manifest property ShowMyCode') then Error('line text %1', C.Line);
      Rows += Format(C."Line No.") + ' ' + Format(C."Line Type") + ' ' + Format(C."No. of Hits") + ' ' + Format(C."Code Coverage Status") + '|';
    until C.Next() = 0;
    if Rows <> '2 Trigger/Function 0 Non Applicable|3 Empty 0 Non Applicable|4 Code 1 Covered|5 Code 1 Covered|6 Code 0 Not Covered|7 Empty 0 Non Applicable|' then Error(Rows);
  end; }"#,
    );
    passes(&o, "Rows");
}

/// A `[Test]` procedure that starts while recording has a Tests Run row, and the methods that run
/// inside it a Test Lookup row each; a call outside any test leaves none
/// (codeunit/TestCodeCoveragePerTest.al).
#[test]
fn code_coverage_notes_the_tests_that_start_while_recording() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  procedure Initialize() var C: Record "Code Coverage"; R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin CodeCoverageLog(false, false); C.DeleteAll(); R.DeleteAll(); L.DeleteAll(); end;
  procedure Helper(): Integer begin exit(1); end;
  [Test] procedure Inner() begin Helper(); end;
  [Test] procedure ATestStartedWhileRecording()
  var R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup";
  begin Initialize();
    CodeCoverageLog(true, false); Inner(); CodeCoverageLog(false, false);
    R.SetRange("Object ID", 50000);
    if R.Count() <> 1 then Error('one test row');
    R.FindFirst(); if R."Method Name" <> 'Inner' then Error('name');
    L.SetRange("Object ID", 50000); L.SetRange("Test ID", R."Test ID");
    if L.Count() <> 1 then Error('one method inside the test');
    L.FindFirst(); if L."Method ID" = R."Method ID" then Error('the test itself is no method inside it');
  end;
  [Test] procedure ACallOutsideAnyTest()
  var R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup";
  begin Initialize();
    CodeCoverageLog(true, false); Helper(); CodeCoverageLog(false, false);
    R.SetRange("Object ID", 50000); L.SetRange("Object ID", 50000);
    if not R.IsEmpty() then Error('tests run');
    if not L.IsEmpty() then Error('lookup');
  end; }"#,
    );
    passes(&o, "ATestStartedWhileRecording");
    passes(&o, "ACallOutsideAnyTest");
}

/// All Profile (profile/TestAllProfileTable.al): stored and seeded from the profile objects. The delete
/// rule for an installed app's row needs an app with a GUID id: the corpus pins it.
#[test]
fn a_profile_object_is_a_row_and_a_tenant_row_can_be_added_and_removed() {
    let o = outcomes(
        r#"page 50001 "Center" { PageType = RoleCenter; }
profile "My Profile" { Caption = 'My Caption'; RoleCenter = Center; Enabled = true; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Present() var P: Record "All Profile"; begin P.SetRange("Profile ID", 'My Profile'); P.FindFirst(); if P.Caption <> 'My Caption' then Error('caption'); if P."Role Center ID" <> 50001 then Error('role center'); end;
  [Test] procedure TenantRow() var P: Record "All Profile"; begin P.Scope := P.Scope::Tenant; P."Profile ID" := 'Mine'; P.Caption := 'Mine'; P.Enabled := false; P.Insert(); P.Get(P.Scope::Tenant, P."App ID", 'Mine'); P.Delete(); if P.Get(P.Scope::Tenant, P."App ID", 'Mine') then Error('still there'); end; }"#,
    );
    passes(&o, "Present");
    passes(&o, "TenantRow");
}

/// Base Application's `profile "ORDER PROCESSOR"` names its role center by page id (`RoleCenter = 9006;`): the row
/// exists with that id (about 320 Microsoft tests start with `AllProfile.SetRange("Profile ID", 'ORDER PROCESSOR')`).
#[test]
fn a_profile_that_names_its_role_center_by_page_id_is_a_row() {
    let o = outcomes(
        r#"page 50001 "Center" { PageType = RoleCenter; }
profile "BY ID" { RoleCenter = 50001; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ById() var P: Record "All Profile"; begin P.SetRange("Profile ID", 'BY ID'); P.FindFirst(); if P."Role Center ID" <> 50001 then Error('role center %1', P."Role Center ID"); end; }"#,
    );
    passes(&o, "ById");
}

/// A platform table nothing provides rows for is still a table: a temporary record of it holds what
/// AL inserts, a variable that is declared and never read does not stop the code that has it, and
/// only a read or write of its persistent rows is refused, naming table and id. The corpus pins that
/// `temporary` borrows the shape of a platform table and the provider never sees the record
/// (record/TestTemporaryVirtualTableRecord.al, for Field, Date and Aggregate Permission Set);
/// WMS Management keeps its error log in `Record "License Information" temporary`.
#[test]
fn a_platform_table_without_provider_works_as_a_temporary_record_and_is_refused_otherwise() {
    let o = outcomes(
        r#"codeunit 50001 "Holder" { var L: Record "License Information" temporary; G: Record "License Information";
  procedure Log(N: Integer; T: Text) begin L."Line No." := N; L.Text := CopyStr(T, 1, 250); L.Insert(); end;
  procedure Count(): Integer begin exit(L.Count()); end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure TemporaryRoundTrips() var H: Codeunit Holder; begin H.Log(1, 'a'); H.Log(2, 'b'); if H.Count() <> 2 then Error('count %1', H.Count()); end;
  [Test] procedure DeclaredAndNeverRead() var G: Record "License Information"; begin end;
  [Test] procedure PersistentReadIsRefused() var G: Record "License Information"; begin if G.Get(1) then; end;
  [Test] procedure PersistentWriteIsRefused() var L: Record "License Information"; begin L."Line No." := 1; L.Insert(); end; }"#,
    );
    passes(&o, "TemporaryRoundTrips");
    passes(&o, "DeclaredAndNeverRead");
    for t in ["PersistentReadIsRefused", "PersistentWriteIsRefused"] {
        let why = not_run(&o, t);
        assert!(why.contains("has no provider"), "{t}: {why}");
    }
    let why = not_run(&o, "PersistentReadIsRefused");
    assert!(
        why.contains("platform table 'License Information' (2000000040)"),
        "{why}"
    );
}

/// Columns that the declaration describes, filled from it: the caption, card page and editability of
/// a page, and the table name, caption, class and enabled flag of a field. Not pinned by a corpus
/// test: Page Management reads CardPageID off the lookup page and Microsoft's tests read the others
/// (docs/probes-needed.md, "Probe batch 5").
#[test]
fn declared_caption_card_page_class_and_enabled_are_read_from_the_declaration() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { Caption = 'Number'; } field(2; Total; Decimal) { FieldClass = FlowField; CalcFormula = count(Thing); Caption = 'The total'; }
    field(3; Filt; Integer) { FieldClass = FlowFilter; } field(4; Off; Integer) { Enabled = false; } field(5; Plain; Text[10]) { } }
  keys { key(PK; "No.") { Clustered = true; } } }
page 50002 "Thing Card" { PageType = Card; SourceTable = Thing; Caption = 'A card'; }
page 50003 "Thing List" { PageType = List; SourceTable = Thing; CardPageID = "Thing Card"; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Pages() var M: Record "Page Metadata"; begin M.Get(50003); if M.CardPageID <> 50002 then Error('card %1', M.CardPageID); if M.Caption <> 'Thing List' then Error('caption %1', M.Caption);
    M.Get(50002); if M.CardPageID <> 0 then Error('no card'); if M.Caption <> 'A card' then Error('card caption %1', M.Caption); end;
  [Test] procedure Fields() var F: Record "Field"; begin
    F.Get(50001, 1); if F.TableName <> 'Thing' then Error('table name'); if F."Field Caption" <> 'Number' then Error('caption %1', F."Field Caption"); if F.Class <> F.Class::Normal then Error('class'); if not F.Enabled then Error('enabled');
    F.Get(50001, 2); if F.Class <> F.Class::FlowField then Error('flowfield'); if F."Field Caption" <> 'The total' then Error('total caption');
    F.Get(50001, 3); if F.Class <> F.Class::FlowFilter then Error('flowfilter'); if F."Field Caption" <> 'Filt' then Error('name as caption');
    F.Get(50001, 4); if F.Enabled then Error('disabled'); end; }"#,
    );
    passes(&o, "Pages");
    passes(&o, "Fields");
}

/// A key states `MaintainSIFTIndex` and `ObsoleteState`, a table its `DrillDownPageId`; the tables
/// report them (probe batch 5: no corpus test reads them).
#[test]
fn a_keys_sift_flag_and_obsolete_state_and_a_tables_drill_down_page_are_read_from_the_declaration() {
    let o = outcomes(
        r#"table 50001 "Thing" { DrillDownPageId = "Thing List"; fields { field(1; "No."; Integer) { } field(2; Amt; Decimal) { } }
  keys { key(PK; "No.") { Clustered = true; } key(S1; Amt) { MaintainSIFTIndex = false; } key(S2; Amt, "No.") { SumIndexFields = "No."; ObsoleteState = Pending; ObsoleteReason = 'x'; ObsoleteTag = '1'; } } }
page 50002 "Thing List" { PageType = List; SourceTable = Thing; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Keys() var K: Record "Key"; begin K.SetRange(TableNo, 50001); K.SetRange("No.", 1); K.FindFirst(); if K.MaintainSIFTIndex then Error('default sift'); if K.ObsoleteState <> K.ObsoleteState::No then Error('default state');
    K.SetRange("No.", 2); K.FindFirst(); if K.MaintainSIFTIndex then Error('sift'); K.SetRange("No.", 3); K.FindFirst(); if not K.MaintainSIFTIndex then Error('sum index fields'); if K.ObsoleteState <> K.ObsoleteState::Pending then Error('pending'); end;
  [Test] procedure Table() var M: Record "Table Metadata"; begin M.Get(50001); if M.DrillDownPageId <> 50002 then Error('drill down %1', M.DrillDownPageId); end; }"#,
    );
    passes(&o, "Keys");
    passes(&o, "Table");
}

#[test]
fn a_published_application_row_passes_the_tenant_filter_of_the_retention_policy_module() {
    // install/TestRetenPolAllowedTables_Tests.al: Base Application registers table 405 only when
    // System Application finds its Published Application row through
    // SetFilter("Tenant ID", '%1|%2', '', TenantId). The row must therefore satisfy that filter.
    let o = outcomes_app(
        "5d86850b-0d76-4eca-bd7b-951ad998e997",
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure OwnRowPassesTheTenantFilter() var P: Record "Published Application"; begin
    P.SetRange(ID, '5d86850b-0d76-4eca-bd7b-951ad998e997');
    P.SetFilter("Tenant ID", '%1|%2', '', 'any-tenant');
    if not P.FindFirst() then Error('no row'); end; }"#,
        al_sema::check::Roots::default(),
    );
    passes(&o, "OwnRowPassesTheTenantFilter");
}

/// A relation whose condition is `Kind = filter(A | B)` lists one row with Condition Type FILTER and the member
/// ordinals joined by `|` (session/PWPlatformTables.al:109-123, BC 27.5).
#[test]
fn a_filter_condition_of_option_members_lists_the_ordinals() {
    let o = outcomes(
        r#"table 50001 "Target" { fields { field(1; Id; Code[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50002 "Source" { fields { field(1; Id; Integer) { } field(2; Kind; Option) { OptionMembers = A,B,C; }
    field(3; Ref; Code[10]) { TableRelation = if (Kind = filter(A | B)) Target.Id; } }
  keys { key(PK; Id) { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var R: Record "Table Relations Metadata"; begin
    R.SetRange("Table ID", 50002); R.SetRange("Field No.", 3); R.FindFirst();
    if R."Condition Field No." <> 2 then Error('field %1', R."Condition Field No.");
    if Format(R."Condition Type") <> 'FILTER' then Error('type %1', Format(R."Condition Type"));
    if R."Condition Value" <> '0|1' then Error('value %1', R."Condition Value"); end; }"#,
    );
    passes(&o, "Rows");
}

/// types/ProbeTypeFormsAndFilters.al:FormatRecord_AssignmentCopiesBlobBytes (BC 27.5): a record assigned from one that
/// was never inserted shows its Blob as filled, and CalcFields then reads the Blob back empty (no stored row).
#[test]
fn calc_fields_of_a_blob_whose_row_is_not_stored_leaves_it_empty() {
    let o = outcomes(
        r#"table 50001 "Blobby" { fields { field(1; Id; Integer) { } field(2; Pic; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Copy() var R: Record "Blobby"; C: Record "Blobby"; Out: OutStream; Ins: InStream; T: Text; begin
    R.Id := 1; R.Pic.CreateOutStream(Out); Out.WriteText('abc'); C := R;
    if not C.Pic.HasValue() then Error('copy has no blob'); C.CalcFields(Pic); C.Pic.CreateInStream(Ins); Ins.ReadText(T);
    if T <> '' then Error('blob %1', T); end; }"#,
    );
    passes(&o, "Copy");
}

/// A filter on "Number of Calls" or "Active Manual Instances" is answered from the session's counts
/// (session/PWPlatformTables.al:EventSubscription_FilterOnNumberOfCalls, BC 27.5 observes it).
#[test]
fn event_subscriptions_can_be_filtered_on_the_session_counts() {
    let o = outcomes(
        r#"codeunit 50001 "Pub" { [IntegrationEvent(false, false)] procedure OnThing() begin end; procedure Raise() begin OnThing(); end; }
codeunit 50002 "Sub" { [EventSubscriber(ObjectType::Codeunit, Codeunit::"Pub", 'OnThing', '', false, false)] local procedure H1() begin end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Counts() var E: Record "Event Subscription"; P: Codeunit "Pub"; begin
    E.SetFilter("Number of Calls", '>0'); if E.Count() <> 0 then Error('before %1', E.Count());
    P.Raise(); E.Reset(); E.SetFilter("Number of Calls", '>0'); E.SetRange("Subscriber Codeunit ID", 50002);
    if E.Count() <> 1 then Error('after %1', E.Count()); E.FindFirst(); if E."Number of Calls" <> 1 then Error('calls');
    E.Reset(); E.SetRange("Subscriber Codeunit ID", 50002); E.SetRange("Active Manual Instances", 0); if E.Count() <> 1 then Error('manual'); end; }"#,
    );
    passes(&o, "Counts");
}

/// Tenant Web Service and Web Service take Insert, Modify and Delete from AL
/// (corpus tests/al-language-onprem/session/PZPlatformSurvey.al:178-187). A fresh tenant holds rows
/// in Tenant Web Service that nothing pins (only the count, 64, in PtWebServiceRows.al:136), so a read that would
/// count or scan them is refused, and a Get that finds nothing too.
#[test]
fn tenant_web_service_takes_writes_and_refuses_reads_of_rows_it_does_not_hold() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Writes() var R: Record "Tenant Web Service"; W: Record "Web Service"; begin
    R.Init(); R."Object Type" := R."Object Type"::Page; R."Service Name" := 'Zed'; R.Insert();
    R."Object ID" := 21; R.Modify(); if not R.Get(R."Object Type"::Page, 'Zed') then Error('lost'); R.Delete();
    W.Init(); W."Object Type" := W."Object Type"::Page; W."Service Name" := 'Zed'; W.Insert(); W.Delete(); end;
  [Test] procedure CountIsRefused() var R: Record "Tenant Web Service"; begin if R.Count() < 0 then; end;
  [Test] procedure MissIsRefused() var R: Record "Tenant Web Service"; begin if R.Get(0, 'Nope') then; end; }"#,
    );
    passes(&o, "Writes");
    for t in ["CountIsRefused", "MissIsRefused"] {
        let why = not_run(&o, t);
        assert!(why.contains("Tenant Web Service"), "{t}: {why}");
    }
}

// Web Service on a fresh tenant: 17 rows, every column (tests/al-language-onprem/platform/PtWebServiceRows.al:39-73,
// WebService_Content). Microsoft's DemoTool (Create Web Services) calls CreateWebService for page 42, which finds SalesOrder.
#[test]
fn web_service_holds_the_rows_of_a_fresh_tenant() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var W: Record "Web Service"; begin if W.Count() <> 17 then Error('count %1', W.Count()); W.FindFirst();
    if (W."Object Type" <> W."Object Type"::Codeunit) or (W."Service Name" <> 'CompanySetupService') or (W."Object ID" <> 1801) or not W.Published or W.ExcludeFieldsOutsideRepeater then Error('first');
    W.FindLast(); if (W."Service Name" <> 'TopCustomerOverview') or (W."Object ID" <> 100) then Error('last');
    if not W.Get(W."Object Type"::Page, 'SalesOrder') then Error('42'); if W."Object ID" <> 42 then Error('id');
    if W.Get(W."Object Type"::Page, 'Nope') then Error('nope'); end; }"#,
    );
    passes(&o, "Rows");
}

// Tenant Report Layout Selection on a fresh tenant: four rows, blank company and user, the layout names; the App ID is not null
// but its value is not pinned (platform/lt-platform-tables/PtCloudTables.al:378-432, QX_TenantReportLayoutSelection_RowsAndDemoShape).
// Insert(true) of an existing key answers false, Modify(true) true: Microsoft's DemoTool (Create Custom Report Layout) does that.
#[test]
fn tenant_report_layout_selection_holds_the_rows_of_a_fresh_tenant() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Rows() var S: Record "Tenant Report Layout Selection"; B: Guid; begin if S.Count() <> 4 then Error('count');
    if not S.Get(1302, '', B) then Error('1302'); if S."Layout Name" <> 'StandardSalesProFormaInv.docx' then Error('name');
    if S.Get(1302, CompanyName(), B) then Error('company'); S.FindLast(); if S."Layout Name" <> 'StandardSalesInvoiceBlueSimple.docx' then Error('last');
    S.Init(); S."Report ID" := 1306; S."Layout Name" := 'X.docx'; if S.Insert(true) then Error('insert'); if not S.Modify(true) then Error('modify');
    S.Get(1306, '', B); if S."Layout Name" <> 'X.docx' then Error('modified'); end;
  [Test] procedure AppIdIsNotPinned() var S: Record "Tenant Report Layout Selection"; begin S.FindFirst(); if IsNullGuid(S."App ID") then; end; }"#,
    );
    passes(&o, "Rows");
    let why = not_run(&o, "AppIdIsNotPinned");
    assert!(why.contains("App ID"), "{why}");
}

// OData Edm Type on a fresh tenant: six keys (PtCloudTables.al:435-492, QX_ODataEdmType_RowsAndEdmXml); their Description and
// Edm Xml are not pinned. An own row inserts, reads back and deletes as in any table.
#[test]
fn odata_edm_type_holds_the_keys_of_a_fresh_tenant() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Keys() var E: Record "OData Edm Type"; K: Text; begin if E.Count() <> 6 then Error('count'); E.FindSet(); repeat K += E."Key" + ','; until E.Next() = 0;
    if K <> 'BOOKINGSDATE,DIMENSION,DOCUMENTLINEOBJECTDETAILS,ITEM-UOM,ITEM-UOM-CONVERSION,POSTALADDRESS,' then Error(K);
    if E.Get('LTQX') then Error('get'); E.Init(); E."Key" := 'LTQX'; E.Description := 'd'; E.Insert(true); E.Get('LTQX'); if E.Description <> 'd' then Error('d'); E.Delete(); end;
  [Test] procedure DescriptionIsNotPinned() var E: Record "OData Edm Type"; begin E.Get('DIMENSION'); if E.Description = '' then; end;
  [Test] procedure EdmXmlIsNotPinned() var E: Record "OData Edm Type"; begin E.SetAutoCalcFields("Edm Xml"); E.Get('DIMENSION'); end; }"#,
    );
    passes(&o, "Keys");
    for t in ["DescriptionIsNotPinned", "EdmXmlIsNotPinned"] {
        let why = not_run(&o, t);
        assert!(why.contains("OData Edm Type"), "{t}: {why}");
    }
}

/// Field.IsPartOfPrimaryKey (column 28) is read in AL code: true for a field of the primary key
/// (corpus metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:237-268).
#[test]
fn is_part_of_primary_key_is_read_from_the_primary_key() {
    let o = outcomes(
        r#"table 50001 "K" { fields { field(1; A; Integer) {} field(2; B; Code[10]) {} field(3; C; Text[30]) {} } keys { key(PK; A, B) { Clustered = true; } key(S; C) {} } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Reads() var F: Record Field; begin
    F.SetRange(TableNo, 50001); F.SetRange(IsPartOfPrimaryKey, true); if F.Count() <> 2 then Error('count %1', F.Count());
    F.Reset(); F.Get(50001, 3); if F.IsPartOfPrimaryKey then Error('C is not in the key'); F.Get(50001, 2); if not F.IsPartOfPrimaryKey then Error('B is'); end; }"#,
    );
    passes(&o, "Reads");
}

/// Page Metadata InsertAllowed, ModifyAllowed and DeleteAllowed (columns 16 to 18): Yes unless the page
/// declares false; a page extension's literal wins over the base page's
/// (corpus pageext/lt-z-page-ext-props/ZPageExtProps.al:685-1110, ZDeleteAllowed_1 to ZModifyAllowed_4).
/// Two extensions that set the same property are not pinned and are refused.
#[test]
fn page_metadata_allowed_columns_follow_the_page_and_its_extension() {
    let o = outcomes(
        r#"page 50010 "B1" { PageType = List; }
pageextension 50011 "E1" extends "B1" { DeleteAllowed = false; }
page 50012 "B2" { PageType = List; InsertAllowed = false; ModifyAllowed = false; }
pageextension 50013 "E2" extends "B2" { InsertAllowed = true; }
page 50014 "B3" { PageType = List; }
page 50015 "B4" { PageType = List; }
pageextension 50016 "E4a" extends "B4" { DeleteAllowed = false; }
pageextension 50017 "E4b" extends "B4" { DeleteAllowed = true; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ExtensionFalse() var M: Record "Page Metadata"; begin M.Get(50010); if not M.InsertAllowed or not M.ModifyAllowed or M.DeleteAllowed then Error('x'); end;
  [Test] procedure BaseFalseExtensionTrue() var M: Record "Page Metadata"; begin M.Get(50012); if not M.InsertAllowed or M.ModifyAllowed or not M.DeleteAllowed then Error('x'); end;
  [Test] procedure Plain() var M: Record "Page Metadata"; begin M.Get(50014); if not (M.InsertAllowed and M.ModifyAllowed and M.DeleteAllowed) then Error('x'); end;
  [Test] procedure TwoExtensions() var M: Record "Page Metadata"; begin M.Get(50015); if M.DeleteAllowed then; end; }"#,
    );
    for t in ["ExtensionFalse", "BaseFalseExtensionTrue", "Plain"] {
        passes(&o, t);
    }
    let why = not_run(&o, "TwoExtensions");
    assert!(
        why.contains("DeleteAllowed") && why.contains("50015") || why.contains("B4"),
        "{why}"
    );
}

/// A column of a platform row whose value nothing pins is unknown: a dynamic read of it (FieldRef.Value, Format, a filter on
/// it, TransferFields into another type, a read or a filter on a copy) is refused and names the table and the field, while
/// the other columns, FindSet and Count of the same rows run, a copy can be stored, and a value written into the buffer
/// reads back. "Type Name" (9) of the Field table is pinned
/// for a Text field only (record/PZFieldObsoleteRows.al:47, `Text30`); for the Integer field it is unknown.
#[test]
fn an_unknown_column_of_a_platform_row_is_refused_where_it_is_read_dynamically() {
    let o = outcomes(
        r#"table 50003 "Buf" { fields { field(1; TableNo; Integer) { } field(2; "No."; Integer) { } field(9; "Type Name"; Text[30]) { } }
  keys { key(PK; TableNo, "No.") { Clustered = true; } } }
table 50004 "Buf2" { fields { field(1; TableNo; Integer) { } field(2; "No."; Integer) { } field(9; "Type Name"; Text[20]) { } }
  keys { key(PK; TableNo, "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure OtherColumnsAndCountRun() var F: Record Field; R: RecordRef; N: Integer; begin
    F.SetRange(TableNo, 50003); F.SetRange("No.", 1, 9); if F.Count() <> 3 then Error('count %1', F.Count()); F.FindSet();
    repeat N += 1; until F.Next() = 0; if N <> 3 then Error('n');
    F.Get(50003, 9); R.GetTable(F); if Format(R.Field(9).Value) <> 'Text30' then Error('text %1', R.Field(9).Value);
    F.Get(50003, 1); R.GetTable(F); if Format(R.Field(4).Value) <> 'TableNo' then Error('name %1', R.Field(4).Value); end;
  [Test] procedure FieldRefValue() var F: Record Field; R: RecordRef; T: Text; begin
    F.Get(50003, 1); R.GetTable(F); T := R.Field(9).Value; end;
  [Test] procedure FormatFieldRef() var F: Record Field; R: RecordRef; FR: FieldRef; begin
    F.Get(50003, 1); R.GetTable(F); FR := R.Field(9); if Format(FR) = 'x' then; end;
  [Test] procedure FilterOnIt() var R: RecordRef; begin
    R.Open(Database::Field); R.Field(1).SetRange(50003); R.Field(9).SetRange('Integer'); if R.IsEmpty() then; end;
  [Test] procedure TransferFieldsCarriesIt() var F: Record Field; B: Record Buf temporary; begin
    F.Get(50003, 1); B.TransferFields(F); B.Insert(); end;
  [Test] procedure TransferFieldsThenRead() var F: Record Field; B: Record Buf temporary; X: Text; begin
    F.Get(50003, 1); B.TransferFields(F); X := B."Type Name"; end;
  [Test] procedure TransferFieldsIntoAnotherType() var F: Record Field; B: Record Buf2 temporary; begin
    F.Get(50003, 1); B.TransferFields(F); end;
  [Test] procedure CopiedIntoATemporaryRecord() var F: Record Field; T: Record Field temporary; begin
    F.Get(50003, 1); T := F; T.Insert(); F.Get(50003, 9); T := F; T.Insert();
    T.SetRange(TableNo, 50003); if T.Count() <> 2 then Error('copies %1', T.Count()); T.Get(50003, 9); end;
  [Test] procedure CopyFilteredOnIt() var F: Record Field; T: Record Field temporary; R: RecordRef; begin
    F.Get(50003, 1); T := F; T.Insert(); R.GetTable(T); R.Field(9).SetRange('Integer'); if R.IsEmpty() then; end;
  [Test] procedure CopyReadBack() var F: Record Field; T: Record Field temporary; R: RecordRef; X: Text; begin
    F.Get(50003, 1); T := F; T.Insert(); T.Get(50003, 1); R.GetTable(T); X := R.Field(9).Value; end;
  [Test] procedure WrittenReadsBack() var F: Record Field; R: RecordRef; begin
    F.Get(50003, 1); R.GetTable(F); R.Field(9).Value := 'mine';
    if Format(R.Field(9).Value) <> 'mine' then Error('read back'); end; }"#,
    );
    passes(&o, "OtherColumnsAndCountRun");
    passes(&o, "WrittenReadsBack");
    // A copy of the row can be stored and counted; a filter on the column, or a read of it, is refused there too.
    passes(&o, "CopiedIntoATemporaryRecord");
    passes(&o, "TransferFieldsCarriesIt");
    for t in [
        "FieldRefValue",
        "FormatFieldRef",
        "FilterOnIt",
        "TransferFieldsThenRead",
        "TransferFieldsIntoAnotherType",
        "CopyFilteredOnIt",
        "CopyReadBack",
    ] {
        let why = not_run(&o, t);
        assert!(
            why.contains("field 'Type Name' of platform table 'Field' is not known"),
            "{t}: {why}"
        );
    }
}

/// A column of a table of `PINNED_COLUMNS` that the list does not name is unknown in every row the provider lists
/// (`platform::mark_unpinned`): a dynamic read of it is refused, the pinned columns of the same row read as before.
/// Column 14 of Table Metadata is not pinned (column 7 is since corpus bump 27); ID (1) and Name (3) are (record/TestTableMetadataVirtualTable.al).
#[test]
fn an_unpinned_column_of_a_listed_platform_table_is_unknown_in_every_row() {
    let o = outcomes(
        r#"table 50005 "Probe Tbl" { fields { field(1; K; Integer) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure PinnedColumnReads() var M: Record "Table Metadata"; R: RecordRef; begin
    M.Get(50005); R.GetTable(M);
    if Format(R.Field(3).Value) <> 'Probe Tbl' then Error('name %1', R.Field(3).Value); end;
  [Test] procedure UnpinnedColumnIsRefused() var M: Record "Table Metadata"; R: RecordRef; T: Text; begin
    M.Get(50005); R.GetTable(M); T := Format(R.Field(14).Value); end; }"#,
    );
    passes(&o, "PinnedColumnReads");
    let why = not_run(&o, "UnpinnedColumnIsRefused");
    assert!(
        why.contains("platform table 'Table Metadata' is not known for any row"),
        "{why}"
    );
}

/// CodeUnit Metadata TestType (9) and RequiredTestIsolation (10) read the codeunit's declared properties; a test
/// codeunit without them reads UnitTest and None (codeunit/lt-x-object-props/XObjectProps.al:255-256, :267-268, :279).
#[test]
fn codeunit_metadata_reads_the_declared_test_type_and_isolation() {
    let o = outcomes(
        r#"codeunit 50001 "Int" { Subtype = Test; TestType = IntegrationTest; RequiredTestIsolation = Disabled; }
codeunit 50002 "Unc" { Subtype = Test; TestType = Uncategorized; RequiredTestIsolation = Codeunit; }
codeunit 50003 "Ai" { Subtype = Test; TestType = AITest; RequiredTestIsolation = Function; }
codeunit 50004 "Def" { Subtype = Test; }
codeunit 50000 "T" { Subtype = Test;
  local procedure Facts(Id: Integer): Text var M: Record "CodeUnit Metadata"; begin
    M.Get(Id); exit(Format(M.TestType) + '|' + Format(M.RequiredTestIsolation) + '|' + Format(M.TestType, 0, 2) + '|' + Format(M.RequiredTestIsolation, 0, 2)); end;
  [Test] procedure Integration() begin if Facts(50001) <> 'IntegrationTest|Disabled|2|1' then Error('%1', Facts(50001)); end;
  [Test] procedure Uncategorized() begin if Facts(50002) <> 'Uncategorized|Codeunit|3|2' then Error('%1', Facts(50002)); end;
  [Test] procedure AiTest() begin if Facts(50003) <> 'AITest|Function|4|3' then Error('%1', Facts(50003)); end;
  [Test] procedure Defaults() begin if Facts(50004) <> 'UnitTest|None|1|0' then Error('%1', Facts(50004)); end; }"#,
    );
    for t in ["Integration", "Uncategorized", "AiTest", "Defaults"] {
        passes(&o, t);
    }
}

/// Field.OptimizeForTextSearch (62) is the declared property of a Text field (lt-ak-fulltext/LtAkFulltext.al:119,
/// :122); the property on another type is pinned by no test, so a read of it is refused.
#[test]
fn field_optimize_for_text_search_reads_the_declared_property_of_a_text_field() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } field(2; Txt; Text[30]) { OptimizeForTextSearch = true; }
  field(3; Plain; Text[30]) { } field(4; Cd; Code[20]) { OptimizeForTextSearch = true; } }
  keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Declared() var F: Record Field; begin F.Get(50001, 2); if not F.OptimizeForTextSearch then Error('declared'); end;
  [Test] procedure NotDeclared() var F: Record Field; begin F.Get(50001, 3); if F.OptimizeForTextSearch then Error('plain'); end;
  [Test] procedure OnCode() var F: Record Field; begin F.Get(50001, 4); if F.OptimizeForTextSearch then; end; }"#,
    );
    passes(&o, "Declared");
    passes(&o, "NotDeclared");
    let why = not_run(&o, "OnCode");
    assert!(why.contains("OptimizeForTextSearch"), "{why}");
}

/// AllObj holds one more row per table than the runtime lists (metadata/lt-all-obj-namespace/AllObjNamespace.al:101:
/// a table, a codeunit and a page in one namespace count 4), most likely its TableData row, whose columns no test
/// pins. A read that may land on such a row is refused; one filtered to another object type is answered.
#[test]
fn an_all_obj_read_that_may_land_on_a_table_data_row_is_refused() {
    let o = outcomes(
        r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure TableTypeCounts() var A: Record AllObj; begin A.SetRange("Object Type", A."Object Type"::Table); A.SetRange("Object ID", 50001); if A.Count() <> 1 then Error('%1', A.Count()); end;
  [Test] procedure AnyTypeCount() var A: Record AllObj; begin A.SetRange("Object ID", 50001); if A.Count() = 0 then; end; }"#,
    );
    passes(&o, "TableTypeCounts");
    let why = not_run(&o, "AnyTypeCount");
    assert!(why.contains("TableData"), "{why}");
}

/// The test session's own Session row: Login Type "Username and Password" (ordinal 2), and "Application Name" the
/// Active Session "Client Type" of the web client (session/W6SessionTimeZone.al:22-23, session/W5SessionFacts.al:135,
/// session/TestSessionVirtualTable.al:166).
#[test]
fn the_session_row_is_a_web_client_session_logged_in_by_user_name_and_password() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure LoginType() var S: Record Session; begin S.SetRange("My Session", true); S.FindFirst();
    if Format(S."Login Type") <> 'Username and Password' then Error('%1', S."Login Type");
    if Format(S."Login Type", 0, 2) <> '2' then Error('ordinal'); end;
  [Test] procedure ClientType() var A: Record "Active Session"; begin A.Get(ServiceInstanceId(), SessionId());
    if A."Client Type" <> A."Client Type"::"Web Client" then Error('%1', A."Client Type"); end; }"#,
    );
    passes(&o, "LoginType");
    passes(&o, "ClientType");
}

/// Code Coverage Tests Run "Method ID" is one number for every test procedure, not below 1000: 469735107
/// (codeunit/PWCodeCoverageRows.al:308; coverage/W6CodeCoverageIds.al:105-107, :139, :168). Only comparisons of it run; a read
/// that shows it is refused (see a_tests_run_method_id_is_refused_where_its_number_would_show).
#[test]
fn code_coverage_tests_run_method_id_is_the_one_number_of_a_test_procedure() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  procedure Initialize() var C: Record "Code Coverage"; R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin CodeCoverageLog(false, false); C.DeleteAll(); R.DeleteAll(); L.DeleteAll(); end;
  [Test] procedure One() begin if StrLen('x') = 0 then Error('x'); end;
  [Test] procedure Two() begin if StrLen('xyz') = 0 then Error('x'); end;
  [Test] procedure Ids()
  var R: Record "Code Coverage Tests Run"; A: Integer;
  begin Initialize();
    CodeCoverageLog(true, false); One(); CodeCoverageLog(false, false);
    R.SetRange("Object ID", 50000); R.SetRange("Method Name", 'One'); R.FindFirst(); A := R."Method ID";
    // Two tests in one recording share a Test ID, which the runtime's table cannot hold: one recording each.
    Initialize(); CodeCoverageLog(true, false); Two(); CodeCoverageLog(false, false);
    R.SetRange("Method Name", 'Two'); R.FindFirst();
    if (A <> 469735107) or (R."Method ID" <> A) then Error('the ids differ');
  end; }"#,
    );
    passes(&o, "Ids");
}

/// Published Application.Description (15) holds the manifest's description (onprem
/// session/PZPublishedApplicationColumns.al:55, :59). The runtime does not keep the description of an app yet, so a read
/// of the column is refused rather than answered blank; the other columns read as before.
#[test]
fn published_application_description_is_refused_until_the_manifest_text_is_kept() {
    let o = outcomes_app(
        "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Name() var A: Record "Published Application"; begin A.SetRange(ID, 'a1b2c3d4-e5f6-7890-abcd-ef1234567890'); A.FindFirst(); if A.Name <> 'main' then Error('%1', A.Name); end;
  [Test] procedure Description() var A: Record "Published Application"; begin A.SetRange(ID, 'a1b2c3d4-e5f6-7890-abcd-ef1234567890'); A.FindFirst(); A.CalcFields(Description); if A.Description.HasValue() then; end; }"#,
        al_sema::check::Roots::default(),
    );
    passes(&o, "Name");
    let why = not_run(&o, "Description");
    assert!(why.contains("Description"), "{why}");
}

/// AllObjWithCaption: a table whose TableType the runtime has no pinned Object Subtype for (CDS) still lists its
/// other columns, so a count over them is answered (install/lt-cm-install-leads/CmInstallObjects.al:73-79 counts
/// the app's objects, record/lt-u-table-field-props/UTableCdsOrdinals.al:9 declares a CDS table); a read of the
/// Object Subtype itself is refused.
#[test]
fn an_object_subtype_nothing_pins_is_refused_and_the_other_columns_of_the_row_read() {
    let o = outcomes(
        r#"table 50001 "Cds Thing" { TableType = CDS; ExternalName = 'cdsthing'; fields { field(1; Id; Guid) { ExternalName = 'cdsthingid'; } }
  keys { key(PK; Id) { Clustered = true; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure CountByName() var A: Record AllObjWithCaption; begin A.SetRange("Object ID", 50001); A.SetRange("Object Name", 'Cds Thing'); if A.Count() <> 1 then Error('count %1', A.Count()); end;
  [Test] procedure ReadSubtype() var A: Record AllObjWithCaption; begin A.Get(A."Object Type"::Table, 50001); if A."Object Subtype" = '' then; end; }"#,
    );
    passes(&o, "CountByName");
    let why = not_run(&o, "ReadSubtype");
    assert!(why.contains("TableType `CDS`"), "{why}");
}

/// An install codeunit that cannot run (here: Confirm outside a test method) holds back the tests that touch the
/// tables it could have written. AllObj and AllObjWithCaption are listed from the compiled objects only, so an
/// install that reads them changes nothing a test reads there: a test that reads them still runs
/// (install/lt-cm-install-leads/CmInstallObjects.al:73-79 reads AllObjWithCaption and then calls Confirm).
#[test]
fn an_install_that_cannot_run_does_not_hold_back_tests_of_the_object_listings() {
    let o = outcomes(
        r#"table 50001 "Seed" { fields { field(1; Code; Code[30]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50002 "Inst" { Subtype = Install;
  trigger OnInstallAppPerCompany() var A: Record AllObjWithCaption; S: Record Seed; begin S.Code := Format(A.Count()); S.Insert(); if Confirm('q') then; end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ReadsAllObj() var A: Record AllObjWithCaption; begin A.Get(A."Object Type"::Table, 50001); end;
  [Test] procedure ReadsSeed() var S: Record Seed; begin if S.FindFirst() then; end; }"#,
    );
    passes(&o, "ReadsAllObj");
    let why = not_run(&o, "ReadsSeed");
    assert!(why.contains("\"Inst\".OnInstallAppPerCompany could not run"), "{why}");
}

/// Table Metadata DataCaptionFields lists the ids of the fields named, a table extension's included; LinkedObject reads the
/// property (record/lt-x-table-props/XTablePropsTests.al:570-591).
#[test]
fn table_metadata_reads_data_caption_fields_of_an_extension_and_linked_object() {
    let o = outcomes(
        r#"table 50001 Thing { fields { field(1; "No."; Integer) { } field(2; A; Text[10]) { } field(4; B; Text[10]) { } } keys { key(PK; "No.") { } } }
tableextension 50003 ThingX extends Thing { DataCaptionFields = "No.", B; }
table 50004 Linked { LinkedObject = true; fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Meta() var M: Record "Table Metadata"; begin M.Get(50001); if M.DataCaptionFields <> '1,4' then Error('fields %1', M.DataCaptionFields);
    if M.LinkedObject then Error('linked'); M.Get(50004); if not M.LinkedObject then Error('not linked'); end; }"#,
    );
    passes(&o, "Meta");
}

/// The session reads the database of the oracle: `CRONUS`, in the Session and the Active Session
/// row (platform/lt-aw-fresh-tenant/AwTests.al:49 and :71).
#[test]
fn the_session_rows_name_the_cronus_database() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure SessionRow() var S: Record Session; begin S.SetRange("My Session", true); S.FindFirst(); if S."Database Name" <> 'CRONUS' then Error('%1', S."Database Name"); end;
  [Test] procedure ActiveSessionRow() var A: Record "Active Session"; begin A.Get(ServiceInstanceId(), SessionId()); if A."Database Name" <> 'CRONUS' then Error('%1', A."Database Name"); end; }"#,
    );
    passes(&o, "SessionRow");
    passes(&o, "ActiveSessionRow");
}

/// A table the app declares with `ObsoleteState = Moved` is not in the runtime app: AllObj has no row for it and
/// `RecordRef.Open` fails with AL:MetadataNotFound (platform/lt-aw-fresh-tenant/AwTests.al:413-442). Issued by the test
/// method itself, the text names the test runner (AwTests.al:434-438, corpus bump 34); issued from a helper it is not
/// pinned and a read of it is refused. A record operation on the table is not pinned and is refused.
#[test]
fn a_moved_table_has_no_all_obj_row_and_does_not_open() {
    let o = outcomes(
        r#"table 50001 "Moved" { ObsoleteState = Moved; ObsoleteReason = 'x'; ObsoleteTag = '1.0'; MovedTo = '11111111-2222-3333-4444-555555555555'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure NoAllObjRow() var A: Record AllObj; begin A.SetRange("Object Type", A."Object Type"::Table); A.SetRange("Object ID", 50001); if A.Count() <> 0 then Error('rows %1', A.Count()); end;
  [Test] procedure OpenFails() var RR: RecordRef; begin asserterror RR.Open(50001); if GetLastErrorCode() <> 'AL:MetadataNotFound' then Error('code %1', GetLastErrorCode()); end;
  [Test] procedure OpenText() var RR: RecordRef; begin asserterror RR.Open(50001); if GetLastErrorText() <> 'You tried to invoke the Table object with the ID 50001 from the object Test Runner - Mgt. An object with that ID does not exist in the current application compiled with emit version 27024.' then Error('%1', GetLastErrorText()); end;
  [Test] procedure OpenTextViaHelper() var H: Codeunit "H"; begin asserterror H.Open(50001); if GetLastErrorText() = '' then; end;
  [Test] procedure RecordUse() var M: Record "Moved"; begin if M.IsEmpty() then; end; }
codeunit 50002 "H" { procedure Open(No: Integer) var RR: RecordRef; begin RR.Open(No); end; }"#,
    );
    passes(&o, "NoAllObjRow");
    passes(&o, "OpenFails");
    passes(&o, "OpenText");
    assert!(not_run(&o, "OpenTextViaHelper").contains("not pinned"));
    assert!(not_run(&o, "RecordUse").contains("ObsoleteState = Moved"));
}

/// BC 29 adds four FlowFields to every table, listed in the Field table after SystemModifiedBy: 2000000005
/// SystemCreatedByUserName and 2000000007 SystemModifiedByUserName (Text 50), 2000000006 SystemCreatedByFullName and
/// 2000000008 SystemModifiedByFullName (Text 80) (system-fields/W6SystemFieldStamps.al:239-246; metadata/W6FieldKeyRows.al:
/// 167-168, :267-268). Their other columns are not pinned, and the runtime keeps no such fields: a FieldRef to one is refused.
#[test]
fn bc_29_lists_the_user_name_system_fields_in_the_field_table() {
    let src = r#"table 50001 "R" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure TextCount() var F: Record Field; begin F.SetRange(TableNo, 50001); F.SetRange(Type, F.Type::Text); if F.Count() <> 5 then Error('count %1', F.Count()); end;
  [Test] procedure Names() var F: Record Field; N: Text; begin F.SetRange(TableNo, 50001); F.SetFilter("No.", '>=%1', 2000000004); if F.FindSet() then repeat N += F.FieldName + ';' until F.Next() = 0; if N <> 'SystemModifiedBy;SystemCreatedByUserName;SystemCreatedByFullName;SystemModifiedByUserName;SystemModifiedByFullName;' then Error('%1', N); end;
  [Test] procedure Columns() var F: Record Field; begin F.Get(50001, 2000000006); if (F.Len <> 80) or (F.Class <> F.Class::FlowField) or (F.Type <> F.Type::Text) then Error('columns'); end;
  [Test] procedure RelationAndState() var F: Record Field; begin F.SetRange(TableNo, 50001); F.SetFilter("No.", '>=%1', 2000000005); F.SetFilter(RelationTableNo, '<>0'); if not F.IsEmpty() then Error('relation'); F.SetRange(RelationTableNo); F.SetRange(ObsoleteState, F.ObsoleteState::Removed); if not F.IsEmpty() then Error('removed'); end;
  [Test] procedure Caption() var F: Record Field; begin F.Get(50001, 2000000006); if F."Field Caption" = '' then; end;
  [Test] procedure FieldRef() var R: Record R; RR: RecordRef; begin RR.GetTable(R); if Format(RR.Field(2000000005).Value()) = '' then; end; }"#;
    let o = outcomes_bc("main", src, al_sema::check::Roots::default(), "29.0.0.0");
    passes(&o, "TextCount");
    passes(&o, "Names");
    passes(&o, "Columns");
    passes(&o, "RelationAndState");
    assert!(not_run(&o, "Caption").contains("not known"));
    assert!(not_run(&o, "FieldRef").contains("user-name system FlowFields"));
    // BC 27 has none of them (W6FieldKeyRows.al:169-170).
    let o = outcomes(&src.replace("<> 5", "<> 1"));
    passes(&o, "TextCount");
}

/// A Tests Run row's `Test ID` is the same in two recordings of one test and is not 1
/// (coverage/W6CodeCoverageIds.al:144-171); four tests recorded together share one Test ID (:96-97, :111-112), which
/// the runtime's Tests Run table, keyed by Test ID alone, cannot hold: a read of the per-test tables is refused then.
#[test]
fn a_tests_run_row_keeps_its_test_id_across_recordings_and_several_recorded_tests_are_refused() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  procedure Initialize() var C: Record "Code Coverage"; R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin CodeCoverageLog(false, false); C.DeleteAll(); R.DeleteAll(); L.DeleteAll(); end;
  [Test] procedure Helper() begin if StrLen('x') = 0 then Error('x'); end;
  [Test] procedure Helper2() begin if StrLen('x') = 0 then Error('x'); end;
  [Test] procedure TwoRecordings()
  var R: Record "Code Coverage Tests Run"; First: Integer;
  begin Initialize(); CodeCoverageLog(true, false); Helper(); CodeCoverageLog(false, false);
    R.FindFirst(); First := R."Test ID";
    Initialize(); CodeCoverageLog(true, false); Helper(); CodeCoverageLog(false, false);
    R.FindFirst(); if R."Test ID" <> First then Error('changed'); if R."Test ID" = 1 then Error('one');
  end;
  [Test] procedure TwoTests()
  var R: Record "Code Coverage Tests Run";
  begin Initialize(); CodeCoverageLog(true, false); Helper(); Helper2(); CodeCoverageLog(false, false);
    if R.Count() <> 2 then Error('rows');
  end;
  [Test] procedure TwoTestsCodeCoverageRows()
  var C: Record "Code Coverage";
  begin Initialize(); CodeCoverageLog(true, false); Helper(); Helper2(); CodeCoverageLog(false, false);
    C.SetRange("Object ID", 50000); if C.IsEmpty() then Error('rows');
  end; }"#,
    );
    passes(&o, "TwoRecordings");
    assert!(not_run(&o, "TwoTests").contains("Test ID"));
    passes(&o, "TwoTestsCodeCoverageRows");
}

/// The corpus pins only shapes of the Test ID: equal between recordings and rows, and not 1 (coverage/W6CodeCoverageIds.al:
/// 111-112, :167-168; codeunit/TestCodeCoveragePerTest.al:116 filters the lookup rows by it). Any other read of its value
/// (formatted, compared with another number, a filter or a key of a literal, a FieldRef) would show the runtime's own
/// number: refused.
#[test]
fn a_read_of_the_exact_test_id_value_is_refused_and_the_pinned_shapes_run() {
    let rec = "Initialize(); CodeCoverageLog(true, false); Helper(); CodeCoverageLog(false, false); R.FindFirst();";
    let src = r#"codeunit 50000 "T" { Subtype = Test;
  procedure Initialize() var C: Record "Code Coverage"; R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin CodeCoverageLog(false, false); C.DeleteAll(); R.DeleteAll(); L.DeleteAll(); end;
  procedure Inner() begin if StrLen('x') = 0 then Error('x'); end;
  [Test] procedure Helper() begin Inner(); end;
  [Test] procedure Formatted() var R: Record "Code Coverage Tests Run"; begin REC if Format(R."Test ID") = '' then; end;
  [Test] procedure ComparedWithTwo() var R: Record "Code Coverage Tests Run"; begin REC if R."Test ID" = 2 then; end;
  [Test] procedure Ordered() var R: Record "Code Coverage Tests Run"; begin REC if R."Test ID" > 1 then; end;
  [Test] procedure HolderFormatted() var R: Record "Code Coverage Tests Run"; X: Integer; begin REC X := R."Test ID"; if Format(X) = '' then; end;
  [Test] procedure LookupFormatted() var R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; begin REC L.FindFirst(); if Format(L."Test ID") = '' then; end;
  [Test] procedure FilterLiteral() var R: Record "Code Coverage Tests Run"; begin REC R.SetRange("Test ID", 2); if R.IsEmpty() then; end;
  [Test] procedure FilterText() var R: Record "Code Coverage Tests Run"; begin REC R.SetFilter("Test ID", '>1'); if R.IsEmpty() then; end;
  [Test] procedure GetLiteral() var R: Record "Code Coverage Tests Run"; begin REC if R.Get(2) then; end;
  [Test] procedure FieldRefValue() var R: Record "Code Coverage Tests Run"; RR: RecordRef; begin REC RR.GetTable(R); if Format(RR.Field(1).Value()) = '' then; end;
  [Test] procedure PinnedShapes()
  var R: Record "Code Coverage Tests Run"; L: Record "Code Coverage Test Lookup"; First: Integer;
  begin REC First := R."Test ID";
    REC if R."Test ID" - First <> 0 then Error('changed'); if R."Test ID" <> First then Error('differs');
    if (R."Test ID" = 1) or (First = 1) then Error('one');
    L.SetRange("Test ID", R."Test ID"); if L.Count() <> 1 then Error('lookup %1', L.Count());
  end; }"#
        .replace("REC", rec);
    let o = outcomes(&src);
    for t in [
        "Formatted",
        "ComparedWithTwo",
        "Ordered",
        "HolderFormatted",
        "LookupFormatted",
        "FilterLiteral",
        "FilterText",
        "GetLiteral",
        "FieldRefValue",
    ] {
        let why = not_run(&o, t);
        assert!(why.contains("Test ID") && why.contains("not pinned"), "{t}: {why}");
    }
    passes(&o, "PinnedShapes");
}

/// BC's test environment has the Test Runner app's codeunits 130450-130455 (permissions/lt-ba-testpermissions/BaTpEnv.al:
/// 26-28). A run that finds no package of that app (this one has no dependency apps) refuses an AllObj read that may land
/// on one of them; ids BC's test environment lacks (130301 there) still answer.
#[test]
fn all_obj_refuses_the_test_runner_codeunits_it_does_not_load() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure RunnerCodeunit() var A: Record AllObj; begin if A.Get(A."Object Type"::Codeunit, 130451) then; end;
  [Test] procedure OtherCodeunit() var A: Record AllObj; begin if A.Get(A."Object Type"::Codeunit, 130301) then Error('row'); end; }"#,
    );
    assert!(not_run(&o, "RunnerCodeunit").contains("Test Runner"));
    passes(&o, "OtherCodeunit");
}

/// A tableextension's Caption is the table's Object Caption in AllObjWithCaption and its Table Metadata Caption; the row of the
/// tableextension itself names it by its name (tableextension/TestTableExtFieldModify.al:172-179).
#[test]
fn a_table_extension_caption_is_the_object_caption_of_the_table() {
    let o = outcomes(
        r#"table 50005 "Probe Base" { fields { field(1; K; Integer) { } } }
tableextension 50006 "Probe Base Ext" extends "Probe Base" { Caption = 'Probe Ext Caption'; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Captions() var A: Record AllObjWithCaption; M: Record "Table Metadata"; begin
    A.Get(A."Object Type"::Table, 50005); if A."Object Caption" <> 'Probe Ext Caption' then Error('table %1', A."Object Caption");
    A.Get(A."Object Type"::TableExtension, 50006); if A."Object Caption" <> 'Probe Base Ext' then Error('ext %1', A."Object Caption");
    M.Get(50005); if M.Caption <> 'Probe Ext Caption' then Error('metadata %1', M.Caption); end; }"#,
    );
    passes(&o, "Captions");
}

/// Table Metadata.Caption (a Text[80] field) holds a 120-character caption whole (metadata/lt-cy-long-caption/CyLongCaption.al:
/// 47-58); a longer caption is not pinned and its read is refused.
#[test]
fn table_metadata_holds_a_caption_of_120_characters_whole() {
    let long = "c".repeat(120);
    let longer = "d".repeat(121);
    let o = outcomes(&format!(
        r#"table 50005 "Probe Long" {{ Caption = '{long}'; fields {{ field(1; K; Integer) {{ }} }} }}
table 50006 "Probe Longer" {{ Caption = '{longer}'; fields {{ field(1; K; Integer) {{ }} }} }}
codeunit 50000 "T" {{ Subtype = Test;
  [Test] procedure Whole() var M: Record "Table Metadata"; begin M.Get(50005); if StrLen(M.Caption) <> 120 then Error('len %1', StrLen(M.Caption)); end;
  [Test] procedure Longer() var M: Record "Table Metadata"; T: Text; begin M.Get(50006); T := M.Caption; end; }}"#
    ));
    passes(&o, "Whole");
    let why = not_run(&o, "Longer");
    assert!(why.contains("120"), "{why}");
}

/// The Field table's FieldName is cut to its 30 characters; FieldName() and FieldRef.Name give the whole name
/// (tableextension/TestTableExtFieldModify.al:200-216).
#[test]
fn the_field_table_cuts_a_field_name_to_30_characters() {
    let o = outcomes(
        r#"table 50005 "Probe Base" { fields { field(1; K; Integer) { } } }
tableextension 50006 "Probe Base Ext" extends "Probe Base" { fields { field(50006; "Probe Field Name Longer Than Thirty"; Integer) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Cut() var F: Record Field; R: Record "Probe Base"; begin
    F.Get(50005, 50006); if F.FieldName <> 'Probe Field Name Longer Than T' then Error('field table %1', F.FieldName);
    if R.FieldName("Probe Field Name Longer Than Thirty") <> 'Probe Field Name Longer Than Thirty' then Error('fieldname'); end; }"#,
    );
    passes(&o, "Cut");
}

/// The Field table's App Package ID and App Runtime Package ID are the table's AllObj ones after FindFirst and Next; a Get gives
/// them for a Base Application table but leaves them null for a table of the app under test
/// (metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:283-311).
#[test]
fn field_table_package_ids_follow_allobj_except_get_on_an_own_table() {
    let o = outcomes_app(
        "0f6a8f8e-0d43-4a0e-9bb3-1c3a6e2b5d71",
        r#"table 50005 "Probe Base" { fields { field(1; K; Integer) { } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Ids() var F: Record Field; A: Record AllObj; begin
    A.Get(A."Object Type"::Table, 50005);
    F.Get(50005, 1); if not IsNullGuid(F."App Package ID") or not IsNullGuid(F."App Runtime Package ID") then Error('get not null');
    Clear(F); F.SetRange(TableNo, 50005); F.FindFirst();
    if F."App Package ID" <> A."App Package ID" then Error('findfirst pkg');
    if F."App Runtime Package ID" <> A."App Runtime Package ID" then Error('findfirst runtime');
    if IsNullGuid(F."App Package ID") then Error('findfirst null');
    F.Get(50005, 1); if F.Next(1) <> 0 then F.Next(-1);
    if F."App Package ID" <> A."App Package ID" then Error('next pkg'); end; }"#,
        al_sema::check::Roots::default(),
    );
    passes(&o, "Ids");
}

/// ADR-046 decision 4a: in the default mode (one company), a read of the Company table that could list a company the
/// runtime does not know is refused; a read filtered to one known name is answered. The corpus database holds a second
/// company (install/lt-bf-install-probes/BfInstallTests.al:60-62), which this runtime does not have.
#[test]
fn company_table_reads_that_could_list_another_company_are_refused() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure FindSetAll() var C: Record Company; begin if C.FindSet() then; end;
  [Test] procedure CountAll() var C: Record Company; begin if C.Count() = 0 then; end;
  [Test] procedure FilteredToSession() var C: Record Company; begin C.SetRange(Name, CompanyName()); if not C.FindFirst() then Error('none'); if C.Count() <> 1 then Error('count'); end;
  [Test] procedure GetSession() var C: Record Company; begin if not C.Get(CompanyName()) then Error('no row'); end; }"#,
    );
    // Count stays answered: refusing it loses the baseline case session/TestProbeSessionMiscContracts.al:Company_Names.
    passes(&o, "CountAll");
    assert!(
        not_run(&o, "FindSetAll").contains("ADR-046"),
        "FindSetAll: {}",
        not_run(&o, "FindSetAll")
    );
    passes(&o, "FilteredToSession");
    passes(&o, "GetSession");
}

// codeunit/PWCodeCoverageRows.al:303,309: BC's Test Lookup "Method ID" is a compiler-assigned id (31658110 for
// `RecordedInner`) whose rule is not known, so a read that shows it is refused; comparing it with another id is kept
// (codeunit/TestCodeCoveragePerTest.al:121, coverage/W6CodeCoverageIds.al:133-134).
#[test]
fn a_test_lookup_method_id_is_refused_where_its_number_would_show() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ShowsIt() var L: Record "Code Coverage Test Lookup"; T: Text; begin if L.FindFirst() then T := StrSubstNo('%1', L."Method ID"); end;
  [Test] procedure ComparesIt() var L: Record "Code Coverage Test Lookup"; R: Record "Code Coverage Tests Run"; B: Boolean; begin if L.FindFirst() then B := L."Method ID" = R."Method ID"; end; }"#,
    );
    let why = not_run(&o, "ShowsIt");
    assert!(why.contains("Method ID is not pinned"), "{why}");
    assert!(
        !matches!(
            o.iter().find(|(n, _)| n == "ComparesIt").map(|(_, o)| o),
            Some(Outcome::NotRun(_))
        ),
        "{o:?}"
    );
}

// codeunit/PWCodeCoverageRows.al:299,308 and coverage/PxCodeCoverageIds.al:186: BC's Tests Run "Method ID" is a number per test
// whose rule is not known; the runtime's constant would show. A comparison, also through an Integer local that is only compared,
// is kept (coverage/W6CodeCoverageIds.al:83-98, :129-139, :156-164).
#[test]
fn a_tests_run_method_id_is_refused_where_its_number_would_show() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ShowsIt() var R: Record "Code Coverage Tests Run"; T: Text; begin if R.FindFirst() then T := StrSubstNo('%1', R."Method ID"); end;
  [Test] procedure HoldsAndShowsIt() var R: Record "Code Coverage Tests Run"; Id: Integer; T: Text; begin if R.FindFirst() then Id := R."Method ID"; T := Format(Id); end;
  [Test] procedure HoldsAndComparesIt() var R: Record "Code Coverage Tests Run"; Id: Integer; B: Boolean; begin if R.FindFirst() then Id := R."Method ID"; B := Id < 1000; B := R."Method ID" = Id; end; }"#,
    );
    for t in ["ShowsIt", "HoldsAndShowsIt"] {
        let why = not_run(&o, t);
        assert!(why.contains("Method ID is not pinned"), "{why}");
    }
    assert!(
        !matches!(
            o.iter().find(|(n, _)| n == "HoldsAndComparesIt").map(|(_, o)| o),
            Some(Outcome::NotRun(_))
        ),
        "{o:?}"
    );
}
