//! A link term `filter(<> '')` (al-runner issue 8873, assumption `link-filter-not-blank`, corpus PR 1548). The parser
//! marks only the two exact terms; the lowering takes them as "not blank" only on the shapes the probe measured: a
//! Normal Code or Text field, a SubPageLink of a ListPart and a RunPageLink of a List. Every other shape keeps main's
//! text.
use super::*;
use crate::pages::{LinkRhs, parse_sub_page_link_kinds};

fn rhs(text: &str) -> LinkRhs {
    parse_sub_page_link_kinds(text).unwrap().remove(0).1
}

#[test]
fn only_the_two_exact_terms_parse_as_not_blank() {
    assert_eq!(rhs("A = filter(<> '')"), LinkRhs::NotBlank("<> ".into()));
    assert_eq!(rhs("A = filter(<>'')"), LinkRhs::NotBlank("<>".into()));
    assert_eq!(rhs("A = filter(<>  '')"), LinkRhs::Filter("<>  ".into()));
    assert_eq!(rhs("A = const(<> '')"), LinkRhs::Filter("<> ".into()));
    assert_eq!(rhs("A = filter('<>''''')"), LinkRhs::Filter("<>''".into()));
    assert_eq!(rhs("A = filter(<> '' | 'A1')"), LinkRhs::Filter("<>  | A1".into()));
}

const SRC: &str = r#"
table 50100 H { fields { field(1; "No."; Code[10]) { } } }
table 50101 L
{
    fields
    {
        field(1; "Header No."; Code[10]) { }
        field(2; Arrival; Code[10]) { }
        field(3; "Arrival Filter"; Code[10]) { FieldClass = FlowFilter; }
        field(4; Lookup; Code[10]) { FieldClass = FlowField; CalcFormula = lookup(H."No." where("No." = field(Arrival))); }
    }
}
page 50100 LP { PageType = ListPart; SourceTable = L; layout { area(Content) { repeater(R) { field(A; Rec.Arrival) { } } } } }
page 50101 CP { PageType = CardPart; SourceTable = L; layout { area(Content) { field(A; Rec.Arrival) { } } } }
page 50102 LT { PageType = List; SourceTable = L; layout { area(Content) { repeater(R) { field(A; Rec.Arrival) { } } } } }
page 50103 CT { PageType = Card; SourceTable = L; layout { area(Content) { field(A; Rec.Arrival) { } } } }
page 50104 Host
{
    PageType = Card;
    SourceTable = H;
    layout
    {
        area(Content)
        {
            part(ListPartNormal; LP) { SubPageLink = Arrival = filter(<> ''); }
            part(CardPartNormal; CP) { SubPageLink = Arrival = filter(<> ''); }
            part(ListPartFlowFilter; LP) { SubPageLink = "Arrival Filter" = filter(<> ''); }
            part(ListPartFlowField; LP) { SubPageLink = Lookup = filter(<> ''); }
        }
    }
    actions
    {
        area(Processing)
        {
            action(RunList) { RunObject = page LT; RunPageLink = Arrival = filter(<> ''); }
            action(RunCard) { RunObject = page CT; RunPageLink = Arrival = filter(<> ''); }
        }
    }
}
"#;

/// The link of the control `name` of page `Host`: its entries and the indices lowered as "not blank".
fn link(name: &str) -> (Vec<ir::LinkEntry>, Vec<usize>) {
    let (_, l) = lower_src(SRC);
    let host = l.program.objects.iter().find(|o| o.name == "Host").unwrap();
    let c = host
        .page
        .as_ref()
        .unwrap()
        .controls
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    let pl = c
        .part_link
        .clone()
        .or_else(|| c.run_object.as_ref().and_then(|r| r.link.clone()))
        .unwrap();
    (pl.entries, pl.not_blank)
}

#[test]
fn the_measured_shapes_lower_to_not_blank() {
    for name in ["ListPartNormal", "RunList"] {
        let (entries, not_blank) = link(name);
        assert_eq!(not_blank, vec![0], "{name}");
        assert!(
            matches!(&entries[0], ir::LinkEntry::Filter(_, t) if t == "<>''"),
            "{name}: {entries:?}"
        );
    }
}

#[test]
fn every_other_page_type_and_field_class_keeps_mains_text() {
    for name in ["CardPartNormal", "ListPartFlowFilter", "ListPartFlowField", "RunCard"] {
        let (entries, not_blank) = link(name);
        assert!(not_blank.is_empty(), "{name}");
        assert!(
            matches!(&entries[0], ir::LinkEntry::Filter(_, t) if t == "<> "),
            "{name}: {entries:?}"
        );
    }
}
