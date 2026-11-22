//! A filter() element and a filter on a column over a Lookup, Count or Exist FlowField of a left-outer-joined dataitem one level
//! below the top dataitem's child: the joined rows are filtered by the value calculated per row, and a row without a match meets
//! the filter with the field's default. Every expected row list is what real BC answered in the corpus:
//! queries/flowfields-filters/QueryOuterFlowBelowJoin.al (codeunit 75097, al-runner issue 7401). The corpus cases also read
//! GetFilters, whose caption BC takes from another field (refused); these tests read the rows only.

use al_builtins::native::Signal;
use al_testkit::text;

const TABLES: &str = r#"
table 50296 Contact
{
    fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50297 Line
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; "Contact No."; Code[10]) { }
        field(3; "Header No."; Code[10]) { }
        field(4; "Hdr Customer"; Code[10]) { FieldClass = FlowField; CalcFormula = lookup(Header."Customer No." where("No." = field("Header No."))); }
    }
    keys { key(PK; "Line No.") { Clustered = true; } }
}

table 50298 Header
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; "Customer No."; Code[10]) { }
        field(10; "Customer Name"; Text[30]) { FieldClass = FlowField; CalcFormula = lookup(Customer.Name where("No." = field("Customer No."))); }
        field(11; "Line Count"; Integer) { FieldClass = FlowField; CalcFormula = count(Line where("Header No." = field("No."))); }
        field(12; "Has Customer"; Boolean) { FieldClass = FlowField; CalcFormula = exist(Customer where("No." = field("Customer No."))); }
        field(13; "Min Line"; Integer) { FieldClass = FlowField; CalcFormula = min(Line."Line No." where("Header No." = field("No."))); }
        field(14; "Customer Amount"; Decimal) { FieldClass = FlowField; CalcFormula = lookup(Customer.Amount where("No." = field("Customer No."))); }
        field(15; "Line One Count"; Integer) { FieldClass = FlowField; CalcFormula = count(Line where("Header No." = field("No."), "Line No." = const(1))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50299 Customer
{
    fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } field(3; Amount; Decimal) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
"#;

const QUERIES: &str = r#"
query 50296 InnerOuterFilter
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    filter(Hdr_Customer_Name; "Customer Name") { }
                    filter(Hdr_Line_Count; "Line Count") { }
                }
            }
        }
    }
}

query 50297 OuterOuterColumn
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = LeftOuterJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    column(Hdr_Customer_Name; "Customer Name") { }
                    column(Hdr_Has_Customer; "Has Customer") { }
                }
            }
        }
    }
}

query 50298 MinBelowJoin
{
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    filter(Hdr_Min_Line; "Min Line") { }
                }
            }
        }
    }
}

query 50299 OrderByFlagged
{
    OrderBy = ascending(Hdr_Customer_Name);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = LeftOuterJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    column(Hdr_Customer_Name; "Customer Name") { }
                }
            }
        }
    }
}

query 50300 TopProperty
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    TopNumberOfRows = 2;
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    filter(Hdr_Customer_Name; "Customer Name") { }
                }
            }
        }
    }
}

query 50301 OtherKinds
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    filter(Hdr_Customer_Amount; "Customer Amount") { }
                }
            }
        }
    }
}

query 50302 ConstCount
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    filter(Hdr_Line_One_Count; "Line One Count") { }
                }
            }
        }
    }
}

query 50303 ChildFlow
{
    OrderBy = ascending(ContactNo), ascending(LineNo);
    elements
    {
        dataitem(Contact; Contact)
        {
            column(ContactNo; "No.") { }
            dataitem(Line; Line)
            {
                DataItemLink = "Contact No." = Contact."No.";
                SqlJoinType = InnerJoin;
                column(LineNo; "Line No.") { }
                column(LineCustomer; "Hdr Customer") { }
                dataitem(Hdr; Header)
                {
                    DataItemLink = "No." = Line."Header No.";
                    SqlJoinType = LeftOuterJoin;
                    column(HdrNo; "No.") { }
                    filter(Hdr_Customer_Name; "Customer Name") { }
                }
            }
        }
    }
}
"#;

const CODE: &str = r#"
codeunit 50296 C
{
    local procedure Seed()
    var
        Contact: Record Contact;
        Customer: Record Customer;
    begin
        Contact."No." := 'C1'; Contact.Insert();
        Contact."No." := 'C2'; Contact.Insert();
        Contact."No." := 'C3'; Contact.Insert();
        AddLine(1, 'C1', 'H1');
        AddLine(2, 'C1', 'H2');
        AddLine(3, 'C1', 'H9');
        AddLine(4, 'C2', 'H3');
        AddHeader('H1', 'K1');
        AddHeader('H2', 'K2');
        AddHeader('H3', 'K9');
        AddHeader('H4', 'K1');
        Customer."No." := 'K1'; Customer.Name := 'Alpha'; Customer.Insert();
        Customer."No." := 'K2'; Customer.Name := 'Beta'; Customer.Insert();
    end;

    local procedure AddLine(LineNo: Integer; ContactNo: Code[10]; HeaderNo: Code[10])
    var
        Line: Record Line;
    begin
        Line."Line No." := LineNo; Line."Contact No." := ContactNo; Line."Header No." := HeaderNo; Line.Insert();
    end;

    local procedure AddHeader(No: Code[10]; CustomerNo: Code[10])
    var
        Header: Record Header;
    begin
        Header."No." := No; Header."Customer No." := CustomerNo; Header.Insert();
    end;

    local procedure FilterRows(Step: Integer): Text
    var
        Q: Query InnerOuterFilter;
        Rows: Text;
    begin
        case Step of
            2: Q.SetFilter(Hdr_Customer_Name, 'Alpha');
            3: Q.SetRange(Hdr_Customer_Name, '');
            7: Q.SetRange(Hdr_Line_Count, 0);
        end;
        Q.Open();
        while Q.Read() do
            Rows += Q.ContactNo + '/' + Format(Q.LineNo) + '/' + Q.HdrNo + ' ';
        exit(Rows);
    end;

    procedure FilterElements(): Text
    begin
        Seed();
        exit(FilterRows(1) + '|' + FilterRows(2) + '|' + FilterRows(3) + '|' + FilterRows(7));
    end;

    local procedure ColumnRows(Step: Integer): Text
    var
        Q: Query OuterOuterColumn;
        Rows: Text;
    begin
        case Step of
            1: ;
            3: Q.SetRange(Hdr_Customer_Name, '');
            10: Q.SetRange(Hdr_Has_Customer, false);
        end;
        Q.Open();
        while Q.Read() do
            Rows += Q.ContactNo + '/' + Format(Q.LineNo) + '/' + Q.HdrNo + '[' + Q.Hdr_Customer_Name + '|' + Format(Q.Hdr_Has_Customer) + '] ';
        exit(Rows);
    end;

    procedure Columns(): Text
    begin
        Seed();
        exit(ColumnRows(1) + '|' + ColumnRows(3) + '|' + ColumnRows(10));
    end;

    procedure Caption(): Text
    var
        Q: Query InnerOuterFilter;
    begin
        Q.SetFilter(Hdr_Customer_Name, 'Alpha');
        exit(Q.GetFilters());
    end;

    procedure MinFilter(): Text
    var
        Q: Query MinBelowJoin;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure OrderByFlagged(): Text
    var
        Q: Query OrderByFlagged;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure TopProperty(): Text
    var
        Q: Query TopProperty;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure TopCall(): Text
    var
        Q: Query InnerOuterFilter;
    begin
        Seed();
        Q.SetFilter(Hdr_Customer_Name, '<>Alpha');
        Q.TopNumberOfRows(2);
        Q.Open();
        Q.Read();
        exit('read');
    end;

    procedure OtherKinds(): Text
    var
        Q: Query OtherKinds;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure ConstCount(): Text
    var
        Q: Query ConstCount;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure ChildFlow(): Text
    var
        Q: Query ChildFlow;
    begin
        Q.Open();
        exit('opened');
    end;

    procedure GetFilterOne(): Text
    var
        Q: Query InnerOuterFilter;
    begin
        Q.SetFilter(Hdr_Customer_Name, '<>Alpha');
        exit(Q.GetFilter(Hdr_Customer_Name));
    end;
}
"#;

fn src() -> String {
    format!("{TABLES}{QUERIES}{CODE}")
}

/// InnerThenLeftOuter_FilterElements_NameFilters `no filter`, `name Alpha`, `name blank` and
/// InnerThenLeftOuter_FilterElements_CountAndExistFilters `count 0` (QueryOuterFlowBelowJoin.al:648-668).
#[test]
fn a_filter_element_below_the_join_filters_the_joined_rows_by_the_value_per_row() {
    assert_eq!(
        text(al_testkit::run(&src(), "FilterElements")),
        "C1/1/H1 C1/2/H2 C1/3/ C2/4/H3 |C1/1/H1 |C1/3/ C2/4/H3 |C1/3/ "
    );
}

/// LeftOuterThenLeftOuter_Columns_NameFilters `no filter`, `name blank` and _CountAndExistFilters `has customer false`
/// (QueryOuterFlowBelowJoin.al:747-773), with the Line Count column left out.
#[test]
fn a_filter_on_a_column_below_the_join_filters_the_joined_rows_by_the_value_per_row() {
    assert_eq!(
        text(al_testkit::run(&src(), "Columns")),
        "C1/1/H1[Alpha|Yes] C1/2/H2[Beta|Yes] C1/3/[|No] C2/4/H3[|No] C3/0/[|No] \
         |C1/3/[|No] C2/4/H3[|No] C3/0/[|No] \
         |C1/3/[|No] C2/4/H3[|No] C3/0/[|No] "
    );
}

/// GetFilters names such a filter by another field's caption in BC; the runtime refuses it.
#[test]
fn get_filters_on_a_filter_below_the_join_refuses() {
    match al_testkit::run(&src(), "Caption") {
        Err(Signal::Unsupported(why)) => assert!(why.to_string().contains("below the top dataitem's child")),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A Min FlowField one level below the join is not in the measured kinds: the query keeps main's refusal.
#[test]
fn a_min_flowfield_below_the_join_keeps_the_refusal() {
    match al_testkit::run(&src(), "MinFilter") {
        Err(Signal::Unsupported(why)) => assert!(why.to_string().contains("outer-joined dataitem"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Query facts the probe did not have keep main's refusal (QueryOuterFlowBelowJoin.al:99-281 has OrderBy on the top and the
/// child's columns, no TopNumberOfRows, a Text Lookup, a Count and an Exist whose CalcFormula links by field only, and no
/// FlowField column on the other dataitems; it reads GetFilters only): OrderBy on the flagged column, TopNumberOfRows as the
/// property and as a call, a Decimal Lookup, a Count with a const filter, a FlowField column on the child, GetFilter of the
/// column.
#[test]
fn query_facts_the_probe_did_not_have_keep_the_refusal() {
    let ran: Vec<String> = [
        "OrderByFlagged",
        "TopProperty",
        "OtherKinds",
        "ConstCount",
        "ChildFlow",
        "TopCall",
        "GetFilterOne",
    ]
    .into_iter()
    .filter_map(|name| match al_testkit::run(&src(), name) {
        Err(Signal::Unsupported(_)) => None,
        other => Some(format!("{name}: {other:?}")),
    })
    .collect();
    assert!(ran.is_empty(), "expected refusals: {ran:?}");
}
