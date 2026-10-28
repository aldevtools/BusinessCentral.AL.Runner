//! A query with `ReadState = ReadShared` or `ReadCommitted` reads the session's own uncommitted row and the committed
//! one, as `ReadUncommitted` does (al-runner issue 6050). Pinned by writer TJ's branch al-runtime-rs-staging-pub-tj,
//! query/TestQueryReadStateContracts.al:42-70 and :135-163 (assumption query-read-state-shared-committed until the pin lands).

use al_builtins::assumptions::{Assumption, take_hits};

fn src(state: &str) -> String {
    format!(
        r#"
table 68952 "Probe ReadState Table"
{{
    fields {{ field(1; Id; Integer) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}

query 89920 Q
{{
    QueryType = Normal;
    ReadState = {state};
    elements
    {{
        dataitem(T; "Probe ReadState Table")
        {{
            column(Id; Id) {{ }}
        }}
    }}
}}

codeunit 50100 C
{{
    local procedure Rows(): Integer
    var
        Q: Query Q;
        Count: Integer;
    begin
        Q.Open();
        while Q.Read() do
            Count += 1;
        Q.Close();
        exit(Count);
    end;

    procedure F(): Text
    var
        R: Record "Probe ReadState Table";
        T: Text;
    begin
        R.DeleteAll();
        Commit();
        R.Id := 1;
        R.Insert();
        T := Format(Rows());
        Commit();
        exit(T + '/' + Format(Rows()));
    end;
}}
"#
    )
}

#[test]
fn read_shared_and_read_committed_read_the_rows_as_assumed() {
    for state in ["ReadShared", "ReadCommitted"] {
        take_hits();
        assert_eq!(
            al_testkit::text(al_testkit::run_in(&src(state), "C", "F")),
            "1/1",
            "{state}"
        );
        assert!(
            take_hits().contains(&Assumption::QueryReadStateSharedCommitted),
            "{state} is not flagged"
        );
    }
}

#[test]
fn read_uncommitted_is_pinned_and_not_flagged() {
    // query/TestQueryReadStateContracts.al (on master).
    take_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src("ReadUncommitted"), "C", "F")),
        "1/1"
    );
    assert!(!take_hits().contains(&Assumption::QueryReadStateSharedCommitted));
}
