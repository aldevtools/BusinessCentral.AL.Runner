//! A Page variable the test client does not show (it never ran): Activate answers Yes, Update(true) and SaveRecord do nothing,
//! SetSelectionFilter filters on the blank key, and its procedures read the row SetRecord gave and the filters SetTableView gave.
//! corpus pages/run-modal/P82PageVar.al (BeforeRun_Members) and pages/currpage/UnopenedCurrPage.al.

use al_testkit::text;

const SRC: &str = r#"
table 50300 "UPV Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

page 50301 "UPV List"
{
    PageType = List;
    SourceTable = "UPV Row";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(KeyCtl; Rec.Id) { }
            }
        }
    }

    procedure Act(Op: Text) Log: Text
    var
        Sel: Record "UPV Row";
    begin
        Log := 'start';
        case Op of
            'update-true':
                CurrPage.Update(true);
            'saverecord':
                CurrPage.SaveRecord();
            'activate':
                Log += ' ' + Format(CurrPage.Activate(true));
            'selection':
                begin
                    CurrPage.SetSelectionFilter(Sel);
                    Log += ' [' + Sel.GetFilters() + '] ' + Format(Sel.Count());
                end;
            'rec':
                Log += ' [' + Format(Rec.Id) + '] [' + Rec.GetFilters() + ']';
        end;
        Log += ' end';
    end;
}

codeunit 50302 "UPV Test"
{
    procedure Members(): Text
    var
        Row: Record "UPV Row";
        P: Page "UPV List";
        Log: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        Log := Format(P.Activate()) + '|' + P.Act('activate') + '|' + P.Act('update-true') + '|' + P.Act('saverecord');
        exit(Log + '|' + P.Act('selection'));
    end;

    procedure GivenRecord(): Text
    var
        Row: Record "UPV Row";
        P: Page "UPV List";
        Q: Page "UPV List";
    begin
        Row.Id := 2;
        Row.Insert();
        P.SetRecord(Row);
        Row.SetRange(Id, 2);
        Q.SetTableView(Row);
        exit(P.Act('rec') + '|' + Q.Act('rec'));
    end;
}
"#;

#[test]
fn members_of_a_page_variable_that_never_ran() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "UPV Test", "Members")),
        "Yes|start Yes end|start end|start end|start [Id: 0] 0 end"
    );
}

#[test]
fn procedures_of_a_page_variable_that_never_ran_read_what_it_was_given() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "UPV Test", "GivenRecord")),
        "start [2] [] end|start [0] [Id: 2] end"
    );
}
