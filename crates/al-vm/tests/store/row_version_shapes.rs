//! Database.LastUsedRowVersion and MinimumActiveRowVersion per write and read shape (writer UK, corpus branch
//! al-runtime-rs-staging-pub-uk beab119c, corpus PR 803, platform/lt-platform-members/PlTRowVersionValue.al UK_*):
//! the rows an Insert wrote are sent at the first read of their table (or at Commit), a read of another table sends
//! nothing and a Modify grows the value at once. MinimumActiveRowVersion is frozen by the test runner's isolation
//! (al-testrun), not checked here.

use al_testkit::text;

const SRC: &str = r#"
table 50160 "Rv A" { fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } } keys { key(PK; Id) { } } }
table 50161 "Rv B" { fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } } keys { key(PK; Id) { } } }
codeunit 50160 C
{
    var
        Prev: BigInteger;
        Log: Text;

    local procedure Start()
    var
        A: Record "Rv A";
        B: Record "Rv B";
    begin
        A.DeleteAll();
        B.DeleteAll();
        Commit();
        B.Id := 1;
        B.Insert();
        Commit();
        Prev := LastUsedRowVersion();
        Log := '';
    end;

    local procedure Snap(Name: Text)
    var
        Last: BigInteger;
    begin
        Last := LastUsedRowVersion();
        if Last > Prev then
            Log += Name + '>'
        else
            if Last = Prev then
                Log += Name + '='
            else
                Log += Name + '<';
        Log += ' ';
        Prev := Last;
    end;

    procedure ModifyOther(): Text
    var
        A: Record "Rv A";
        B: Record "Rv B";
    begin
        Start();
        A.Id := 1;
        A.Insert();
        Snap('insert');
        B.Get(1);
        B.Amount := 7;
        B.Modify();
        Snap('modify-other');
        A.FindFirst();
        Snap('findfirst');
        Commit();
        Snap('commit');
        exit(Log);
    end;

    procedure ReadOtherThenSame(): Text
    var
        A: Record "Rv A";
        B: Record "Rv B";
    begin
        Start();
        A.Id := 1;
        A.Insert();
        Snap('insert');
        B.Get(1);
        Snap('get-other');
        if B.Count() = 1 then;
        Snap('count-other');
        A.Get(1);
        Snap('get-same');
        Commit();
        Snap('commit');
        exit(Log);
    end;

    procedure TwoTables(): Text
    var
        A: Record "Rv A";
        B: Record "Rv B";
    begin
        Start();
        A.Id := 1;
        A.Insert();
        B.Id := 2;
        B.Insert();
        Snap('insert-both');
        A.Get(1);
        Snap('get-a');
        B.Get(2);
        Snap('get-b');
        Commit();
        Snap('commit');
        exit(Log);
    end;

    procedure InsertReadInsertRead(): Text
    var
        A: Record "Rv A";
    begin
        Start();
        A.Id := 1;
        A.Insert();
        Snap('insert');
        A.FindFirst();
        Snap('read');
        A.FindFirst();
        Snap('read-again');
        A.Id := 2;
        A.Insert();
        Snap('insert-2');
        A.FindLast();
        Snap('read-2');
        exit(Log);
    end;

    procedure ReadKinds(): Text
    var
        A: Record "Rv A";
    begin
        Start();
        A.Id := 1;
        A.Insert();
        if A.FindSet() then;
        Snap('findset');
        A.Id := 2;
        A.Insert();
        if A.IsEmpty() then;
        Snap('isempty');
        A.Id := 3;
        A.Insert();
        if A.Count() = 0 then;
        Snap('count');
        A.Id := 4;
        A.Amount := 5;
        A.Insert();
        A.CalcSums(Amount);
        Snap('calcsums');
        exit(Log);
    end;
}
"#;

// UK_InsertThenModifyOtherTable.
#[test]
fn a_modify_of_another_table_grows_it_and_the_later_read_grows_it_again() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ModifyOther")),
        "insert= modify-other> findfirst> commit= "
    );
}

// UK_InsertThenReadOtherThenSame, UK_InsertThenReadOtherTable.
#[test]
fn a_read_of_another_table_sends_nothing() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ReadOtherThenSame")),
        "insert= get-other= count-other= get-same> commit= "
    );
}

// UK_InsertTwoTablesThenGetOne.
#[test]
fn each_table_is_sent_at_its_own_read() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "TwoTables")),
        "insert-both= get-a> get-b> commit= "
    );
}

// UK_InsertReadInsertRead.
#[test]
fn a_second_read_sends_nothing_and_a_new_insert_is_sent_at_the_next_read() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "InsertReadInsertRead")),
        "insert= read> read-again= insert-2= read-2> "
    );
}

// UK_InsertThenFindSet, UK_InsertThenIsEmpty, UK_InsertThenCount, UK_InsertThenCalcSums.
#[test]
fn findset_isempty_count_and_calcsums_send_the_inserted_rows() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ReadKinds")),
        "findset> isempty> count> calcsums> "
    );
}
