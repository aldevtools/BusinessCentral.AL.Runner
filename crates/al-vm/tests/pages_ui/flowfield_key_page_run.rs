//! Page.Run with a record keyed on a FlowField whose value depends on the row: the order the page shows is not pinned
//! (records/flowfield_key.rs builds it for the record's own finds only), so the page refuses to open. The Base App does this
//! in ActivitiesMgt.DrillDownOverduePurchaseInvoiceAmount (Vendor Ledger Entry by "Remaining Amt. (LCY)").

use al_builtins::native::Signal;

const SRC: &str = r#"
table 51520 "FKP Line" { fields { field(1; Id; Integer) { } field(2; Owner; Integer) { } } keys { key(PK; Id) { } } }
table 51521 "FKP Owner"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("FKP Line" where(Owner = field(Id))); }
    }
    keys { key(PK; Id) { } }
}
page 51522 "FKP List"
{
    PageType = List;
    SourceTable = "FKP Owner";
    layout { area(Content) { repeater(R) { field(IdF; Rec.Id) { } } } }
}
codeunit 51523 "FKP Probe"
{
    procedure Open(): Integer
    var O: Record "FKP Owner"; TP: TestPage "FKP List";
    begin
        O.Id := 1; O.Insert(); O.Id := 2; O.Insert();
        O.SetCurrentKey(Cnt);
        TP.Trap();
        Page.Run(Page::"FKP List", O);
        TP.First();
        exit(TP.IdF.AsInteger());
    end;
}
"#;

#[test]
fn a_page_run_on_a_record_keyed_on_a_flowfield_is_refused() {
    let r = al_testkit::run_in(SRC, "FKP Probe", "Open");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("FlowField")),
        "{r:?}"
    );
}
