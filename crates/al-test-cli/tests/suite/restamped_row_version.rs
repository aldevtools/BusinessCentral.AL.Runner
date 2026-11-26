//! Rows whose Insert row version is below one an earlier send of another table handed out (al-vm records/stale.rs
//! `note_read_for_row_versions`): BC gives them their row version when it sends them (corpus PR 1509, merged, not pinned:
//! session-system/row-version-writes/RowVersionWrites.al:392 InsertTwoTables `row/other` greater), this runtime keeps the
//! Insert's. A read of their SystemRowVersion gives a value known only to be greater than 0, so `Format` of it is refused, and
//! a find or query that filters or sorts on it is refused; other reads run,
//! in install code too, and the rows install code left that way are refused to a test. A Delete, Rename or ModifyAll of a
//! committed row while the table holds an inserted row keeps the refusal of `last_used_row_version`.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "RS Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69981 "RS Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69982 "RS Inst A"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
table 69983 "RS Inst B"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
query 69984 "RS Versions"
{
    elements
    {
        dataitem(Row; "RS Row")
        {
            column(Id; Id) { }
            column(RowVersion; SystemRowVersion) { }
        }
    }
}
codeunit 69985 Inst
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var A: Record "RS Inst A"; B: Record "RS Inst B";
    begin
        A.Id := 1; A.Name := 'a'; A.Insert();
        B.Id := 1; B.Insert();
        B.Get(1);
        A.Get(1);
    end;
}
codeunit 69986 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure TwoTables()
    var Row: Record "RS Row"; Other: Record "RS Other";
    begin
        Row.DeleteAll(); Other.DeleteAll(); Commit();
        Row.Id := 1; Row.Name := 'r'; Row.Insert();
        Other.Id := 1; Other.Insert();
        Other.Get(1);
        Row.Get(1);
    end;

    [Test]
    procedure FieldRead()
    var Row: Record "RS Row"; V: BigInteger;
    begin
        TwoTables();
        Row.Get(1);
        V := Row.SystemRowVersion;
        Error(Format(V));
    end;

    [Test]
    procedure FieldRefRead()
    var Row: Record "RS Row"; RecRef: RecordRef; V: BigInteger;
    begin
        TwoTables();
        RecRef.Open(Database::"RS Row");
        RecRef.FindFirst();
        V := RecRef.Field(Row.FieldNo(SystemRowVersion)).Value;
        Error(Format(V));
    end;

    [Test]
    procedure FilterFind()
    var Row: Record "RS Row";
    begin
        TwoTables();
        Row.SetFilter(SystemRowVersion, '>%1', 0);
        Row.FindFirst();
    end;

    [Test]
    procedure KeyFind()
    var Row: Record "RS Row";
    begin
        TwoTables();
        Row.SetCurrentKey(SystemRowVersion);
        Row.FindLast();
    end;

    [Test]
    procedure QueryColumn()
    var Q: Query "RS Versions";
    begin
        TwoTables();
        Q.Open();
        Q.Read();
    end;

    [Test]
    procedure PlainRead()
    var Row: Record "RS Row"; Last: BigInteger;
    begin
        Last := LastUsedRowVersion();
        TwoTables();
        Row.FindFirst();
        if Row.Name <> 'r' then Error('wrong name');
        if LastUsedRowVersion() <= Last then Error('did not grow');
    end;

    [Test]
    procedure InstallRowsRowVersion()
    var A: Record "RS Inst A"; V: BigInteger;
    begin
        A.Get(1);
        V := A.SystemRowVersion;
        Error(Format(V));
    end;

    [Test]
    procedure InstallRowsPlain()
    var A: Record "RS Inst A";
    begin
        A.Get(1);
        if A.Name <> 'a' then Error('wrong install row');
    end;

    [Test]
    procedure DeleteCommittedRow()
    var Row: Record "RS Row"; Committed: Record "RS Row"; Last: BigInteger;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Commit();
        Row.Id := 2; Row.Insert();
        Committed.Id := 1; Committed.Delete();
        Last := LastUsedRowVersion();
    end;

    [Test]
    procedure RenameCommittedRow()
    var Row: Record "RS Row"; Committed: Record "RS Row"; Last: BigInteger;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Commit();
        Row.Id := 2; Row.Insert();
        Committed.Id := 1; Committed.Rename(5);
        Last := LastUsedRowVersion();
    end;

    [Test]
    procedure ModifyAllWithCommittedRow()
    var Row: Record "RS Row"; Last: BigInteger;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Commit();
        Row.Id := 2; Row.Insert();
        Row.ModifyAll(Name, 'y');
        Last := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteInsertedRow()
    var Row: Record "RS Row"; Last: BigInteger;
    begin
        Row.DeleteAll(); Commit();
        Row.Id := 1; Row.Insert();
        Last := LastUsedRowVersion();
        Row.Delete();
        if LastUsedRowVersion() <= Last then Error('did not grow');
    end;
}
"#;

#[test]
fn row_versions_of_rows_sent_after_another_table_are_refused_where_they_show() {
    let out = outcomes_in(SRC, "T");
    let of = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    let refused = |t: &str, why: &str| match of(t) {
        Outcome::NotRun(r) => assert!(r.contains(why), "{t}: {r}"),
        other => panic!("{t}: {other:?}"),
    };
    for t in ["FieldRead", "FieldRefRead", "InstallRowsRowVersion"] {
        refused(t, "SystemRowVersion of a row");
    }
    for t in ["FilterFind", "KeyFind", "QueryColumn"] {
        refused(t, "on SystemRowVersion of a table");
    }
    for t in ["PlainRead", "InstallRowsPlain"] {
        assert!(matches!(of(t), Outcome::Pass), "{t}: {:?}", of(t));
    }
    // Writes of a committed row: main's refusal of `last_used_row_version`.
    for t in ["DeleteCommittedRow", "RenameCommittedRow", "ModifyAllWithCommittedRow"] {
        refused(t, "a Delete, Rename, DeleteAll, ModifyAll or Truncate after an Insert");
    }
    // A Delete of the inserted row itself: the assumption row-version-write-after-insert (RowVersionWrites.al:147).
    assert!(
        matches!(of("DeleteInsertedRow"), Outcome::Pass),
        "{:?}",
        of("DeleteInsertedRow")
    );
}
