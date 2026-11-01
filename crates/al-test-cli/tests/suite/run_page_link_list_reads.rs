//! What a List page reads when an action's RunPageLink opens it on one row and a page handler walks it with First/Next
//! (misc/lt-probes-needed-7/PnGRunObjectOptions.al:111 LinkAndViewOnAList, codeunit 70188).

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 62750 "RL Item"
{
    fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } }
}
codeunit 62751 "RL Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ';'; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
}
page 62752 "RL List"
{
    PageType = List; SourceTable = "RL Item";
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } } }
    var Log: Codeunit "RL Log";
    trigger OnOpenPage() begin Log.Add('OnOpenPage'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('OnNewRecord'); end;
    trigger OnAfterGetRecord() begin Log.Add('AGR(' + Rec."No." + ',' + Rec.Name + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('AGCR(' + Rec."No." + ',' + xRec."No." + ')'); end;
}
page 62753 "RL Host"
{
    PageType = Card; SourceTable = "RL Item";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions
    {
        area(Processing)
        {
            action(RunList) { RunObject = page "RL List"; }
            action(RunLink) { RunObject = page "RL List"; RunPageLink = "No." = field("No."); }
            action(RunView) { RunObject = page "RL List"; RunPageView = where(Qty = filter('>1')); }
        }
    }
}
codeunit 62754 T
{
    Subtype = Test;
    var Log: Codeunit "RL Log";

    local procedure Seed()
    var Item: Record "RL Item";
    begin
        Item.DeleteAll();
        Item."No." := 'RP1'; Item.Name := 'One'; Item.Qty := 1; Item.Insert();
        Item."No." := 'RP2'; Item.Name := 'Two'; Item.Qty := 2; Item.Insert();
        Item."No." := 'RP3'; Item.Name := 'Three'; Item.Qty := 3; Item.Insert();
    end;

    [Test]
    [HandlerFunctions('ListHandler')]
    procedure Run()
    var Host: TestPage "RL Host"; R: Text;
    begin
        Seed();
        Host.OpenView();
        Host.GoToKey('RP2');
        Log.Take();
        Host.__ACTION__.Invoke();
        R := Log.Take();
        Host.Close();
        if R <> '__EXPECT__' then
            Error('%1', R);
    end;

    [PageHandler]
    procedure ListHandler(var P: TestPage "RL List")
    var Rows: Text; More: Boolean;
    begin
        More := P.First();
        while More do begin
            Rows += P.NoCtl.Value() + ',';
            More := P.Next();
        end;
        Log.Add('List(' + Rows + ')');
    end;
}
"#;

fn run(action: &str, expect: &str) {
    let src = SRC.replace("__ACTION__", action).replace("__EXPECT__", expect);
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

/// PnGRunObjectOptions.al:129, observation `plain-list`.
#[test]
fn a_plain_list_reads_each_row_twice_when_the_handler_walks_it() {
    run(
        "RunList",
        "OnOpenPage;AGR(RP1,One);AGR(RP2,Two);AGR(RP3,Three);AGR(RP1,One);AGR(RP1,One);AGCR(RP1,);AGR(RP2,Two);AGR(RP2,Two);AGCR(RP2,RP1);AGR(RP3,Three);AGR(RP3,Three);AGCR(RP3,RP2);OnNewRecord;AGCR(,);List(RP1,RP2,RP3,,)",
    );
}

/// PnGRunObjectOptions.al:130, observation `link`: the list opened on the one linked row reads it once at open, and First
/// reads it once more before OnAfterGetCurrRecord.
#[test]
fn a_linked_list_reads_its_one_row_twice_before_the_current_record() {
    run(
        "RunLink",
        "OnOpenPage;AGR(RP2,Two);AGR(RP2,Two);AGCR(RP2,RP2);OnNewRecord;AGCR(RP2,RP2);List(RP2,RP2,)",
    );
}

/// PnGRunObjectOptions.al:131, observation `view`.
#[test]
fn a_viewed_list_reads_each_row_twice_when_the_handler_walks_it() {
    run(
        "RunView",
        "OnOpenPage;AGR(RP2,Two);AGR(RP3,Three);AGR(RP2,Two);AGR(RP2,Two);AGCR(RP2,);AGR(RP3,Three);AGR(RP3,Three);AGCR(RP3,RP2);OnNewRecord;AGCR(,);List(RP2,RP3,,)",
    );
}
