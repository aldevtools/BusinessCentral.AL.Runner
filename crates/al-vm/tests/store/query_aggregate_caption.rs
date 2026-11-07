//! The caption of a totals column (Sum, Count, Average, Min, Max) without a Caption property is the column name, not the
//! caption of its source field: `ColumnCaption` and the `Caption: filter` pairs of `GetFilters`.
//! queries/CfQueryLeads.al:174-182 (Min, Max, Sum), :276 (Count);
//! queries/flowfields-filters/Y78QueryOuterAggFilter.al:1041 (`SumQ: >0`), the same file for `AvgQ`.

const SRC: &str = r#"
table 68960 "Probe AggCaption Table"
{
    fields { field(1; Id; Integer) { } field(2; Qty; Decimal) { Caption = 'Quantity Caption'; } }
    keys { key(PK; Id) { Clustered = true; } }
}

query 89960 Q
{
    QueryType = Normal;
    elements
    {
        dataitem(T; "Probe AggCaption Table")
        {
            column(Qty; Qty) { }
            column(SumQ; Qty) { Method = Sum; }
            column(AvgQ; Qty) { Method = Average; }
            column(MinQ; Qty) { Method = Min; }
            column(MaxQ; Qty) { Method = Max; }
            column(CntQ) { Method = Count; }
        }
    }
}

codeunit 50100 C
{
    procedure Captions(): Text
    var
        Q: Query Q;
    begin
        exit(Q.ColumnCaption(Qty) + '|' + Q.ColumnCaption(SumQ) + '|' + Q.ColumnCaption(AvgQ) + '|' + Q.ColumnCaption(MinQ) + '|' + Q.ColumnCaption(MaxQ) + '|' + Q.ColumnCaption(CntQ));
    end;

    procedure Filters(): Text
    var
        Q: Query Q;
    begin
        Q.SetFilter(Qty, '>0');
        Q.SetFilter(SumQ, '>0');
        Q.SetFilter(MinQ, '<10');
        exit(Q.GetFilters());
    end;
}
"#;

#[test]
fn a_totals_column_is_captioned_by_its_name() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Captions")),
        "Quantity Caption|SumQ|AvgQ|MinQ|MaxQ|CntQ"
    );
}

#[test]
fn get_filters_names_a_totals_column_by_its_name() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Filters")),
        "Quantity Caption: >0, SumQ: >0, MinQ: <10"
    );
}
