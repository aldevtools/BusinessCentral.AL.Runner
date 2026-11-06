//! The license as a ceiling of the permission check (ADR-062 amendment 1 point 5, ADR-060 decision 6), with a synthetic
//! license (made-up ranges, no signature; no real license file is ever a fixture).
//!
//! License facts measured on BC 28.5 with a customer license, 2026-11-06 (owner decision): with a table licensed Read Yes
//! and Insert, Modify, Delete Indirect, a SUPER user's Insert from code without a `Permissions` property fails with
//! `Sorry, the current permissions prevented the action. (TableData <id> <name> Insert: <app>)`, code
//! `DB:ClientInsertDenied`; from a codeunit whose `Permissions` names the table it works; a read works. Modify, Delete and
//! Read at Indirect reuse the denials the corpus pins for a user-level indirect grant (CpPermLeads.al:91-103,
//! Y19PermWrites.al:172). A license level that is blank stays refused (ADR-060 decision 6).

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};
use std::sync::Arc;

const MAIN: &str = r#"
table 50100 "LC Ind"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50101 "LC Full"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50102 "LC Read Ind"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50103 "LC Blank"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50104 "LC Head"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("LC Read Ind".Amount); }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50110 "LC Granted"
{
    Permissions = tabledata "LC Ind" = rimd, tabledata "LC Read Ind" = rimd;

    procedure InsertInd(NewId: Integer)
    var
        R: Record "LC Ind";
    begin
        R.Id := NewId;
        R.Insert();
    end;

    procedure InsertReadInd(NewId: Integer)
    var
        R: Record "LC Read Ind";
    begin
        R.Id := NewId;
        R.Amount := 5;
        R.Insert();
    end;

    procedure CountReadInd(): Integer
    var
        R: Record "LC Read Ind";
    begin
        exit(R.Count());
    end;

    procedure ModifyInd(Id: Integer)
    var
        R: Record "LC Ind";
    begin
        R.Get(Id);
        R.Amount := 7;
        R.Modify();
    end;
}

codeunit 50120 "LC Tests"
{
    Subtype = Test;

    local procedure Expect(Txt: Text; Code: Text)
    begin
        if GetLastErrorText() <> Txt then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> Code then
            Error('code: %1', GetLastErrorCode());
    end;

    [Test]
    procedure IndirectInsertDenied()
    var
        R: Record "LC Ind";
    begin
        R.Id := 1;
        asserterror R.Insert();
        Expect('Sorry, the current permissions prevented the action. (TableData 50100 LC Ind Insert: main)', 'DB:ClientInsertDenied');
    end;

    [Test]
    procedure IndirectRecordRefInsertDenied()
    var
        RR: RecordRef;
    begin
        RR.Open(Database::"LC Ind");
        RR.Field(1).Value := 2;
        asserterror RR.Insert();
        Expect('Sorry, the current permissions prevented the action. (TableData 50100 LC Ind Insert: main)', 'DB:ClientInsertDenied');
    end;

    [Test]
    procedure IndirectInsertThroughGrant()
    var
        G: Codeunit "LC Granted";
        R: Record "LC Ind";
    begin
        G.InsertInd(3);
        G.ModifyInd(3);
        if not R.FindFirst() then
            Error('no row');
    end;

    [Test]
    procedure IndirectModifyAndDeleteDenied()
    var
        G: Codeunit "LC Granted";
        R: Record "LC Ind";
    begin
        G.InsertInd(4);
        R.Get(4);
        R.Amount := 1;
        asserterror R.Modify();
        Expect('Sorry, the current permissions prevented the action. (TableData 50100 LC Ind Modify: main)', 'DB:ClientModifyDenied');
        // asserterror rolls the insert back.
        G.InsertInd(4);
        asserterror R.Delete();
        Expect('Sorry, the current permissions prevented the action. (TableData 50100 LC Ind Delete: main)', 'DB:ClientDeleteDenied');
    end;

    [Test]
    procedure TemporaryIsNotChecked()
    var
        R: Record "LC Ind" temporary;
    begin
        R.Id := 5;
        R.Insert();
    end;

    [Test]
    procedure FullLicenseInserts()
    var
        R: Record "LC Full";
    begin
        R.Id := 1;
        R.Insert();
    end;

    [Test]
    procedure IndirectReadDenied()
    var
        G: Codeunit "LC Granted";
        R: Record "LC Read Ind";
    begin
        G.InsertReadInd(1);
        asserterror R.FindFirst();
        Expect('Sorry, the current permissions prevented the action. (TableData 50102 LC Read Ind Read: main)', 'DB:ClientReadDenied');
        // asserterror rolls the insert back.
        G.InsertReadInd(1);
        asserterror R.Count();
        Expect('Sorry, the current permissions prevented the action. (TableData 50102 LC Read Ind Read: main)', 'DB:ClientReadDenied');
        G.InsertReadInd(1);
        asserterror R.IsEmpty();
        Expect('Sorry, the current permissions prevented the action. (TableData 50102 LC Read Ind Read: main)', 'DB:ClientReadDenied');
    end;

    [Test]
    procedure IndirectReadOfAnEmptyTable()
    var
        R: Record "LC Read Ind";
    begin
        if not R.IsEmpty() then
            Error('rows');
    end;

    [Test]
    procedure IndirectReadThroughGrant()
    var
        G: Codeunit "LC Granted";
    begin
        G.InsertReadInd(11);
        if G.CountReadInd() < 1 then
            Error('count');
    end;

    [Test]
    procedure IndirectReadByFlowField()
    var
        G: Codeunit "LC Granted";
        H: Record "LC Head";
    begin
        G.InsertReadInd(12);
        H.Id := 1;
        H.Insert();
        H.CalcFields(Total);
        if H.Total < 5 then
            Error('total %1', H.Total);
    end;

    [Test]
    procedure IndirectCalcSums()
    var
        G: Codeunit "LC Granted";
        R: Record "LC Read Ind";
    begin
        G.InsertReadInd(13);
        R.CalcSums(Amount);
    end;

    [Test]
    procedure IndirectReadPermission()
    var
        R: Record "LC Read Ind";
        I: Record "LC Ind";
    begin
        if R.ReadPermission() then
            Error('read');
        if not I.ReadPermission() then
            Error('direct read');
        if I.WritePermission() then
            Error('write');
    end;

    [Test]
    procedure BlankInsert()
    var
        R: Record "LC Blank";
    begin
        R.Id := 1;
        R.Insert();
    end;
}
"#;

fn license() -> Arc<al_host::license::License> {
    let bytes = al_host::license::synthetic::file(
        "Synthetic Licensee",
        "1/2/2040",
        &[
            (
                "TableData",
                &[
                    (1, 50099, 31),
                    (50100, 50100, 449),
                    (50101, 50101, 31),
                    (50102, 50102, 480),
                    (50104, 99999, 31),
                ],
            ),
            ("Codeunit", &[(1, 999999, 31)]),
            ("TableDescription", &[(1, 999999, 31)]),
        ],
    );
    Arc::new(al_host::license::parse(&bytes).expect("the synthetic license parses"))
}

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        run_main(
            MAIN,
            &RunOptions {
                license: Some(license()),
                ..Default::default()
            },
        )
    })
}

fn outcome(test: &str) -> Outcome {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

fn passes(test: &str) {
    assert_eq!(outcome(test), Outcome::Pass, "{test}");
}

fn refused(test: &str, reason: &str) {
    match outcome(test) {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{test}: {why}"),
        other => panic!("{test}: {other:?}"),
    }
}

#[test]
fn an_insert_the_license_grants_only_indirectly_needs_the_permissions_property() {
    passes("IndirectInsertDenied");
    passes("IndirectRecordRefInsertDenied");
    passes("IndirectInsertThroughGrant");
    passes("TemporaryIsNotChecked");
    passes("FullLicenseInserts");
}

#[test]
fn modify_and_delete_at_indirect_reuse_the_user_level_denials() {
    passes("IndirectModifyAndDeleteDenied");
}

#[test]
fn a_read_the_license_grants_only_indirectly_needs_the_permissions_property() {
    passes("IndirectReadDenied");
    passes("IndirectReadOfAnEmptyTable");
    passes("IndirectReadThroughGrant");
    passes("IndirectReadByFlowField");
    passes("IndirectReadPermission");
    // CalcSums at an indirect level is not pinned (Y24PermReads.al pins it under a set without Read).
    refused("IndirectCalcSums", "read shape other than");
}

#[test]
fn a_blank_license_level_is_refused() {
    refused("BlankInsert", "license");
}
