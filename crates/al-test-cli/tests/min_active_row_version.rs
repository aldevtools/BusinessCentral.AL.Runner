//! `Database.MinimumActiveRowVersion` under TestIsolation = Codeunit: the isolation keeps one transaction open across
//! `Commit()`, so the value stays at what it was when the codeunit began, through Insert, Modify and Commit
//! (corpus platform/lt-platform-members/PlTRowVersionValue.al: TV_MinimumActiveRowVersion_AcrossInsertModifyCommit and
//! TV_MinimumActiveRowVersion_ReadOnlyAfterCommittedWrite). `LastUsedRowVersion` stays put after an Insert alone, covers the
//! row after a read of its table and grows after a Modify (the same file, and PlTDatabase.al:220-245). Where it is not pinned
//! (an Insert left in one table while another was read or modified, a Delete or bulk write meeting inserted rows), both are
//! refused.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69990 "MR Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69992 Inst
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var Row: Record "MR Row";
    begin Row.Id := 100; Row.Insert(); end;
}
codeunit 69991 T
{
    Subtype = Test;
    var Start: BigInteger;

    [Test]
    procedure FirstWritesAndCommits()
    var Row: Record "MR Row";
    begin
        Start := MinimumActiveRowVersion();
        Row.Id := 1; Row.Insert(); Commit();
        if MinimumActiveRowVersion() <> Start then Error('moved after commit');
        Row.Id := 2; Row.Insert();
        if MinimumActiveRowVersion() <> Start then Error('moved in write');
        Commit();
        Row.Get(1);
        if Row.SystemRowVersion < MinimumActiveRowVersion() then Error('row below min');
    end;

    [Test]
    procedure LaterTestKeepsIt()
    var Row: Record "MR Row";
    begin
        Commit();
        if MinimumActiveRowVersion() <> Start then Error('moved in a later test');
        Row.FindFirst();
        if MinimumActiveRowVersion() <> Start then Error('moved after read');
    end;

    [Test]
    procedure AfterInsertAndRead()
    var Row: Record "MR Row"; Last: BigInteger;
    begin
        Last := LastUsedRowVersion();
        Row.Id := 7; Row.Insert();
        if LastUsedRowVersion() <> Last then Error('grew after insert alone');
        Row.Get(7);
        if MinimumActiveRowVersion() <> Start then Error('min moved');
        if LastUsedRowVersion() <= Last then Error('did not grow after insert and read');
        if Row.SystemRowVersion > LastUsedRowVersion() then Error('row above last');
    end;

    [Test]
    procedure AfterModify()
    var Row: Record "MR Row"; Last: BigInteger;
    begin
        Last := LastUsedRowVersion();
        Row.Get(1); Row.Name := 'y'; Row.Modify();
        if LastUsedRowVersion() <= Last then Error('did not grow after modify');
        if MinimumActiveRowVersion() <> Start then Error('min moved');
    end;

    [Test]
    procedure InsertHereModifyThere()
    var Row: Record "MR Row"; Other: Record "MR Other"; V: BigInteger;
    begin
        Other.Id := 1; Other.Insert();
        Row.Get(1); Row.Name := 'z'; Row.Modify();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteOfInsertedTable()
    var Row: Record "MR Row"; V: BigInteger;
    begin
        Row.Id := 8; Row.Insert();
        Row.Id := 1; Row.Delete();
        V := LastUsedRowVersion();
    end;
}
table 69993 "MR Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;

#[test]
fn minimum_active_row_version_stays_at_the_codeunit_start_under_codeunit_isolation() {
    let out = outcomes_in(SRC, "T");
    let of = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert!(
        matches!(of("FirstWritesAndCommits"), Outcome::Pass),
        "{:?}",
        of("FirstWritesAndCommits")
    );
    assert!(
        matches!(of("LaterTestKeepsIt"), Outcome::Pass),
        "{:?}",
        of("LaterTestKeepsIt")
    );
    for t in ["AfterInsertAndRead", "AfterModify"] {
        assert!(matches!(of(t), Outcome::Pass), "{t}: {:?}", of(t));
    }
    for t in ["InsertHereModifyThere", "DeleteOfInsertedTable"] {
        match of(t) {
            Outcome::NotRun(r) => assert!(r.contains("Database.LastUsedRowVersion after"), "{t}: {r}"),
            other => panic!("{t}: {other:?}"),
        }
    }
}
