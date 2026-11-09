//! Query.GetFilters with two filter elements, with columns whose column order differs from their
//! field order, and with filters on two dataitems (issue 7395).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

/// The table of queries/PnAQueryObjects.al:9-25 and query "LT PW Q Mixed" (:85-99).
const MIXED: &str = r#"
table 70123 "PnA Src"
{
    fields
    {
        field(1; Entry; Integer) { }
        field(2; Grp; Code[10]) { }
        field(3; Name; Text[30]) { }
        field(4; Qty; Integer) { }
        field(5; Amt; Decimal) { }
    }
    keys { key(PK; Entry) { Clustered = true; } }
}
query 86143 "PW Mixed"
{
    QueryType = Normal;
    elements
    {
        dataitem(S; "PnA Src")
        {
            filter(FGrp; Grp) { }
            column(Qty; Qty) { }
            column(Entry; Entry) { }
            filter(FAmt; Amt) { }
            column(Txt; Name) { }
        }
    }
}
query 86144 "PW Three"
{
    QueryType = Normal;
    elements
    {
        dataitem(S; "PnA Src")
        {
            filter(FGrp; Grp) { }
            filter(FAmt; Amt) { }
            filter(FQty; Qty) { }
            column(Entry; Entry) { }
        }
    }
}
"#;

/// The tables of queries/P133QueryOuterFilters.al:8-42 and query "LT PX Q Two Items" (:913-931).
const TWO_ITEMS: &str = r#"
table 74054 "P133 Parent"
{
    fields
    {
        field(1; DocNo; Code[20]) { }
        field(2; PostDate; Date) { }
    }
    keys { key(PK; DocNo, PostDate) { Clustered = true; } }
}
table 74055 "P133 Child"
{
    fields
    {
        field(1; EntryNo; Integer) { }
        field(2; DocNo; Code[20]) { }
        field(3; PostDate; Date) { }
        field(4; Kind; Integer) { }
    }
    keys { key(PK; EntryNo) { Clustered = true; } }
}
query 86174 "PX Two Items"
{
    QueryType = Normal;
    elements
    {
        dataitem(Parent; "P133 Parent")
        {
            column(DocNo; DocNo) { }
            filter(FPost; PostDate) { }
            dataitem(Child; "P133 Child")
            {
                DataItemLink = DocNo = Parent.DocNo, PostDate = Parent.PostDate;
                filter(FKind; Kind) { }
                column(EntryNo; EntryNo) { }
            }
        }
    }
}
"#;

fn run(objects: &str, body: &str) -> Result<Value, Signal> {
    let src = format!("{objects}\ncodeunit 50100 C {{ {body} }}");
    al_testkit::run_in(&src, "C", "F")
}

// record/PnAQueryFiltersColumnNo.al:263-342 (PW_GetFilters_Order_Mixed): two filter elements keep
// the order they were set in, the columns come in column order, not field order.
#[test]
fn get_filters_with_two_filter_elements_and_columns_out_of_field_order() {
    let r = run(
        MIXED,
        "procedure F(): Text
        var M: Query \"PW Mixed\"; s: Text;
        begin
            M.SetRange(Entry, 1); M.SetRange(Qty, 3); M.SetRange(FGrp, 'G');
            s := M.GetFilters() + ';';
            Clear(M);
            M.SetRange(FAmt, 1.5); M.SetRange(FGrp, 'G');
            s += M.GetFilters() + ';';
            Clear(M);
            M.SetRange(Entry, 1); M.SetRange(FAmt, 1.5);
            s += M.GetFilters() + ';';
            Clear(M);
            M.SetRange(Txt, 'a'); M.SetRange(FAmt, 1.5); M.SetRange(Qty, 3);
            s += M.GetFilters() + ';';
            Clear(M);
            M.SetRange(FAmt, 1.5); M.SetRange(Txt, 'a'); M.SetRange(FGrp, 'G'); M.SetRange(Entry, 1); M.SetRange(Qty, 3);
            s += M.GetFilters();
            exit(s);
        end;",
    );
    assert_eq!(
        text(r),
        "Qty: 3, Grp: G, Entry: 1;Amt: 1.5, Grp: G;Amt: 1.5, Entry: 1;Amt: 1.5, Qty: 3, Name: a;\
         Amt: 1.5, Grp: G, Qty: 3, Entry: 1, Name: a"
    );
}

// queries/P133QueryOuterFilters.al:657-718 (PX_Query_GetFilters_TwoDataItems_FilterElements_*):
// filter elements on two dataitems.
#[test]
fn get_filters_with_filter_elements_on_two_dataitems() {
    let r = run(
        TWO_ITEMS,
        "procedure F(): Text
        var Q: Query \"PX Two Items\"; s: Text;
        begin
            Q.SetRange(DocNo, 'D1'); Q.SetRange(FPost, DMY2Date(1, 1, 2024)); Q.SetRange(EntryNo, 2); Q.SetRange(FKind, 1);
            s := Q.GetFilters() + ';';
            Clear(Q);
            Q.SetRange(EntryNo, 2); Q.SetRange(FKind, 1); Q.SetRange(DocNo, 'D1'); Q.SetRange(FPost, DMY2Date(1, 1, 2024));
            s += Q.GetFilters() + ';';
            Clear(Q);
            Q.SetRange(FPost, DMY2Date(1, 1, 2024)); Q.SetRange(EntryNo, 2); Q.SetRange(FKind, 1); Q.SetRange(DocNo, 'D1');
            s += Q.GetFilters();
            exit(s);
        end;",
    );
    assert_eq!(
        text(r),
        "DocNo: D1, PostDate: 01/01/24, Kind: 1, EntryNo: 2;Kind: 1, DocNo: D1, PostDate: 01/01/24, EntryNo: 2;\
         PostDate: 01/01/24, Kind: 1, DocNo: D1, EntryNo: 2"
    );
}

// Three filter elements are not pinned.
#[test]
fn get_filters_with_three_filter_elements_is_refused() {
    let r = run(
        MIXED,
        "procedure F(): Text
        var Q: Query \"PW Three\";
        begin
            Q.SetRange(FGrp, 'G'); Q.SetRange(FAmt, 1.5); Q.SetRange(FQty, 3);
            exit(Q.GetFilters());
        end;",
    );
    match r {
        Err(Signal::Unsupported(s)) => assert!(s.contains("more than two filter elements"), "{s}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
