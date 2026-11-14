//! A second Open on a query that was read from while rows were left, with no filter change between (al-runner issue 7827).
//! Pinned by corpus queries/read-lifecycle/QueryClosedAfterLastRow.al:162-182 (Query_RowsLeft_OpenAgainAndSetRange): on a
//! global query variable of a codeunit, cleared, then Open, one Read whose result is used and one read of the Integer column,
//! the second Open as a statement raises nothing, and one SetRange on the Code column and an Open read the new filter. Each
//! test below is one neighbor of that shape: it runs to the pinned rows, refuses after the second Open, or keeps main's
//! refusal of the second Open.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const OBJECTS: &str = r#"
table 62810 "QR Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Grp; Code[10]) { }
        field(3; Amt; Decimal) { }
        field(4; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("QR Row" where(Grp = field(Grp))); }
        field(5; Grp20; Code[20]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 62813 "QR Child"
{
    fields { field(1; Id; Integer) { } field(2; RowId; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
query 62811 "QR Rows" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } }
query 62820 "QR Trig" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } trigger OnBeforeOpen() begin end; }
query 62821 "QR Filt" { QueryType = Normal; elements { dataitem(Row; "QR Row") { DataItemTableFilter = Id = filter(> 0); column(Id; Id) { } column(Grp; Grp) { } } } }
query 62822 "QR Top" { QueryType = Normal; TopNumberOfRows = 3; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } }
query 62823 "QR Order" { QueryType = Normal; OrderBy = ascending(Id); elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } }
query 62824 "QR Flow" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } column(Cnt; Cnt) { } } } }
query 62825 "QR Three" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } column(Amt; Amt) { } } } }
query 62826 "QR VarProc" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } var N: Integer; procedure Bump() begin N += 1; end; }
query 62827 "QR Var" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } var N: Integer; }
query 62828 "QR Join" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } dataitem(Child; "QR Child") { DataItemLink = RowId = Row.Id; column(ChildId; Id) { } } } } }
query 62829 "QR Sum" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } column(Amt; Amt) { Method = Sum; } } } }
query 62830 "QR ColFilter" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { ColumnFilter = Id = filter(> 0); } column(Grp; Grp) { } } } }
query 62831 "QR FilterEl" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } filter(AmtF; Amt) { } } } }
query 62832 "QR ReadState" { QueryType = Normal; ReadState = ReadUncommitted; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } }
query 62833 "QR Caption" { QueryType = Normal; Caption = 'Rows'; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp) { } } } }
query 62834 "QR ColCaption" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { Caption = 'The Id'; } column(Grp; Grp) { } } } }
query 62835 "QR ColDescr" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { Description = 'x'; } column(Grp; Grp) { } } } }
query 62836 "QR Code20" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Id; Id) { } column(Grp; Grp20) { } } } }
query 62837 "QR InnerRoot" { QueryType = Normal; elements { dataitem(Row; "QR Row") { SqlJoinType = InnerJoin; column(Id; Id) { } column(Grp; Grp) { } } } }
query 62838 "QR Swapped" { QueryType = Normal; elements { dataitem(Row; "QR Row") { column(Grp; Grp) { } column(Id; Id) { } } } }
"#;

/// Which outcome a case must have.
#[derive(Clone, Copy, Debug)]
enum Want {
    /// Runs to its end (each case checks its rows itself).
    Pass,
    /// Refuses a call after the pinned second Open.
    Reopen,
    /// Keeps main's refusal of the second Open.
    Main,
}

/// The pinned start: cleared, Open, one Read whose result is used, one read of the Integer column (:42, :165-166, :69-70).
const PRE: &str = "Seed(); Q.Open(); if Q.Read() then Rows := Format(Q.Id);";
/// The pinned end after the second Open (:169-171, :179).
const END: &str = "Q.SetRange(Grp, 'B'); Q.Open(); ReadAll(); Check('3,');";

/// Runs each case as one test procedure of one codeunit and checks its outcome.
fn check(globals: &str, cases: &[(&str, String, Want)]) {
    let mut procs = String::new();
    for (name, body, _) in cases {
        procs.push_str(&format!(
            "[Test] procedure {name}() var R: Record \"QR Row\"; L: Query \"QR Rows\"; begin {body} end;\n"
        ));
    }
    let src = format!(
        r#"{OBJECTS}
codeunit 62812 T
{{
    Subtype = Test;
    var Q: Query "QR Rows"; Q2: Query "QR Rows"; {globals} Rows: Text;

    local procedure Seed()
    var R: Record "QR Row"; C: Record "QR Child";
    begin
        R.DeleteAll(); C.DeleteAll();
        R.Id := 1; R.Grp := 'A'; R.Grp20 := 'A'; R.Insert();
        R.Id := 2; R.Grp := 'A'; R.Grp20 := 'A'; R.Insert();
        R.Id := 3; R.Grp := 'B'; R.Grp20 := 'B'; R.Insert();
        C.Id := 1; C.RowId := 1; C.Insert(); C.Id := 2; C.RowId := 2; C.Insert(); C.Id := 3; C.RowId := 3; C.Insert();
        Clear(Q);
        Rows := '';
    end;
    local procedure ReadAll() begin Rows := ''; while Q.Read() do Rows += Format(Q.Id) + ','; end;
    local procedure Check(Want: Text) begin if Rows <> Want then Error('rows %1', Rows); end;
    local procedure OpenIn(var P: Query "QR Rows") begin P.Open(); end;
    local procedure RangeIn(var P: Query "QR Rows") begin P.SetRange(Grp, 'B'); end;
    {procs}
}}"#
    );
    let outcomes = outcomes_in(&src, "T");
    let mut wrong = Vec::new();
    for (name, body, want) in cases {
        let o = &outcomes
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no outcome for {name}"))
            .1;
        let ok = match want {
            Want::Pass => *o == Outcome::Pass,
            Want::Reopen => matches!(o, Outcome::NotRun(why) if why.contains("opened again while rows were left")),
            Want::Main => matches!(o, Outcome::NotRun(why) if why.contains("already open and was read from")),
        };
        if !ok {
            wrong.push(format!("{name} ({body}) wants {want:?}, got {o:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn case(name: &str, body: String, want: Want) -> (&str, String, Want) {
    (name, body, want)
}

#[test]
fn the_pinned_shape_and_its_value_neighbors_run() {
    check(
        "",
        &[
            case("Pinned", format!("{PRE} Q.Open(); {END}"), Want::Pass),
            // Read with its result used in another form, then the column read (QueryClosedAfterLastRow.al:69-70).
            case(
                "ReadUsedOtherForm",
                format!("Seed(); Q.Open(); if not Q.Read() then Error('r1'); Rows := Format(Q.Id); Q.Open(); {END}"),
                Want::Pass,
            ),
            case(
                "OneRowLeft",
                "Seed(); R.Get(3); R.Delete(); Clear(Q); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); Q.SetRange(Grp, 'A'); Q.Open(); ReadAll(); Check('1,2,');".to_string(),
                Want::Pass,
            ),
            case(
                "NoRowMatches",
                format!("{PRE} Q.Open(); Q.SetRange(Grp, 'Z'); Q.Open(); ReadAll(); Check('');"),
                Want::Pass,
            ),
            case(
                "LowerCaseValue",
                format!("{PRE} Q.Open(); Q.SetRange(Grp, 'b'); Q.Open(); ReadAll(); Check('3,');"),
                Want::Pass,
            ),
        ],
    );
}

/// A query variable nothing used yet is the same as a cleared one.
#[test]
fn a_new_variable_runs_like_a_cleared_one() {
    let src = format!(
        r#"{OBJECTS}
codeunit 62812 T
{{
    Subtype = Test;
    var Q: Query "QR Rows"; Rows: Text;
    [Test] procedure New()
    var R: Record "QR Row";
    begin
        R.Id := 1; R.Grp := 'A'; R.Insert(); R.Id := 2; R.Grp := 'A'; R.Insert(); R.Id := 3; R.Grp := 'B'; R.Insert();
        Q.Open(); if Q.Read() then Rows := Format(Q.Id);
        Q.Open(); Q.SetRange(Grp, 'B'); Q.Open();
        Rows := ''; while Q.Read() do Rows += Format(Q.Id) + ',';
        if Rows <> '3,' then Error('rows %1', Rows);
    end;
}}"#
    );
    assert_eq!(outcomes_in(&src, "T")[0].1, Outcome::Pass);
}

#[test]
fn every_call_after_the_second_open_but_the_pinned_ones_refuses() {
    let after = |name, tail: &str| case(name, format!("{PRE} Q.Open(); {tail}"), Want::Reopen);
    check(
        "",
        &[
            after("Read", "if Q.Read() then;"),
            after("Column", "Rows := Format(Q.Id);"),
            after("Close", "Q.Close();"),
            after("SetFilter", "Q.SetFilter(Grp, 'B'); Q.Open();"),
            after("OpenAgain", "Q.Open();"),
            after("TopNumberOfRows", "Q.TopNumberOfRows(1); Q.Open();"),
            after("GetFilters", "Rows := Q.GetFilters();"),
            after(
                "SecurityFiltering",
                "Q.SecurityFiltering(SecurityFilter::Filtered); Q.Open();",
            ),
            after("ClearIt", "Clear(Q); Q.Open(); ReadAll();"),
            after("RangeFromTo", "Q.SetRange(Grp, 'A', 'B'); Q.Open();"),
            after("RangeRemoves", "Q.SetRange(Grp); Q.Open();"),
            after("RangeIntegerColumn", "Q.SetRange(Id, 2); Q.Open();"),
            after("RangeTwoColumns", "Q.SetRange(Grp, 'A'); Q.SetRange(Id, 2); Q.Open();"),
            after(
                "RangeSameColumnTwice",
                "Q.SetRange(Grp, 'B'); Q.SetRange(Grp, 'A'); Q.Open();",
            ),
            after("RangeThroughVar", "RangeIn(Q); Q.Open();"),
            after("ThirdOpenResultUsed", "Q.SetRange(Grp, 'B'); if Q.Open() then;"),
            after("RangeThenRead", "Q.SetRange(Grp, 'B'); if Q.Read() then;"),
            after(
                "RangeThenSetFilter",
                "Q.SetRange(Grp, 'B'); Q.SetFilter(Id, '>0'); Q.Open();",
            ),
            after("RangeThenClose", "Q.SetRange(Grp, 'B'); Q.Close();"),
            after("RangeThenColumn", "Q.SetRange(Grp, 'B'); Rows := Format(Q.Id);"),
            after(
                "WriteThenRange",
                "R.Id := 4; R.Grp := 'B'; R.Insert(); Q.SetRange(Grp, 'B'); Q.Open();",
            ),
            after(
                "RangeWriteOpen",
                "Q.SetRange(Grp, 'B'); R.Id := 4; R.Grp := 'B'; R.Insert(); Q.Open();",
            ),
            after("CommitThenRange", "Commit(); Q.SetRange(Grp, 'B'); Q.Open();"),
        ],
    );
}

#[test]
fn a_call_sequence_other_than_the_pin_before_the_second_open_keeps_mains_refusal() {
    let main = |name, body: String| case(name, body, Want::Main);
    check(
        "",
        &[
            main(
                "NoColumnRead",
                format!("Seed(); Q.Open(); if Q.Read() then Rows := 'x'; Q.Open(); {END}"),
            ),
            main("TwoColumnReads", format!("{PRE} Rows := Format(Q.Id); Q.Open(); {END}")),
            main(
                "CodeColumnRead",
                format!("Seed(); Q.Open(); if Q.Read() then Rows := Format(Q.Grp); Q.Open(); {END}"),
            ),
            main(
                "ReadAsStatement",
                format!("Seed(); Q.Open(); Q.Read(); Rows := Format(Q.Id); Q.Open(); {END}"),
            ),
            main("OpenResultUsed", format!("{PRE} if Q.Open() then; {END}")),
            main(
                "TwoReads",
                format!("Seed(); Q.Open(); if Q.Read() then; if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"),
            ),
            main(
                "WriteBefore",
                format!("{PRE} R.Id := 4; R.Grp := 'B'; R.Insert(); Q.Open(); {END}"),
            ),
            main("CommitBefore", format!("{PRE} Commit(); Q.Open(); {END}")),
            main(
                "FilterBefore",
                format!(
                    "Seed(); Q.SetRange(Grp, 'A'); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "FilterSetAndRemoved",
                format!(
                    "Seed(); Q.SetRange(Grp, 'A'); Q.SetRange(Grp); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "TopBefore",
                format!(
                    "Seed(); Q.TopNumberOfRows(2); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "SecurityFilteringBefore",
                format!(
                    "Seed(); Q.SecurityFiltering(SecurityFilter::Filtered); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "EarlierRun",
                format!(
                    "Seed(); Q.Open(); ReadAll(); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "EarlierClose",
                format!(
                    "Seed(); Q.Open(); if Q.Read() then; Q.Close(); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main(
                "NoRowLeft",
                format!(
                    "Seed(); R.Get(3); R.Delete(); R.Get(2); R.Delete(); Clear(Q); Q.Open(); if Q.Read() then Rows := Format(Q.Id); Q.Open(); {END}"
                ),
            ),
            main("SecondOpenThroughVar", format!("{PRE} OpenIn(Q); {END}")),
            main(
                "Copied",
                format!("{PRE} Q2 := Q; Q2.Open(); Q2.SetRange(Grp, 'B'); Q2.Open();"),
            ),
            main(
                "LocalVariable",
                "Seed(); L.Open(); if L.Read() then Rows := Format(L.Id); L.Open(); L.SetRange(Grp, 'B'); L.Open();"
                    .to_string(),
            ),
        ],
    );
}

#[test]
fn a_query_object_other_than_the_pinned_one_keeps_mains_refusal() {
    let variants = [
        ("Trig", "QR Trig"),
        ("Filt", "QR Filt"),
        ("Top", "QR Top"),
        ("Order", "QR Order"),
        ("Flow", "QR Flow"),
        ("Three", "QR Three"),
        ("VarProc", "QR VarProc"),
        ("Var", "QR Var"),
        ("Join", "QR Join"),
        ("Sum", "QR Sum"),
        ("ColFilter", "QR ColFilter"),
        ("FilterEl", "QR FilterEl"),
        ("ReadState", "QR ReadState"),
        ("Caption", "QR Caption"),
        ("ColCaption", "QR ColCaption"),
        ("ColDescr", "QR ColDescr"),
        ("Code20", "QR Code20"),
        ("InnerRoot", "QR InnerRoot"),
        ("Swapped", "QR Swapped"),
    ];
    let globals: String = variants.iter().map(|(v, q)| format!("V{v}: Query \"{q}\"; ")).collect();
    let names: Vec<String> = variants.iter().map(|(v, _)| format!("Query{v}")).collect();
    let cases: Vec<(&str, String, Want)> = variants
        .iter()
        .zip(&names)
        .map(|((v, _), name)| {
            let x = format!("V{v}");
            let body = format!(
                "Seed(); Clear({x}); {x}.Open(); if {x}.Read() then Rows := Format({x}.Id); {x}.Open(); {x}.SetRange(Grp, 'B'); {x}.Open();"
            );
            (name.as_str(), body, Want::Main)
        })
        .collect();
    check(&globals, &cases);
}

/// A test that ends right after the second Open raises nothing. The global keeps its state into the next test of the codeunit,
/// where the pinned end is not pinned (the table versions or the commit count changed): it refuses. Main ran it, as main refused the
/// second Open in the test before.
#[test]
fn the_next_test_refuses_to_continue_the_sequence() {
    check(
        "",
        &[
            case("EndsReopened", format!("{PRE} Q.Open();"), Want::Pass),
            case("NextTest", END.to_string(), Want::Reopen),
        ],
    );
}
