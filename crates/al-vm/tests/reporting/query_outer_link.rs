//! A query DataItemLink to an outer data item (the grandparent), as probed on BC 27.5: corpus branch
//! al-runtime-rs-staging, tests/al-language/query/PXQueryOuterLink.al (objects :14-198, expectations :274-349).
//! The link joins on the named ancestor's current row, like a link to the parent joins on the parent's.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const OBJECTS: &str = r#"
table 69186 "PX Grand"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 69187 "PX Parent"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; GrandId; Integer) { }
        field(3; Amount; Decimal) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 69188 "PX Leaf"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; GrandId; Integer) { }
        field(3; ParentId; Integer) { }
        field(4; Amount; Decimal) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

query 69188 "PX Outer Inner"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }

            dataitem(Parent; "PX Parent")
            {
                DataItemLink = GrandId = Grand.Id;
                SqlJoinType = InnerJoin;

                column(ParentId; Id) { }

                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = GrandId = Grand.Id;
                    SqlJoinType = InnerJoin;

                    column(LeafId; Id) { }
                }
            }
        }
    }
}

query 69189 "PX Outer Left"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }

            dataitem(Parent; "PX Parent")
            {
                DataItemLink = GrandId = Grand.Id;
                SqlJoinType = LeftOuterJoin;

                column(ParentId; Id) { }

                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = GrandId = Grand.Id;
                    SqlJoinType = LeftOuterJoin;

                    column(LeafId; Id) { }
                }
            }
        }
    }
}

query 69190 "PX Outer Cross"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }

            dataitem(Parent; "PX Parent")
            {
                SqlJoinType = CrossJoin;

                column(ParentId; Id) { }

                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = GrandId = Grand.Id;
                    SqlJoinType = InnerJoin;

                    column(LeafId; Id) { }
                }
            }
        }
    }
}

query 69191 "PX Parent Link"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }

            dataitem(Parent; "PX Parent")
            {
                DataItemLink = GrandId = Grand.Id;
                SqlJoinType = InnerJoin;

                column(ParentId; Id) { }

                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = ParentId = Parent.Id;
                    SqlJoinType = InnerJoin;

                    column(LeafId; Id) { }
                }
            }
        }
    }
}

query 69192 "PX Outer Sum"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }

            dataitem(Parent; "PX Parent")
            {
                DataItemLink = GrandId = Grand.Id;
                SqlJoinType = InnerJoin;

                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = GrandId = Grand.Id;
                    SqlJoinType = InnerJoin;

                    column(LeafSum; Amount)
                    {
                        Method = Sum;
                    }
                }
            }
        }
    }
}

query 69193 "PX Mixed"
{
    elements
    {
        dataitem(Grand; "PX Grand")
        {
            column(GrandId; Id) { }
            dataitem(Parent; "PX Parent")
            {
                DataItemLink = GrandId = Grand.Id;
                SqlJoinType = InnerJoin;
                column(ParentId; Id) { }
                dataitem(Leaf; "PX Leaf")
                {
                    DataItemLink = GrandId = Grand.Id, ParentId = Parent.Id;
                    SqlJoinType = InnerJoin;
                    column(LeafId; Id) { }
                }
            }
        }
    }
}
"#;

const SEED: &str = "G.Id := 1; G.Amount := 10; G.Insert(); G.Id := 2; G.Amount := 20; G.Insert(); G.Id := 3; G.Amount := 30; G.Insert();
    P.Id := 11; P.GrandId := 1; P.Insert(); P.Id := 12; P.GrandId := 1; P.Insert(); P.Id := 21; P.GrandId := 2; P.Insert();
    L.Id := 101; L.GrandId := 1; L.ParentId := 11; L.Amount := 1; L.Insert();
    L.Id := 102; L.GrandId := 1; L.ParentId := 12; L.Amount := 2; L.Insert();
    L.Id := 103; L.GrandId := 2; L.ParentId := 21; L.Amount := 4; L.Insert();
    L.Id := 104; L.GrandId := 2; L.ParentId := 11; L.Amount := 8; L.Insert();";

/// The rows of query `q`, each as `[a,b,c]`, sorted as the probe sorts them.
fn rows(q: &str, cols: &[&str]) -> Result<String, Signal> {
    let read = cols
        .iter()
        .map(|c| format!("Format(Q.{c})"))
        .collect::<Vec<_>>()
        .join(" + ',' + ");
    let body = format!(
        "procedure F(): Text var G: Record \"PX Grand\"; P: Record \"PX Parent\"; L: Record \"PX Leaf\"; Q: Query \"{q}\"; s: Text;
         begin {SEED} Q.Open(); while Q.Read() do s += '[' + {read} + ']|'; exit(s); end;"
    );
    let src = format!("{OBJECTS}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked = al_testkit::blocked(&l);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.first()) {
        return Err(Signal::Unsupported(r.clone()));
    }
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    let v = s.call(oi, 0, inst, vec![]).map(|(v, _)| v)?;
    let Value::Text(t) = v else { panic!("{v:?}") };
    let mut rows: Vec<&str> = t.split('|').filter(|r| !r.is_empty()).collect();
    rows.sort();
    Ok(rows.join(" "))
}

#[test]
fn a_link_to_the_grandparent_joins_on_its_row() {
    let three = ["GrandId", "ParentId", "LeafId"];
    // PXQueryOuterLink.al:274-290 (Inner), :290-305 (LeftOuter), :306-321 (CrossJoin), :322-337 (parent link).
    assert_eq!(
        rows("PX Outer Inner", &three).unwrap(),
        "[1,11,101] [1,11,102] [1,12,101] [1,12,102] [2,21,103] [2,21,104]"
    );
    assert_eq!(
        rows("PX Outer Left", &three).unwrap(),
        "[1,11,101] [1,11,102] [1,12,101] [1,12,102] [2,21,103] [2,21,104] [3,0,0]"
    );
    assert_eq!(
        rows("PX Outer Cross", &three).unwrap(),
        "[1,11,101] [1,11,102] [1,12,101] [1,12,102] [1,21,101] [1,21,102] [2,11,103] [2,11,104] [2,12,103] [2,12,104] [2,21,103] [2,21,104]"
    );
    assert_eq!(
        rows("PX Parent Link", &three).unwrap(),
        "[1,11,101] [1,11,104] [1,12,102] [2,21,103]"
    );
}

#[test]
fn a_sum_over_a_leaf_linked_to_the_grandparent_sums_the_pairs() {
    // PXQueryOuterLink.al:338-349.
    assert_eq!(rows("PX Outer Sum", &["GrandId", "LeafSum"]).unwrap(), "[1,6] [2,12]");
}

// Base Application's "Analysis View Source" links one data item to the grandparent and to the parent in one
// DataItemLink. Each equality reads the row of the item it names (an assumption: the probe links to one item,
// docs/probes-needed.md). Leaf 104 (grand 2, parent 11) matches no pair.
#[test]
fn a_link_naming_the_parent_and_the_grandparent_reads_each() {
    assert_eq!(
        rows("PX Mixed", &["GrandId", "ParentId", "LeafId"]).unwrap(),
        "[1,11,101] [1,12,102] [2,21,103]"
    );
}

/// `DataItemTableFilter = "Code" = filter('')` selects the rows whose Code is blank, as `const('')` does
/// (corpus query/TestQueryDataItemTableFilter.al:374 Sq_EmptyFilterTerm_Rows, query 89590).
#[test]
fn a_table_filter_of_an_empty_term_selects_blank_rows() {
    let src = r#"
table 69970 "QE Row" { fields { field(1; Id; Integer) { } field(2; "Code"; Code[10]) { } } keys { key(PK; Id) { } } }
query 69971 "QE Q"
{
    QueryType = Normal; OrderBy = ascending(RowId);
    elements { dataitem(Row; "QE Row") { DataItemTableFilter = "Code" = filter(''); column(RowId; Id) { } } }
}
codeunit 69972 C
{
    procedure Go(): Text
    var R: Record "QE Row"; Q: Query "QE Q"; T: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R."Code" := 'A'; R.Insert(); R.Id := 3; R."Code" := ''; R.Insert();
        Q.Open();
        while Q.Read() do T += Format(Q.RowId) + ',';
        exit(T);
    end;
}
"#;
    assert_eq!(al_testkit::text(al_testkit::run(src, "Go")), "1,3,");
}
