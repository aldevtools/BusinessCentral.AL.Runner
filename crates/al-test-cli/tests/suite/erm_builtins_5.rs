//! Queries that kept about 250 Tests-ERM tests from running (issue 24): the property `AboutText`
//! (and `AboutTitle`), and a `DataItemLink` to a data item that is not the enclosing one.

use al_sema::workspace::AppSource;

const OBJECTS: &str = r#"
table 63000 "Q5 Head" { fields { field(1; Code; Code[20]) { } field(2; Kind; Integer) { } } keys { key(PK; Code) { } } }
table 63001 "Q5 Mid" { fields { field(1; Id; Integer) { } field(2; Code; Code[20]) { } } keys { key(PK; Id) { } } }
table 63002 "Q5 Leaf" { fields { field(1; Id; Integer) { } field(2; Code; Code[20]) { } } keys { key(PK; Id) { } } }
query 63010 "Q5 About"
{
    Caption = 'About';
    AboutTitle = 'Title';
    AboutText = 'Text';
    elements { dataitem(Head; "Q5 Head") { column(Code; Code) { } } }
}
query 63011 "Q5 Outer Link"
{
    elements
    {
        dataitem(Head; "Q5 Head")
        {
            column(Code; Code) { }
            dataitem(Mid; "Q5 Mid")
            {
                SqlJoinType = CrossJoin;
                column(MidId; Id) { }
                dataitem(Leaf; "Q5 Leaf")
                {
                    DataItemLink = Code = Head.Code;
                    column(LeafId; Id) { }
                }
            }
        }
    }
}
codeunit 63012 C
{
    procedure Open(): Boolean var Q: Query "Q5 About"; begin exit(Q.Open()); end;
    procedure OpenOuter(): Boolean var Q: Query "Q5 Outer Link"; begin exit(Q.Open()); end;
}
"#;

fn refusal(routine: &str) -> String {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), OBJECTS.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    l.unsupported[oi][ri].first().cloned().unwrap_or_default()
}

// AboutTitle and AboutText are help texts of the object; they change neither the rows nor their order
// (docs/probes-needed.md, issue 24). Not in any corpus test, like the other texts in
// `HARMLESS_QUERY_PROPERTIES`.
#[test]
fn the_about_properties_of_a_query_do_not_stop_it() {
    assert_eq!(refusal("Open"), "");
}

// A link to an outer data item joins on that item's row since the probe pinned it (issue 38: corpus branch
// al-runtime-rs-staging, query/PXQueryOuterLink.al); it no longer stops the query.
#[test]
fn a_link_to_an_outer_data_item_lowers() {
    assert_eq!(refusal("OpenOuter"), "");
}
