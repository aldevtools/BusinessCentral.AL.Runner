//! `Database.MinimumActiveRowVersion` under TestIsolation = Codeunit: the isolation keeps one transaction open across
//! `Commit()`, so the value stays at what it was when the codeunit began, through Insert, Modify and Commit
//! (corpus platform/lt-platform-members/PlTRowVersionValue.al: TV_MinimumActiveRowVersion_AcrossInsertModifyCommit and
//! TV_MinimumActiveRowVersion_ReadOnlyAfterCommittedWrite). While the open transaction holds a Modify, or an Insert whose table
//! was read since, the runtime's `LastUsedRowVersion` is not BC's (the same file shows it grows after Insert + Get), and tests
//! compare the two: the value is refused there.

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
    var Row: Record "MR Row"; V: BigInteger;
    begin
        Row.Id := 7; Row.Insert(); Row.Get(7);
        V := MinimumActiveRowVersion();
    end;

    [Test]
    procedure AfterModify()
    var Row: Record "MR Row"; V: BigInteger;
    begin
        Row.Get(1); Row.Name := 'y'; Row.Modify();
        V := MinimumActiveRowVersion();
    end;
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
        match of(t) {
            Outcome::NotRun(r) => assert!(r.contains("LastUsedRowVersion is not pinned"), "{t}: {r}"),
            other => panic!("{t}: {other:?}"),
        }
    }
}
