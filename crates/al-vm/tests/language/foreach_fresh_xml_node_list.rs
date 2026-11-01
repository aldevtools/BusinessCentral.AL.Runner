//! foreach over an XmlNodeList that was never assigned runs zero times, also over the unassigned named return value of a
//! procedure; Get as a statement on such a list raises (xml/lt-y44-fresh-nodelist/Y44ForeachFreshNodeList.al, corpus PR 962).

/// Y44ForeachFreshNodeList.al ForeachNodeList_NeverAssigned, ForeachNodeList_LocalEmptyReturn.
#[test]
fn foreach_over_a_never_assigned_node_list_runs_zero_times() {
    let src = "codeunit 50600 C {
        local procedure EmptyReturn() Nodes: XmlNodeList begin end;
        procedure F(): Text var L: XmlNodeList; M: XmlNodeList; N: XmlNode; V: Integer; begin
            foreach N in L do V += 1;
            M := EmptyReturn(); foreach N in M do V += 1;
            foreach N in EmptyReturn() do V += 1;
            exit(Format(V)); end; }";
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "0");
}

/// Y44ForeachFreshNodeList.al NodeList_NeverAssigned_Members.
#[test]
fn get_as_a_statement_on_a_never_assigned_node_list_raises_index_out_of_bounds() {
    let src = "codeunit 50600 C { procedure F(): Text var L: XmlNodeList; N: XmlNode; begin
        asserterror L.Get(1, N); exit(Format(L.Count()) + '|' + Format(L.Get(1, N)) + '|' + GetLastErrorText() + '|' + GetLastErrorCode()); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "0|No|Index out of bounds.|IndexOutOfBounds"
    );
}
